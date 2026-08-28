use super::{AdapterError, EmbroideryFormatAdapter, EmbroideryMetadata};
use std::path::{Path, PathBuf};
use std::process::Command;

// Embed the compiled standalone engine executable directly into the Rust binary
const EMBEDDED_ENGINE: &[u8] = include_bytes!("../../embroidery-engine/dist/engine.exe");

pub struct PythonSidecarAdapter {
    binary_or_python: PathBuf,
    engine_script: Option<PathBuf>,
}

impl PythonSidecarAdapter {
    pub fn new() -> Result<Self, AdapterError> {
        let (binary_or_python, engine_script) = Self::locate_components()?;
        Ok(Self {
            binary_or_python,
            engine_script,
        })
    }

    fn locate_components() -> Result<(PathBuf, Option<PathBuf>), AdapterError> {
        let cur_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Priority 1: Check for standalone compiled engine.exe adjacent to executable or in current dir
        let mut binary_candidates = vec![
            cur_dir.join("src-tauri/embroidery-engine/dist/engine.exe"),
            cur_dir.join("src-tauri/embroidery-engine/engine.exe"),
            cur_dir.join("embroidery-engine/engine.exe"),
            cur_dir.join("engine.exe"),
        ];

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                binary_candidates.push(exe_dir.join("engine.exe"));
                binary_candidates.push(exe_dir.join("embroidery-engine/engine.exe"));
                binary_candidates.push(exe_dir.join("embroidery-engine/dist/engine.exe"));
                binary_candidates.push(exe_dir.join("../embroidery-engine/engine.exe"));
            }
        }

        for bin in &binary_candidates {
            if bin.exists() {
                return Ok((bin.clone(), None));
            }
        }

        // Priority 2: Check or self-extract embedded engine into %LOCALAPPDATA%\Stitchflow\bin\engine.exe
        if EMBEDDED_ENGINE.len() > 100_000 {
            let appdata_bin = dirs::data_local_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("Stitchflow")
                .join("bin")
                .join("engine.exe");

            let should_extract = if appdata_bin.exists() {
                match std::fs::metadata(&appdata_bin) {
                    Ok(meta) => meta.len() != EMBEDDED_ENGINE.len() as u64,
                    Err(_) => true,
                }
            } else {
                true
            };

            if should_extract {
                if let Some(parent) = appdata_bin.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&appdata_bin, EMBEDDED_ENGINE).is_ok() {
                    return Ok((appdata_bin, None));
                }
            } else {
                return Ok((appdata_bin, None));
            }
        }

        // Priority 3: Fallback to engine.py script with Python interpreter

        let script_candidates = [
            cur_dir.join("src-tauri/embroidery-engine/engine.py"),
            cur_dir.join("embroidery-engine/engine.py"),
            cur_dir.join("../src-tauri/embroidery-engine/engine.py"),
            cur_dir.join("../../src-tauri/embroidery-engine/engine.py"),
        ];

        let mut engine_path = None;
        for c in &script_candidates {
            if c.exists() {
                engine_path = Some(c.clone());
                break;
            }
        }

        if engine_path.is_none() {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let candidates = [
                        exe_dir.join("embroidery-engine/engine.py"),
                        exe_dir.join("../embroidery-engine/engine.py"),
                        exe_dir.join("../../src-tauri/embroidery-engine/engine.py"),
                    ];
                    for c in &candidates {
                        if c.exists() {
                            engine_path = Some(c.clone());
                            break;
                        }
                    }
                }
            }
        }

        let engine_script = engine_path.ok_or_else(|| {
            AdapterError::ProcessFailed("Could not locate embroidery engine (neither engine.exe nor engine.py found)".into())
        })?;

        // Search for Python executable
        let venv_candidates = [
            cur_dir.join(".venv/Scripts/python.exe"),
            cur_dir.join("../.venv/Scripts/python.exe"),
            cur_dir.join("../../.venv/Scripts/python.exe"),
        ];

        let mut python_path = None;
        for v in &venv_candidates {
            if v.exists() {
                python_path = Some(v.clone());
                break;
            }
        }

        let python_bin = python_path.unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from("python.exe")
            } else {
                PathBuf::from("python3")
            }
        });

        Ok((python_bin, Some(engine_script)))
    }

    fn run_engine(&self, args: &[&str]) -> Result<String, AdapterError> {
        let mut cmd = Command::new(&self.binary_or_python);
        if let Some(ref script) = self.engine_script {
            cmd.arg(script);
        }
        for a in args {
            cmd.arg(a);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output().map_err(|e| {
            AdapterError::ProcessFailed(format!("Failed to execute embroidery engine: {e}"))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            let msg = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                format!("Process exited with status code: {:?}", output.status.code())
            };
            return Err(AdapterError::ProcessFailed(msg));
        }

        Ok(stdout)
    }
}

impl EmbroideryFormatAdapter for PythonSidecarAdapter {
    fn inspect(&self, path: &Path) -> Result<EmbroideryMetadata, AdapterError> {
        let path_str = path.to_string_lossy();
        let out = self.run_engine(&["inspect", &path_str])?;
        serde_json::from_str::<EmbroideryMetadata>(&out).map_err(|e| {
            AdapterError::Serialization(format!("Failed to deserialize metadata: {e} | raw: {out}"))
        })
    }

    fn render_preview(&self, path: &Path, out_png: &Path, out_svg: Option<&Path>) -> Result<(), AdapterError> {
        let path_str = path.to_string_lossy();
        let png_str = out_png.to_string_lossy();
        let svg_str = out_svg.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

        if svg_str.is_empty() {
            self.run_engine(&["render", &path_str, &png_str])?;
        } else {
            self.run_engine(&["render", &path_str, &png_str, &svg_str])?;
        }
        Ok(())
    }

    fn export(&self, src_path: &Path, dst_path: &Path, target_format: &str) -> Result<(), AdapterError> {
        let src_str = src_path.to_string_lossy();
        let dst_str = dst_path.to_string_lossy();
        self.run_engine(&["export", &src_str, &dst_str, target_format])?;
        Ok(())
    }
}
