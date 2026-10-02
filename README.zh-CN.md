<h1 align="center">taodio</h1>

<p align="center"><strong>简体中文</strong> · <a href="README.md">English</a></p>

<p align="center"><strong>录音、命名、保存，都在终端里。</strong><br>为课程、会议与语音笔记准备的 macOS 录音工具。</p>

<p align="center">
  <img alt="macOS" src="https://img.shields.io/badge/platform-macOS-111111?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/built_with-Rust-111111?style=flat-square">
  <img alt="Ratatui" src="https://img.shields.io/badge/TUI-Ratatui-007AFF?style=flat-square">
  <a href="https://github.com/pentaoa/taodio/releases">下载最新版本</a>
</p>

```text
 ___                  
  |  _.  _   _| o  _  
  | (_| (_) (_| | (_) 
                      
```

圆角细线框、白色文字、蓝色选择标记。直接读取音频输入设备，无需打开 QuickTime，音频在本机处理。

## 快速开始

从 [Releases](https://github.com/pentaoa/taodio/releases) 下载 macOS Apple Silicon 压缩包，解压后安装：

```bash
mkdir -p ~/.local/bin
cp taodio ~/.local/bin/taodio
taodio
```

确保 `~/.local/bin` 在 PATH 中。在「设置」选择输入设备，返回主菜单开始录音。按 Enter 停止后可以重命名；默认保存到启动时所在目录。

首次录音可能需要麦克风权限：系统设置 → 隐私与安全性 → 麦克风，允许所用终端访问。

源码安装需要 Rust 和 Xcode Command Line Tools：

```bash
git clone https://github.com/pentaoa/taodio.git
cd taodio
cargo install --path . --locked
```

## 功能

| 功能 | 说明 |
| --- | --- |
| 原生录音 | CoreAudio / CPAL，支持内置、外接和虚拟输入设备 |
| 保存格式 | 默认 M4A，可选 AIFF |
| 录后命名 | 修改文件名，或保留自动生成的名称 |
| 取消确认 | 按 Esc 后确认丢弃，默认继续录音 |
| 三语界面 | 中文、日文、英文 |
| 当前目录保存 | 任意目录启动，默认保存到该目录 |

录制对象是音频输入设备，系统播放声音不会自动被录制。音频环境提供虚拟输入设备时，可在设置中选择。

## 操作

| 界面 | 按键 | 操作 |
| --- | --- | --- |
| 主菜单 | ↑ / ↓、Enter | 选择、打开 |
| 主菜单 | q / Esc | 退出 |
| 设置 | ↑ / ↓、← / → | 选择字段、修改并保存 |
| 设置 / 配置 | q / Esc | 返回 |
| 录音 | Enter | 停止保存，进入命名框 |
| 录音 | Esc | 打开取消确认框 |
| 确认框 | ← / → / Tab、Enter | 选择继续或丢弃并确认 |
| 确认框 | Esc | 继续录音 |
| 命名 | Backspace | 删除末尾字符 |
| 命名 | Ctrl+U | 清空整行，支持终端将 Command+Backspace 映射为 Ctrl+U |
| 命名 | Enter / Esc | 确认名称 / 保留原名 |

## 命令行

```bash
taodio devices
taodio record --name lecture-01
taodio record --device "MacBook Air麦克风" --dir ~/Desktop --name lecture
taodio record --format aiff --name voice-note
taodio config
```

`record` 按 Enter 停止保存；录后命名框位于主菜单交互流程中。`--output FILE` 指定路径，`--force` 允许覆盖。

## 配置与实现

设置立即写入 `~/.config/taodio/config.json`，包含语言 `zh` / `ja` / `en`、保存格式和输入设备。设备为 null 时使用系统默认。`taodio init` 重置设置。

录音先写入临时 WAV，停止后用 macOS 自带 `afconvert` 编码，成功后删除临时文件，无需额外安装编码器。完整配置示例见 [英文说明](README.md#configuration)。
