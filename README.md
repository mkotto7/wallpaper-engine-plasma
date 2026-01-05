# wallpaper-engine-plasma

Wallpaper engine for KDE Plasma. Currently only tested on Arch Linux.

## Features

- Set a specific wallpaper
- Loop a directory at a specified interval
- 512x512 pixel wallpaper generation using Stable Diffusion 1.5, with GPU or CPU
- Specify arguments such as generation prompt, fill mode or screen
- Daemon and client structure

## Prerequisites

1.  KDE Plasma 6 (only tested on Arch Linux)
2.  Rust
3.  If using image generation:
    - CUDA toolkit (NVIDIA GPU only)
    - 2 GB of free space for model weights

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
Note: if you do not have `.cargo/bin` in your PATH, then use `./wepd` and `./wep` respectively for the commands below.

### 1. Start the daemon
```bash
wepd &
```

### 2. Send commands with client

```bash
# Display help:
wep --help

# Apply a specific image with the fill mode:
wep --file ~/Pictures/landscape.png --mode fill

# Generate a wallpaper using Stable Diffusion:
# Note: the first time may take time, since the weights have to be downloaded from HuggingFace.
wep --generate "aurora borealis"

# Apply each image in a directory every 30 minutes:
wep --directory ~/Wallpapers --period 30m

# Stop the current loop:
wep --stop-loop
```

## Autostart with systemd

To have the daemon start automatically, you can set it up as a systemd user service.
There is a `wepd.service` included in the repository.

### Setup
```bash
cp wp-d.service ~/.config/systemd/user/
systemctl --user daemon-reload
```

```bash
# Enable the service (starts automatically on login)
systemctl --user enable wepd.service

# Start the service right now
systemctl --user start wepd.service

# Check if it's running
systemctl --user status wepd.service

# See live logs (e.g. image generation info)
journalctl --user -u wepd.service -f
```

## License

This project is licensed under the MIT License.

## To-do
- [ ] Client check if daemon is running
- [ ] Daemon returns status messages when generating
- [ ] Better loop (interval check if new?)
- [ ] Better argument checking (can't provide file and dir at same time)

