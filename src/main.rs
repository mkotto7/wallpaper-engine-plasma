use std::path::PathBuf;
use clap::Parser;

// ref: https://docs.rs/clap/latest/clap/
#[derive(Parser)]
struct Args {
    path: PathBuf,
    period: u32
}

fn set_wallpaper(path: PathBuf) {
    /*
    ref: https://github.com/KDE/plasma-workspace/blob/master/wallpapers/image/plasma-apply-wallpaperimage.cpp
    plasma's own wrapper
    initial logic: just use daemon for applying wallpaper
     */

    println!("We got path: {:?}", path);
}

fn main() {
    // ref: https://rust-cli.github.io/book/tutorial/cli-args.html
    let args = Args::parse();
    set_wallpaper(args.path);
    println!("{}", args.period);
}
