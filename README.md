# wallpaper-engine-plasma

Wallpaper engine for KDE Plasma. 

## To-do
- [ ] GenAI depends on something that is currently going on, date of time
- [ ] Daemon (server + client?)
- [ ] Proper README


## Features

- Set a single wallpaper
- Loop a directory at a specified interval
- 512x512 wallpaper generation using Stable Diffusion 1.5, with GPU or CPU
- Specify arguments such as fill mode or screen

## Prerequisites

1.  KDE Plasma 6
2.  Rust
3.  CUDA toolkit (if using NVIDIA GPU for image generation)

## Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/mkotto7/wallpaper-engine-plasma.git
    cd wallpaper-engine-plasma
    ```

2.  Build:
    ```bash
    cargo build --release
    ```

## Usage and examples
`./target/release/wallpaper-engine-plasma --help` to list all possible arguments.

### Apply a specific image
```bash
./target/release/wallpaper-engine-plasma --file ~/Pictures/landscape.png --fill-mode fill
```

### Image generation
Generate a wallpaper from a text prompt and apply it immediately on screen 0:
```bash
./target/release/wallpaper-engine-plasma --prompt "aurora borealis" --screen 0
```

### Directory loop
Loop and rotate through all images in a folder every 30 minutes with the fit fill mode:
```bash
./target/release/wallpaper-engine-plasma --directory ~/Pictures/Wallpapers --period 30m --fill-mode fit
```

### List available screens
```bash
./target/release/wallpaper-engine-plasma --get-screens
```

## License

This project is licensed under the MIT License.