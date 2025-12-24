use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_transformers::models::stable_diffusion;
use clap::error::ErrorKind;
use clap::{Error, Parser};
use dbus::arg::Variant;
use dbus::blocking::Connection;
use humantime;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs::{canonicalize, read_dir},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use tokenizers::Tokenizer;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, value_parser = validate_file)]
    file: Option<PathBuf>,
    #[arg(short, long, value_parser = validate_dir, default_value = ".")]
    directory: Option<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration, default_value = "3600s")]
    period: Option<Duration>,
    #[arg(long)]
    prompt: Option<String>,
    // TODO! add arg for specific desktop
}

const EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];

fn validate_dir(dir: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(dir);
    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display()),
        ));
    }

    if !path.is_dir() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a directory", path.display()),
        ));
    }

    Ok(path)
}

fn validate_file(file: &str) -> Result<PathBuf, Error> {
    let path = PathBuf::from(file);

    if !path.exists() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} does not exist", path.display()),
        ));
    }

    if !path.is_file() {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            format!("path {} is not a file", path.display()),
        ));
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            Error::raw(
                ErrorKind::ValueValidation,
                "could not read extension".to_string(),
            )
        })?;

    if !EXTENSIONS.contains(&ext) {
        return Err(Error::raw(
            ErrorKind::ValueValidation,
            "file is not supported",
        ));
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
    ref:
    https://github.com/KDE/plasma-workspace/blob/master/wallpapers/image/plasma-apply-wallpaperimage.cpp
    plasma's own wrapper
    initial logic: just use daemon for applying wallpaper
     */
    // dbus ref: https://docs.rs/crate/dbus/latest/source/examples/client.rs

    let conn = Connection::new_session().expect("Failed to connect to daemon");
    let proxy = conn.with_proxy(
        "org.kde.plasmashell",
        "/PlasmaShell",
        Duration::from_millis(5000),
    );

    // image parameters: /usr/share/plasma/wallpapers/org.kde.image/contents
    let screen: u32 = 0;
    let fill_mode = "0".to_string();
    let mut params: HashMap<String, Variant<String>> = HashMap::new();
    params.insert("Image".to_string(), Variant(path.display().to_string()));
    params.insert("FillMode".to_string(), Variant(fill_mode));
    println!("path: {}", path.display());

    let (): () = proxy
        .method_call(
            "org.kde.PlasmaShell",
            "setWallpaper",
            ("org.kde.image", params, screen),
        )
        .expect("Daemon call failed");
}

fn get_desktops() -> u64 {
    // TODO! return desktops from plasmashell daemon call
    0
}

fn get_seed() -> u64 {
    let mut seed = 0;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seed = seed + time;
    // TODO! read more variables, like cpu/gpu load, etc.

    println!("seed: {}", time);

    seed
}

fn generate_image(prompt: String) -> anyhow::Result<PathBuf> {
    /*
    ref: https://github.com/huggingface/candle/blob/main/candle-examples/examples/stable-diffusion/main.rs
     */
    println!("Generating image...");
    // TODO! add cpu option if gpu not available
    let device = Device::new_cuda(0)?;
    let dtype = DType::F16;

    let seed = get_seed();
    device.set_seed(seed)?;

    let n_steps = 30;
    let guidance_scale = 7.5;

    let api = hf_hub::api::sync::Api::new()?;
    let repo = api.model("runwayml/stable-diffusion-v1-5".to_string());

    let tokenizer_path = api
        .model("openai/clip-vit-base-patch32".to_string())
        .get("tokenizer.json")?;
    let unet_weights = repo.get("unet/diffusion_pytorch_model.fp16.safetensors")?;
    let vae_weights = repo.get("vae/diffusion_pytorch_model.fp16.safetensors")?;
    let clip_weights = repo.get("text_encoder/model.fp16.safetensors")?;

    let sd_config = stable_diffusion::StableDiffusionConfig::v1_5(None, Some(512), Some(512));
    let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;
    let mut scheduler = sd_config.build_scheduler(n_steps)?;

    let unet = sd_config.build_unet(unet_weights, &device, 4, false, dtype)?;
    let vae = sd_config.build_vae(vae_weights, &device, dtype)?;
    let clip =
        stable_diffusion::build_clip_transformer(&sd_config.clip, clip_weights, &device, dtype)?;

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

    let mut uncond_tokens = tokenizer
        .encode("", true)
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
        println!("step {}/{n_steps} done, {:.4}s", i + 1, dt);
    }

    let image = vae.decode(&(latents / 0.18215)?)?;
    let image = ((image / 2.)? + 0.5)?.clamp(0f32, 1.)?;
    let image = (image * 255.)?
        .to_device(&Device::Cpu)?
        .to_dtype(DType::U8)?;

    let image = image.i(0)?;
    let image = image.permute((1, 2, 0))?;
    let (height, width, _channels) = image.dims3()?;
    let raw_data = image.flatten_all()?.to_vec1::<u8>()?;

    let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let file = PathBuf::from(format!("image_{}.png", time));
    let mut output = canonicalize(".")?;
    output.push(file);

    image::save_buffer(
        &output,
        &raw_data,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )?;

    println!("Saved image as {}", output.file_name().unwrap().display());
    Ok(output)
}

fn main() {
    let args: Args = Args::parse();
    let image = if let Some(file) = args.file {
        file.canonicalize().expect("Failed to load file")
    } else if let Some(prompt) = args.prompt {
        generate_image(prompt).expect("Failed to generate image")
    } else {
        generate_image(
            "A futuristic neon city in Rust programming language style, cinematic lighting"
                .to_string(),
        )
        .expect("Failed to generate image")
    };

    let dir = if let Some(dir) = args.directory {
        dir
    } else {
        PathBuf::from(".")
    };

    set_wallpaper(Path::new(&image));
    println!("set wallpaper: {}", image.display());

    let files: Vec<PathBuf> = read_dir(dir)
        .unwrap()
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|f| is_valid_image(f))
        .collect();

    let mut wallpapers_set = 0;

    let period = if let Some(period) = args.period {
        period
    } else {
        Duration::from_secs(3600)
    };

    for entry in files {
        let path = entry.as_path();
        println!("Setting wallpaper: {}", path.display());
        set_wallpaper(path.canonicalize().expect("Failed to read file").as_path());
        wallpapers_set += 1;
        sleep(period);
    }
    println!("wallpapers set: {}", wallpapers_set);
}
