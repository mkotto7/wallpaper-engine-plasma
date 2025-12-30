use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_transformers::models::stable_diffusion;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Networks, System};
use tokenizers::Tokenizer;

pub fn get_seed() -> u64 {
    let mut seed = 0;
    let mut sys = System::new();
    sys.refresh_all();

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    seed = seed + time + sys.total_memory() - sys.used_memory();

    let networks = Networks::new_with_refreshed_list();
    for (_, data) in &networks {
        let received = data.total_received();
        let transmitted = data.total_transmitted();
        if transmitted > 0 {
            seed = seed + (received % transmitted)
        }
    }

    println!("Seed: {}", seed);
    seed
}

pub fn generate_image(prompt: String, use_cpu: bool) -> anyhow::Result<PathBuf> {
    /*
    ref: https://github.com/huggingface/candle/blob/main/candle-examples/examples/stable-diffusion/main.rs
     */
    println!("Generating image...");
    println!("Prompt: {}", prompt);
    let device = if use_cpu {
        Device::Cpu
    } else {
        Device::new_cuda(0)?
    };
    let dtype = DType::F16;

    let seed = get_seed();
    if !use_cpu {
        device.set_seed(seed)?;
    }

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

    let mut total_time: f32 = 0.0;
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
        total_time += dt;
        println!("Step {}/{n_steps} done, {:.4}s", i + 1, dt);
    }

    println!("Total time: {:.4}s", total_time);

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
