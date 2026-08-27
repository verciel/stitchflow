# Stitchflow

An offline-first, Windows-first desktop application for managing embroidery designs, associated source artwork, and lightweight production job records. Built with **Tauri 2**, **React 18 + TypeScript**, **Rust**, **SQLite with FTS5**, and a packaged **`pyembroidery` sidecar engine**.

---

## Key Features

- **Multi-Format Embroidery Catalog**: Inspect dimensions, stitch counts, color changes, jump stitches, trims, and thread color palettes across 10 formats (`DST`, `PES`, `JEF`, `VP3`, `EXP`, `HUS`, `XXX`, `SEW`, `PCS`, `PEC`).
- **High-Fidelity 2D Previews**: Anti-aliased 2D stitch preview rendering with thread color styling, fabric texture, and hoop crosshairs (PNG and SVG).
- **Relational Source Artwork**: Link customer vector logos (`SVG`), mockups (`PNG`, `JPG`), and specification sheets (`PDF`) directly to stitch designs and production jobs.
- **Production Job Containers**: Track machine setups, garment types, stabilizer notes, hoop sizes, and status lifecycle (`draft`, `active`, `completed`, `archived`) without duplicating files.
- **Curated Collections & Tags**: Organize designs into thematic collections and searchable `#tags` with instant SQLite FTS5 full-text indexing.
- **Format Conversion & Export**: Seamlessly convert embroidery patterns between supported machine formats.
- **Safe Quarantine & Recovery**: Soft-delete into a managed Recycle Area with one-click restore or permanent file removal.
- **Ink/Stitch Handoff**: Configure your local Inkscape installation and launch designs directly into Ink/Stitch for digitizing and editing.
- **Portable ZIP Backups**: Create and restore single-file archives containing the database, managed files, stitch previews, and a SHA-256 checksummed `manifest.json`.
- **Privacy-First AI Vision**: Optional, opt-in AI analysis using an OpenAI-compatible vision endpoint (OpenAI, Ollama, LM Studio). Transmits **only** rendered 2D preview images and extracted technical metadata—never raw binary embroidery files—with approval-only catalog updates.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                   React 18 + TypeScript                  │
│   App Shell · Inspector Drawer · Views · Modals · CSS    │
└────────────────────────────┬─────────────────────────────┘
                             │ Tauri 2 IPC
┌────────────────────────────▼─────────────────────────────┐
│                      Rust Backend                        │
│   AppState · SQLite (WAL/FTS5) · IPC Commands · Backup    │
└──────────────┬────────────────────────────┬──────────────┘
               │                            │
┌──────────────▼─────────────┐ ┌────────────▼──────────────┐
│  Python Sidecar (engine.py) │ │    OpenAI-Compatible     │
│   pyembroidery & Pillow     │ │    Vision API Endpoint   │
│  (Inspect, Render, Export)  │ │ (Opt-in Preview Analysis)│
└─────────────────────────────┘ └──────────────────────────┘
```

---

## Development Setup

### Prerequisites
- **Node.js**: v20+ / v22+
- **Python**: v3.11+
- **Rust**: Stable (MSVC toolchain on Windows)

### Initial Setup
```powershell
# 1. Install frontend dependencies
npm install

# 2. Setup Python environment and install pyembroidery + Pillow
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install pyembroidery pillow
```

### Launch Development Server
```powershell
npm run tauri dev
```

---

## Running Tests

```powershell
# Run frontend unit tests (Vitest)
npm test

# Run Rust unit tests (SQLite schema, checksums, triggers)
$env:CARGO_TARGET_DIR = 'C:\Users\Hp\AppData\Local\Stitchflow-cargo-target'; cargo test --lib --manifest-path src-tauri/Cargo.toml

# Run Python sidecar engine tests
.\.venv\Scripts\python.exe tests/test_engine.py
```

---

## Production Build

```powershell
# Build frontend bundle
npm run build

# Build standalone desktop application
npm run tauri build
```

---

## Privacy & Security

- All catalog management, preview generation, and search indexing operate **100% offline**.
- Originals are preserved: all imported embroidery patterns and artwork assets are stored as application-owned copies in your local AppData directory.
- AI analysis is disabled by default and requires explicit user activation. When active, requests submit only rendered 2D preview images and approved technical facts. Raw embroidery files are never transmitted.
