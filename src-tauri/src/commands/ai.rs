use crate::db::now;
use crate::models::{AiConfig, AiSuggestion, FilterOptions};
use crate::AppState;
use base64::prelude::*;

use rusqlite::params;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_ai_config(state: State<AppState>) -> Result<AiConfig, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let get_val = |k: &str, def: &str| -> String {
        db.query_row(
            "SELECT value FROM user_settings WHERE key = ?1",
            params![k],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| def.to_string())
    };

    Ok(AiConfig {
        endpoint: get_val("ai_endpoint", "https://api.openai.com/v1"),
        model: get_val("ai_model", "gpt-4o-mini"),
        api_key: get_val("ai_api_key", ""),
        enabled: get_val("ai_enabled", "false") == "true",
    })
}

#[tauri::command]
pub fn save_ai_config(state: State<AppState>, config: AiConfig) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;
    let time = now();

    let settings = [
        ("ai_endpoint", config.endpoint.trim().to_string()),
        ("ai_model", config.model.trim().to_string()),
        ("ai_api_key", config.api_key.trim().to_string()),
        ("ai_enabled", config.enabled.to_string()),
    ];

    for (k, v) in settings {
        let _ = db.execute(
            "INSERT INTO user_settings(key, value, updated_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
            params![k, v, time],
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn test_ai_connection(config: AiConfig) -> Result<String, String> {
    let endpoint = config.endpoint.trim_end_matches('/');
    let url = format!("{endpoint}/chat/completions");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;

    let payload = json!({
        "model": config.model.trim(),
        "messages": [
            { "role": "user", "content": "Respond with OK." }
        ],
        "max_tokens": 10,
        "temperature": 0.1
    });

    let mut req = client.post(&url).json(&payload);
    if !config.api_key.trim().is_empty() {
        req = req.bearer_auth(config.api_key.trim());
    }

    let resp = req.send().await.map_err(|e| {
        if e.is_connect() {
            format!("Cannot connect to AI endpoint at {endpoint}. Verify the server address and ensure your internet connection or local model server is running.")
        } else if e.is_timeout() {
            format!("AI request timed out after 12 seconds. Check endpoint latency or model responsiveness.")
        } else {
            format!("AI network request failed: {e}")
        }
    })?;

    let status = resp.status();
    if status.is_success() {
        Ok(format!(
            "Connection verified! Model '{}' responded successfully (HTTP {})",
            config.model.trim(),
            status.as_u16()
        ))
    } else {
        let err_txt = resp.text().await.unwrap_or_default();
        if status.as_u16() == 401 {
            Err("Authentication failed (HTTP 401 Unauthorized). Please check your API key.".into())
        } else if status.as_u16() == 404 {
            Err(format!("Model '{model}' not found on provider (HTTP 404). Please verify the model name.", model = config.model))
        } else if status.as_u16() == 429 {
            Err("Rate limit or quota exceeded (HTTP 429). Check your API quota.".into())
        } else {
            Err(format!("Provider returned HTTP {status}: {err_txt}"))
        }
    }
}

#[tauri::command]
pub async fn analyze_designs(
    state: State<'_, AppState>,
    design_ids: Vec<String>,
) -> Result<Vec<AiSuggestion>, String> {
    let config = get_ai_config(state.clone())?;
    if !config.enabled && config.api_key.trim().is_empty() && config.endpoint.contains("api.openai.com") {
        return Err("AI features are disabled. Please enter your API key in Settings.".into());
    }

    let mut suggestions = Vec::new();

    for id in design_ids {
        // 1. Fetch design technical metadata
        let (title, filename, format, width, height, stitches, colors, prev_path_opt, managed_path_str) = {
            let db = state.db.lock().map_err(|_| "Database is busy")?;
            db.query_row(
                "SELECT title, filename, format, width_mm, height_mm, stitches, colors, preview_path, managed_path FROM designs WHERE id = ?1",
                params![id],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, Option<f64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, String>(8)?,
                )),
            )
            .map_err(|e| format!("Design {id} not found: {e}"))?
        };

        // 2. Fetch thread colors
        let thread_desc = {
            let db = state.db.lock().map_err(|_| "Database is busy")?;
            let mut stmt = db.prepare("SELECT hex_code, brand, description FROM thread_colors WHERE design_id = ?1 ORDER BY color_index ASC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![id], |r| Ok(format!("{} ({} - {})", r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get::<_, Option<String>>(2)?.unwrap_or_default()))).map_err(|e| e.to_string())?;
            let list: Vec<String> = rows.flatten().collect();
            list.join(", ")
        };

        // 3. Ensure preview image exists and encode as base64
        let preview_file = match prev_path_opt {
            Some(ref p) if PathBuf::from(p).exists() => PathBuf::from(p),
            _ => {
                let p = state.library_root.join("library/previews").join(format!("{id}.png"));
                if !p.exists() {
                    let m_path = PathBuf::from(&managed_path_str);
                    if m_path.exists() {
                        let _ = state.adapter.render_preview(&m_path, &p, None);
                    }
                }
                p
            }
        };

        let base64_image_opt = if preview_file.exists() {
            fs::read(&preview_file).ok().map(|b| format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&b)))
        } else {
            None
        };

        let tech_meta = format!(
            "Design Title: {title}\nFilename: {filename}\nFormat: {format}\nDimensions: {:.1}x{:.1} mm\nStitches: {}\nColor Stops: {}\nThread Palette: {}",
            width.unwrap_or(0.0),
            height.unwrap_or(0.0),
            stitches.unwrap_or(0),
            colors.unwrap_or(0),
            if thread_desc.is_empty() { "Standard default colors".to_string() } else { thread_desc }
        );

        let system_prompt = "You are an expert commercial embroidery digitizer and catalog assistant. You analyze embroidery design metadata and stitch previews to extract clean catalog categories, visual subject, digitizing style, concise description, dominant thread colors, and search tags. Return strictly valid JSON.";

        let user_prompt = format!(
            "Analyze this embroidery design:\n{tech_meta}\n\nReturn strictly JSON matching this schema:\n{{\n  \"category\": \"e.g. Floral & Botanical, Animals & Wildlife, Monograms, Crests & Badges, Seasonal & Holiday, Sports\",\n  \"subject\": \"Primary visual subject\",\n  \"style\": \"e.g. Satin Stitch Outline, Tatami Fill, Applique, Cross-Stitch, Vintage\",\n  \"description\": \"One or two sentence catalog description summarizing the artwork and embroidery characteristics.\",\n  \"dominantColors\": [\"#hex1\", \"#hex2\"],\n  \"proposedTags\": [\"lowercase-tag-1\", \"lowercase-tag-2\", \"lowercase-tag-3\", \"lowercase-tag-4\"],\n  \"confidence\": 0.95\n}}"
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| e.to_string())?;

        let endpoint = config.endpoint.trim_end_matches('/');
        let url = format!("{endpoint}/chat/completions");

        // Attempt 1: Multimodal Vision payload (if image is available)
        let is_vision_model = config.model.contains("vision") || config.model.contains("4o") || config.model.contains("llava") || config.model.contains("vl");
        
        let mut request_body = if let (true, Some(b64)) = (is_vision_model, base64_image_opt.as_ref()) {
            json!({
                "model": config.model.trim(),
                "messages": [
                    { "role": "system", "content": system_prompt },
                    {
                        "role": "user",
                        "content": [
                            { "type": "text", "text": user_prompt },
                            { "type": "image_url", "image_url": { "url": b64 } }
                        ]
                    }
                ],
                "temperature": 0.2
            })
        } else {
            // Text-only string payload for text LLMs (Groq llama-3.3, mixtral, gpt-3.5, etc.)
            json!({
                "model": config.model.trim(),
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_prompt }
                ],
                "temperature": 0.2
            })
        };

        if config.endpoint.contains("api.openai.com") {
            request_body["response_format"] = json!({ "type": "json_object" });
        }

        let mut req = client.post(&url).json(&request_body);
        if !config.api_key.trim().is_empty() {
            req = req.bearer_auth(config.api_key.trim());
        }

        let mut resp = req.send().await.map_err(|e| {
            if e.is_connect() {
                format!("Cannot connect to AI endpoint at {}. Verify the server address and ensure your internet connection or local model server is running.", config.endpoint)
            } else if e.is_timeout() {
                format!("AI request timed out after 45 seconds.")
            } else {
                format!("AI network request failed: {e}")
            }
        })?;

        // Fallback retry: If provider rejected array format (e.g. Groq "messages[1].content must be a string")
        if resp.status().as_u16() == 400 {
            let err_txt = resp.text().await.unwrap_or_default();
            if err_txt.contains("must be a string") || err_txt.contains("invalid_request_error") {
                let fallback_body = json!({
                    "model": config.model.trim(),
                    "messages": [
                        { "role": "system", "content": system_prompt },
                        { "role": "user", "content": user_prompt }
                    ],
                    "temperature": 0.2
                });

                let mut retry_req = client.post(&url).json(&fallback_body);
                if !config.api_key.trim().is_empty() {
                    retry_req = retry_req.bearer_auth(config.api_key.trim());
                }
                resp = retry_req.send().await.map_err(|e| format!("AI network retry failed: {e}"))?;
            } else {
                return Err(format!("AI provider returned HTTP 400 Bad Request: {err_txt}"));
            }
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let err_txt = resp.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err("Authentication failed (HTTP 401 Unauthorized). Please verify your API key in Settings.".into());
            } else if status.as_u16() == 429 {
                return Err("Rate limit or quota exceeded (HTTP 429). Please verify your API balance or rate limits.".into());
            } else if status.as_u16() == 404 {
                return Err(format!("Model or endpoint not found (HTTP 404). Please verify the Model name '{model}' and Endpoint '{endpoint}' in Settings.", model = config.model, endpoint = config.endpoint));
            } else {
                return Err(format!("AI provider returned HTTP {status}: {err_txt}"));
            }
        }

        let resp_json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response JSON: {e}"))?;

        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "Missing message content in AI response".to_string())?;

        // Strip markdown code block wrapping if present
        let trimmed = content.trim();
        let stripped = if trimmed.starts_with("```") {
            let without_lead = trimmed.strip_prefix("```json").unwrap_or_else(|| trimmed.strip_prefix("```").unwrap_or(trimmed));
            without_lead.strip_suffix("```").unwrap_or(without_lead).trim()
        } else {
            trimmed
        };

        let parsed: Value = serde_json::from_str(stripped).map_err(|e| {
            format!("Invalid JSON in AI content: {e}. Content was: {content}")
        })?;

        let category = parsed["category"].as_str().map(str::to_string);
        let subject = parsed["subject"].as_str().map(str::to_string);
        let style = parsed["style"].as_str().map(str::to_string);
        let description = parsed["description"].as_str().map(str::to_string);
        let confidence = parsed["confidence"].as_f64().unwrap_or(0.95);

        let proposed_tags: Vec<String> = parsed["proposedTags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_lowercase().replace(' ', "-")))
                    .collect()
            })
            .unwrap_or_default();

        let dominant_colors: Vec<String> = parsed["dominantColors"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        // Store suggestion in database
        let suggestion_id = Uuid::new_v4().to_string();
        let time_now = now();

        {
            let db = state.db.lock().map_err(|_| "Database is busy")?;
            db.execute(
                "INSERT INTO ai_suggestions(id, design_id, category, subject, style, description, tags, colors, status, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)",
                params![
                    suggestion_id,
                    id,
                    category,
                    subject,
                    style,
                    description,
                    serde_json::to_string(&proposed_tags).unwrap_or_default(),
                    serde_json::to_string(&dominant_colors).unwrap_or_default(),
                    time_now
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        suggestions.push(AiSuggestion {
            id: suggestion_id,
            design_id: id,
            category,
            subject,
            style,
            description,
            tags: proposed_tags,
            dominant_colors,
            confidence,
            status: "pending".into(),
            provider: Some(config.endpoint.clone()),
            model: Some(config.model.clone()),
            created_at: time_now,
        });

    }

    Ok(suggestions)
}

#[tauri::command]
pub async fn ask_ai_custom(
    state: State<'_, AppState>,
    design_id: String,
    user_prompt: String,
) -> Result<String, String> {
    let config = get_ai_config(state.clone())?;

    // 1. Fetch design technical metadata
    let (title, filename, format, width, height, stitches, colors) = {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        db.query_row(
            "SELECT title, filename, format, width_mm, height_mm, stitches, colors FROM designs WHERE id = ?1",
            params![design_id],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<f64>>(3)?,
                r.get::<_, Option<f64>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<i64>>(6)?,
            )),
        )
        .map_err(|e| format!("Design {design_id} not found: {e}"))?
    };

    let tech_meta = format!(
        "Design Title: {title}\nFilename: {filename}\nFormat: {format}\nDimensions: {:.1}x{:.1} mm\nStitches: {}\nColors: {}",
        width.unwrap_or(0.0),
        height.unwrap_or(0.0),
        stitches.unwrap_or(0),
        colors.unwrap_or(0)
    );

    let system_prompt = format!(
        "You are an expert commercial embroidery digitizer and production advisor assistant. The user is asking about the following embroidery design:\n{tech_meta}\n\nProvide practical, concise, actionable advice regarding fabric stabilization, needle sizes, color adaptations, machine settings, or digitizing modifications in markdown format."
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let endpoint = config.endpoint.trim_end_matches('/');
    let url = format!("{endpoint}/chat/completions");

    let payload = json!({
        "model": config.model.trim(),
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0.4
    });

    let mut req = client.post(&url).json(&payload);
    if !config.api_key.trim().is_empty() {
        req = req.bearer_auth(config.api_key.trim());
    }

    let resp = req.send().await.map_err(|e| format!("AI request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let err_txt = resp.text().await.unwrap_or_default();
        return Err(format!("AI provider returned HTTP {status}: {err_txt}"));
    }

    let resp_json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Missing message content in response".to_string())?;

    Ok(content.to_string())
}

#[tauri::command]
pub fn apply_ai_suggestion(
    state: State<AppState>,
    suggestion_id: String,
    accepted: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let (design_id, category, subject, style, description, tags_json, colors_json): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
    ) = db
        .query_row(
            "SELECT design_id, category, subject, style, description, tags, colors FROM ai_suggestions WHERE id = ?1",
            params![suggestion_id],
            |r| Ok((
                r.get(0)?,
                r.get(1).ok(),
                r.get(2).ok(),
                r.get(3).ok(),
                r.get(4).ok(),
                r.get::<_, Option<String>>(5)?.unwrap_or_default(),
                r.get::<_, Option<String>>(6)?.unwrap_or_default(),
            )),
        )
        .map_err(|e| format!("Suggestion not found: {e}"))?;

    let new_status = if accepted { "accepted" } else { "rejected" };
    db.execute(
        "UPDATE ai_suggestions SET status = ?1 WHERE id = ?2",
        params![new_status, suggestion_id],
    )
    .map_err(|e| e.to_string())?;

    if accepted {
        db.execute(
            "UPDATE designs SET
                ai_category = ?1,
                ai_subject = ?2,
                ai_style = ?3,
                ai_description = ?4,
                dominant_colors = ?5
             WHERE id = ?6",
            params![category, subject, style, description, colors_json, design_id],
        )
        .map_err(|e| e.to_string())?;

        // Apply proposed tags
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_json) {
            for tag_name in tags {
                let clean_name = tag_name.trim().to_lowercase();
                if clean_name.is_empty() {
                    continue;
                }

                let tag_id = Uuid::new_v4().to_string();
                let _ = db.execute(
                    "INSERT INTO tags(id, name) VALUES(?1, ?2) ON CONFLICT(name) DO NOTHING",
                    params![tag_id, clean_name],
                );

                let actual_tag_id: String = db
                    .query_row(
                        "SELECT id FROM tags WHERE name = ?1",
                        params![clean_name],
                        |r| r.get(0),
                    )
                    .unwrap_or_default();

                if !actual_tag_id.is_empty() {
                    let _ = db.execute(
                        "INSERT INTO design_tags(design_id, tag_id) VALUES(?1, ?2) ON CONFLICT DO NOTHING",
                        params![design_id, actual_tag_id],
                    );
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn natural_language_search(query: String) -> Result<FilterOptions, String> {
    let lower = query.to_lowercase();
    let mut filters = FilterOptions::default();

    for fmt in ["dst", "pes", "jef", "vp3", "exp", "hus", "xxx", "sew", "pcs", "pec"] {
        if lower.contains(fmt) {
            filters.format = Some(fmt.to_uppercase());
            break;
        }
    }

    filters.query = Some(query);
    Ok(filters)
}
