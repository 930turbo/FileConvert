# M5Convert

A private, Windows-first local file converter built as a fresh implementation of the workflow requested for this project: select files in Explorer, choose a target format, convert beside the source file, and exit.

This repository is **not a fork of `opencoredev/convt`**. It does not vendor, link to, or require that project's source code. The public product behavior and format coverage were used as a reference; this implementation has its own source tree and deliberately removes the parts that are irrelevant to a single private Windows installation.

## What was removed

There is no:

- web application
- cloud API or worker
- account system
- billing or trial logic
- license activation
- telemetry
- updater
- browser extension
- database
- always-running desktop app
- startup entry
- Windows service
- resident tray process

When no conversion is running there is no `m5convert.exe` process. The Explorer integration is a small native COM DLL that Windows loads only to construct/invoke the context menu and can unload afterwards.

## User workflow

1. Right-click a supported file in Explorer.
2. Choose **Convert with M5Convert**.
3. Choose one of the short, relevant target formats.
4. The converter process starts, writes the new file beside the original, and exits.
5. If conversion fails, Windows shows a small native error dialog. Successful conversions are silent.

Multiple selected files are supported. The menu only offers targets common to the whole selection.

## Formats

The catalog contains the same 40 user-facing format families targeted for this project:

**Images:** JPEG, PNG, WebP, HEIC, AVIF, GIF, TIFF, BMP, ICO, TGA, PPM/PNM, QOI, OpenEXR  
**Vector:** SVG  
**Video:** MP4, MOV, WebM, MKV, AVI  
**Audio:** MP3, WAV, FLAC, AAC, M4A, OGG, Opus  
**PDF:** PDF  
**Documents:** DOCX, DOC, ODT, RTF, TXT, HTML  
**Presentations:** PPTX, PPT, ODP  
**Spreadsheets:** XLSX, XLS, ODS, CSV

Routes are only shown when the engine required for that route is actually available.

## Engines

### Built into `m5convert.exe`

- Raster images through the Rust `image` library.
- SVG rasterization through `resvg`.

These do not launch another process.

### Optional local engines

- **FFmpeg**: video, audio, HEIC and animation-preserving media paths. Network protocols are explicitly blacklisted on every FFmpeg invocation.
- **PDFium**: PDF pages to PNG/JPEG.
- **LibreOffice**: local document/presentation/spreadsheet conversion.

M5Convert looks for `ffmpeg.exe` and `pdfium.dll` next to itself first. LibreOffice is found in its normal Windows installation directory or PATH. Environment overrides are documented in `vendor/README.md`.

## Build on your Windows PC

Prerequisites are build-time only:

- Official Rust toolchain (`rustup` / Cargo).
- Microsoft Visual C++ Build Tools / Windows SDK for the `x86_64-pc-windows-msvc` target.

No .NET runtime is required by M5Convert itself.

From PowerShell:

```powershell
cd path\to\M5Convert
.\scripts\build.ps1
.\scripts\install.ps1
```

For the Windows 11 modern top-level context menu as well as the classic menu:

```powershell
.\scripts\install.ps1 -ModernMenu
```

The modern menu uses Microsoft's supported `IExplorerCommand` + sparse-package mechanism. The script creates a local self-signed package certificate for this machine/user and uses the installed Microsoft Windows SDK tools to package/sign it.

## CLI

The Explorer extension is just a front-end to the same tiny executable:

```powershell
m5convert formats
m5convert engines
m5convert targets photo.heic --menu
m5convert convert --to webp photo.png
m5convert convert --to mp3 clip.mp4
```

Outputs are written beside the input. Existing files are not replaced by default; collisions become `name (2).ext`, `name (3).ext`, and so on.

## Idle behavior

There is intentionally no daemon. Check it yourself after installation:

```powershell
Get-Process m5convert -ErrorAction SilentlyContinue
```

When no conversion is active, that should return nothing. The shell extension does not perform conversion work inside Explorer; it only discovers menu targets and launches the worker after a click.

## Source layout

```text
crates/m5convert-core   format catalog, routing, engines, output publishing
crates/m5convert-cli    short-lived converter executable
crates/m5convert-shell  Windows IExplorerCommand DLL
scripts/                build/install/uninstall/modern-menu scripts
package/                sparse-package manifest and shell assets
vendor/                 optional local runtime engines
```

## Current verification status

The code was assembled in an environment without the Rust toolchain and without a Windows Explorer host, so a real `cargo check`, MSVC build, COM load, and Explorer right-click test still need to be run on the target Windows machine. The included scripts are designed for that next step; do not treat the Explorer integration as verified until those checks pass there.
