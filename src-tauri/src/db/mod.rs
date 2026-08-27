use chrono::Utc;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;


pub fn now() -> String {
    Utc::now().to_rfc3339()
}

pub fn checksum(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn initialize(root: &Path) -> Result<Connection, String> {
    fs::create_dir_all(root.join("library/designs")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("library/artwork")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("library/previews")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("recycle")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("backups")).map_err(|e| e.to_string())?;

    let db_path = root.join("stitchflow.db");
    let db = Connection::open(&db_path).map_err(|e| e.to_string())?;

    db.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS designs (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             filename TEXT NOT NULL,
             managed_path TEXT NOT NULL,
             preview_path TEXT,
             checksum TEXT NOT NULL,
             format TEXT NOT NULL,
             width_mm REAL,
             height_mm REAL,
             stitches INTEGER,
             colors INTEGER,
             size_bytes INTEGER NOT NULL,
             source_path TEXT,
             status TEXT NOT NULL DEFAULT 'active',
             ai_category TEXT,
             ai_subject TEXT,
             ai_style TEXT,
             ai_description TEXT,
             dominant_colors TEXT,
             threads_json TEXT,
             imported_at TEXT NOT NULL,
             deleted_at TEXT
         );

         CREATE INDEX IF NOT EXISTS idx_designs_status ON designs(status);
         CREATE INDEX IF NOT EXISTS idx_designs_checksum ON designs(checksum);

         CREATE TABLE IF NOT EXISTS design_revisions (
             id TEXT PRIMARY KEY,
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             revision_number INTEGER NOT NULL,
             filename TEXT NOT NULL,
             managed_path TEXT NOT NULL,
             checksum TEXT NOT NULL,
             format TEXT NOT NULL,
             size_bytes INTEGER NOT NULL,
             created_at TEXT NOT NULL,
             note TEXT NOT NULL DEFAULT ''
         );

         CREATE TABLE IF NOT EXISTS tags (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL UNIQUE
         );

         CREATE TABLE IF NOT EXISTS design_tags (
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             PRIMARY KEY(design_id, tag_id)
         );

         CREATE TABLE IF NOT EXISTS collections (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL UNIQUE,
             description TEXT NOT NULL DEFAULT '',
             created_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS collection_designs (
             collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             PRIMARY KEY(collection_id, design_id)
         );

         CREATE TABLE IF NOT EXISTS jobs (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             notes TEXT NOT NULL DEFAULT '',
             status TEXT NOT NULL DEFAULT 'draft',
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS job_designs (
             job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             PRIMARY KEY(job_id, design_id)
         );

         CREATE TABLE IF NOT EXISTS artwork_assets (
             id TEXT PRIMARY KEY,
             filename TEXT NOT NULL,
             managed_path TEXT NOT NULL,
             checksum TEXT NOT NULL,
             mime_type TEXT NOT NULL,
             size_bytes INTEGER NOT NULL,
             source_path TEXT,
             status TEXT NOT NULL DEFAULT 'active',
             imported_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS design_assets (
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             asset_id TEXT NOT NULL REFERENCES artwork_assets(id) ON DELETE CASCADE,
             PRIMARY KEY(design_id, asset_id)
         );

         CREATE TABLE IF NOT EXISTS job_assets (
             job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
             asset_id TEXT NOT NULL REFERENCES artwork_assets(id) ON DELETE CASCADE,
             PRIMARY KEY(job_id, asset_id)
         );

         CREATE TABLE IF NOT EXISTS imports (
             id TEXT PRIMARY KEY,
             total_files INTEGER NOT NULL,
             imported_count INTEGER NOT NULL,
             duplicate_count INTEGER NOT NULL,
             failed_count INTEGER NOT NULL,
             timestamp TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS ai_analyses (
             id TEXT PRIMARY KEY,
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             provider TEXT,
             model TEXT,
             prompt TEXT,
             status TEXT NOT NULL,
             created_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS ai_suggestions (
             id TEXT PRIMARY KEY,
             analysis_id TEXT,
             design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
             category TEXT,
             subject TEXT,
             style TEXT,
             description TEXT,
             proposed_tags TEXT,
             dominant_colors TEXT,
             confidence REAL,
             status TEXT NOT NULL DEFAULT 'pending',
             provider TEXT,
             model TEXT,
             created_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS user_settings (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS provider_configurations (
             id TEXT PRIMARY KEY,
             provider_type TEXT NOT NULL,
             base_url TEXT NOT NULL,
             model TEXT NOT NULL,
             is_active INTEGER NOT NULL DEFAULT 1,
             created_at TEXT NOT NULL
         );

         CREATE VIRTUAL TABLE IF NOT EXISTS design_search USING fts5(
             design_id UNINDEXED,
             title,
             filename,
             tags,
             ai_category,
             ai_subject,
             ai_description
         );
        ",
    )
    .map_err(|e| format!("Schema initialization error: {e}"))?;

    // Migrate any missing columns in existing pre-release databases
    migrate_schema(&db)?;

    // Add triggers to keep FTS5 in sync with designs
    let _ = db.execute_batch(
        "DROP TRIGGER IF EXISTS trg_designs_ai;
         CREATE TRIGGER trg_designs_ai AFTER INSERT ON designs BEGIN
             INSERT INTO design_search(design_id, title, filename, tags, ai_category, ai_subject, ai_description)
             VALUES(new.id, new.title, new.filename, '', COALESCE(new.ai_category, ''), COALESCE(new.ai_subject, ''), COALESCE(new.ai_description, ''));
         END;

         DROP TRIGGER IF EXISTS trg_designs_ad;
         CREATE TRIGGER trg_designs_ad AFTER DELETE ON designs BEGIN
             DELETE FROM design_search WHERE design_id = old.id;
         END;

         DROP TRIGGER IF EXISTS trg_designs_au;
         CREATE TRIGGER trg_designs_au AFTER UPDATE ON designs BEGIN
             UPDATE design_search SET
                 title = new.title,
                 filename = new.filename,
                 ai_category = COALESCE(new.ai_category, ''),
                 ai_subject = COALESCE(new.ai_subject, ''),
                 ai_description = COALESCE(new.ai_description, '')
             WHERE design_id = new.id;
         END;
        ",
    );

    seed_initial_data(&db, root)?;

    Ok(db)
}

fn migrate_schema(db: &Connection) -> Result<(), String> {
    // 1. Migrate designs table columns
    let existing_design_cols: Vec<String> = {
        let mut stmt = db
            .prepare("PRAGMA table_info(designs)")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };

    let required_design_cols = [
        ("preview_path", "TEXT"),
        ("width_mm", "REAL"),
        ("height_mm", "REAL"),
        ("stitches", "INTEGER"),
        ("colors", "INTEGER"),
        ("ai_category", "TEXT"),
        ("ai_subject", "TEXT"),
        ("ai_style", "TEXT"),
        ("ai_description", "TEXT"),
        ("dominant_colors", "TEXT"),
        ("threads_json", "TEXT"),
    ];

    for (col_name, col_type) in required_design_cols {
        if !existing_design_cols
            .iter()
            .any(|c| c.eq_ignore_ascii_case(col_name))
        {
            let _ = db.execute(
                &format!("ALTER TABLE designs ADD COLUMN {col_name} {col_type}"),
                [],
            );
        }
    }

    // 2. Migrate artwork_assets table columns
    let existing_art_cols: Vec<String> = {
        let mut stmt = db
            .prepare("PRAGMA table_info(artwork_assets)")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };

    if !existing_art_cols
        .iter()
        .any(|c| c.eq_ignore_ascii_case("size_bytes"))
    {
        let _ = db.execute(
            "ALTER TABLE artwork_assets ADD COLUMN size_bytes INTEGER NOT NULL DEFAULT 0",
            [],
        );
    }
    if !existing_art_cols
        .iter()
        .any(|c| c.eq_ignore_ascii_case("status"))
    {
        let _ = db.execute(
            "ALTER TABLE artwork_assets ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
            [],
        );
    }

    // 3. Migrate ai_suggestions table columns
    let existing_sugg_cols: Vec<String> = {
        let mut stmt = db
            .prepare("PRAGMA table_info(ai_suggestions)")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };

    let required_sugg_cols = [
        ("analysis_id", "TEXT"),
        ("category", "TEXT"),
        ("subject", "TEXT"),
        ("style", "TEXT"),
        ("description", "TEXT"),
        ("proposed_tags", "TEXT"),
        ("dominant_colors", "TEXT"),
    ];
    for (col_name, col_type) in required_sugg_cols {
        if !existing_sugg_cols
            .iter()
            .any(|c| c.eq_ignore_ascii_case(col_name))
        {
            let _ = db.execute(
                &format!("ALTER TABLE ai_suggestions ADD COLUMN {col_name} {col_type}"),
                [],
            );
        }
    }

    // 4. Validate FTS5 table columns: if ai_category is missing from design_search, recreate it
    let fts_sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'design_search'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();

    if !fts_sql.contains("ai_category") {
        let _ = db.execute_batch(
            "DROP TABLE IF EXISTS design_search;
             CREATE VIRTUAL TABLE design_search USING fts5(
                 design_id UNINDEXED,
                 title,
                 filename,
                 tags,
                 ai_category,
                 ai_subject,
                 ai_description
             );",
        );
    }

    Ok(())
}


fn seed_initial_data(db: &Connection, root: &Path) -> Result<(), String> {
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM designs", [], |r| r.get(0))
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }

    let timestamp = now();

    let samples = [
        (
            "sample-rose",
            "English Garden Rose",
            "garden_rose.pes",
            "PES",
            82.0,
            76.0,
            12480,
            5,
            vec!["floral", "rose", "botanical"],
            "Floral",
            "Red Rose",
            "Traditional satin stitch",
            "A delicate garden rose motif in bloom.",
            vec!["#d32f2f", "#388e3c", "#fbc02d"],
        ),
        (
            "sample-butterfly",
            "Meadow Butterfly",
            "meadow_butterfly.dst",
            "DST",
            94.0,
            68.0,
            9760,
            4,
            vec!["animal", "spring", "insect"],
            "Nature",
            "Butterfly",
            "Detailed fill",
            "Symmetrical butterfly with accented wings.",
            vec!["#0288d1", "#7b1fa2", "#ffb300"],
        ),
        (
            "sample-star",
            "Little Star Emblem",
            "little_star.jef",
            "JEF",
            38.0,
            37.0,
            2160,
            2,
            vec!["kids", "small", "celestial"],
            "Children",
            "Star",
            "Minimalist outline",
            "Simple five-point star with soft fill.",
            vec!["#fbc02d", "#f57c00"],
        ),
    ];

    for (
        id,
        title,
        filename,
        format,
        w,
        h,
        stitches,
        colors,
        tags,
        category,
        subject,
        style,
        desc,
        dominant_colors,
    ) in samples
    {
        let managed_path = root.join("library/designs").join(format!("{id}-{filename}"));
        if !managed_path.exists() {
            fs::write(
                &managed_path,
                format!("Stitchflow demo pattern: {title} ({format})"),
            )
            .map_err(|e| e.to_string())?;
        }

        let hash = checksum(&managed_path).unwrap_or_else(|_| "demo-hash".into());
        let size = fs::metadata(&managed_path)
            .map(|m| m.len() as i64)
            .unwrap_or(1024);

        let dom_colors_json = serde_json::to_string(&dominant_colors).unwrap_or_default();

        db.execute(
            "INSERT INTO designs(id, title, filename, managed_path, checksum, format, width_mm, height_mm, stitches, colors, size_bytes, source_path, status, ai_category, ai_subject, ai_style, ai_description, dominant_colors, imported_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'Stitchflow sample', 'active', ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                id,
                title,
                filename,
                managed_path.to_string_lossy(),
                hash,
                format,
                w,
                h,
                stitches,
                colors,
                size,
                category,
                subject,
                style,
                desc,
                dom_colors_json,
                timestamp
            ],
        )
        .map_err(|e| e.to_string())?;

        // Add initial revision
        db.execute(
            "INSERT INTO design_revisions(id, design_id, revision_number, filename, managed_path, checksum, format, size_bytes, created_at, note)
             VALUES(?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, 'Initial sample import')",
            params![
                format!("rev-{id}"),
                id,
                filename,
                managed_path.to_string_lossy(),
                hash,
                format,
                size,
                timestamp
            ],
        )
        .map_err(|e| e.to_string())?;

        // Add tags
        let tag_names = tags.join(" ");
        for tag in &tags {
            let tag_id = format!("tag-{tag}");
            let _ = db.execute(
                "INSERT OR IGNORE INTO tags(id, name) VALUES(?1, ?2)",
                params![tag_id, tag],
            );
            let _ = db.execute(
                "INSERT OR IGNORE INTO design_tags(design_id, tag_id) VALUES(?1, ?2)",
                params![id, tag_id],
            );
        }

        // Update tags in FTS5
        let _ = db.execute(
            "UPDATE design_search SET tags = ?1 WHERE design_id = ?2",
            params![tag_names, id],
        );
    }


    // Collections
    db.execute(
        "INSERT OR IGNORE INTO collections(id, name, description, created_at)
         VALUES('col-botanicals', 'Botanical Collection', 'Nature and floral embroidery motifs', ?1)",
        params![timestamp],
    )
    .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT OR IGNORE INTO collection_designs(collection_id, design_id)
         VALUES('col-botanicals', 'sample-rose')",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Jobs
    db.execute(
        "INSERT OR IGNORE INTO jobs(id, title, notes, status, created_at, updated_at)
         VALUES('job-spring-26', 'Spring Collection Batch', 'Prep sample garments for the trade show.', 'active', ?1, ?1)",
        params![timestamp],
    )
    .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT OR IGNORE INTO job_designs(job_id, design_id)
         VALUES('job-spring-26', 'sample-rose')",
        [],
    )
    .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT OR IGNORE INTO job_designs(job_id, design_id)
         VALUES('job-spring-26', 'sample-butterfly')",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Default settings
    let default_settings = [
        ("duplicate_policy", "skip"),
        ("ai_enabled", "false"),
        ("ai_endpoint", "https://api.openai.com/v1"),
        ("ai_model", "gpt-4o-mini"),
        ("inkscape_path", ""),
    ];

    for (k, v) in default_settings {
        let _ = db.execute(
            "INSERT OR IGNORE INTO user_settings(key, value, updated_at) VALUES(?1, ?2, ?3)",
            params![k, v, timestamp],
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_initialize_and_seed() {
        let tmp = std::env::temp_dir().join(format!("stitchflow_test_{}", uuid::Uuid::new_v4()));
        let db = initialize(&tmp).expect("Failed to initialize test DB");

        // Verify designs count
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM designs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);

        // Verify FTS5 sync
        let fts_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM design_search WHERE design_search MATCH 'Rose*'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 1);

        // Clean up
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_checksum() {
        let tmp = std::env::temp_dir().join(format!("test_hash_{}", uuid::Uuid::new_v4()));
        fs::write(&tmp, b"Stitchflow deterministic embroidery checksum test").unwrap();

        let hash = checksum(&tmp).unwrap();
        assert_eq!(hash.len(), 64);

        let _ = fs::remove_file(tmp);
    }
}



