mod image_generation;
mod wallpaper;
mod validate;

use wallpaper::{set_wallpaper, get_screens};
use image_generation::generate_image;
use validate::{is_valid_image, validate_dir, validate_file};
use clap::{Parser, ValueEnum};
use humantime;
use std::{
    fs::{read_dir},
    path::{PathBuf},
    thread::sleep,
    time::Duration,
};

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum FillMode {
    Stretch,
    Fit,
    Crop,
    Tile,
    TileVertical,
    TileHorizontal,
    Pad,
}

impl FillMode {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Stretch => 0,
            Self::Fit => 1,
            Self::Crop => 2,
            Self::Tile => 3,
            Self::TileVertical => 4,
            Self::TileHorizontal => 5,
            Self::Pad => 6,
        }
    }
}

#[derive(Parser, Debug)]
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
        help = "Directory to loop"
    )]
    directory: Option<PathBuf>,
    #[arg(
        short,
        long,
        value_parser = humantime::parse_duration,
        default_value = "1h",
        help = "Interval between applying each wallpaper"
    )]
    period: Option<Duration>,
    #[arg(
        long,
        default_value = "scale",
        help = "Change wallpaper fill mode"
    )]
    fill_mode: FillMode,
    #[arg(
        short,
        long,
        default_value = "0",
        help = "What screen to apply wallpaper for"
    )]
    screen: u32,
    #[arg(
        short,
        long,
        help = "Print available screen IDs")]
    get_screens: bool,
    #[arg(long, help = "Specify prompt to use for image generation")]
    prompt: Option<String>,
    #[arg(
        short,
        long,
        help = "Use CPU for image generation (very slow)"
    )]
    use_cpu: bool,
}

fn main() {
    let args: Args = Args::parse();

    if args.get_screens {
        let screens = get_screens();
        println!("Available screens:\n{}", screens);
        return;
    }

    let use_cpu = args.use_cpu;
    let screen = args.screen;
    let fill_mode = args.fill_mode.to_u8();

    if let Some(file) = args.file {
        let path = file.canonicalize().expect("Failed to load file");
        set_wallpaper(&path, screen, fill_mode);
    } else if let Some(prompt) = args.prompt {
        let path = generate_image(prompt, use_cpu).expect("Failed to generate image");
        set_wallpaper(&path, screen, fill_mode);
    }

    if args.directory.is_some() {
        let dir = args.directory.expect("Directory not specified");

        let files: Vec<PathBuf> = read_dir(dir)
            .unwrap()
            .filter_map(|f| f.ok())
            .map(|f| f.path())
            .filter(|f| is_valid_image(f))
            .collect();

        let period = if let Some(period) = args.period {
            period
        } else {
            Duration::from_secs(3600)
        };

        sleep(Duration::from_secs(5));

        loop {
            for entry in &files {
                let path = entry.as_path();
                set_wallpaper(path.canonicalize().expect("Failed to read file").as_path(),
                              screen, fill_mode);
                sleep(period);
            }
        }
    }
}
