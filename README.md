# OMT OBS Plugin

Pure-Rust Open Media Transport plugin for OBS Studio 32.2.1.

[![Build](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml/badge.svg)](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml)
[![License: GPL-2.0-only](https://img.shields.io/github/license/MikanseiLaboratory/omt-obs-plugin)](./LICENSE)

## Official plugin

This project is **not** a replacement for the official C# [omtplugin](https://github.com/openmediatransport/omtplugin). Both can be installed at the same time.

| | This plugin | Official `omtplugin` |
|---|---|---|
| Binary / folder | `omt_obs_plugin` / `omt-obs-plugin` | `omtplugin` |
| Source / output IDs | `omtobs_source`, `omtobs_output`, `omtobs_filter` | `omtsource`, `omtoutput`, `omt_filter` |
| Settings keys | `omtobs_*` | `sourceProperty`, `nameProperty`, … |
| Tools menu | **OMT Output Settings (Rust)** | **OMT Output Settings** |
| Default sender names | `OBS Output (Rust)`, `OBS Preview (Rust)` | `OBS Output`, `OBS Preview` |

Add **OMT Source (Rust)** when you want this plugin. The official source remains **OMT Source**.

## Features

| Feature | OBS ID | Notes |
|---|---|---|
| Receive source | `omtobs_source` | mDNS, URL, preview decode, auto-reconnect |
| Program output | `omtobs_output` | Tools → OMT Output Settings (Rust) |
| Preview output | — | Studio Mode preview, or the current scene |
| Dedicated filter | `omtobs_filter` | Sends the filtered source or scene |

Settings keys: `omtobs_source_url`, `omtobs_quality`, `omtobs_enabled`, `omtobs_name`, plus `omtobs_preview_enabled` / `omtobs_preview_name`. Receive sources also have `omtobs_bandwidth_policy` (and write `omtobs_preview` when policy is Always). Program and Dedicated Output also have Embedded / Video Only / Audio Only (`omtobs_program_mode`, `omtobs_filter_mode`).

OMT Source **Save bandwidth when** switches between the sender's 1/8 Preview stream and **Suggested Quality**:

- **None (always full)** — always Suggested Quality.
- **Not on Program** — Preview unless the source is on Program (nested scenes included). Studio Preview / Multiview stay on Preview.
- **Not on Preview/Program** — Preview unless the source is shown anywhere (`showing`: Program, Preview, Multiview, projector).
- **Always** — always Preview (legacy `omtobs_preview=true`).

OBS 32.2 cannot distinguish Studio Preview from Multiview or a projector. Opening the source properties preview also counts as `showing`.

## Formats

Send: NV12, UYVY, YUY2, BGRA/BGRX, FPA1.  
Receive: BGRA, FPA1.  
P010 / P216: output start is refused.

## Requirements

- OBS Studio 32.2.1
- Rust 1.97+ (to build from source)

## Install

[Get the latest release](https://github.com/MikanseiLaboratory/omt-obs-plugin/releases/latest) and restart OBS after installing.

### Windows

Run `omt-obs-plugin-*-windows-x64-setup.exe`. It installs to `%PROGRAMDATA%\obs-studio\plugins\omt-obs-plugin\` and does not overwrite official `omtplugin`.

### macOS

Open the `.pkg` that matches the Mac, then follow the prompts. Both install to `/Library/Application Support/obs-studio/plugins/omt-obs-plugin.plugin`.

- Apple silicon: `omt-obs-plugin-*-macos-arm64.pkg`
- Intel: `omt-obs-plugin-*-macos-x64.pkg`

### Linux

```bash
sudo dpkg -i omt-obs-plugin_*_amd64.deb
```

Installs `libomt_obs_plugin.so` to `/usr/lib/x86_64-linux-gnu/obs-plugins/`.

### Manual zip

- **Windows:** `%PROGRAMDATA%\obs-studio\plugins\omt-obs-plugin\bin\64bit\omt_obs_plugin.dll`
- **macOS:** `~/Library/Application Support/obs-studio/plugins/omt-obs-plugin.plugin/Contents/MacOS/` (`macos-arm64` or `macos-x64` zip)
- **Linux:** `~/.config/obs-studio/plugins/omt-obs-plugin/bin/64bit/libomt_obs_plugin.so`

## Build

```bash
cargo build --release
```

Output: `omt_obs_plugin.dll` / `libomt_obs_plugin.so` / `libomt_obs_plugin.dylib`.

Windows installer (after a release build, with [Inno Setup](https://jrsoftware.org/isinfo.php) installed):

```powershell
iscc /DMyAppVersion=0.1.0 installer\windows\omt-obs-plugin.iss
```

## Usage

1. **Receive:** Add Source → OMT Source (Rust), pick a discovered `omt://` URL or type one.
2. **Program:** Tools → OMT Output Settings (Rust) → enable Program Output (default name `OBS Output (Rust)`). Media: Embedded, Video Only, or Audio Only.
3. **Preview:** enable Preview Output in the same dialog (default name `OBS Preview (Rust)`). Preview is video-only (studio preview, or the current scene).
4. **Filter:** Filters → OMT Dedicated Output (Rust). Name tokens: `${source}`, `${filter}`. Media: Embedded, Video Only, or Audio Only.

## License

[GNU General Public License v2.0 only](./LICENSE).
