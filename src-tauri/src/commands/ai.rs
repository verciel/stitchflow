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
    let url = format!("{endpoint}/models");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(&url);
    if !config.api_key.trim().is_empty() {
        req = req.bearer_auth(config.api_key.trim());
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Failed to reach AI endpoint: {e}"))?;

    if resp.status().is_success() {
        Ok(format!(
            "Connection successful! HTTP status {}",
            resp.status()
        ))
    } else {
        Err(format!(
            "AI endpoint returned HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ))
    }
}

#[tauri::command]
pub async fn analyze_designs(
    state: State<'_, AppState>,
    design_ids: Vec<String>,
) -> Result<Vec<AiSuggestion>, String> {
    let config = get_ai_config(state.clone())?;
    if !config.enabled {
        return Err("AI features are disabled. Please enable AI in Settings and configure your OpenAI-compatible endpoint or API key.".into());
    }

    if config.endpoint.contains("api.openai.com") && config.api_key.trim().is_empty() {
        return Err("OpenAI API key is missing. Please enter your API key in Settings (or configure a local endpoint such as Ollama or LM Studio).".into());
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

        // 2. Ensure preview image exists and encode as base64
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

        if !preview_file.exists() {
            return Err(format!("Stitch preview image not found for design '{title}'. Please ensure the embroidery file exists in your library."));
        }

        let image_bytes = fs::read(&preview_file)
            .map_err(|e| format!("Failed to read preview image for analysis: {e}"))?;
        let base64_image = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&image_bytes));

        let tech_meta = format!(
            "Design Title: {title}\nFilename: {filename}\nFormat: {format}\nDimensions: {}x{} mm\nStitches: {}\nColors: {}",
            width.unwrap_or(0.0),
            height.unwrap_or(0.0),
            stitches.unwrap_or(0),
            colors.unwrap_or(0)
        );

        let system_prompt = "You are an expert embroidery catalog assistant. You analyze rendered 2D embroidery preview images and technical facts to extract clean catalog classifications, concise description, dominant thread colors, and tags. Return strictly JSON.";

        let user_prompt = format!(
            "Analyze this embroidery preview image alongside its metadata:\n{tech_meta}\n\nReturn JSON matching this schema:\n{{\n  \"category\": \"e.g. Floral, Animals, Monogram, Crest, Seasonal\",\n  \"subject\": \"Primary visual subject\",\n  \"style\": \"e.g. Satin stitch, Fill stitch, Outline, Cross-stitch\",\n  \"description\": \"One or two sentence summary of the design suitable for a catalog description.\",\n  \"dominantColors\": [\"#hex1\", \"#hex2\"],\n  \"proposedTags\": [\"lowercase-tag-1\", \"lowercase-tag-2\", \"lowercase-tag-3\", \"lowercase-tag-4\"],\n  \"confidence\": 0.95\n}}"
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|e| e.to_string())?;

        let endpoint = config.endpoint.trim_end_matches('/');
        let url = format!("{endpoint}/chat/completions");

        let mut request_body = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": user_prompt },
                        { "type": "image_url", "image_url": { "url": base64_image } }
                    ]
                }
            ],
            "temperature": 0.2
        });

        // Add response_format json_object for OpenAI endpoints
        if config.endpoint.contains("api.openai.com") {
            request_body["response_format"] = json!({ "type": "json_object" });
        }

        let mut req = client.post(&url).json(&request_body);
        if !config.api_key.trim().is_empty() {
            req = req.bearer_auth(config.api_key.trim());
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_connect() {
                format!("Cannot connect to AI endpoint at {}. Verify the server address and ensure your internet connection or local model server is running.", config.endpoint)
            } else if e.is_timeout() {
                format!("AI request timed out after 45 seconds. Check endpoint latency or model responsiveness.")
            } else {
                format!("AI network request failed: {e}")
            }
        })?;

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

        let proposed_tags: Vec<String> = parsed["proposedTags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_lowercase()))
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

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.9);

        let sugg_id = format!("sugg-{}", Uuid::new_v4());
        let analysis_id = format!("analysis-{}", Uuid::new_v4());
        let time = now();

        // Record in database
        {
            let db = state.db.lock().map_err(|_| "Database is busy")?;

            let _ = db.execute(
                "INSERT INTO ai_analyses(id, design_id, provider, model, prompt, status, created_at)
                 VALUES(?1, ?2, 'OpenAI-compatible', ?3, ?4, 'completed', ?5)",
                params![analysis_id, id, config.model, user_prompt, time],
            );

            let _ = db.execute(
                "INSERT INTO ai_suggestions(id, analysis_id, design_id, category, subject, style, description, proposed_tags, dominant_colors, confidence, status, provider, model, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', 'OpenAI-compatible', ?11, ?12)",
                params![
                    sugg_id,
                    analysis_id,
                    id,
                    category,
                    subject,
                    style,
                    description,
                    serde_json::to_string(&proposed_tags).unwrap_or_default(),
                    serde_json::to_string(&dominant_colors).unwrap_or_default(),
                    confidence,
                    config.model,
                    time
                ],
            );
        }

        suggestions.push(AiSuggestion {
            id: sugg_id,
            design_id: id,
            category,
            subject,
            style,
            description,
            tags: proposed_tags,
            dominant_colors,
            confidence,
            status: "pending".into(),
            provider: Some("OpenAI-compatible".into()),
            model: Some(config.model.clone()),
            created_at: time,
        });
    }

    Ok(suggestions)
}

#[tauri::command]
pub fn apply_ai_suggestion(
    state: State<AppState>,
    id: String,
    accepted: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    if !accepted {
        db.execute(
            "UPDATE ai_suggestions SET status = 'dismissed' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }

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
            "SELECT design_id, category, subject, style, description, proposed_tags, dominant_colors FROM ai_suggestions WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1).ok(), r.get(2).ok(), r.get(3).ok(), r.get(4).ok(), r.get(5)?, r.get(6)?)),
        )
        .map_err(|e| format!("Suggestion not found: {e}"))?;

    // Apply metadata to design
    db.execute(
        "UPDATE designs SET 
            ai_category = COALESCE(?1, ai_category),
            ai_subject = COALESCE(?2, ai_subject),
            ai_style = COALESCE(?3, ai_style),
            ai_description = COALESCE(?4, ai_description),
            dominant_colors = ?5
         WHERE id = ?6",
        params![
            category,
            subject,
            style,
            description,
            colors_json,
            design_id
        ],
    )
    .map_err(|e| e.to_string())?;

    // Add proposed tags
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    for tag in tags {
        let tag_clean = tag.trim().to_lowercase();
        if tag_clean.is_empty() {
            continue;
        }

        let tag_id = format!("tag-{tag_clean}");
        let _ = db.execute(
            "INSERT OR IGNORE INTO tags(id, name) VALUES(?1, ?2)",
            params![tag_id, tag_clean],
        );
        let _ = db.execute(
            "INSERT OR IGNORE INTO design_tags(design_id, tag_id) VALUES(?1, ?2)",
            params![design_id, tag_id],
        );
    }

    // Mark suggestion as accepted
    db.execute(
        "UPDATE ai_suggestions SET status = 'accepted' WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    // Sync FTS5
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
pub fn natural_language_search(query: String) -> Result<FilterOptions, String> {
    let lower = query.to_lowercase();
    let mut filters = FilterOptions::default();

    // Detect format mentions
    for fmt in ["dst", "pes", "jef", "vp3", "exp", "hus", "xxx", "sew", "pcs", "pec"] {
        if lower.contains(fmt) {
            filters.format = Some(fmt.to_uppercase());
            break;
        }
    }

    // Pass the query as live search
    filters.query = Some(query);
    Ok(filters)
}

#[tauri::command]
pub fn get_workflow_advice(state: State<AppState>, design_id: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|_| "Database is busy")?;

    let (title, format, width, height, stitches, colors): (String, String, Option<f64>, Option<f64>, Option<i64>, Option<i64>) = db
        .query_row(
            "SELECT title, format, width_mm, height_mm, stitches, colors FROM designs WHERE id = ?1",
            params![design_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2).ok(), r.get(3).ok(), r.get(4).ok(), r.get(5).ok())),
        )
        .map_err(|e| e.to_string())?;

    let w = width.unwrap_or(0.0);
    let h = height.unwrap_or(0.0);
    let st = stitches.unwrap_or(0);
    let col = colors.unwrap_or(1);

    let mut advice = format!("### Production Assessment for \"{title}\" ({format})\n\n");
    advice.push_str(&format!("- **Dimensions**: {w:.1} × {h:.1} mm\n"));
    advice.push_str(&format!("- **Stitch Count**: {st} stitches\n"));
    advice.push_str(&format!("- **Color Changes**: {col} colors\n\n"));

    if w <= 100.0 && h <= 100.0 {
        advice.push_str("✅ **Placement**: Ideal for left-chest embroidery, caps, beanie cuffs, or sleeve accents (fits standard 100×100 mm hoop).\n");
    } else if w <= 180.0 && h <= 130.0 {
        advice.push_str("✅ **Placement**: Ideal for 5×7\" (130×180 mm) garment fronts, tote bags, or mid-size jacket decorations.\n");
    } else {
        advice.push_str("⚠️ **Placement**: Large design; requires a 200×200 mm or larger jacket-back hoop.\n");
    }

    let density = if w > 0.0 && h > 0.0 {
        (st as f64) / (w * h)
    } else {
        0.0
    };

    if density > 3.0 {
        advice.push_str("⚠️ **Fabric Recommendation**: High stitch density detected. Use heavy cut-away stabilizer to prevent fabric puckering, especially on knit materials or pique polos.\n");
    } else {
        advice.push_str("✅ **Fabric Recommendation**: Moderate stitch density. Suitable for standard woven cotton, denim, canvas, and stabilized knits with medium tear-away or cut-away backing.\n");
    }

    advice.push_str(&format!("\n💡 **Run Time Estimate**: Approx. {} minutes at 650 stitches/min (including color stops).", (st / 650) + (col * 1)));

    Ok(advice)
}
