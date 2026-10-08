use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Image,
    Vector,
    Video,
    Audio,
    Pdf,
    Document,
    Presentation,
    Spreadsheet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub extensions: &'static [&'static str],
    pub mime: &'static str,
}

impl Format {
    pub const fn extension(&self) -> &'static str {
        self.extensions[0]
    }

    pub const fn is_raster(&self) -> bool {
        matches!(self.category, Category::Image)
    }
}

macro_rules! f {
    ($id:literal,$name:literal,$cat:ident,[$($ext:literal),+],$mime:literal) => {
        Format { id:$id, name:$name, category:Category::$cat, extensions:&[$($ext),+], mime:$mime }
    };
}

pub static FORMATS: &[Format] = &[
    f!("jpeg","JPEG",Image,["jpg","jpeg","jfif"],"image/jpeg"),
    f!("png","PNG",Image,["png"],"image/png"),
    f!("webp","WebP",Image,["webp"],"image/webp"),
    f!("heic","HEIC",Image,["heic","heif"],"image/heic"),
    f!("avif","AVIF",Image,["avif"],"image/avif"),
    f!("gif","GIF",Image,["gif"],"image/gif"),
    f!("tiff","TIFF",Image,["tiff","tif"],"image/tiff"),
    f!("bmp","BMP",Image,["bmp"],"image/bmp"),
    f!("ico","ICO",Image,["ico"],"image/x-icon"),
    f!("tga","TGA",Image,["tga"],"image/x-tga"),
    f!("ppm","PPM",Image,["ppm","pgm","pbm","pnm"],"image/x-portable-anymap"),
    f!("qoi","QOI",Image,["qoi"],"image/qoi"),
    f!("exr","OpenEXR",Image,["exr"],"image/x-exr"),
    f!("svg","SVG",Vector,["svg"],"image/svg+xml"),
    f!("mp4","MP4",Video,["mp4","m4v"],"video/mp4"),
    f!("mov","MOV",Video,["mov"],"video/quicktime"),
    f!("webm","WebM",Video,["webm"],"video/webm"),
    f!("mkv","MKV",Video,["mkv"],"video/x-matroska"),
    f!("avi","AVI",Video,["avi"],"video/x-msvideo"),
    f!("mp3","MP3",Audio,["mp3"],"audio/mpeg"),
    f!("wav","WAV",Audio,["wav"],"audio/wav"),
    f!("flac","FLAC",Audio,["flac"],"audio/flac"),
    f!("aac","AAC",Audio,["aac"],"audio/aac"),
    f!("m4a","M4A",Audio,["m4a"],"audio/mp4"),
    f!("ogg","OGG",Audio,["ogg","oga"],"audio/ogg"),
    f!("opus","Opus",Audio,["opus"],"audio/opus"),
    f!("pdf","PDF",Pdf,["pdf"],"application/pdf"),
    f!("docx","DOCX",Document,["docx"],"application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    f!("doc","DOC",Document,["doc"],"application/msword"),
    f!("odt","ODT",Document,["odt"],"application/vnd.oasis.opendocument.text"),
    f!("rtf","RTF",Document,["rtf"],"application/rtf"),
    f!("txt","Plain text",Document,["txt"],"text/plain"),
    f!("html","HTML",Document,["html","htm"],"text/html"),
    f!("pptx","PPTX",Presentation,["pptx"],"application/vnd.openxmlformats-officedocument.presentationml.presentation"),
    f!("ppt","PPT",Presentation,["ppt"],"application/vnd.ms-powerpoint"),
    f!("odp","ODP",Presentation,["odp"],"application/vnd.oasis.opendocument.presentation"),
    f!("xlsx","XLSX",Spreadsheet,["xlsx"],"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    f!("xls","XLS",Spreadsheet,["xls"],"application/vnd.ms-excel"),
    f!("ods","ODS",Spreadsheet,["ods"],"application/vnd.oasis.opendocument.spreadsheet"),
    f!("csv","CSV",Spreadsheet,["csv"],"text/csv"),
];

pub fn format_by_id(id: &str) -> Option<&'static Format> {
    let q = id.trim_start_matches('.').to_ascii_lowercase();
    FORMATS.iter().find(|x| x.id == q || x.extensions.iter().any(|e| *e == q))
}

pub fn format_by_extension(path: &Path) -> Option<&'static Format> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    FORMATS.iter().find(|x| x.extensions.iter().any(|e| *e == ext))
}
