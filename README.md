# OMT OBS Plugin

Pure-Rust Open Media Transport plugin for OBS Studio 32.2.1.

[![Build](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml/badge.svg)](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml)
[![License: GPL-2.0-only](https://img.shields.io/github/license/MikanseiLaboratory/omt-obs-plugin)](./LICENSE)

## Features

| Feature | OBS ID | Notes |
|---|---|---|
| Receive source | `omtsource` | mDNS, URL, preview decode, auto-reconnect |
| Program output | `omtoutput` | Tools → OMT Output Settings |
| Preview output | — | Studio Mode preview, or the current scene |
| Dedicated filter | `omt_filter` | Sends the filtered source or scene |

Settings keys: `sourceProperty`, `qualityProperty`, `enabledProperty`, `nameProperty`, plus `previewEnabledProperty` / `previewNameProperty`. Program and Dedicated Output also have Embedded / Video Only / Audio Only (`programModeProperty`, `filterModeProperty`).

## Formats

Send: NV12, UYVY, YUY2, BGRA/BGRX, FPA1.  
Receive: BGRA, FPA1.  
P010 / P216: output start is refused.

## Requirements

- OBS Studio 32.2.1
- Rust 1.97+

## Build

```bash
cargo build --release
```

Output: `omtplugin.dll` / `libomtplugin.so` / `libomtplugin.dylib`.

### Install

- **Windows:** `%PROGRAMDATA%\obs-studio\plugins\omtplugin\bin\64bit\omtplugin.dll`
- **macOS:** `~/Library/Application Support/obs-studio/plugins/omtplugin.plugin/Contents/MacOS/`
- **Linux:** `~/.config/obs-studio/plugins/omtplugin/bin/64bit/libomtplugin.so`

## Usage

1. **Receive:** Add Source → OMT Source, pick a discovered `omt://` URL or type one.
2. **Program:** Tools → OMT Output Settings → enable Program Output (default name `OBS Output`). Media: Embedded, Video Only, or Audio Only.
3. **Preview:** enable Preview Output in the same dialog (default name `OBS Preview`). Preview is video-only (studio preview, or the current scene).
4. **Filter:** Filters → OMT Dedicated Output. Name tokens: `${source}`, `${filter}`. Media: Embedded, Video Only, or Audio Only.

IDs `omtsource` / `omtoutput` / `omtoutputsettings` are shared with the C# `omtplugin`.

## License

[GNU General Public License v2.0 only](./LICENSE).
