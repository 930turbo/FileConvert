use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Toolset {
    pub ffmpeg: Option<PathBuf>,
    pub soffice: Option<PathBuf>,
    pub pdfium_dir: Option<PathBuf>,
}

impl Toolset {
    pub fn discover() -> Self {
        let exe_dir = env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
        let ffmpeg = env_path("M5CONVERT_FFMPEG")
            .filter(|p| p.is_file())
            .or_else(|| exe_dir.as_ref().and_then(|d| existing(d.join(exe("ffmpeg")))))
            .or_else(|| find_path(&[exe("ffmpeg")]));
        let soffice = env_path("M5CONVERT_SOFFICE")
            .filter(|p| p.is_file())
            .or_else(|| exe_dir.as_ref().and_then(|d| existing(d.join(exe("soffice")))))
            .or_else(find_libreoffice)
            .or_else(|| find_path(&[exe("soffice"), exe("soffice.com")]));
        let pdfium_dir = env_path("M5CONVERT_PDFIUM_DIR")
            .filter(|p| pdfium_exists(p))
            .or_else(|| exe_dir.filter(|d| pdfium_exists(d)));
        Self { ffmpeg, soffice, pdfium_dir }
    }

    pub fn summary(&self) -> Vec<(&'static str, bool, Option<String>)> {
        vec![
            ("native-image", true, Some("built in".into())),
            ("svg", true, Some("built in".into())),
            ("ffmpeg", self.ffmpeg.is_some(), self.ffmpeg.as_ref().map(|p| p.display().to_string())),
            ("pdfium", self.pdfium_dir.is_some(), self.pdfium_dir.as_ref().map(|p| p.display().to_string())),
            ("libreoffice", self.soffice.is_some(), self.soffice.as_ref().map(|p| p.display().to_string())),
        ]
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

fn find_path(names: &[String]) -> Option<PathBuf> {
    for dir in env::var_os("PATH").map(|v| env::split_paths(&v).collect::<Vec<_>>()).unwrap_or_default() {
        for name in names {
            let p = dir.join(name);
            if p.is_file() { return Some(p); }
        }
    }
    None
}

#[cfg(windows)]
fn find_libreoffice() -> Option<PathBuf> {
    let roots = [env::var_os("ProgramFiles"), env::var_os("ProgramFiles(x86)")];
    roots.into_iter().flatten().map(PathBuf::from)
        .map(|r| r.join("LibreOffice").join("program").join("soffice.com"))
        .find(|p| p.is_file())
}

#[cfg(not(windows))]
fn find_libreoffice() -> Option<PathBuf> { None }

fn pdfium_exists(dir: &Path) -> bool {
    #[cfg(windows)]
    let name = "pdfium.dll";
    #[cfg(target_os = "macos")]
    let name = "libpdfium.dylib";
    #[cfg(all(unix, not(target_os = "macos")))]
    let name = "libpdfium.so";
    dir.join(name).is_file()
}
