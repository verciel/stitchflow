use crate::db::now;
use crate::models::InkstitchConfig;
use crate::AppState;
use rusqlite::params;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<HashMap<String, String>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare("SELECT key, value FROM user_settings")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for item in rows.flatten() {
        map.insert(item.0, item.1);
    }

    Ok(map)
}

#[tauri::command]
pub fn save_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    db.execute(
        "INSERT INTO user_settings(key, value, updated_at) VALUES(?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
        params![key.trim(), value.trim(), now()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_inkstitch_config(state: State<AppState>) -> Result<InkstitchConfig, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let path_str: String = db
        .query_row(
            "SELECT value FROM user_settings WHERE key = 'inkscape_path'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let configured = !path_str.trim().is_empty() && Path::new(&path_str).exists();

    Ok(InkstitchConfig {
        inkscape_path: path_str,
        is_configured: configured,
    })
}

#[tauri::command]
pub fn set_inkstitch_config(state: State<AppState>, path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !path.trim().is_empty() && !p.exists() {
        return Err(format!("The path '{path}' does not exist on disk."));
    }

    save_setting(state, "inkscape_path".into(), path)
}

#[tauri::command]
pub fn open_in_inkstitch(state: State<AppState>, design_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let inkscape_path: String = db
        .query_row(
            "SELECT value FROM user_settings WHERE key = 'inkscape_path'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();

    if inkscape_path.trim().is_empty() || !Path::new(&inkscape_path).exists() {
        return Err("Inkscape / Ink/Stitch executable is not configured. Please specify the path in Settings.".into());
    }

    let managed_path_str: String = db
        .query_row(
            "SELECT managed_path FROM designs WHERE id = ?1",
            params![design_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Design not found: {e}"))?;

    // Check if an SVG file exists or pass the design file
    let preview_svg = state
        .library_root
        .join("library/previews")
        .join(format!("{design_id}.svg"));
    let target_file = if preview_svg.exists() {
        preview_svg.to_string_lossy().to_string()
    } else {
        managed_path_str
    };

    Command::new(&inkscape_path)
        .arg(&target_file)
        .spawn()
        .map_err(|e| format!("Failed to launch Inkscape: {e}"))?;

    Ok(())
}
