# OMT (Rust) OBS Plugin

Pure-Rust Open Media Transport plugin for OBS Studio **32.2.1**.
It sends and receives SDR video plus FPA1 audio without `libvmx.dll` or .NET.

[![Build](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml/badge.svg)](https://github.com/MikanseiLaboratory/omt-obs-plugin/actions/workflows/build.yml)

## Features

| Feature | OBS ID | Notes |
|---|---|---|
| OMT receive source | `omtsource` | mDNS list, manual URL, preview decode, auto-reconnect |
| Program output | `omtoutput` | Tools → OMT Output Settings |
| Preview output | (internal) | Studio Mode preview (or current scene) |
| Dedicated filter | `omt_filter` | Continuously sends the filtered source/scene |

Settings keys match the official C# `omtplugin` (`sourceProperty`, `qualityProperty`, `enabledProperty`, `nameProperty`). Preview enable/name are extra keys and are ignored by the official plugin.

## Supported formats (this build)

**Send:** NV12, UYVY, YUY2, BGRA/BGRX + planar float audio (FPA1).  
**Receive:** decoded BGRA + FPA1.  
**Not supported:** P010 / P216 HDR. Output start is refused for HDR canvas formats.

## Requirements

- OBS Studio 32.2.1 (Windows / macOS / Linux)
- Rust **1.97+**
- [`rust-obs-plugins`](https://github.com/MikanseiLaboratory/rust-obs-plugins) pinned to the OBS 32.2.1 ABI revision (`obs-32-abi`)

Do **not** load this plugin together with the official C# `omtplugin` — they share `omtsource` / `omtoutput` IDs.

## Build

```bash
cargo build --release
```

The cdylib name is `omtplugin` (`omtplugin.dll` / `libomtplugin.so` / `libomtplugin.dylib`).

### Install

- **Windows:** `%PROGRAMDATA%\obs-studio\plugins\omtplugin\bin\64bit\omtplugin.dll`
- **macOS:** `~/Library/Application Support/obs-studio/plugins/omtplugin.plugin/Contents/MacOS/`
- **Linux:** `~/.config/obs-studio/plugins/omtplugin/bin/64bit/libomtplugin.so`

No extra native libraries are required (`vmx-rs` is linked statically via `openmediatransport-rs`).

## Usage

1. **Receive:** Add Source → OMT Source, pick a discovered `omt://` URL (or type one), optionally enable Preview Mode.
2. **Program send:** Tools → OMT Output Settings → enable Program Output and set the source name (default `OBS Output`).
3. **Preview send:** enable Preview Output in the same dialog (default name `OBS Preview`). In Studio Mode this follows the preview scene; otherwise it follows the current scene. Preview is video-only.
4. **Filter send:** Filters → OMT Dedicated Output. Name tokens: `${source}`, `${filter}`.

## Manual verification (OBS 32.2.1)

Do not load this plugin together with the official C# `omtplugin`.

- Receive an SDR source from an OMT sender or official `omtplugin`.
- Program Output ↔ Preview Output between two OBS instances.
- Filter on a source and on a scene; rename; add/remove repeatedly.
- Profile switch and OBS exit: no leftover Sender / mDNS / worker threads.
- HDR canvas (P010/P216): Program Output start is refused.

## License

GPL-2.0 (required by `obs-wrapper` / libobs). The OMT protocol crate remains MIT.
