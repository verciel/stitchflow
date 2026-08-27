use crate::models::Tag;
use crate::AppState;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_tags(state: State<AppState>) -> Result<Vec<Tag>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare(
            "SELECT t.id, t.name, COUNT(dt.design_id) 
             FROM tags t
             LEFT JOIN design_tags dt ON dt.tag_id = t.id
             LEFT JOIN designs d ON d.id = dt.design_id AND d.status = 'active'
             GROUP BY t.id, t.name
             ORDER BY t.name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let tags = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(tags)
}

#[tauri::command]
pub fn add_tag_to_design(
    state: State<AppState>,
    design_id: String,
    tag_name: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let normalized = tag_name.trim().to_lowercase();
    if normalized.is_empty() {
        return Ok(());
    }

    let tag_id: String = db
        .query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![normalized],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| {
            let new_id = format!("tag-{}", Uuid::new_v4());
            let _ = db.execute(
                "INSERT INTO tags(id, name) VALUES(?1, ?2)",
                params![new_id, normalized],
            );
            new_id
        });

    db.execute(
        "INSERT OR IGNORE INTO design_tags(design_id, tag_id) VALUES(?1, ?2)",
        params![design_id, tag_id],
    )
    .map_err(|e| e.to_string())?;

    // Update FTS5 index for this design
    let tags_list: String = db
        .query_row(
            "SELECT GROUP_CONCAT(t.name, ' ') FROM design_tags dt JOIN tags t ON t.id = dt.tag_id WHERE dt.design_id = ?1",
            params![design_id],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let _ = db.execute(
        "UPDATE design_search SET tags = ?1 WHERE design_id = ?2",
        params![tags_list, design_id],
    );

    Ok(())
}

#[tauri::command]
pub fn remove_tag_from_design(
    state: State<AppState>,
    design_id: String,
    tag_name: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let normalized = tag_name.trim().to_lowercase();

    let tag_id: Option<String> = db
        .query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![normalized],
            |r| r.get(0),
        )
        .ok();

    if let Some(t_id) = tag_id {
        db.execute(
            "DELETE FROM design_tags WHERE design_id = ?1 AND tag_id = ?2",
            params![design_id, t_id],
        )
        .map_err(|e| e.to_string())?;

        let tags_list: String = db
            .query_row(
                "SELECT GROUP_CONCAT(t.name, ' ') FROM design_tags dt JOIN tags t ON t.id = dt.tag_id WHERE dt.design_id = ?1",
                params![design_id],
                |r| r.get(0),
            )
            .unwrap_or_default();

        let _ = db.execute(
            "UPDATE design_search SET tags = ?1 WHERE design_id = ?2",
            params![tags_list, design_id],
        );
    }

    Ok(())
}
