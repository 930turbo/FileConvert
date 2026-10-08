use crate::{Category, FORMATS, Format, Toolset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    NativeImage,
    SvgRaster,
    Ffmpeg,
    Pdfium,
    LibreOffice,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub from: &'static Format,
    pub to: &'static Format,
    pub steps: Vec<(Backend, &'static Format, &'static Format)>,
}

#[derive(Debug, Clone)]
pub struct Planner {
    tools: Toolset,
}

impl Planner {
    pub fn new(tools: Toolset) -> Self { Self { tools } }
    pub fn tools(&self) -> &Toolset { &self.tools }

    pub fn plan(&self, from: &'static Format, to: &'static Format) -> Option<Plan> {
        if from.id == to.id { return None; }

        if let Some(backend) = self.direct(from, to) {
            return Some(Plan { from, to, steps: vec![(backend, from, to)] });
        }

        // Small, lossless intermediates only. This keeps route behavior predictable
        // and avoids nonsense such as a still image becoming a video through GIF.
        let intermediates = ["png", "tiff", "wav", "flac", "pdf", "odt", "ods", "odp"];
        for mid_id in intermediates {
            let mid = FORMATS.iter().find(|f| f.id == mid_id)?;
            if mid.id == from.id || mid.id == to.id { continue; }
            if let (Some(a), Some(b)) = (self.direct(from, mid), self.direct(mid, to)) {
                return Some(Plan {
                    from,
                    to,
                    steps: vec![(a, from, mid), (b, mid, to)],
                });
            }
        }
        None
    }

    pub fn targets(&self, from: &'static Format) -> Vec<&'static Format> {
        let mut out: Vec<_> = FORMATS.iter().filter(|to| self.plan(from, to).is_some()).collect();
        out.sort_by_key(|f| (f.category, f.name));
        out
    }

    pub fn menu_targets(&self, from: &'static Format) -> Vec<&'static Format> {
        let preferred: &[&str] = match (from.id, from.category) {
            ("gif", _) => &["mp4", "webp", "png"],
            (_, Category::Image) => &["jpeg", "png", "webp"],
            (_, Category::Vector) => &["png", "jpeg", "pdf"],
            (_, Category::Video) => &["mp4", "mov", "gif", "mp3"],
            (_, Category::Audio) => &["mp3", "m4a", "wav"],
            (_, Category::Pdf) => &["png", "jpeg", "docx"],
            (_, Category::Document) => &["pdf", "docx", "txt"],
            (_, Category::Spreadsheet) => &["pdf", "xlsx", "csv"],
            (_, Category::Presentation) => &["pdf", "pptx"],
        };
        let reachable = self.targets(from);
        let mut picked = Vec::new();
        for id in preferred {
            if let Some(f) = reachable.iter().copied().find(|f| f.id == *id) {
                if !picked.iter().any(|p: &&Format| p.id == f.id) { picked.push(f); }
            }
        }
        if picked.is_empty() { reachable.into_iter().take(4).collect() } else { picked }
    }

    fn direct(&self, from: &'static Format, to: &'static Format) -> Option<Backend> {
        use Category::*;

        // Built-in raster path is preferred because it starts instantly and has
        // no subprocess. HEIC needs an external codec; animated GIF to WebP/video
        // is left to FFmpeg so animation is preserved.
        if from.category == Image && to.category == Image {
            let needs_external_decode = matches!(from.id, "heic" | "avif");
            let needs_external_encode = to.id == "heic";
            let preserve_animation = from.id == "gif" && to.id == "webp";
            if !(needs_external_decode || needs_external_encode || preserve_animation) {
                return Some(Backend::NativeImage);
            }
            if self.tools.ffmpeg.is_some() { return Some(Backend::Ffmpeg); }
        }

        if from.category == Vector && to.category == Image && to.id != "heic" {
            return Some(Backend::SvgRaster);
        }

        if from.category == Pdf && matches!(to.id, "png" | "jpeg") && self.tools.pdfium_dir.is_some() {
            return Some(Backend::Pdfium);
        }

        if self.tools.soffice.is_some() {
            let same_office_family = matches!(from.category, Document | Spreadsheet | Presentation)
                && from.category == to.category;
            let office_to_pdf = matches!(from.category, Document | Spreadsheet | Presentation) && to.category == Pdf;
            if same_office_family || office_to_pdf {
                return Some(Backend::LibreOffice);
            }
        }

        if self.tools.ffmpeg.is_some() && ffmpeg_pair(from, to) {
            return Some(Backend::Ffmpeg);
        }

        None
    }
}

fn ffmpeg_pair(from: &Format, to: &Format) -> bool {
    use Category::*;
    match (from.category, to.category) {
        (Video, Video) | (Video, Audio) | (Audio, Audio) => true,
        (Image, Video) if from.id == "gif" => true,
        (Video, Image) if to.id == "gif" => true,
        (Image, Image) if from.id == "heic" || to.id == "heic" => true,
        (Image, Image) if from.id == "gif" && to.id == "webp" => true,
        _ => false,
    }
}
