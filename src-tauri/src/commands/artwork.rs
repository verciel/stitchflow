use crate::models::ArtworkAsset;
use crate::AppState;
use rusqlite::params;
use std::fs;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn list_artwork(state: State<AppState>) -> Result<Vec<ArtworkAsset>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare(
            "SELECT id, filename, managed_path, checksum, mime_type, size_bytes, source_path, status, imported_at
             FROM artwork_assets WHERE status = 'active' ORDER BY imported_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            let m_path: String = r.get(2)?;
            Ok(ArtworkAsset {
                id: r.get(0)?,
                filename: r.get(1)?,
                preview_url: Some(m_path.clone()),
                managed_path: m_path,
                checksum: r.get(3)?,
                mime_type: r.get(4)?,
                size_bytes: r.get(5)?,
                source_path: r.get(6).ok(),
                status: r.get(7)?,
                imported_at: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let assets = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(assets)
}

#[tauri::command]
pub fn link_artwork_to_design(
    state: State<AppState>,
    design_id: String,
    asset_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "INSERT OR IGNORE INTO design_assets(design_id, asset_id) VALUES(?1, ?2)",
        params![design_id, asset_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn unlink_artwork_from_design(
    state: State<AppState>,
    design_id: String,
    asset_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "DELETE FROM design_assets WHERE design_id = ?1 AND asset_id = ?2",
        params![design_id, asset_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_artwork(state: State<AppState>, asset_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let path_str: String = db
        .query_row(
            "SELECT managed_path FROM artwork_assets WHERE id = ?1",
            params![asset_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let path = PathBuf::from(path_str);
    if path.exists() {
        let _ = fs::remove_file(path);
    }

    db.execute("DELETE FROM artwork_assets WHERE id = ?1", params![asset_id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn read_image_data(path: String) -> Result<String, String> {
    use base64::prelude::*;
    let clean_path = path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let p = std::path::Path::new(&clean_path);
    if !p.exists() || !p.is_file() {
        return Err("Image file not found".into());
    }

    let bytes = fs::read(p).map_err(|e| e.to_string())?;

    let ext = p
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    let encoded = BASE64_STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

