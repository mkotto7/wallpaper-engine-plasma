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
    interval: Duration,
    #[arg(long, help = "Stop the directory loop")]
    stop_loop: bool,
    #[arg(short, long, default_value = "crop", help = "Change wallpaper fill mode")]
    mode: FillMode,
    #[arg(
        short,
        long,
        default_value = "0",
        help = "What screen to apply wallpaper for"
    )]
    screen: u32,
    #[arg(short, long, help = "Print available screen IDs")]
    print_screens: bool,
    #[arg(short, long, help = "Specify prompt to use for image generation")]
    generate: Option<String>,
    #[arg(
        short,
        long,
        help = "Use CPU for image generation (very slow)",
        requires = "generate"
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
    let fill_mode = args.mode.to_u8();

    if args.print_screens {
        let screens = proxy.get_screens().await.expect("Failed to get screens");
        println!("Available screens:\n{}", screens);
        return Ok(());
    } else if args.stop_loop {
        let stopped = proxy.stop_loop().await.expect("Failed to send stop signal");
        if stopped {
            println!("Sent stop loop signal to daemon.");
        } else {
            println!("Failed to stop loop.")
        }
        return Ok(());
    } else if let Some(file) = args.file {
        let path = file
            .canonicalize()
            .expect("Failed to load file")
            .to_str()
            .expect("Failed to load file")
            .to_owned();
        let _ = proxy.set_image(&path, screen, fill_mode).await;
    } else if let Some(prompt) = args.generate {
        let path = proxy
            .generate_from_prompt_and_apply(&prompt, use_cpu, screen, fill_mode)
            .await?;
        println!("Generated image at: {}", path)
    } else if let Some(dir) = args.directory {
        let period = args.interval.as_secs();
        let _ = proxy
            .start_directory_loop(dir.to_str().unwrap(), screen, fill_mode, period)
            .await;
    }
    Ok(())
}
