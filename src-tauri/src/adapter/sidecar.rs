use super::{AdapterError, EmbroideryFormatAdapter, EmbroideryMetadata};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct PythonSidecarAdapter {
    python_bin: PathBuf,
    engine_script: PathBuf,
}

impl PythonSidecarAdapter {
    pub fn new() -> Result<Self, AdapterError> {
        let (python_bin, engine_script) = Self::locate_components()?;
        Ok(Self {
            python_bin,
            engine_script,
        })
    }

    fn locate_components() -> Result<(PathBuf, PathBuf), AdapterError> {
        let cur_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Search candidate locations for engine.py
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

        // Also search relative to the executable
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
            AdapterError::ProcessFailed("Could not locate embroidery engine.py script".into())
        })?;

        // Search for Python executable
        // Priority 1: .venv/Scripts/python.exe in workspace or parent
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

        // Priority 2: System python/py
        let python_bin = python_path.unwrap_or_else(|| {
            if cfg!(windows) {
                PathBuf::from("python.exe")
            } else {
                PathBuf::from("python3")
            }
        });

        Ok((python_bin, engine_script))
    }

    fn run_engine(&self, args: &[&str]) -> Result<String, AdapterError> {
        let mut cmd = Command::new(&self.python_bin);
        cmd.arg(&self.engine_script);
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
            AdapterError::ProcessFailed(format!("Failed to execute python engine: {e}"))
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
        let stdout = self.run_engine(&["inspect", &path_str])?;

        let meta: EmbroideryMetadata = serde_json::from_str(&stdout).map_err(|e| {
            AdapterError::Serialization(format!("Failed to parse inspect output '{stdout}': {e}"))
        })?;

        Ok(meta)
    }

    fn render_preview(
        &self,
        path: &Path,
        out_png: &Path,
        out_svg: Option<&Path>,
    ) -> Result<(), AdapterError> {
        let path_str = path.to_string_lossy();
        let out_png_str = out_png.to_string_lossy();

        if let Some(svg_path) = out_svg {
            let out_svg_str = svg_path.to_string_lossy();
            self.run_engine(&["render", &path_str, &out_png_str, &out_svg_str])?;
        } else {
            self.run_engine(&["render", &path_str, &out_png_str])?;
        }

        Ok(())
    }

    fn export(
        &self,
        src_path: &Path,
        dst_path: &Path,
        target_format: &str,
    ) -> Result<(), AdapterError> {
        let src_str = src_path.to_string_lossy();
        let dst_str = dst_path.to_string_lossy();

        let stdout = self.run_engine(&["export", &src_str, &dst_str, target_format])?;

        let val: Value = serde_json::from_str(&stdout).map_err(|e| {
            AdapterError::Serialization(format!("Failed to parse export output '{stdout}': {e}"))
        })?;

        if val["status"] == "ok" {
            Ok(())
        } else {
            Err(AdapterError::ProcessFailed(format!(
                "Export failed: {}",
                val["error"]
            )))
        }
    }
}
