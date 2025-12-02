use std::{path::{Path, PathBuf}, process::Command, fs, thread::sleep, time::Duration};
use humantime;
use clap::{Error, Parser};
use clap::error::ErrorKind;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, value_parser = validate_file)]
    file: Option<PathBuf>,
    #[arg(short, long, value_parser = validate_dir, default_value = ".")]
    directory: Option<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration, default_value = "3600s")]
    period: Option<Duration>
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

fn is_valid_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| EXTENSIONS.contains(&ext))
        .unwrap_or(false)
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
        println!("Setting wallpaper: {:?}", path);
        set_wallpaper(path);
        wallpapers_set += 1;
        sleep(period);
    }

    println!("wallpapers set: {}", wallpapers_set);
}
