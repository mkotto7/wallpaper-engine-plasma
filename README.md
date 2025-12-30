# wallpaper-engine-plasma

Wallpaper engine for KDE Plasma. 

## To-do
- [ ] Client argument to stop directory loop
- [ ] Daemon returns status messages when generating
- [ ] Better loop (interval check if new)
- [ ] Better argument checking (can't provide file and dir at same time)
- [ ] systemd? + stopping daemon
- [ ] Client check if daemon is running
- [ ] GenAI depends on something that is currently going on, date of time
- [ ] Better README (stop loop)

## Features

- Set a specific wallpaper
- Loop a directory at a specified interval
- 512x512 pixel wallpaper generation using Stable Diffusion 1.5, with GPU or CPU
- Specify arguments such as generation prompt, fill mode or screen
- Daemon and client structure

## Prerequisites

1.  KDE Plasma 6 (only tested on Arch Linux)
2.  Rust
3.  CUDA toolkit (if using image generation and an NVIDIA GPU)

## Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/mkotto7/wallpaper-engine-plasma.git
    cd wallpaper-engine-plasma
    ```

2.  Install:
    ```bash
    cargo install --path .
    ```
    
The binaries will be in `~/.cargo/bin`.

## Usage

The engine consists of two parts: the daemon which runs in the background and the client.
Make sure you are in the folder with the binaries, by default:
```bash
cd ~/.cargo/bin
```
Note: if you do not have `.cargo/bin` in your PATH, then use `./wp-d` and `./wp-c` respectively for the commands below.

### 1. Start the daemon
```bash
wp-d &
```

### 2. Send commands with client

Examples:
Display help:
```bash
wp-c --help
```

Apply a specific image:
```bash
wp-c --file ~/Pictures/landscape.png --fill-mode fill
```

Generate a wallpaper using Stable Diffusion:
```bash
wp-c --prompt "aurora borealis"
```
Note: the first time may take time, since the weights have to be downloaded from HuggingFace.

Apply each image in a directory every 30 minutes:
```bash
wp-c --directory ~/Wallpapers --period 30m
```

Stop the current loop:
```bash
busctl --user call org.wallpaper.PlasmaEngine /org/wallpaper/PlasmaEngine org.wallpaper.PlasmaEngine StopLoop
```

## License

This project is licensed under the MIT License.