use crate::adapter::ThreadInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Design {
    pub id: String,
    pub title: String,
    pub filename: String,
    pub format: String,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitches: Option<i64>,
    pub colors: Option<i64>,
    pub size_bytes: i64,
    pub tags: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<String>,
    pub job: Option<String>,
    pub job_id: Option<String>,
    pub imported_at: String,
    pub duplicate: bool,
    pub preview_url: Option<String>,
    pub preview_path: Option<String>,
    pub managed_path: Option<String>,
    pub status: String,
    pub ai_category: Option<String>,
    pub ai_subject: Option<String>,
    pub ai_style: Option<String>,
    pub ai_description: Option<String>,
    pub dominant_colors: Vec<String>,
    pub threads: Vec<ThreadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignRevision {
    pub id: String,
    pub design_id: String,
    pub revision_number: i64,
    pub filename: String,
    pub managed_path: String,
    pub checksum: String,
    pub format: String,
    pub size_bytes: i64,
    pub created_at: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkAsset {
    pub id: String,
    pub filename: String,
    pub managed_path: String,
    pub preview_url: Option<String>,
    pub checksum: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub source_path: Option<String>,
    pub imported_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub design_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub status: String,
    pub design_count: i64,
    pub artwork_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub path: String,
    pub status: String,
    pub design: Option<Design>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    pub id: String,
    pub design_id: String,
    pub category: Option<String>,
    pub subject: Option<String>,
    pub style: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub dominant_colors: Vec<String>,
    pub confidence: f64,
    pub status: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InkstitchConfig {
    pub inkscape_path: String,
    pub is_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignDetails {
    pub design: Design,
    pub revisions: Vec<DesignRevision>,
    pub linked_artwork: Vec<ArtworkAsset>,
    pub linked_jobs: Vec<Job>,
    pub pending_suggestions: Vec<AiSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    pub query: Option<String>,
    pub tag: Option<String>,
    pub format: Option<String>,
    pub collection_id: Option<String>,
    pub job_id: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
}
