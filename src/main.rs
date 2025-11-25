use std::path::PathBuf;
use std::process::Command;
use std::fs;
use std::thread::sleep;
use std::time::Duration;
use clap::Parser;

// ref: https://docs.rs/clap/latest/clap/
#[derive(Parser)]
struct Args {
    path: PathBuf
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
    let args = Args::parse();

    let files = fs::read_dir(args.path).unwrap();
    for entry in files {
        let path = &entry.unwrap().path();
        println!("Setting wallpaper: {:?}", path);
        set_wallpaper(path);
        sleep(Duration::from_millis(3000));
    }
}
