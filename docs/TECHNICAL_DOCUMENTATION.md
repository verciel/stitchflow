# Stitchflow — Technical Architecture & Developer Documentation

This document describes the internal engineering architecture, IPC protocols, format adapters, threading models, and data pipelines of **Stitchflow**.

---

## 1. High-Level System Architecture

Stitchflow follows a multi-tier, offline-first desktop architecture combining high-performance native systems programming with modern, responsive web UI technologies:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                           Presentation Layer                            │
│                 React 18 · TypeScript · Vite · Lucide Icons             │
│        (Modular UI Components, Responsive Drawers, SVG/Canvas Previews) │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ Tauri 2 IPC (JSON-RPC)
┌────────────────────────────────────▼────────────────────────────────────┐
│                            Rust Core Runtime                            │
│  - AppState Supervision & Mutex<Connection> SQLite Connection Pool      │
│  - IPC Command Handlers (import, designs, jobs, collections, backup, ai)│
│  - Trait-Based EmbroideryFormatAdapter Layer                            │
│  - File System Management & SHA-256 Checksum Engine                     │
│  - Zip Backup & SHA-256 Manifest Generation Engine                      │
└───────────────────┬─────────────────────────────────┬───────────────────┘
                    │ Subprocess STDIO Bridge         │ Embedded C-API
┌───────────────────▼───────────────┐   ┌─────────────▼───────────────────┐
│     Python Sidecar Engine         │   │       Embedded SQLite 3         │
│  - pyembroidery Low-Level Parser  │   │  - 10 Relational Tables         │
│  - 2D Anti-aliased Stitch Render  │   │  - FTS5 Full-Text Search Engine │
│  - Cross-Format Transcoding       │   │  - Synchronous Data Triggers    │
└───────────────────────────────────┘   └─────────────────────────────────┘
```

---

## 2. Component Layers & Responsibilities

### 2.1 Presentation Layer (`src/`)
- **Technology**: React 18 with TypeScript compiled via Vite.
- **State Management**: Local React state with optimistic UI updates and reactive polling synchronization.
- **Preview Rendering**: `DesignImage` component rendering both real-time SVG vector stitch paths and high-resolution antialiased PNG previews.
- **Key Modules**:
  - `App.tsx`: Primary shell, sidebar routing, breadcrumb navigation, search and filter toolbar.
  - `DesignDetailsDrawer.tsx`: Inspector drawer displaying technical stitch facts, thread color sequence swatches, AI classifications, tag editing, linked artwork, and format conversion.
  - `BatchImportModal.tsx`: Multi-file and recursive folder staging queue with SHA-256 duplicate policies.
  - `AiReviewModal.tsx`: Side-by-side Vision AI analysis review dialog with interactive category/tag acceptance.

---

### 2.2 Rust Core Runtime (`src-tauri/`)
- **Technology**: Rust 2021 edition built with Tauri 2.
- **Supervision**: `AppState` struct holding a thread-safe `Arc<Mutex<rusqlite::Connection>>`.
- **Key Subsystems**:
  - `src-tauri/src/commands/`:
    - `designs.rs`: Catalog querying, metadata updates, soft/permanent deletion, and format export.
    - `import.rs`: Multi-file recursive directory walker, SHA-256 checksumming, duplicate policy enforcement, and provenance tracking.
    - `collections.rs`: Relational CRUD for design collections.
    - `jobs.rs`: Production batch containers and status workflows.
    - `artwork.rs`: Digital asset management for customer proofs and vector graphics.
    - `backup.rs`: Full library backup creation into `.zip` archives with cryptographic `manifest.json`.
    - `ai.rs`: OpenAI-compatible vision client with structured JSON parsing and natural language query extraction.

---

### 2.3 Python Embroidery Engine Sidecar (`src-tauri/embroidery-engine/`)
- **Technology**: Python 3 with `pyembroidery` and `Pillow`.
- **Purpose**: Specialized binary parsing, stitch coordinate decoding, thread block boundary detection, and cross-format transcoding.
- **Protocol**: Invoked via standard subprocess IPC using JSON CLI arguments:
  ```powershell
  python engine.py parse <input_file> <preview_output_path>
  python engine.py convert <input_file> <output_file> <target_format>
  ```

#### JSON Output Contract:
```json
{
  "status": "success",
  "metadata": {
    "format": "PES",
    "widthMm": 82.0,
    "heightMm": 76.0,
    "stitches": 12480,
    "colors": 5,
    "bounds": [-41.0, -38.0, 41.0, 38.0],
    "threads": [
      {
        "index": 1,
        "hex": "#D32F2F",
        "brand": "Madeira Polyneon",
        "colorCode": "1801",
        "description": "Classic Red"
      }
    ]
  },
  "previewPath": "C:\\...\\library\\previews\\abc123_preview.png"
}
```

---

## 3. Format Adapter Trait Design Pattern

Located in [`src-tauri/src/adapter/mod.rs`](file:///c:/Users/Hp/Documents/ChatGPT/embroidery%20management%20system/src-tauri/src/adapter/mod.rs):

```rust
pub trait EmbroideryFormatAdapter: Send + Sync {
    /// Format identifier (e.g. "DST", "PES", "JEF")
    fn format_name(&self) -> &'static str;

    /// File extensions supported by this adapter
    fn supported_extensions(&self) -> &'static [&'static str];

    /// Extract metadata, stitches, dimensions, and render 2D preview
    fn parse(&self, file_path: &Path, preview_dir: &Path) -> Result<ParsedDesign, AdapterError>;

    /// Transcode an embroidery file to a different target format
    fn convert(&self, source_path: &Path, target_path: &Path, target_format: &str) -> Result<(), AdapterError>;

    /// Whether this adapter supports exporting to target formats
    fn can_export(&self) -> bool;
}
```

---

## 4. Vision AI Structured Extraction Pipeline

When a user initiates AI analysis:
1. Stitchflow fetches the local high-resolution preview image (`.png`) from `%LOCALAPPDATA%\Stitchflow\library\previews\`.
2. The image is encoded as a base64 `data:image/png;base64,...` URI.
3. A strictly typed JSON prompt is sent to the configured OpenAI-compatible endpoint:

```json
{
  "model": "gpt-4o-mini",
  "messages": [
    {
      "role": "system",
      "content": "You are an expert embroidery digitizer. Analyze the rendered embroidery design and return JSON matching this schema: {\"category\": string, \"subject\": string, \"style\": string, \"description\": string, \"tags\": string[], \"dominantColors\": string[]}."
    },
    {
      "role": "user",
      "content": [
        { "type": "text", "text": "Analyze this embroidery preview." },
        { "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }
      ]
    }
  ],
  "response_format": { "type": "json_object" }
}
```
4. The structured output is received, sanitized, and presented to the user in the **AI Review Modal** for confirmation before saving to SQLite.

---

## 5. Threading & Concurrency Model

1. **Database Access**:
   SQLite is wrapped in `Arc<Mutex<rusqlite::Connection>>`. All write operations lock the mutex briefly and execute in WAL mode, ensuring queries never block catalog rendering.
2. **Background Jobs**:
   Batch import operations run asynchronously in dedicated Tokio background tasks, reporting progress incrementally to the frontend.
3. **Subprocess Isolation**:
   The Python embroidery engine is executed as an isolated child process, guaranteeing that corrupted embroidery binary files cannot crash the main desktop application.
