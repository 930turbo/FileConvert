# Third-party components

M5Convert itself is proprietary for this private project. It uses independent open-source libraries for file-format implementation and Windows bindings, including `image`, `resvg/usvg/tiny-skia`, `pdfium-render`, `windows-rs`, `clap`, `tempfile`, `url`, `thiserror`, and their transitive dependencies.

Optional runtime engines may include FFmpeg, PDFium and LibreOffice. Their licenses remain their own. Do not remove their required license/notice files if you later bundle those binaries into the install directory.

No source files from `opencoredev/convt` are included in this repository.
