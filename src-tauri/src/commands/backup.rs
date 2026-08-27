use crate::db::now;
use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub version: String,
    pub created_at: String,
    pub app_version: String,
    pub design_count: i64,
    pub artwork_count: i64,
    pub file_checksums: HashMap<String, String>,
}

#[tauri::command]
pub fn create_backup(
    state: State<AppState>,
    destination_dir: Option<String>,
) -> Result<String, String> {
    let out_dir = match destination_dir {
        Some(d) => PathBuf::from(d),
        None => state.library_root.join("backups"),
    };
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let zip_name = format!("stitchflow-backup-{timestamp}.zip");
    let zip_path = out_dir.join(&zip_name);

    let zip_file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut checksums = HashMap::new();

    // Checkpoint SQLite WAL to make sure stitchflow.db is complete
    {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        let _ = db.execute_batch("PRAGMA wal_checkpoint(FULL);");
    }

    let db_src = state.library_root.join("stitchflow.db");
    if db_src.exists() {
        let bytes = fs::read(&db_src).map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        zip.start_file("stitchflow.db", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
        checksums.insert("stitchflow.db".to_string(), hash);
    }

    // Helper to zip directory recursively
    let folders = [
        ("library/designs", "designs"),
        ("library/artwork", "artwork"),
        ("library/previews", "previews"),
    ];

    for (rel_src, zip_folder) in folders {
        let src_dir = state.library_root.join(rel_src);
        if src_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            let zip_rel = format!("{zip_folder}/{file_name}");
                            if let Ok(bytes) = fs::read(&path) {
                                let hash = format!("{:x}", Sha256::digest(&bytes));
                                if zip.start_file(&zip_rel, options).is_ok() {
                                    let _ = zip.write_all(&bytes);
                                    checksums.insert(zip_rel, hash);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Counts from database
    let (design_count, artwork_count) = {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        let d_c: i64 = db
            .query_row("SELECT COUNT(*) FROM designs", [], |r| r.get(0))
            .unwrap_or(0);
        let a_c: i64 = db
            .query_row("SELECT COUNT(*) FROM artwork_assets", [], |r| r.get(0))
            .unwrap_or(0);
        (d_c, a_c)
    };

    // Write manifest.json
    let manifest = BackupManifest {
        version: "1.0".into(),
        created_at: now(),
        app_version: "0.1.0".into(),
        design_count,
        artwork_count,
        file_checksums: checksums,
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;

    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn validate_backup(archive_path: String) -> Result<BackupManifest, String> {
    let path = PathBuf::from(&archive_path);
    if !path.is_file() {
        return Err("Backup archive file not found".into());
    }

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    // Read manifest
    let mut manifest_file = archive
        .by_name("manifest.json")
        .map_err(|_| "Archive does not contain a valid manifest.json".to_string())?;

    let mut manifest_str = String::new();
    manifest_file
        .read_to_string(&mut manifest_str)
        .map_err(|e| e.to_string())?;
    drop(manifest_file);

    let manifest: BackupManifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Invalid manifest format: {e}"))?;

    // Validate checksums
    for (rel_path, expected_hash) in &manifest.file_checksums {
        let mut f = archive
            .by_name(rel_path)
            .map_err(|_| format!("Missing file in archive: {rel_path}"))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        let actual_hash = format!("{:x}", Sha256::digest(&buf));
        if actual_hash != *expected_hash {
            return Err(format!(
                "Checksum mismatch for {rel_path}: expected {expected_hash}, got {actual_hash}"
            ));
        }
    }

    Ok(manifest)
}

#[tauri::command]
pub fn restore_backup(
    state: State<AppState>,
    archive_path: String,
    target_directory: String,
) -> Result<String, String> {
    // 1. Validate
    let _ = validate_backup(archive_path.clone())?;

    let restore_dir = PathBuf::from(&target_directory);

    // Safety guard: ensure restore_dir does not match current active library root
    if restore_dir == state.library_root {
        return Err("Restoring into the active library directory is not permitted. Please choose a separate restore location.".into());
    }

    fs::create_dir_all(&restore_dir).map_err(|e| e.to_string())?;

    let file = File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match file.enclosed_name() {
            Some(path) => restore_dir.join(path),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    Ok(restore_dir.to_string_lossy().to_string())
}
