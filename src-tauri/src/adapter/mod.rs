use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub mod sidecar;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Sidecar process failed: {0}")]
    ProcessFailed(String),
    #[error("JSON serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub index: usize,
    pub hex: String,
    pub brand: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbroideryMetadata {
    pub format: String,
    pub stitches: usize,
    pub colors: usize,
    #[serde(alias = "widthMm", alias = "width_mm")]
    pub width_mm: f64,
    #[serde(alias = "heightMm", alias = "height_mm")]
    pub height_mm: f64,
    pub bounds: (f64, f64, f64, f64),
    pub jumps: usize,
    pub trims: usize,
    pub threads: Vec<ThreadInfo>,
    pub filename: String,
}


pub trait EmbroideryFormatAdapter: Send + Sync {
    fn inspect(&self, path: &Path) -> Result<EmbroideryMetadata, AdapterError>;
    fn render_preview(&self, path: &Path, out_png: &Path, out_svg: Option<&Path>) -> Result<(), AdapterError>;
    fn export(&self, src_path: &Path, dst_path: &Path, target_format: &str) -> Result<(), AdapterError>;
}
