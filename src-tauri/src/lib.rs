pub mod adapter;
pub mod commands;
pub mod db;
pub mod models;

use adapter::sidecar::PythonSidecarAdapter;
use adapter::EmbroideryFormatAdapter;
use commands::ai::*;
use commands::artwork::*;
use commands::backup::*;
use commands::collections::*;
use commands::designs::*;
use commands::import::*;
use commands::jobs::*;
use commands::settings::*;
use commands::tags::*;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};


pub struct AppState {
    pub db: Mutex<Connection>,
    pub library_root: PathBuf,
    pub adapter: Arc<dyn EmbroideryFormatAdapter>,
}

fn app_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Stitchflow")
}

pub fn run() {
    let root = app_root();
    let _ = std::fs::create_dir_all(&root);

    let db = match db::initialize(&root) {
        Ok(d) => d,
        Err(e) => {
            let log_file = root.join("startup_error.log");
            let _ = std::fs::write(&log_file, format!("Database initialization failed: {e}\n"));
            eprintln!("Database error: {e}");
            panic!("Unable to initialize Stitchflow database: {e}");
        }
    };

    let adapter: Arc<dyn EmbroideryFormatAdapter> = match PythonSidecarAdapter::new() {
        Ok(ad) => Arc::new(ad),
        Err(e) => {
            let log_file = root.join("startup_error.log");
            let _ = std::fs::write(&log_file, format!("Engine adapter initialization failed: {e}\n"));
            eprintln!("Warning: Failed to initialize PythonSidecarAdapter: {e}");
            panic!("Cannot start Stitchflow without embroidery engine sidecar: {e}");
        }
    };


    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db: Mutex::new(db),
            library_root: root,
            adapter,
        })
        .invoke_handler(tauri::generate_handler![
            // Designs
            list_designs,
            get_design_details,
            update_design_metadata,
            delete_design,
            restore_design,
            permanent_delete_design,
            empty_recycle_bin,
            export_design,
            reveal_in_folder,
            find_similar_designs,
            // Imports
            import_files,
            // Tags
            list_tags,
            add_tag_to_design,
            remove_tag_from_design,
            // Collections
            list_collections,
            create_collection,
            update_collection,
            delete_collection,
            add_design_to_collection,
            remove_design_from_collection,
            // Jobs
            list_jobs,
            create_job,
            update_job,
            delete_job,
            add_design_to_job,
            remove_design_from_job,
            add_artwork_to_job,
            remove_artwork_from_job,
            // Artwork
            list_artwork,
            link_artwork_to_design,
            unlink_artwork_from_design,
            delete_artwork,
            // Backup & Restore
            create_backup,
            validate_backup,
            restore_backup,
            // Settings & Ink/Stitch
            get_settings,
            save_setting,
            get_inkstitch_config,
            set_inkstitch_config,
            open_in_inkstitch,
            // AI
            get_ai_config,
            save_ai_config,
            test_ai_connection,
            test_hf_connection,
            analyze_designs,
            apply_ai_suggestion,
            natural_language_search,
            ask_ai_custom,
            generate_ai_design_image,
            digitize_and_import_design,
            propose_ai_edit,
            apply_proposed_edit,

            // Utility

            read_image_data,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Stitchflow application");
}
