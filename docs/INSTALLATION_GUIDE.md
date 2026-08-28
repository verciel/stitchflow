# Stitchflow — Installation & Configuration Guide

This guide walks you through installing, running, and configuring **Stitchflow** on Windows, macOS, and Linux.

---

## 1. System Requirements

### Minimum Requirements
- **Operating System**: Windows 10/11 (64-bit), macOS 12+ (Apple Silicon or Intel), or Ubuntu 20.04+ / Debian 11+
- **Processor**: Dual-Core 64-bit CPU (2.0 GHz+)
- **Memory (RAM)**: 4 GB RAM
- **Disk Space**: 250 MB for application + space for your embroidery design files
- **Display**: 1280 × 800 minimum screen resolution

### Recommended Requirements
- **Processor**: Quad-Core CPU (Intel Core i5 / AMD Ryzen 5 / Apple M1 or newer)
- **Memory (RAM)**: 8 GB+ RAM
- **Display**: 1920 × 1080 (Full HD)

---

## 2. Option A: Pre-Built Binary Installation (Fastest)

Download the latest release package from the **[Stitchflow Releases](https://github.com/verciel/stitchflow/releases)** page.

### 1. Windows Installer (`.msi`)
1. Download `stitchflow_0.1.0_x64_en-US.msi`.
2. Double-click the installer and follow the guided setup.
3. Stitchflow will be installed into your user profile and added to the Start Menu.

### 2. Standalone Executable (`.exe`)
1. Download `stitchflow.exe`.
2. Move it to any folder of your choice (e.g. `C:\Tools\Stitchflow\`).
3. Double-click `stitchflow.exe` to run immediately. No installation required.

### 3. Portable Archive (`.zip`)
1. Download `stitchflow-v0.1.0-windows-x64.zip`.
2. Extract the archive to a USB drive or local folder.
3. Launch `stitchflow.exe` directly from the extracted directory.

---

## 3. Option B: Building & Running from Source

### Step 1: Install Development Prerequisites

1. **Node.js**:
   - Install **Node.js LTS (v20.x or v22.x)** from [nodejs.org](https://nodejs.org/).
   - Verify: `node -v` and `npm -v`

2. **Rust Toolchain**:
   - Install Rust via [rustup.rs](https://rustup.rs/):
     ```powershell
     # Windows (PowerShell)
     winget install Rustlang.Rustup
     # Or run rustup-init.exe from rustup.rs
     ```
   - Verify: `cargo --version` and `rustc --version`

3. **Python (for Embroidery Engine Sidecar)**:
   - Install **Python 3.10 or 3.11** from [python.org](https://www.python.org/).
   - Ensure *"Add Python to PATH"* is checked during installation.
   - Verify: `python --version`

---

### Step 2: Clone and Configure the Repository

```powershell
# 1. Clone the repository
git clone https://github.com/verciel/stitchflow.git
cd stitchflow

# 2. Create and activate a Python virtual environment
python -m venv .venv
.\.venv\Scripts\Activate.ps1

# 3. Install Python embroidery engine dependencies
python -m pip install --upgrade pip
pip install -r src-tauri/embroidery-engine/requirements.txt

# 4. Install Node.js frontend dependencies
npm.cmd install
```

---

### Step 3: Run Development Server

```powershell
# Launches Vite dev server and Tauri desktop window with live reload
npm.cmd run tauri dev
```

---

### Step 4: Build Release Binary

```powershell
# Compiles optimized release binary with embedded assets
npm.cmd run tauri build
```

The compiled output will be located at:
`src-tauri/target/release/stitchflow.exe` or `%LOCALAPPDATA%\Stitchflow-cargo-target\release\stitchflow.exe`.

---

## 4. Configuration

### 4.1 Ink/Stitch & Inkscape Integration Setup

Stitchflow seamlessly launches embroidery designs directly into **Inkscape** with the **Ink/Stitch** vector embroidery extension.

#### Installation:
1. Download and install **Inkscape** from [inkscape.org](https://inkscape.org/).
2. Download and install the **Ink/Stitch** extension from [inkstitch.org](https://inkstitch.org/).

#### Configuration in Stitchflow:
1. In Stitchflow, open **Settings** from the bottom-left sidebar.
2. Under **Ink/Stitch Integration**, click **Browse…** and select your `inkscape.exe` path:
   - Typical path: `C:\Program Files\Inkscape\bin\inkscape.exe`
3. Click **Save Configuration**.
4. You can now click **"Open in Ink/Stitch"** on any design in the Inspector Drawer.

---

### 4.2 AI Assistant & Vision Endpoint Configuration

Stitchflow supports any **OpenAI-compatible Vision API** (including cloud and 100% offline local LLMs).

1. In Stitchflow, navigate to **Settings**.
2. Under **AI Assistant Configuration**:
   - **Enable AI Assistant**: Toggle switch to **ON**.
   - **API Endpoint**:
     - *OpenAI Cloud*: `https://api.openai.com/v1`
     - *Local Ollama*: `http://localhost:11434/v1`
     - *Local LM Studio*: `http://localhost:1234/v1`
     - *OpenRouter*: `https://openrouter.ai/api/v1`
   - **Model Name**:
     - For OpenAI: `gpt-4o-mini` or `gpt-4o`
     - For Ollama: `llava` or `minicpm-v`
     - For LM Studio: `vision-model-id`
   - **API Key**: Enter your API key (leave blank or enter `ollama` for local models).
3. Click **Save AI Settings**.
4. Click **"Test Connection"** to verify that your endpoint responds correctly.
