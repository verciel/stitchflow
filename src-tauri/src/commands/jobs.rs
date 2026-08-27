use crate::db::now;
use crate::models::Job;
use crate::AppState;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_jobs(state: State<AppState>) -> Result<Vec<Job>, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let mut stmt = db
        .prepare(
            "SELECT 
                j.id, 
                j.title, 
                j.notes, 
                j.status, 
                COUNT(DISTINCT jd.design_id) as d_count,
                COUNT(DISTINCT ja.asset_id) as a_count,
                j.created_at, 
                j.updated_at 
             FROM jobs j 
             LEFT JOIN job_designs jd ON j.id = jd.job_id 
             LEFT JOIN job_assets ja ON j.id = ja.job_id
             GROUP BY j.id 
             ORDER BY j.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Job {
                id: row.get(0)?,
                title: row.get(1)?,
                notes: row.get(2)?,
                status: row.get(3)?,
                design_count: row.get(4)?,
                artwork_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let jobs = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(jobs)
}

#[tauri::command]
pub fn create_job(
    state: State<AppState>,
    title: String,
    notes: Option<String>,
    status: Option<String>,
) -> Result<Job, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let id = format!("job-{}", Uuid::new_v4());
    let note_text = notes.unwrap_or_default();
    let job_status = status.unwrap_or_else(|| "draft".into());
    let time = now();

    db.execute(
        "INSERT INTO jobs(id, title, notes, status, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, title.trim(), note_text.trim(), job_status, time],
    )
    .map_err(|e| format!("Could not create job: {e}"))?;

    Ok(Job {
        id,
        title: title.trim().to_string(),
        notes: note_text.trim().to_string(),
        status: job_status,
        design_count: 0,
        artwork_count: 0,
        created_at: time.clone(),
        updated_at: time,
    })
}

#[tauri::command]
pub fn update_job(
    state: State<AppState>,
    id: String,
    title: String,
    notes: Option<String>,
    status: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let note_text = notes.unwrap_or_default();
    let job_status = status.unwrap_or_else(|| "draft".into());
    let time = now();

    db.execute(
        "UPDATE jobs SET title = ?1, notes = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
        params![title.trim(), note_text.trim(), job_status, time, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_job(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute("DELETE FROM jobs WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn add_design_to_job(
    state: State<AppState>,
    job_id: String,
    design_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "INSERT OR IGNORE INTO job_designs(job_id, design_id) VALUES(?1, ?2)",
        params![job_id, design_id],
    )
    .map_err(|e| e.to_string())?;

    let _ = db.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![now(), job_id],
    );

    Ok(())
}

#[tauri::command]
pub fn remove_design_from_job(
    state: State<AppState>,
    job_id: String,
    design_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "DELETE FROM job_designs WHERE job_id = ?1 AND design_id = ?2",
        params![job_id, design_id],
    )
    .map_err(|e| e.to_string())?;

    let _ = db.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![now(), job_id],
    );

    Ok(())
}

#[tauri::command]
pub fn add_artwork_to_job(
    state: State<AppState>,
    job_id: String,
    asset_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "INSERT OR IGNORE INTO job_assets(job_id, asset_id) VALUES(?1, ?2)",
        params![job_id, asset_id],
    )
    .map_err(|e| e.to_string())?;

    let _ = db.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![now(), job_id],
    );

    Ok(())
}

#[tauri::command]
pub fn remove_artwork_from_job(
    state: State<AppState>,
    job_id: String,
    asset_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    db.execute(
        "DELETE FROM job_assets WHERE job_id = ?1 AND asset_id = ?2",
        params![job_id, asset_id],
    )
    .map_err(|e| e.to_string())?;

    let _ = db.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![now(), job_id],
    );

    Ok(())
}
