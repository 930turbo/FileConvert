use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported input format: {0}")]
    UnsupportedInput(PathBuf),
    #[error("unknown target format: {0}")]
    UnknownTarget(String),
    #[error("no conversion route from {from} to {to}")]
    NoRoute { from: String, to: String },
    #[error("required conversion engine is not available: {0}")]
    EngineUnavailable(&'static str),
    #[error("conversion engine {engine} failed: {message}")]
    EngineFailed { engine: &'static str, message: String },
    #[error("output already exists and could not be renamed: {0}")]
    OutputCollision(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("invalid file name: {0}")]
    InvalidName(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
