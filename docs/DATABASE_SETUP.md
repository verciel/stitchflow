# Stitchflow Database Architecture & Setup Guide

This document provides a comprehensive technical reference for the database architecture, schema definitions, indexing strategies, triggers, full-text search (FTS5), and integrity verification mechanisms used in **Stitchflow**.

---

## 1. Database Engine Overview

Stitchflow utilizes an embedded, zero-configuration **SQLite 3** engine managed via Rust's `rusqlite` crate. It operates locally with zero external server dependencies, ensuring complete offline operation and sub-millisecond query performance.

### Core PRAGMA Configuration
Every connection initialized by Stitchflow applies the following pragmas:
```sql
PRAGMA journal_mode = WAL;          -- Write-Ahead Logging for high concurrency
PRAGMA synchronous = NORMAL;        -- Balances durability with high write throughput
PRAGMA foreign_keys = ON;           -- Enforces strict relational integrity
PRAGMA busy_timeout = 5000;         -- 5-second wait timeout for locked operations
PRAGMA cache_size = -64000;         -- 64MB memory cache for catalog paging
```

### Storage Location
The SQLite database resides in the user's application data directory:
- **Windows**: `%LOCALAPPDATA%\Stitchflow\stitchflow.db`
- **macOS**: `~/Library/Application Support/Stitchflow/stitchflow.db`
- **Linux**: `~/.local/share/stitchflow/stitchflow.db`

---

## 2. Relational Schema Architecture

The database consists of **10 normalized relational tables** and **1 FTS5 virtual full-text search table**:

```text
┌─────────────────┐       ┌──────────────────────┐       ┌────────────────────┐
│   collections   │◄──────┤  collection_designs  ├──────►│      designs       │
└─────────────────┘       └──────────────────────┘       └─────────┬──────────┘
                                                                   │ 1
┌─────────────────┐       ┌──────────────────────┐                 │
│      jobs       │◄──────┤     job_designs      ├─────────────────┤
└─────────────────┘       └──────────────────────┘                 │
                                                                   │ *
┌─────────────────┐       ┌──────────────────────┐       ┌─────────▼──────────┐
│ artwork_assets  │◄──────┤    design_artwork    ├──────►│  design_revisions  │
└─────────────────┘       └──────────────────────┘       └────────────────────┘
                                                                   │ 1
┌─────────────────┐       ┌──────────────────────┐                 │
│      tags       │◄──────┤     design_tags      ├─────────────────┤
└─────────────────┘       └──────────────────────┘                 │ *
                                                                   │
                                                         ┌─────────▼──────────┐
                                                         │   thread_colors    │
                                                         └────────────────────┘
```

---

## 3. Detailed Table Definitions

### 3.1 `designs`
Stores primary metadata and production facts for each indexed embroidery pattern.
```sql
CREATE TABLE IF NOT EXISTS designs (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    filename        TEXT NOT NULL,
    managed_path    TEXT NOT NULL,
    source_path     TEXT,
    preview_path    TEXT,
    format          TEXT NOT NULL,
    width_mm        REAL,
    height_mm       REAL,
    stitches        INTEGER,
    colors          INTEGER,
    size_bytes      INTEGER NOT NULL,
    checksum        TEXT NOT NULL,
    imported_at     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active', -- 'active' | 'recycled' | 'archived'
    ai_category     TEXT,
    ai_subject      TEXT,
    ai_style        TEXT,
    ai_description  TEXT,
    dominant_colors TEXT,                          -- JSON Array: ["#D32F2F", "#388E3C"]
    collection_id   TEXT REFERENCES collections(id) ON DELETE SET NULL,
    job_id          TEXT REFERENCES jobs(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_designs_format ON designs(format);
CREATE INDEX IF NOT EXISTS idx_designs_status ON designs(status);
CREATE INDEX IF NOT EXISTS idx_designs_imported_at ON designs(imported_at DESC);
CREATE INDEX IF NOT EXISTS idx_designs_checksum ON designs(checksum);
```

### 3.2 `design_revisions`
Maintains immutable historical snapshots of modified or re-imported files.
```sql
CREATE TABLE IF NOT EXISTS design_revisions (
    id              TEXT PRIMARY KEY,
    design_id       TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL,
    filename        TEXT NOT NULL,
    managed_path    TEXT NOT NULL,
    checksum        TEXT NOT NULL,
    format          TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    created_at      TEXT NOT NULL,
    note            TEXT,
    UNIQUE(design_id, revision_number)
);

CREATE INDEX IF NOT EXISTS idx_revisions_design ON design_revisions(design_id);
```

### 3.3 `thread_colors`
Stores the complete thread sequence and brand swatch palettes extracted from file headers.
```sql
CREATE TABLE IF NOT EXISTS thread_colors (
    id          TEXT PRIMARY KEY,
    design_id   TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    color_index INTEGER NOT NULL,
    hex_code    TEXT NOT NULL,
    brand       TEXT,
    color_code  TEXT,
    description TEXT,
    UNIQUE(design_id, color_index)
);

CREATE INDEX IF NOT EXISTS idx_thread_colors_design ON thread_colors(design_id);
```

### 3.4 `collections` & `collection_designs`
Manages themed series and seasonal folders.
```sql
CREATE TABLE IF NOT EXISTS collections (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_designs (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    design_id     TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    added_at      TEXT NOT NULL,
    PRIMARY KEY(collection_id, design_id)
);
```

### 3.5 `jobs` & `job_designs`
Production batch containers linking designs, artwork, and production notes.
```sql
CREATE TABLE IF NOT EXISTS jobs (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    notes       TEXT,
    status      TEXT NOT NULL DEFAULT 'draft', -- 'draft' | 'active' | 'completed' | 'archived'
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS job_designs (
    job_id      TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    design_id   TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    assigned_at TEXT NOT NULL,
    PRIMARY KEY(job_id, design_id)
);
```

### 3.6 `artwork_assets` & `design_artwork`
Links original customer artwork, proofs, vector graphics (PNG, JPG, SVG, PDF) to embroidery files.
```sql
CREATE TABLE IF NOT EXISTS artwork_assets (
    id           TEXT PRIMARY KEY,
    filename     TEXT NOT NULL,
    managed_path TEXT NOT NULL,
    mime_type    TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    checksum     TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS design_artwork (
    design_id  TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    artwork_id TEXT NOT NULL REFERENCES artwork_assets(id) ON DELETE CASCADE,
    linked_at  TEXT NOT NULL,
    PRIMARY KEY(design_id, artwork_id)
);
```

### 3.7 `tags` & `design_tags`
Arbitrary categorization and keyword indexing.
```sql
CREATE TABLE IF NOT EXISTS tags (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS design_tags (
    design_id TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    tag_id    TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY(design_id, tag_id)
);
```

### 3.8 `ai_suggestions`
Stores candidate AI metadata proposals pending user approval.
```sql
CREATE TABLE IF NOT EXISTS ai_suggestions (
    id          TEXT PRIMARY KEY,
    design_id   TEXT NOT NULL REFERENCES designs(id) ON DELETE CASCADE,
    category    TEXT,
    subject     TEXT,
    style       TEXT,
    description TEXT,
    tags        TEXT, -- JSON Array
    colors      TEXT, -- JSON Array
    status      TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'accepted' | 'rejected'
    created_at  TEXT NOT NULL
);
```

---

## 4. SQLite FTS5 Full-Text Search Engine

Stitchflow embeds a dedicated **FTS5 virtual table** configured with the **Porter Stemmer** tokenizer to support instant natural language queries across title, filename, tags, AI categories, and descriptions.

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS design_search USING fts5(
    design_id UNINDEXED,
    title,
    filename,
    format,
    tags,
    ai_category,
    ai_subject,
    ai_description,
    tokenize = 'porter unicode61'
);
```

### Automatic Sync Triggers
Three database triggers keep the FTS5 index 100% in sync with the relational table:

```sql
-- 1. Insert Sync
CREATE TRIGGER IF NOT EXISTS trg_designs_ai AFTER INSERT ON designs
BEGIN
    INSERT INTO design_search(design_id, title, filename, format, tags, ai_category, ai_subject, ai_description)
    VALUES (
        new.id,
        new.title,
        new.filename,
        new.format,
        (SELECT GROUP_CONCAT(t.name, ' ') FROM design_tags dt JOIN tags t ON dt.tag_id = t.id WHERE dt.design_id = new.id),
        new.ai_category,
        new.ai_subject,
        new.ai_description
    );
END;

-- 2. Update Sync
CREATE TRIGGER IF NOT EXISTS trg_designs_au AFTER UPDATE ON designs
BEGIN
    UPDATE design_search SET
        title = new.title,
        filename = new.filename,
        format = new.format,
        tags = (SELECT GROUP_CONCAT(t.name, ' ') FROM design_tags dt JOIN tags t ON dt.tag_id = t.id WHERE dt.design_id = new.id),
        ai_category = new.ai_category,
        ai_subject = new.ai_subject,
        ai_description = new.ai_description
    WHERE design_id = new.id;
END;

-- 3. Delete Sync
CREATE TRIGGER IF NOT EXISTS trg_designs_ad AFTER DELETE ON designs
BEGIN
    DELETE FROM design_search WHERE design_id = old.id;
END;
```

---

## 5. Backup & Integrity Verification

### Integrity Check Command
Stitchflow validates database integrity using SQLite's built-in integrity check:
```sql
PRAGMA integrity_check;
```

### Checksum Validation
Every embroidery file and artwork asset is verified using **SHA-256**:
- Deduplication on import matches checksums against `designs.checksum`.
- Backups generate a `manifest.json` containing each file's relative path, size, and SHA-256 hash.
