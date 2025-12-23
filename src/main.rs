use std::{path::{Path, PathBuf}, process::Command, fs, thread::sleep, time::Duration};
use humantime;
use clap::{Error, Parser};
use clap::error::ErrorKind;
use candle_core::{DType, Device, Tensor, Module, IndexOp};
use candle_transformers::models::stable_diffusion;
use tokenizers::Tokenizer;


#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, value_parser = validate_file)]
    file: Option<PathBuf>,
    #[arg(short, long, value_parser = validate_dir, default_value = ".")]
    directory: Option<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration, default_value = "3600s")]
    period: Option<Duration>,
    #[arg(long, default_value = "")]
    prompt: String,
    #[arg(long, default_value = "output.png", help = "Filename for generated image")]
    output: String
}

const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

fn validate_dir(dir: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(dir);
    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display())
        ))
    }

    if !path.is_dir() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a directory", path.display())
        ))
    }

    Ok(path)
}

fn validate_file(file: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(file);

    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display())
        ))
    }

    if !path.is_file() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a file", path.display())
        ))
    }

    let ext = path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| Error::raw(
            ErrorKind::ValueValidation,
            "could not read extension".to_string()
        ))?;

    if !EXTENSIONS.contains(&ext) {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            "file is not supported"));
    }

    Ok(path)
}

fn is_valid_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}


fn set_wallpaper(path: &Path) {
    /*
    ref: https://github.com/KDE/plasma-workspace/blob/master/wallpapers/image/plasma-apply-wallpaperimage.cpp
    plasma's own wrapper
    initial logic: just use daemon for applying wallpaper
     */
    let script = format!("for (var key in desktops()) {{ \
    var d = desktops()[key]; \
    d.wallpaperPlugin = 'org.kde.image'; \
    d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General']; \
    d.writeConfig('Image', 'file://{}'); \
    d.writeConfig('FillMode', 2); }}", path.to_str().expect("Invalid path"));

    let output = Command::new("qdbus6")
        .arg("org.kde.plasmashell")
        .arg("/PlasmaShell")
        .arg("org.kde.PlasmaShell.evaluateScript")
        .arg(&script)
        .output()
        .expect("qdbus command failed!");

    if !output.status.success() {
        println!("failed to set wallpaper!");
        println!("{:?}", output);
    }
}

fn generate_image(prompt: String, output: String) -> anyhow::Result<(String)> {
    /*
    ref: https://github.com/huggingface/candle/blob/main/candle-examples/examples/stable-diffusion/main.rs
     */
    println!("Generating image...");
    let device = Device::new_cuda(0)?;
    let dtype = DType::F16;

    let n_steps = 30;
    let guidance_scale = 7.5;

    let api = hf_hub::api::sync::Api::new()?;
    let repo = api.model("runwayml/stable-diffusion-v1-5".to_string());

    let tokenizer_path = api.model("openai/clip-vit-base-patch32".to_string()).get("tokenizer.json")?;
    let unet_weights = repo.get("unet/diffusion_pytorch_model.fp16.safetensors")?;
    let vae_weights = repo.get("vae/diffusion_pytorch_model.fp16.safetensors")?;
    let clip_weights = repo.get("text_encoder/model.fp16.safetensors")?;

    let sd_config = stable_diffusion::StableDiffusionConfig::v1_5(None, Some(512), Some(512));
    let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;
    let mut scheduler = sd_config.build_scheduler(n_steps)?;

    let unet = sd_config.build_unet(unet_weights, &device, 4, false, dtype)?;
    let vae = sd_config.build_vae(vae_weights, &device, dtype)?;
    let clip = stable_diffusion::build_clip_transformer(&sd_config.clip, clip_weights, &device, dtype)?;

    let mut tokens = tokenizer
        .encode(prompt, true)
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    let pad_id = *tokenizer
        .get_vocab(true)
        .get("<|endoftext|>")
        .unwrap_or(&49407);

    if tokens.len() > 77 {
        tokens.truncate(77);
    } else {
        while tokens.len() < 77 {
            tokens.push(pad_id);
        }
    }

    let tokens = Tensor::new(tokens.as_slice(), &device)?.unsqueeze(0)?;
    let text_embeddings = clip.forward(&tokens)?;

    // --- Do the same for the Negative Prompt (Unconditional) ---
    let mut uncond_tokens = tokenizer
        .encode("", true) // Empty prompt
        .map_err(anyhow::Error::msg)?
        .get_ids()
        .to_vec();

    while uncond_tokens.len() < 77 {
        uncond_tokens.push(pad_id);
    }

    let uncond_tokens = Tensor::new(uncond_tokens.as_slice(), &device)?.unsqueeze(0)?;
    let uncond_embeddings = clip.forward(&uncond_tokens)?;

    let text_embeddings = Tensor::cat(&[uncond_embeddings, text_embeddings], 0)?;

    let mut latents = Tensor::randn(0f32, 1f32, (1, 4, 64, 64), &device)?.to_dtype(dtype)?;
    latents = (latents * scheduler.init_noise_sigma())?;

    let timesteps = scheduler.timesteps().to_vec();
    for (i, &t) in timesteps.iter().enumerate() {
        let start_time = std::time::Instant::now();

        let latent_model_input = Tensor::cat(&[&latents, &latents], 0)?;
        let latent_model_input = scheduler.scale_model_input(latent_model_input, t)?;

        let noise_pred = unet.forward(&latent_model_input, t as f64, &text_embeddings)?;

        let noise_pred_chunks = noise_pred.chunk(2, 0)?;
        let (noise_uncond, noise_text) = (&noise_pred_chunks[0], &noise_pred_chunks[1]);
        let noise_pred = (noise_uncond + ((noise_text - noise_uncond)? * guidance_scale)?)?;

        latents = scheduler.step(&noise_pred, t, &latents)?;

        let dt = start_time.elapsed().as_secs_f32();
        println!("step {}/{n_steps} done, {:.2}s", i + 1, dt);
    }

    let image = vae.decode(&(latents / 0.18215)?)?;
    let image = ((image / 2.)? + 0.5)?.clamp(0f32, 1.)?;
    let image = (image * 255.)?.to_device(&Device::Cpu)?.to_dtype(DType::U8)?;

    let image = image.i(0)?;

    let image = image.permute((1, 2, 0))?;

    let (height, width, _channels) = image.dims3()?;
    let raw_data = image.flatten_all()?.to_vec1::<u8>()?;

    image::save_buffer(
        &output,
        &raw_data,
        width as u32,
        height as u32,
        image::ColorType::Rgb8
    )?;

    println!("Saved image to {}", output);
    Ok((output))
}

fn main() {
    let args: Args = Args::parse();

    match args.file {
        Some(file) => {
            set_wallpaper(file.as_path());
            return;
        },
        None => {}
    }


    let prompt = args.prompt;
    let output = args.output;
    // "A futuristic neon city in Rust programming language style, cinematic lighting";
    let gen_image = generate_image(prompt, output).expect("Failed to generate image");
    set_wallpaper(Path::new(&gen_image));

    let period= args.period.unwrap();
    let dir = args.directory.unwrap();

    let files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|f| is_valid_image(f))
        .collect();

    let mut wallpapers_set = 0;

    for entry in files {
        let path = entry.as_path();
        println!("Setting wallpaper: {}", path.display());
        set_wallpaper(path);
        wallpapers_set += 1;
        sleep(period);
    }

    println!("wallpapers set: {}", wallpapers_set);
}
