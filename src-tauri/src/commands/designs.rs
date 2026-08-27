use crate::adapter::ThreadInfo;
use crate::db::now;
use crate::models::{
    ArtworkAsset, Design, DesignDetails, DesignRevision, FilterOptions, Job,
};
use crate::AppState;
use rusqlite::{params, Row};
use std::fs;
use std::path::{Path, PathBuf};

use std::process::Command;
use tauri::State;

pub fn row_to_design(row: &Row<'_>) -> rusqlite::Result<Design> {
    let tags_str: String = row.get(11).unwrap_or_default();
    let dom_colors_str: String = row.get(18).unwrap_or_default();
    let threads_json: String = row.get(19).unwrap_or_default();

    let tags = tags_str
        .split('|')
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();

    let dominant_colors = serde_json::from_str(&dom_colors_str).unwrap_or_default();
    let threads: Vec<ThreadInfo> = serde_json::from_str(&threads_json).unwrap_or_default();

    let collection_name: Option<String> = row.get(12).ok();
    let collection_id: Option<String> = row.get(13).ok();
    let job_title: Option<String> = row.get(14).ok();
    let job_id: Option<String> = row.get(15).ok();
    let preview_path: Option<String> = row.get(16).ok();
    let managed_path: Option<String> = row.get(17).ok();

    Ok(Design {
        id: row.get(0)?,
        title: row.get(1)?,
        filename: row.get(2)?,
        format: row.get(3)?,
        width_mm: row.get(4).ok(),
        height_mm: row.get(5).ok(),
        stitches: row.get(6).ok(),
        colors: row.get(7).ok(),
        size_bytes: row.get(8)?,
        imported_at: row.get(9)?,
        status: row.get(10)?,
        tags,
        collection: collection_name.filter(|s| !s.is_empty()),
        collection_id: collection_id.filter(|s| !s.is_empty()),
        job: job_title.filter(|s| !s.is_empty()),
        job_id: job_id.filter(|s| !s.is_empty()),
        preview_url: preview_path.clone(),
        preview_path,
        managed_path,
        duplicate: false,
        ai_category: row.get(20).ok(),
        ai_subject: row.get(21).ok(),
        ai_style: row.get(22).ok(),
        ai_description: row.get(23).ok(),
        dominant_colors,
        threads,
    })
}

const SELECT_DESIGN_BASE: &str = "
    SELECT 
        d.id,
        d.title,
        d.filename,
        d.format,
        d.width_mm,
        d.height_mm,
        d.stitches,
        d.colors,
        d.size_bytes,
        d.imported_at,
        d.status,
        COALESCE((SELECT GROUP_CONCAT(t.name, '|') FROM design_tags dt JOIN tags t ON t.id = dt.tag_id WHERE dt.design_id = d.id), ''),
        COALESCE((SELECT c.name FROM collection_designs cd JOIN collections c ON c.id = cd.collection_id WHERE cd.design_id = d.id LIMIT 1), ''),
        COALESCE((SELECT c.id FROM collection_designs cd JOIN collections c ON c.id = cd.collection_id WHERE cd.design_id = d.id LIMIT 1), ''),
        COALESCE((SELECT j.title FROM job_designs jd JOIN jobs j ON j.id = jd.job_id WHERE jd.design_id = d.id LIMIT 1), ''),
        COALESCE((SELECT j.id FROM job_designs jd JOIN jobs j ON j.id = jd.job_id WHERE jd.design_id = d.id LIMIT 1), ''),
        d.preview_path,
        d.managed_path,
        COALESCE(d.dominant_colors, '[]'),
        COALESCE(d.threads_json, '[]'),
        d.ai_category,
        d.ai_subject,
        d.ai_style,
        d.ai_description
    FROM designs d
";

#[tauri::command]
pub fn list_designs(
    state: State<AppState>,
    filters: Option<FilterOptions>,
) -> Result<Vec<Design>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let filters = filters.unwrap_or_default();

    let target_status = filters.status.unwrap_or_else(|| "active".into());

    let mut sql = format!("{SELECT_DESIGN_BASE} WHERE d.status = ?1 ");
    let mut param_values: Vec<String> = vec![target_status];

    // Search query via FTS5 or LIKE
    if let Some(ref q) = filters.query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let param_idx = param_values.len() + 1;
            sql.push_str(&format!(
                "AND (
                    d.id IN (SELECT design_id FROM design_search WHERE design_search MATCH ?{idx})
                    OR d.title LIKE ?{idx_like}
                    OR d.filename LIKE ?{idx_like}
                    OR d.ai_description LIKE ?{idx_like}
                ) ",
                idx = param_idx,
                idx_like = param_idx + 1
            ));
            // Format query for FTS5 (append wildcard)
            let fts_query = format!("{}*", trimmed.replace('"', ""));
            let like_query = format!("%{trimmed}%");
            param_values.push(fts_query);
            param_values.push(like_query);
        }
    }

    // Filter by tag
    if let Some(ref tag) = filters.tag {
        if !tag.trim().is_empty() {
            let idx = param_values.len() + 1;
            sql.push_str(&format!(
                "AND d.id IN (SELECT dt.design_id FROM design_tags dt JOIN tags t ON t.id = dt.tag_id WHERE t.name = ?{idx} OR t.id = ?{idx}) "
            ));
            param_values.push(tag.trim().to_string());
        }
    }

    // Filter by format
    if let Some(ref fmt) = filters.format {
        if !fmt.trim().is_empty() && fmt != "all" {
            let idx = param_values.len() + 1;
            sql.push_str(&format!("AND UPPER(d.format) = UPPER(?{idx}) "));
            param_values.push(fmt.trim().to_string());
        }
    }

    // Filter by collection
    if let Some(ref col_id) = filters.collection_id {
        if !col_id.trim().is_empty() {
            let idx = param_values.len() + 1;
            sql.push_str(&format!(
                "AND d.id IN (SELECT design_id FROM collection_designs WHERE collection_id = ?{idx}) "
            ));
            param_values.push(col_id.trim().to_string());
        }
    }

    // Filter by job
    if let Some(ref j_id) = filters.job_id {
        if !j_id.trim().is_empty() {
            let idx = param_values.len() + 1;
            sql.push_str(&format!(
                "AND d.id IN (SELECT design_id FROM job_designs WHERE job_id = ?{idx}) "
            ));
            param_values.push(j_id.trim().to_string());
        }
    }

    // Sort order
    let sort_sql = match filters.sort_by.as_deref() {
        Some("date_asc") => "ORDER BY d.imported_at ASC",
        Some("stitches_desc") => "ORDER BY d.stitches DESC NULLS LAST",
        Some("stitches_asc") => "ORDER BY d.stitches ASC NULLS LAST",
        Some("title_asc") => "ORDER BY d.title ASC",
        Some("size_desc") => "ORDER BY d.size_bytes DESC",
        _ => "ORDER BY d.imported_at DESC",
    };
    sql.push_str(sort_sql);

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let rows = stmt
        .query_map(&params_refs[..], row_to_design)
        .map_err(|e| e.to_string())?;

    let designs = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(designs)
}

#[tauri::command]
pub fn get_design_details(state: State<AppState>, id: String) -> Result<DesignDetails, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let sql = format!("{SELECT_DESIGN_BASE} WHERE d.id = ?1");
    let design = db
        .query_row(&sql, params![id], row_to_design)
        .map_err(|e| format!("Design not found: {e}"))?;

    // Load revisions
    let mut rev_stmt = db
        .prepare(
            "SELECT id, design_id, revision_number, filename, managed_path, checksum, format, size_bytes, created_at, note 
             FROM design_revisions WHERE design_id = ?1 ORDER BY revision_number DESC",
        )
        .map_err(|e| e.to_string())?;

    let rev_rows = rev_stmt
        .query_map(params![id], |r| {
            Ok(DesignRevision {
                id: r.get(0)?,
                design_id: r.get(1)?,
                revision_number: r.get(2)?,
                filename: r.get(3)?,
                managed_path: r.get(4)?,
                checksum: r.get(5)?,
                format: r.get(6)?,
                size_bytes: r.get(7)?,
                created_at: r.get(8)?,
                note: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let revisions = rev_rows.collect::<Result<Vec<_>, _>>().unwrap_or_default();

    // Load linked artwork
    let mut art_stmt = db
        .prepare(
            "SELECT a.id, a.filename, a.managed_path, a.checksum, a.mime_type, a.size_bytes, a.source_path, a.status, a.imported_at
             FROM artwork_assets a
             JOIN design_assets da ON da.asset_id = a.id
             WHERE da.design_id = ?1 AND a.status = 'active'
             ORDER BY a.imported_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let art_rows = art_stmt
        .query_map(params![id], |r| {
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
    let linked_artwork = art_rows.collect::<Result<Vec<_>, _>>().unwrap_or_default();

    // Load linked jobs
    let mut job_stmt = db
        .prepare(
            "SELECT j.id, j.title, j.notes, j.status, 
                    (SELECT COUNT(*) FROM job_designs WHERE job_id = j.id),
                    (SELECT COUNT(*) FROM job_assets WHERE job_id = j.id),
                    j.created_at, j.updated_at
             FROM jobs j
             JOIN job_designs jd ON jd.job_id = j.id
             WHERE jd.design_id = ?1
             ORDER BY j.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let job_rows = job_stmt
        .query_map(params![id], |r| {
            Ok(Job {
                id: r.get(0)?,
                title: r.get(1)?,
                notes: r.get(2)?,
                status: r.get(3)?,
                design_count: r.get(4)?,
                artwork_count: r.get(5)?,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let linked_jobs = job_rows.collect::<Result<Vec<_>, _>>().unwrap_or_default();

    // Load pending suggestions
    let mut sugg_stmt = db
        .prepare(
            "SELECT id, design_id, category, subject, style, description, proposed_tags, dominant_colors, confidence, status, provider, model, created_at
             FROM ai_suggestions WHERE design_id = ?1 AND status = 'pending' ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let sugg_rows = sugg_stmt
        .query_map(params![id], |r| {
            let tags_raw: String = r.get(6).unwrap_or_default();
            let colors_raw: String = r.get(7).unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
            let colors: Vec<String> = serde_json::from_str(&colors_raw).unwrap_or_default();

            Ok(crate::models::AiSuggestion {
                id: r.get(0)?,
                design_id: r.get(1)?,
                category: r.get(2).ok(),
                subject: r.get(3).ok(),
                style: r.get(4).ok(),
                description: r.get(5).ok(),
                tags,
                dominant_colors: colors,
                confidence: r.get(8).unwrap_or(0.0),
                status: r.get(9)?,
                provider: r.get(10).ok(),
                model: r.get(11).ok(),
                created_at: r.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let pending_suggestions = sugg_rows.collect::<Result<Vec<_>, _>>().unwrap_or_default();

    Ok(DesignDetails {
        design,
        revisions,
        linked_artwork,
        linked_jobs,
        pending_suggestions,
    })
}

#[tauri::command]
pub fn update_design_metadata(
    state: State<AppState>,
    id: String,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    if let Some(t) = title {
        db.execute(
            "UPDATE designs SET title = ?1 WHERE id = ?2",
            params![t.trim(), id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(d) = description {
        db.execute(
            "UPDATE designs SET ai_description = ?1 WHERE id = ?2",
            params![d.trim(), id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn delete_design(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let managed_path_str: String = db
        .query_row(
            "SELECT managed_path FROM designs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let src = PathBuf::from(&managed_path_str);
    if src.exists() {
        if let Some(file_name) = src.file_name() {
            let dst = state.library_root.join("recycle").join(file_name);
            let _ = fs::rename(&src, &dst);
        }
    }

    db.execute(
        "UPDATE designs SET status = 'recycled', deleted_at = ?1 WHERE id = ?2",
        params![now(), id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn restore_design(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let managed_path_str: String = db
        .query_row(
            "SELECT managed_path FROM designs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let orig_path = PathBuf::from(&managed_path_str);
    if let Some(file_name) = orig_path.file_name() {
        let recycled_path = state.library_root.join("recycle").join(file_name);
        if recycled_path.exists() {
            let _ = fs::rename(&recycled_path, &orig_path);
        }
    }

    db.execute(
        "UPDATE designs SET status = 'active', deleted_at = NULL WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn permanent_delete_design(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let (managed_path_str, preview_path_str): (String, Option<String>) = db
        .query_row(
            "SELECT managed_path, preview_path FROM designs WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1).ok())),
        )
        .map_err(|e| e.to_string())?;

    // Delete managed file if in recycle or library
    let managed_path = PathBuf::from(&managed_path_str);
    if managed_path.exists() {
        let _ = fs::remove_file(&managed_path);
    }
    if let Some(file_name) = managed_path.file_name() {
        let recycle_file = state.library_root.join("recycle").join(file_name);
        if recycle_file.exists() {
            let _ = fs::remove_file(recycle_file);
        }
    }

    // Delete preview file
    if let Some(p) = preview_path_str {
        let prev_path = PathBuf::from(p);
        if prev_path.exists() {
            let _ = fs::remove_file(prev_path);
        }
    }

    // Delete all related database rows
    db.execute("DELETE FROM designs WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn empty_recycle_bin(state: State<AppState>) -> Result<usize, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare("SELECT id FROM designs WHERE status = 'recycled'")
        .map_err(|e| e.to_string())?;

    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default();

    drop(stmt);
    drop(db);

    let count = ids.len();
    for id in ids {
        let _ = permanent_delete_design(state.clone(), id);
    }

    Ok(count)
}

#[tauri::command]
pub fn export_design(
    state: State<AppState>,
    id: String,
    target_path: String,
    target_format: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let managed_path_str: String = db
        .query_row(
            "SELECT managed_path FROM designs WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let src = PathBuf::from(managed_path_str);
    let dst = PathBuf::from(target_path);

    state
        .adapter
        .export(&src, &dst, &target_format)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err("File does not exist on disk".into());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open file explorer: {e}"))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(parent) = target.parent() {
            let _ = open::that(parent);
        }
    }

    Ok(())
}
