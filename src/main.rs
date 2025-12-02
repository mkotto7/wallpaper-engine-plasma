use std::fmt::format;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use std::thread::sleep;
use std::time::Duration;
use humantime;
use clap::{Error, Parser};
use clap::error::ErrorKind;

// ref: https://docs.rs/clap/latest/clap/
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, value_parser = validate_file)]
    file: Option<PathBuf>,
    #[arg(short, long, value_parser = validate_dir, default_value = ".")]
    directory: Option<PathBuf>,
    #[arg(short, long, value_parser = humantime::parse_duration)]
    period: Option<Duration>
}

fn validate_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    if !path.exists() {
        return Err(format!("path {} does not exist", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("path {} is not a directory", path.display()));
    }

    Ok(path)
}

fn validate_file(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);

    if !path.exists() {
        return Err(format!("path {:?} does not exist", path));
    }

    if !path.is_file() {
        return Err(format!("path {:?} is not a file", path));
    }

    const VALID_EXTENSIONS : &[&str] = &["png", "jpg", "jpeg"];
    let ext = path.extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| "could not read extension".to_string())?;

    if !VALID_EXTENSIONS.contains(&ext) {
        return Err("file is not supported".to_string());
    }

    Ok(path)
}

fn set_wallpaper(path: &PathBuf) {
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
    // println!("SCRIPT:\n {}", &script);

    if !output.status.success() {
        println!("failed to set wallpaper!");
        println!("{:?}", output);
    }
}

fn main() {
    // ref: https://rust-cli.github.io/book/tutorial/cli-args.html
    let args: Args = Args::parse();
    println!("{:#?}", args);

    // let files = fs::read_dir(args.directory).unwrap();
    // for entry in files {
    //     let path = &entry.unwrap().path();
    //     println!("Setting wallpaper: {:?}", path);
    //     set_wallpaper(path);
    //     sleep(Duration::from_millis(3000));
    // }
}
