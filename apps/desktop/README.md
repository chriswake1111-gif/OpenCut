# Desktop

The native desktop app, built with [GPUI](https://gpui.rs).

## Getting started

**1. Install Rust:**

```bash
# Linux / macOS / WSL
./script/setup-rust
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -File .\script\setup-rust.ps1
```

Both scripts skip installation if Rust is already present. On Linux/macOS/WSL only: after a fresh install, reload your shell with `source "$HOME/.cargo/env"`

**2. Install native dependencies:**

```bash
# Linux / macOS / WSL
./apps/desktop/script/setup
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -File .\apps\desktop\script\setup.ps1
```

**3. Run:**

```bash
cargo run -p opencut-desktop
```

## Platform notes

**Linux:** supports apt (Debian/Ubuntu/Mint), dnf (Fedora/RHEL), and pacman (Arch).

**macOS:** installs Xcode Command Line Tools if missing.

**Windows:** the setup script checks for Visual Studio Build Tools. If missing, it prints the install link.

**WSL:** runs the same scripts as Linux. Window rendering works via WSLg on Windows 11 and Windows 10 22H2+. If you're on an older build, test on the host instead.

## FFmpeg Dependencies

OpenCut requires `ffmpeg` and `ffprobe` to be installed and available in your system's `PATH` for exporting videos.

### Installation Instructions

- **Windows:**
  - Install via [Scoop](https://scoop.sh/): `scoop install ffmpeg`
  - Install via [Chocolatey](https://chocolatey.org/): `choco install ffmpeg`
  - Or download the build from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) and add the `bin` folder to your system environment variables `PATH`.

- **macOS:**
  - Install via [Homebrew](https://brew.sh/): `brew install ffmpeg`

- **Linux:**
  - Install via your package manager:
    ```bash
    # Debian/Ubuntu
    sudo apt update && sudo apt install ffmpeg
    
    # Fedora
    sudo dnf install ffmpeg
    
    # Arch Linux
    sudo pacman -S ffmpeg
    ```

