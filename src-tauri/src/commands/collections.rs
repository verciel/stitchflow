use crate::db::now;
use crate::models::Collection;
use crate::AppState;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_collections(state: State<AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare(
            "SELECT c.id, c.name, c.description, COUNT(cd.design_id), c.created_at
             FROM collections c
             LEFT JOIN collection_designs cd ON cd.collection_id = c.id
             LEFT JOIN designs d ON d.id = cd.design_id AND d.status = 'active'
             GROUP BY c.id, c.name, c.description, c.created_at
             ORDER BY c.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok(Collection {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                design_count: r.get(3)?,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let collections = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(collections)
}

#[tauri::command]
pub fn create_collection(
    state: State<AppState>,
    name: String,
    description: Option<String>,
) -> Result<Collection, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let id = format!("col-{}", Uuid::new_v4());
    let desc = description.unwrap_or_default();
    let time = now();

    db.execute(
        "INSERT INTO collections(id, name, description, created_at) VALUES(?1, ?2, ?3, ?4)",
        params![id, name.trim(), desc.trim(), time],
    )
    .map_err(|e| format!("Could not create collection: {e}"))?;

    Ok(Collection {
        id,
        name: name.trim().to_string(),
        description: desc.trim().to_string(),
        design_count: 0,
        created_at: time,
    })
}

#[tauri::command]
pub fn update_collection(
    state: State<AppState>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let desc = description.unwrap_or_default();

    db.execute(
        "UPDATE collections SET name = ?1, description = ?2 WHERE id = ?3",
        params![name.trim(), desc.trim(), id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_collection(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute("DELETE FROM collections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn add_design_to_collection(
    state: State<AppState>,
    collection_id: String,
    design_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "INSERT OR IGNORE INTO collection_designs(collection_id, design_id) VALUES(?1, ?2)",
        params![collection_id, design_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn remove_design_from_collection(
    state: State<AppState>,
    collection_id: String,
    design_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "DELETE FROM collection_designs WHERE collection_id = ?1 AND design_id = ?2",
        params![collection_id, design_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
