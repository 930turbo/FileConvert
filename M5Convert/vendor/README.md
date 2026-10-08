# Optional runtime engines

M5Convert is designed to stay dormant when it is not being used. The Rust executable contains the common raster-image and SVG converters. Extra engines are discovered only when a conversion needs them.

You may place these files next to `m5convert.exe` before installing:

- `ffmpeg.exe` — video, audio, HEIC and animated GIF/WebP paths.
- `pdfium.dll` — PDF rendering to PNG/JPEG.
- LibreOffice does not need to be copied here if it is installed normally; M5Convert checks the standard Windows installation directory and PATH.

Environment overrides are also supported:

- `M5CONVERT_FFMPEG`
- `M5CONVERT_PDFIUM_DIR`
- `M5CONVERT_SOFFICE`

No engine is started until you actually request a conversion.
