use std::path::Path;

fn main() {
    let engine_dist_dir = Path::new("embroidery-engine/dist");
    let engine_exe = engine_dist_dir.join("engine.exe");
    if !engine_exe.exists() {
        let _ = std::fs::create_dir_all(&engine_dist_dir);
        let _ = std::fs::write(&engine_exe, b"DUMMY_ENGINE_PLACEHOLDER");
    }
    tauri_build::build();
}
