<h1 align="center">taodio</h1>

<p align="center"><a href="README.zh-CN.md">简体中文</a> · <strong>English</strong></p>

<p align="center"><strong>Record, name, and save. In your terminal.</strong><br>A minimal macOS recorder for lectures, meetings, and voice notes.</p>

<p align="center">
  <img alt="macOS" src="https://img.shields.io/badge/platform-macOS-111111?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/built_with-Rust-111111?style=flat-square">
  <img alt="Ratatui" src="https://img.shields.io/badge/TUI-Ratatui-007AFF?style=flat-square">
  <a href="https://github.com/pentaoa/taodio/releases"><img alt="Release" src="https://img.shields.io/github/v/release/pentaoa/taodio?style=flat-square&color=111111"></a>
</p>

<p align="center"><a href="#quick-start">Get started</a> · <a href="#features">Features</a> · <a href="#controls">Controls</a> · <a href="#configuration">Config</a></p>

```text
 ___                  
  |  _.  _   _| o  _  
  | (_| (_) (_| | (_) 
                      
```

Rounded panels, white text, and blue selection markers keep recording in the terminal. Capture directly from your Mac's input devices, without a QuickTime window. Audio is processed locally.

## Quick Start

Download the macOS Apple Silicon archive from [Releases](https://github.com/pentaoa/taodio/releases), extract it, then install:

```bash
mkdir -p ~/.local/bin
cp taodio ~/.local/bin/taodio
taodio
```

Ensure `~/.local/bin` is on your PATH. Choose an input device in **Settings**, then **Start recording**. Press **Enter** to stop and open the filename field. Files default to the directory where you launched `taodio`.

On first recording, allow your terminal's **Microphone** access in **System Settings → Privacy & Security → Microphone**.

Build from source with Rust and Xcode Command Line Tools:

```bash
git clone https://github.com/pentaoa/taodio.git
cd taodio
cargo install --path . --locked
taodio
```

## Features

| Feature | Behavior |
| --- | --- |
| Native capture | CoreAudio through CPAL; built-in, external, or virtual input devices |
| Local output | M4A by default; AIFF available in Settings |
| Naming | Rename after stopping, or keep the generated filename |
| Cancellation | Esc opens confirmation before discarding the recording |
| Languages | Chinese, Japanese, and English |
| Current directory | Launch anywhere and save there by default |

Capture uses an **input device**, not automatic system-output capture. Select a virtual input if your audio setup provides one.

## Controls

| View | Key | Action |
| --- | --- | --- |
| Menu | ↑ / ↓, Enter | Select and open |
| Menu | q / Esc | Quit |
| Settings | ↑ / ↓, ← / → | Select a field; change and save |
| Settings / Config | q / Esc | Return |
| Recording | Enter | Stop, save, and open naming |
| Recording | Esc | Open cancellation confirmation |
| Confirmation | ← / → / Tab, Enter | Choose Continue or Discard; confirm |
| Confirmation | Esc | Continue recording |
| Naming | Backspace | Delete the last character |
| Naming | Ctrl+U | Clear the line; handles Command+Backspace when the terminal maps it to Ctrl+U |
| Naming | Enter / Esc | Rename / keep original |

## Command Line

```bash
taodio devices
taodio record --name lecture-01
taodio record --device "MacBook Air麦克风" --dir ~/Desktop --name lecture
taodio record --format aiff --name voice-note
taodio config
```

`record` stops and saves on Enter. The rename dialog belongs to the interactive menu flow. `--output FILE` sets a path; `--force` allows overwriting.

## Configuration

Settings save immediately to `~/.config/taodio/config.json`:

```json
{
  "version": 1,
  "settings": {
    "language": "zh",
    "save_format": "m4a",
    "input_device": null
  }
}
```

Language codes: `zh`, `ja`, `en`. A null device uses the system default. `taodio init` resets configuration.

## How It Works

CPAL captures through CoreAudio into a temporary WAV. After stopping, macOS's built-in `afconvert` encodes the selected format. The temporary WAV is removed after successful conversion. No external encoder is required.

## Development

```bash
cargo fmt
cargo build --release --locked
```

Live audio and terminal interactions are checked manually.
