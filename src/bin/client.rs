use clap::Parser;
use core::time::Duration;
use std::path::PathBuf;
use wallpaper_engine_plasma::validation::{validate_dir, validate_file};
use wallpaper_engine_plasma::{FillMode, WallpaperEngineProxy};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Args {
    #[arg(
        short,
        long,
        value_parser = validate_file,
        help = "Image to apply"
    )]
    file: Option<PathBuf>,
    #[arg(
        short,
        long,
        value_parser = validate_dir,
        help = "Directory to loop",
    )]
    directory: Option<PathBuf>,
    #[arg(
        short,
        long,
        value_parser = humantime::parse_duration,
        default_value = "1h",
        help = "Interval between applying each wallpaper"
    )]
    period: Duration,
    #[arg(long, default_value = "crop", help = "Change wallpaper fill mode")]
    fill_mode: FillMode,
    #[arg(
        short,
        long,
        default_value = "0",
        help = "What screen to apply wallpaper for"
    )]
    screen: u32,
    #[arg(short, long, help = "Print available screen IDs")]
    get_screens: bool,
    #[arg(long, help = "Specify prompt to use for image generation")]
    prompt: Option<String>,
    #[arg(
        short,
        long,
        help = "Use CPU for image generation (very slow)",
        requires = "prompt"
    )]
    use_cpu: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let conn = zbus::Connection::session().await?;
    let proxy = WallpaperEngineProxy::new(&conn).await?;

    let use_cpu = args.use_cpu;
    let screen = args.screen;
    let fill_mode = args.fill_mode.to_u8();

    if args.get_screens {
        let screens = proxy.get_screens().await.expect("Failed to get screens");
        println!("Available screens:\n{}", screens);
        return Ok(());
    } else if let Some(file) = args.file {
        let path = file
            .canonicalize()
            .expect("Failed to load file")
            .to_str()
            .expect("Failed to load file")
            .to_owned();
        let _ = proxy.set_image(&path, screen, fill_mode).await;
    } else if let Some(prompt) = args.prompt {
        let path = proxy
            .generate_from_prompt_and_apply(&prompt, use_cpu, screen, fill_mode)
            .await?;
        let _ = proxy.set_image(&path, screen, fill_mode).await;
    } else if let Some(dir) = args.directory {
        let period = args.period.as_secs();
        let _ = proxy
            .start_directory_loop(dir.to_str().unwrap(), screen, fill_mode, period)
            .await;
    }
    Ok(())
}
