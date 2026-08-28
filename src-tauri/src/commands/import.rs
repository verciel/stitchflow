use crate::db::{checksum, now};
use crate::models::{Design, ImportResult};
use crate::AppState;
use rusqlite::params;
use std::fs;
use std::path::PathBuf;

use tauri::State;
use uuid::Uuid;

const EMBROIDERY_FORMATS: &[&str] = &[
    "dst", "pes", "jef", "vp3", "exp", "hus", "xxx", "sew", "pcs", "pec",
];
const ARTWORK_FORMATS: &[&str] = &["png", "jpg", "jpeg", "svg", "pdf"];

fn clean_title(stem: &str) -> String {
    let raw = stem.replace(['_', '-'], " ");
    raw.split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_paths_recursive(raw_path: &str, out: &mut Vec<String>) {
    let p = PathBuf::from(raw_path);
    if p.is_file() {
        out.push(raw_path.to_string());
    } else if p.is_dir() {
        if let Ok(entries) = fs::read_dir(&p) {
            for entry in entries.flatten() {
                let sub_p = entry.path();
                if sub_p.is_file() {
                    if let Some(ext) = sub_p.extension().and_then(|x| x.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if EMBROIDERY_FORMATS.contains(&ext_lower.as_str())
                            || ARTWORK_FORMATS.contains(&ext_lower.as_str())
                        {
                            out.push(sub_p.to_string_lossy().to_string());
                        }
                    }
                } else if sub_p.is_dir() {
                    collect_paths_recursive(&sub_p.to_string_lossy(), out);
                }
            }
        }
    }
}

#[tauri::command]
pub fn import_files(
    state: State<AppState>,
    paths: Vec<String>,
    duplicate_policy: String,
) -> Result<Vec<ImportResult>, String> {
    let mut resolved_paths = Vec::new();
    for p in paths {
        collect_paths_recursive(&p, &mut resolved_paths);
    }

    let mut results = Vec::new();
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut imported_count = 0;
    let mut duplicate_count = 0;
    let mut failed_count = 0;

    for raw_path in resolved_paths {
        let src = PathBuf::from(&raw_path);
        let ext = src
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_lowercase();


        let is_embroidery = EMBROIDERY_FORMATS.contains(&ext.as_str());
        let is_artwork = ARTWORK_FORMATS.contains(&ext.as_str());

        if !is_embroidery && !is_artwork {
            results.push(ImportResult {
                path: raw_path,
                status: "unsupported".into(),
                design: None,
                message: Some(format!(
                    "Format .{ext} is not supported. Supported embroidery: DST, PES, JEF, VP3, EXP, HUS, XXX, SEW, PCS, PEC."
                )),
            });
            failed_count += 1;
            continue;
        }

        if !src.is_file() {
            results.push(ImportResult {
                path: raw_path,
                status: "invalid".into(),
                design: None,
                message: Some("The specified file does not exist or is inaccessible.".into()),
            });
            failed_count += 1;
            continue;
        }

        let hash = match checksum(&src) {
            Ok(h) => h,
            Err(e) => {
                results.push(ImportResult {
                    path: raw_path,
                    status: "failed".into(),
                    design: None,
                    message: Some(format!("Could not calculate checksum: {e}")),
                });
                failed_count += 1;
                continue;
            }
        };

        // Handle Artwork Asset Import
        if is_artwork {
            let existing_artwork: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM artwork_assets WHERE checksum = ?1 AND status = 'active')",
                    params![hash],
                    |r| r.get(0),
                )
                .unwrap_or(false);

            if existing_artwork && duplicate_policy == "skip" {
                results.push(ImportResult {
                    path: raw_path,
                    status: "duplicate".into(),
                    design: None,
                    message: Some("Exact duplicate artwork already exists in library.".into()),
                });
                duplicate_count += 1;
                continue;
            }

            let asset_id = Uuid::new_v4().to_string();
            let file_name = src
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("artwork")
                .to_string();
            let dst = state
                .library_root
                .join("library/artwork")
                .join(format!("{asset_id}-{file_name}"));

            if let Err(e) = fs::copy(&src, &dst) {
                results.push(ImportResult {
                    path: raw_path,
                    status: "failed".into(),
                    design: None,
                    message: Some(format!("Failed to copy artwork into library: {e}")),
                });
                failed_count += 1;
                continue;
            }

            let size = fs::metadata(&dst).map(|m| m.len() as i64).unwrap_or(0);
            let mime = format!("image/{ext}");

            let _ = db.execute(
                "INSERT INTO artwork_assets(id, filename, managed_path, checksum, mime_type, size_bytes, source_path, status, imported_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
                params![
                    asset_id,
                    file_name,
                    dst.to_string_lossy(),
                    hash,
                    mime,
                    size,
                    raw_path,
                    now()
                ],
            );

            results.push(ImportResult {
                path: raw_path,
                status: "imported".into(),
                design: None,
                message: Some("Source artwork asset imported successfully.".into()),
            });
            imported_count += 1;
            continue;
        }

        // Handle Embroidery File Import
        let existing_design: Option<(String, String)> = db
            .query_row(
                "SELECT id, title FROM designs WHERE checksum = ?1 AND status = 'active' LIMIT 1",
                params![hash],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        let is_duplicate = existing_design.is_some();

        if is_duplicate && duplicate_policy == "skip" {
            let (_, title) = existing_design.unwrap();
            results.push(ImportResult {
                path: raw_path,
                status: "duplicate".into(),
                design: None,
                message: Some(format!("Exact duplicate of '{title}' skipped.")),
            });
            duplicate_count += 1;
            continue;
        }

        if is_duplicate && duplicate_policy == "replace_revision" {
            let (target_design_id, _) = existing_design.unwrap();

            let rev_num: i64 = db
                .query_row(
                    "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM design_revisions WHERE design_id = ?1",
                    params![target_design_id],
                    |r| r.get(0),
                )
                .unwrap_or(2);

            let file_name = src
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or("design")
                .to_string();

            let dst = state.library_root.join("library/designs").join(format!(
                "{target_design_id}-rev{rev_num}-{file_name}"
            ));

            if let Err(e) = fs::copy(&src, &dst) {
                results.push(ImportResult {
                    path: raw_path,
                    status: "failed".into(),
                    design: None,
                    message: Some(format!("Failed to write replacement revision: {e}")),
                });
                failed_count += 1;
                continue;
            }

            let meta_res = state.adapter.inspect(&dst);
            let prev_png = state
                .library_root
                .join("library/previews")
                .join(format!("{target_design_id}.png"));
            let prev_svg = state
                .library_root
                .join("library/previews")
                .join(format!("{target_design_id}.svg"));
            let _ = state.adapter.render_preview(&dst, &prev_png, Some(&prev_svg));

            let size = fs::metadata(&dst).map(|m| m.len() as i64).unwrap_or(0);
            let time = now();

            let (w, h, st, col, threads_json) = if let Ok(ref m) = meta_res {
                (
                    Some(m.width_mm),
                    Some(m.height_mm),
                    Some(m.stitches as i64),
                    Some(m.colors as i64),
                    serde_json::to_string(&m.threads).unwrap_or_default(),
                )
            } else {
                (None, None, None, None, "[]".into())
            };

            // Add revision record
            let _ = db.execute(
                "INSERT INTO design_revisions(id, design_id, revision_number, filename, managed_path, checksum, format, size_bytes, created_at, note)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Replaced file revision')",
                params![
                    Uuid::new_v4().to_string(),
                    target_design_id,
                    rev_num,
                    file_name,
                    dst.to_string_lossy(),
                    hash,
                    ext.to_uppercase(),
                    size,
                    time
                ],
            );

            // Update main design record
            let _ = db.execute(
                "UPDATE designs SET 
                    managed_path = ?1, 
                    preview_path = ?2,
                    checksum = ?3,
                    format = ?4,
                    width_mm = ?5,
                    height_mm = ?6,
                    stitches = ?7,
                    colors = ?8,
                    size_bytes = ?9,
                    threads_json = ?10
                 WHERE id = ?11",
                params![
                    dst.to_string_lossy(),
                    prev_png.to_string_lossy(),
                    hash,
                    ext.to_uppercase(),
                    w,
                    h,
                    st,
                    col,
                    size,
                    threads_json,
                    target_design_id
                ],
            );

            results.push(ImportResult {
                path: raw_path,
                status: "imported".into(),
                design: None,
                message: Some(format!("Updated as revision #{rev_num}.")),
            });
            imported_count += 1;
            continue;
        }

        // New design (or keep_both)
        let design_id = Uuid::new_v4().to_string();
        let file_name = src
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("design")
            .to_string();
        let stem = src
            .file_stem()
            .and_then(|x| x.to_str())
            .unwrap_or("Untitled");
        let title = clean_title(stem);

        let dst = state
            .library_root
            .join("library/designs")
            .join(format!("{design_id}-{file_name}"));

        if let Err(e) = fs::copy(&src, &dst) {
            results.push(ImportResult {
                path: raw_path,
                status: "failed".into(),
                design: None,
                message: Some(format!("Failed to copy design to managed folder: {e}")),
            });
            failed_count += 1;
            continue;
        }

        let meta_res = state.adapter.inspect(&dst);
        let prev_png = state
            .library_root
            .join("library/previews")
            .join(format!("{design_id}.png"));
        let prev_svg = state
            .library_root
            .join("library/previews")
            .join(format!("{design_id}.svg"));
        let _ = state.adapter.render_preview(&dst, &prev_png, Some(&prev_svg));

        let size = fs::metadata(&dst).map(|m| m.len() as i64).unwrap_or(0);
        let time = now();

        let (w, h, st, col, threads_json, threads_vec) = if let Ok(ref m) = meta_res {
            (
                Some(m.width_mm),
                Some(m.height_mm),
                Some(m.stitches as i64),
                Some(m.colors as i64),
                serde_json::to_string(&m.threads).unwrap_or_default(),
                m.threads.clone(),
            )
        } else {
            (None, None, None, None, "[]".into(), vec![])
        };

        if let Err(e) = db.execute(
            "INSERT INTO designs(id, title, filename, managed_path, preview_path, checksum, format, width_mm, height_mm, stitches, colors, size_bytes, source_path, status, threads_json, imported_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'active', ?14, ?15)",
            params![
                design_id,
                title,
                file_name,
                dst.to_string_lossy(),
                prev_png.to_string_lossy(),
                hash,
                ext.to_uppercase(),
                w,
                h,
                st,
                col,
                size,
                raw_path,
                threads_json,
                time
            ],
        ) {
            results.push(ImportResult {
                path: raw_path,
                status: "failed".into(),
                design: None,
                message: Some(format!("Database insert failed: {e}")),
            });
            failed_count += 1;
            continue;
        }

        // Insert revision 1
        let _ = db.execute(
            "INSERT INTO design_revisions(id, design_id, revision_number, filename, managed_path, checksum, format, size_bytes, created_at, note)
             VALUES(?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, 'Initial import')",
            params![
                Uuid::new_v4().to_string(),
                design_id,
                file_name,
                dst.to_string_lossy(),
                hash,
                ext.to_uppercase(),
                size,
                time
            ],
        );

        let created_design = Design {
            id: design_id,
            title,
            filename: file_name,
            format: ext.to_uppercase(),
            width_mm: w,
            height_mm: h,
            stitches: st,
            colors: col,
            size_bytes: size,
            tags: vec![],
            collection: None,
            collection_id: None,
            job: None,
            job_id: None,
            imported_at: time,
            duplicate: is_duplicate,
            preview_url: Some(prev_png.to_string_lossy().to_string()),
            preview_path: Some(prev_png.to_string_lossy().to_string()),
            managed_path: Some(dst.to_string_lossy().to_string()),
            status: "active".into(),
            ai_category: None,
            ai_subject: None,
            ai_style: None,
            ai_description: None,
            dominant_colors: vec![],
            threads: threads_vec,
        };

        results.push(ImportResult {
            path: raw_path,
            status: if is_duplicate {
                "duplicate".into()
            } else {
                "imported".into()
            },
            design: Some(created_design),
            message: Some(format!(
                "{} · {} sts · {:.1}×{:.1} mm",
                ext.to_uppercase(),
                st.unwrap_or(0),
                w.unwrap_or(0.0),
                h.unwrap_or(0.0)
            )),
        });
        imported_count += 1;
    }


    // Record audit entry in imports table
    let _ = db.execute(
        "INSERT INTO imports(id, total_files, imported_count, duplicate_count, failed_count, timestamp)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            Uuid::new_v4().to_string(),
            results.len() as i64,
            imported_count,
            duplicate_count,
            failed_count,
            now()
        ],
    );

    Ok(results)
}
