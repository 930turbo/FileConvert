use std::ffi::OsStr;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use image::{DynamicImage, ImageFormat};
use pdfium_render::prelude::*;
use resvg::{tiny_skia, usvg};

use crate::{Backend, Error, Format, Planner, Result, Toolset, format_by_extension, format_by_id};

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub jpeg_quality: u8,
    pub overwrite: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self { jpeg_quality: 92, overwrite: false }
    }
}

pub fn convert_file(
    planner: &Planner,
    input: &Path,
    target_id: &str,
    options: &ConvertOptions,
) -> Result<Vec<PathBuf>> {
    let from = format_by_extension(input).ok_or_else(|| Error::UnsupportedInput(input.to_path_buf()))?;
    let to = format_by_id(target_id).ok_or_else(|| Error::UnknownTarget(target_id.to_string()))?;
    let plan = planner.plan(from, to).ok_or_else(|| Error::NoRoute {
        from: from.id.to_string(), to: to.id.to_string()
    })?;

    let parent = input.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let work = tempfile::Builder::new().prefix(".m5convert-").tempdir_in(parent)?;
    let mut current = vec![input.to_path_buf()];

    for (index, (backend, step_from, step_to)) in plan.steps.iter().enumerate() {
        let step_dir = work.path().join(index.to_string());
        fs::create_dir_all(&step_dir)?;
        let mut next = Vec::new();
        for file in &current {
            let mut made = run_backend(*backend, planner.tools(), file, *step_from, *step_to, &step_dir, options)?;
            next.append(&mut made);
        }
        current = next;
    }

    publish(input, &current, to, options)
}

fn run_backend(
    backend: Backend,
    tools: &Toolset,
    input: &Path,
    _from: &'static Format,
    to: &'static Format,
    out_dir: &Path,
    options: &ConvertOptions,
) -> Result<Vec<PathBuf>> {
    match backend {
        Backend::NativeImage => native_image(input, to, out_dir, options),
        Backend::SvgRaster => svg_raster(input, to, out_dir, options),
        Backend::Ffmpeg => ffmpeg(tools, input, to, out_dir),
        Backend::Pdfium => pdfium(tools, input, to, out_dir, options),
        Backend::LibreOffice => libreoffice(tools, input, to, out_dir),
    }
}

fn native_image(input: &Path, to: &Format, out_dir: &Path, options: &ConvertOptions) -> Result<Vec<PathBuf>> {
    let img = image::ImageReader::open(input)?.with_guessed_format()?.decode()?;
    let target = out_dir.join(format!("1.{}", to.extension()));
    save_dynamic(&img, to.id, &target, options)?;
    Ok(vec![target])
}

fn save_dynamic(img: &DynamicImage, target_id: &str, path: &Path, options: &ConvertOptions) -> Result<()> {
    if target_id == "jpeg" {
        let file = fs::File::create(path)?;
        let mut writer = BufWriter::new(file);
        let rgb = img.to_rgb8();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, options.jpeg_quality);
        enc.encode_image(&DynamicImage::ImageRgb8(rgb))?;
        return Ok(());
    }
    let fmt = image_format(target_id).ok_or_else(|| Error::EngineFailed {
        engine: "native-image", message: format!("unsupported output {target_id}")
    })?;
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    img.write_to(&mut writer, fmt)?;
    Ok(())
}

fn image_format(id: &str) -> Option<ImageFormat> {
    Some(match id {
        "jpeg" => ImageFormat::Jpeg,
        "png" => ImageFormat::Png,
        "webp" => ImageFormat::WebP,
        "avif" => ImageFormat::Avif,
        "gif" => ImageFormat::Gif,
        "tiff" => ImageFormat::Tiff,
        "bmp" => ImageFormat::Bmp,
        "ico" => ImageFormat::Ico,
        "tga" => ImageFormat::Tga,
        "ppm" => ImageFormat::Pnm,
        "qoi" => ImageFormat::Qoi,
        "exr" => ImageFormat::OpenExr,
        _ => return None,
    })
}

fn svg_raster(input: &Path, to: &Format, out_dir: &Path, options: &ConvertOptions) -> Result<Vec<PathBuf>> {
    let data = fs::read(input)?;
    let mut opts = usvg::Options::default();
    opts.resources_dir = input.parent().map(Path::to_path_buf);
    let tree = usvg::Tree::from_data(&data, &opts).map_err(|e| Error::EngineFailed {
        engine: "svg", message: e.to_string()
    })?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or_else(|| Error::EngineFailed {
        engine: "svg", message: "SVG dimensions are too large".into()
    })?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    let png = out_dir.join("svg-stage.png");
    pixmap.save_png(&png).map_err(|e| Error::EngineFailed { engine: "svg", message: e.to_string() })?;
    if to.id == "png" {
        let target = out_dir.join("1.png");
        fs::rename(png, &target)?;
        return Ok(vec![target]);
    }
    native_image(&png, to, out_dir, options)
}

fn ffmpeg(tools: &Toolset, input: &Path, to: &Format, out_dir: &Path) -> Result<Vec<PathBuf>> {
    let exe = tools.ffmpeg.as_ref().ok_or(Error::EngineUnavailable("ffmpeg"))?;
    let target = out_dir.join(format!("1.{}", to.extension()));
    let mut cmd = Command::new(exe);
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
        // Explicitly refuse network protocols. The converter is local-only even
        // when a media file contains a nested reference or playlist.
        .args(["-protocol_blacklist", "http,https,tcp,tls,udp,rtmp,rtmps,ftp,crypto"])
        .arg("-i").arg(input);

    match to.id {
        "mp4" | "mov" => { cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart"]); }
        "webm" => { cmd.args(["-c:v", "libvpx-vp9", "-c:a", "libopus"]); }
        "mkv" => { cmd.args(["-c:v", "libx264", "-c:a", "aac"]); }
        "avi" => { cmd.args(["-c:v", "mpeg4", "-q:v", "4"]); }
        "mp3" => { cmd.args(["-vn", "-c:a", "libmp3lame", "-b:a", "192k"]); }
        "wav" => { cmd.args(["-vn", "-c:a", "pcm_s16le"]); }
        "flac" => { cmd.args(["-vn", "-c:a", "flac"]); }
        "aac" | "m4a" => { cmd.args(["-vn", "-c:a", "aac", "-b:a", "192k"]); }
        "ogg" => { cmd.args(["-vn", "-c:a", "libvorbis", "-q:a", "5"]); }
        "opus" => { cmd.args(["-vn", "-c:a", "libopus", "-b:a", "160k"]); }
        "gif" => { cmd.args(["-vf", "fps=15"]); }
        _ => {}
    }
    cmd.arg(&target);
    hide_console(&mut cmd);
    let output = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output()?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        return Err(Error::EngineFailed {
            engine: "ffmpeg",
            message: trim_error(&message),
        });
    }
    if !target.is_file() {
        return Err(Error::EngineFailed { engine: "ffmpeg", message: "no output file was produced".into() });
    }
    Ok(vec![target])
}

fn pdfium(tools: &Toolset, input: &Path, to: &Format, out_dir: &Path, options: &ConvertOptions) -> Result<Vec<PathBuf>> {
    let dir = tools.pdfium_dir.as_ref().ok_or(Error::EngineUnavailable("pdfium"))?;
    let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
    let bindings = Pdfium::bind_to_library(&lib).map_err(|e| Error::EngineFailed { engine: "pdfium", message: e.to_string() })?;
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium.load_pdf_from_file(input, None).map_err(|e| Error::EngineFailed { engine: "pdfium", message: e.to_string() })?;
    let pages = doc.pages();
    let config = PdfRenderConfig::new().set_target_width(2000).set_maximum_height(4000);
    let mut outputs = Vec::new();
    for i in 0..pages.len() {
        let page = pages.get(i).map_err(|e| Error::EngineFailed { engine: "pdfium", message: e.to_string() })?;
        let image = page.render_with_config(&config)
            .map_err(|e| Error::EngineFailed { engine: "pdfium", message: e.to_string() })?
            .as_image()
            .map_err(|e| Error::EngineFailed { engine: "pdfium", message: e.to_string() })?;
        let target = out_dir.join(format!("{}.{}", i + 1, to.extension()));
        save_dynamic(&image, to.id, &target, options)?;
        outputs.push(target);
    }
    Ok(outputs)
}

fn libreoffice(tools: &Toolset, input: &Path, to: &Format, out_dir: &Path) -> Result<Vec<PathBuf>> {
    let exe = tools.soffice.as_ref().ok_or(Error::EngineUnavailable("libreoffice"))?;
    let profile = tempfile::tempdir()?;
    let office_out = tempfile::tempdir()?;
    let profile_url = url::Url::from_directory_path(profile.path()).map_err(|_| Error::EngineFailed {
        engine: "libreoffice", message: "could not construct a private profile URL".into()
    })?;
    let mut cmd = Command::new(exe);
    cmd.arg(format!("-env:UserInstallation={profile_url}"))
        .args(["--headless", "--nologo", "--nodefault", "--norestore", "--convert-to"])
        .arg(office_filter(to.id))
        .arg("--outdir")
        .arg(office_out.path())
        .arg(input);
    hide_console(&mut cmd);
    let output = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output()?;
    if !output.status.success() {
        let message = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        return Err(Error::EngineFailed { engine: "libreoffice", message: trim_error(&message) });
    }
    let produced = fs::read_dir(office_out.path())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().and_then(OsStr::to_str).is_some_and(|e| to.extensions.iter().any(|x| x.eq_ignore_ascii_case(e))))
        .ok_or_else(|| Error::EngineFailed { engine: "libreoffice", message: "LibreOffice produced no matching output".into() })?;
    let target = out_dir.join(format!("1.{}", to.extension()));
    fs::copy(produced, &target)?;
    Ok(vec![target])
}

fn office_filter(id: &str) -> String {
    match id {
        "docx" => "docx:Office Open XML Text",
        "doc" => "doc:MS Word 97",
        "rtf" => "rtf:Rich Text Format",
        "txt" => "txt:Text (encoded):UTF8",
        "html" => "html:HTML",
        "pptx" => "pptx:Impress MS PowerPoint 2007 XML",
        "ppt" => "ppt:MS PowerPoint 97",
        "xlsx" => "xlsx:Calc MS Excel 2007 XML",
        "xls" => "xls:MS Excel 97",
        "pdf" => "pdf",
        "odt" => "odt",
        "odp" => "odp",
        "ods" => "ods",
        "csv" => "csv",
        other => other,
    }.to_string()
}

fn publish(input: &Path, staged: &[PathBuf], to: &Format, options: &ConvertOptions) -> Result<Vec<PathBuf>> {
    let dir = input.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().and_then(OsStr::to_str).ok_or_else(|| Error::InvalidName(input.to_path_buf()))?;
    let mut published = Vec::new();
    for (i, source) in staged.iter().enumerate() {
        let base = if staged.len() == 1 || i == 0 { stem.to_string() } else { format!("{stem}-{}", i + 1) };
        let target = if options.overwrite {
            dir.join(format!("{base}.{}", to.extension()))
        } else {
            free_name(dir, &base, to.extension())?
        };
        if options.overwrite && target.exists() { fs::remove_file(&target)?; }
        match fs::rename(source, &target) {
            Ok(()) => {}
            Err(_) => { fs::copy(source, &target)?; }
        }
        published.push(target);
    }
    Ok(published)
}

fn free_name(dir: &Path, stem: &str, ext: &str) -> Result<PathBuf> {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() { return Ok(first); }
    for n in 2..10000 {
        let candidate = dir.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() { return Ok(candidate); }
    }
    Err(Error::OutputCollision(first))
}

fn trim_error(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 1800 { s.to_string() } else { format!("...{}", &s[s.len()-1800..]) }
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NO_WINDOW;
    cmd.creation_flags(CREATE_NO_WINDOW.0);
}

#[cfg(not(windows))]
fn hide_console(_: &mut Command) {}
