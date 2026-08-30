# OMT OBS Plugin

Pure-Rust Open Media Transport plugin for OBS Studio 32.2.1.

[![Build](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml/badge.svg)](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml)
[![License: GPL-2.0-only](https://img.shields.io/github/license/MikanseiLaboratory/omt-obs-plugin)](./LICENSE)

Works alongside the official C# [omtplugin](https://github.com/openmediatransport/omtplugin).

## Features

- **OMT Source (Rust)** — receive audio/video from `omt://` sources
- **Program / Preview send** — Tools → OMT Output Settings (Rust)
- **OMT Dedicated Output (Rust)** — filter that sends a source or scene

Receive sources can request the sender's Preview stream when the source is off Program, off Preview/Program, or always.

## Formats

Send: NV12, UYVY, YUY2, BGRA/BGRX, FPA1.  
Receive: BGRA, FPA1.  
P010 / P216: output start is refused.

## Install

[Latest release](https://github.com/MikanseiLaboratory/omt-obs-plugin/releases/latest). Restart OBS after installing.

| Platform | Installer |
|---|---|
| Windows | `omt-obs-plugin-*-windows-x64-setup.exe` |
| macOS (Apple silicon) | `omt-obs-plugin-*-macos-arm64.pkg` |
| macOS (Intel) | `omt-obs-plugin-*-macos-x64.pkg` |
| Linux | `omt-obs-plugin_*_amd64.deb` |

Zip packages are also attached if you prefer a manual copy.

- Windows: `%PROGRAMDATA%\obs-studio\plugins\omt-obs-plugin\bin\64bit\`
- macOS: `/Library/Application Support/obs-studio/plugins/omt-obs-plugin.plugin` (pkg) or `~/Library/Application Support/obs-studio/plugins/` (zip)
- Linux: `/usr/lib/x86_64-linux-gnu/obs-plugins/` (deb) or `~/.config/obs-studio/plugins/omt-obs-plugin/bin/64bit/` (zip)

## Usage

1. **Receive:** Add Source → OMT Source (Rust), pick or type an `omt://` URL.
2. **Program:** Tools → OMT Output Settings (Rust) → enable Program Output (`OBS Output (Rust)`).
3. **Preview:** enable Preview Output in the same dialog (`OBS Preview (Rust)`). Video only.
4. **Filter:** Filters → OMT Dedicated Output (Rust). Name tokens: `${source}`, `${filter}`.

Program and filter can send Embedded, Video Only, or Audio Only.

## Build

Requires Rust 1.97+.

```bash
cargo build --release
```

Output: `omt_obs_plugin.dll` / `libomt_obs_plugin.so` / `libomt_obs_plugin.dylib`.

## License

[GNU General Public License v2.0 only](./LICENSE).
