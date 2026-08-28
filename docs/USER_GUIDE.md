# Stitchflow — User Manual & Operations Guide

Welcome to the **Stitchflow** user manual! Stitchflow is an offline-first desktop application designed to organize, inspect, search, convert, and manage your commercial embroidery designs and customer artwork.

---

## 1. Interface Tour

```text
┌──────────────┬────────────────────────────────────────────────────────┬─────────────────────┐
│  SIDEBAR     │  TOOLBAR: [ 🔍 Search... ] [Tags ▾] [Sort ▾] [Format]  │  DESIGN INSPECTOR   │
│  ──────────  ├────────────────────────────────────────────────────────┤  ────────────────   │
│  📁 Library  │                                                        │  English Garden     │
│  📂 Collect. │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │  Rose (PES)         │
│  📦 Jobs     │   │              │  │              │  │              │ │  ────────────────   │
│  🖼 Artwork  │   │   PREVIEW    │  │   PREVIEW    │  │   PREVIEW    │ │  Facts:             │
│  🗑 Recycle  │   │              │  │              │  │              │ │  82.0 × 76.0 mm     │
│              │   │  Garden Rose │  │  Butterfly   │  │ Celestial    │ │  12,480 stitches    │
│  ──────────  │   │  12,480 sts  │  │  8,920 sts   │  │ 4,310 sts    │ │  5 Colors           │
│  ⚙ Settings  │   └──────────────┘  └──────────────┘  └──────────────┘ │  [Open in Ink/Stitch│
└──────────────┴────────────────────────────────────────────────────────┴─────────────────────┘
```

- **Left Sidebar**: Quick navigation between **Library**, **Collections**, **Jobs**, **Artwork**, **Recycle area**, and **Settings**. Can be collapsed into a compact icon rail using the top collapse button.
- **Top Workspace Toolbar**: Instant full-text search, tag filters, sorting options (Newest, Stitches, Title, Size), and format filter pills (`DST`, `PES`, `JEF`, `VP3`, `EXP`, etc.).
- **Design Catalog**: Grid view with visual stitch cards and Table view with dense technical columns.
- **Design Inspector Drawer**: Opens on the right when clicking any design to show dimensions, stitches, thread sequence, AI classifications, revision history, and conversion tools.

---

## 2. Importing Embroidery Designs & Artwork

1. Click **"Import designs"** in the sidebar or **"+ Import files"** in the top header.
2. The **Batch Import Modal** will appear:
   - Click **"+ Add Files…"** to stage individual files.
   - Click **"+ Add Folder…"** to automatically scan an entire directory and all its subfolders.
3. Supported formats:
   - **Embroidery**: `DST`, `PES`, `JEF`, `VP3`, `EXP`, `HUS`, `XXX`, `SEW`, `PCS`, `PEC`
   - **Artwork**: `PNG`, `JPG`, `SVG`, `PDF`
4. Choose your **Exact Duplicate Policy**:
   - **Skip duplicates (Recommended)**: Ignores files that already exist in your library based on SHA-256 checksums.
   - **Replace as revision**: Keeps the existing design ID and adds the new file as a new revision (e.g. `v2`, `v3`).
   - **Keep both**: Imports as a distinct new design record.
5. Click **"Import Items"**. Stitchflow creates application-owned copies in its local storage — your original files are never altered or moved.

---

## 3. Searching, Filtering & Sorting

### 3.1 Instant Full-Text Search
Type keywords into the search box:
- Search by **title**: `rose`, `varsity`, `crest`
- Search by **filename**: `WF028.pes`, `garden_rose.dst`
- Search by **tags**: `#floral`, `#jacket-back`, `#cap`
- Search by **AI descriptions**: `dense satin stitch`, `geometric star`, `nautical academy`

### 3.2 Format Filter Pills
Click any format pill (`DST`, `PES`, `JEF`, `VP3`, `EXP`, etc.) to instantly filter the catalog to that specific machine format. Click **"All Formats"** to reset.

### 3.3 Sorting Options
Use the sort dropdown to arrange designs by:
- **Newest Imported** / **Oldest Imported**
- **Most Stitches** / **Least Stitches**
- **Title (A to Z)**
- **Largest File Size**

---

## 4. Inspecting & Converting Designs

Click any design in the catalog to open the **Design Inspector Drawer** on the right:

### 4.1 Technical Facts & Thread Palette
- View real dimensions in millimeters, exact stitch count, color change stops, and file size.
- The **Thread Palette** displays each color stop with its real hex color chip, color code, and thread brand (e.g. *Madeira Polyneon*, *Robison-Anton*).

### 4.2 Format Conversion & Export
To convert a design to another machine format (e.g. `PES` to `DST` for a commercial Tajima machine):
1. In the Inspector Drawer, scroll to **"Convert & Export"**.
2. Select your target format from the dropdown (`DST`, `PES`, `JEF`, `VP3`, `EXP`, `XXX`, `PEC`).
3. Click **"Export File"**.
4. Choose the destination folder on your computer and save.

---

## 5. Organization: Collections & Production Jobs

### 5.1 Collections (Themed Folders)
1. Navigate to **Collections** in the sidebar.
2. Click **"+ Create New Collection"**.
3. Enter a name (e.g. *Spring Floral Series 2026*) and optional description.
4. To add designs: In the **Library**, click a design and choose the collection in the Inspector Drawer under **Organization**.

### 5.2 Production Jobs (Order Batches)
1. Navigate to **Jobs** in the sidebar.
2. Click **"+ Create New Job Container"**.
3. Enter the Job Title (e.g. *Polo Crest Order #4082*), status (*Draft*, *Active*, *Completed*), and placement notes.
4. Assign designs and customer proofs to the job to keep production orders organized.

---

## 6. AI Vision Assistant

Stitchflow includes an optional **Vision AI Assistant** that analyzes rendered stitch previews to auto-tag and categorize your library:

1. Click **"Analyze with AI"** on any design.
2. The **AI Review Modal** displays:
   - Proposed **Category** (e.g. *Floral & Botanical*)
   - Proposed **Subject** & **Style**
   - Generated **Tags** (e.g. `#rose`, `#satin-stitch`, `#botanical`)
   - Natural language description of the stitch work
3. Click **"Apply to Design"** to save the metadata to your catalog.

---

## 7. Ink/Stitch (Inkscape) Vector Handoff

To edit or simulate stitch paths in Inkscape:
1. In the Inspector Drawer, click **"Open in Ink/Stitch (Inkscape)"**.
2. If this is your first time, Stitchflow will prompt you to locate `inkscape.exe` (e.g. `C:\Program Files\Inkscape\bin\inkscape.exe`).
3. Stitchflow will launch Inkscape with your design loaded and ready for vector manipulation.

---

## 8. Recycle Bin & Backup Management

### 8.1 2-Stage Safe Deletion
- **Stage 1 (Recycle Area)**: Clicking *"Move to Recycle Area"* moves the design to the **Recycle area** tab. The physical file remains safe and can be restored at any time with one click.
- **Stage 2 (Permanent Purge)**: In the **Recycle area**, clicking *"Delete Forever"* or *"Empty Recycle Bin"* permanently removes the managed copy to reclaim disk space.

### 8.2 Portable Backups
1. Go to **Settings** in the sidebar.
2. Under **Backup & Restore**, click **"Create Backup Archive (.zip)"**.
3. Stitchflow creates a compressed `.zip` archive containing all your designs, previews, artwork, database records, and a cryptographic `manifest.json`.
4. To restore on another computer, select **"Restore from Backup"** and choose your `.zip` archive.
