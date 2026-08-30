use crate::adapter::ThreadInfo;
use crate::db::now;
use crate::models::{AiConfig, AiSuggestion, FilterOptions};
use crate::AppState;
use base64::prelude::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};
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
        let (title, filename, format, width, height, stitches, colors, prev_path_opt, managed_path_str, dom_colors_str, threads_json_str) = {
            let db = state.db.lock().map_err(|_| "Database is busy")?;
            db.query_row(
                "SELECT title, filename, format, width_mm, height_mm, stitches, colors, preview_path, managed_path, dominant_colors, threads_json FROM designs WHERE id = ?1",
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
                    r.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(10)?.unwrap_or_default(),
                )),
            )
            .map_err(|e| format!("Design {id} not found: {e}"))?
        };

        // 2. Fetch thread colors
        let thread_desc = if !threads_json_str.is_empty() {
            if let Ok(threads) = serde_json::from_str::<Vec<crate::adapter::ThreadInfo>>(&threads_json_str) {
                let list: Vec<String> = threads
                    .into_iter()
                    .map(|t| format!("{} ({} - {})", t.hex, t.brand, t.description))
                    .collect();
                list.join(", ")
            } else {
                dom_colors_str
            }
        } else {
            dom_colors_str
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
                "INSERT INTO ai_suggestions(id, design_id, payload, category, subject, style, description, proposed_tags, dominant_colors, confidence, status, provider, model, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?12, ?13)",
                params![
                    suggestion_id,
                    id,
                    stripped,
                    category,
                    subject,
                    style,
                    description,
                    serde_json::to_string(&proposed_tags).unwrap_or_default(),
                    serde_json::to_string(&dominant_colors).unwrap_or_default(),
                    confidence,
                    config.endpoint.clone(),
                    config.model.clone(),
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
        "You are an expert commercial embroidery digitizer, artist, and studio production director.\n\
        The current reference design being inspected by the user is:\n\
        {tech_meta}\n\n\
        CRITICAL RULES FOR RESPONSES:\n\
        1. When the user asks for 'similar designs', 'companion designs', 'matching series', or 'ideas for this theme':\n\
           DO NOT give generic web search tutorials, software search instructions, or marketplace browsing advice.\n\
           INSTEAD, directly brainstorm and detail 3 to 5 SPECIFIC, creative companion embroidery designs that match the exact same visual style, motif, and aesthetic!\n\
           For EACH companion design concept, provide:\n\
           - **Design Title & Motif**: (e.g. 'Haunted Crescent Moon with Bats', 'Black Cat on Jack-o-Lantern')\n\
           - **Visual Description & Aesthetic Style**: (e.g. matching single-color silhouette, fine satin outline, tatami fill)\n\
           - **Recommended Target Dimensions**: (e.g. 54x52 mm to match original frame)\n\
           - **Target Stitch Count**: (e.g. ~2,800 stitches)\n\
           - **Thread & Needle Recommendation**: (e.g. Madeira Polyneon #1800 Black, 75/11 Ballpoint)\n\
           - **Digitizing Direction**: (e.g. 'Start with center underlay, satin edge with 0.4mm density')\n\n\
        2. For questions about fabric stabilization, needle selection, cap hoops, resizing, or recoloring for dark fabrics:\n\
           Provide direct, shop-ready production recipes and technical digitizing parameters in clean markdown."
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
            "SELECT design_id, category, subject, style, description, proposed_tags, dominant_colors FROM ai_suggestions WHERE id = ?1",
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedArtworkResult {
    pub image_data: String,
    pub temp_path: String,
    pub prompt_used: String,
}

#[tauri::command]
pub async fn generate_ai_design_image(
    state: State<'_, AppState>,
    design_id: String,
    custom_prompt: Option<String>,
    style_mode: Option<String>,
) -> Result<GeneratedArtworkResult, String> {
    // 1. Fetch design title and metadata for context
    let (title, _format, _width, _height, ai_subject, ai_category, _ai_style): (
        String,
        String,
        Option<f64>,
        Option<f64>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        db.query_row(
            "SELECT d.title, d.format, d.width_mm, d.height_mm, s.subject, s.category, s.style 
             FROM designs d 
             LEFT JOIN ai_suggestions s ON s.design_id = d.id 
             WHERE d.id = ?1",
            params![design_id],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<f64>>(2)?,
                r.get::<_, Option<f64>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            )),
        )
        .unwrap_or_else(|_| (design_id.clone(), "PES".to_string(), Some(50.0), Some(50.0), None, None, None))
    };

    // 2. Fetch optional custom endpoint settings
    let (custom_token, custom_endpoint) = {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        let get_val = |k: &str, def: &str| -> String {
            db.query_row(
                "SELECT value FROM user_settings WHERE key = ?1",
                params![k],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| def.to_string())
        };
        (
            get_val("hf_api_token", ""),
            get_val("hf_model", ""),
        )
    };

    // 3. Construct curated embroidery style prompt
    let base_theme = custom_prompt.unwrap_or_else(|| {
        let subj = ai_subject.as_deref().unwrap_or(&title);
        let cat = ai_category.as_deref().unwrap_or("Embroidery Motif");
        format!("{cat}, {subj}")
    });

    let clean_theme = base_theme.trim().trim_matches(',').trim();
    let mode = style_mode.as_deref().unwrap_or("patch");

    let prompt = match mode {
        "silhouette" => format!(
            "solid black silhouette cutout of {clean_theme}, flat black stencil icon, single color solid shape, isolated on pure solid white background, high contrast vector graphic, no interior details, no shading, no gradients, no photorealism"
        ),
        "patch" => format!(
            "die-cut embroidered patch badge of {clean_theme}, bold thick black satin outline border, 3 flat solid color fills, clean vector sticker art, isolated on pure white background, screenprint graphic, no gradients, no soft shading, no 3D textures, no shadows"
        ),
        "line_art" => format!(
            "continuous single line art outline of {clean_theme}, vintage redwork embroidery line drawing, clean black line illustration, no fills, no color, no shading, minimalist contour drawing on pure white background, no gradients"
        ),
        "crest" => format!(
            "vintage heraldic crest emblem of {clean_theme}, symmetrical collegiate shield badge with laurel wreath and ribbon, flat vector engraving style, clean bold shapes, isolated on pure white background, no gradients, no shading"
        ),
        "floral" => format!(
            "stylized botanical floral embroidery motif of {clean_theme}, flat folk art meadow flowers and leaves, clean solid shapes, bold color separation, isolated on pure white background, no gradients, no 3D textures"
        ),
        "applique" => format!(
            "minimalist flat appliqué shapes of {clean_theme}, simple bold outlines, clean solid cartoon shapes, minimal interior lines, isolated on pure white background, no texture, no gradients"
        ),
        _ => format!(
            "simple flat 2D vector embroidery patch of {clean_theme}, bold black contour lines, solid flat color fills, minimalist clip art, die-cut embroidered sticker graphic, isolated on pure white background, no gradients, no soft shading, no 3D rendering, no shadows, no photorealism"
        ),
    };


    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;


    let image_bytes: Vec<u8> = if custom_endpoint.starts_with("http://") || custom_endpoint.starts_with("https://") {
        // Dedicated custom endpoint (OpenAI, Together, Hugging Face, Automatic1111, or Custom API)
        let mut req = client.post(&custom_endpoint);
        if !custom_token.trim().is_empty() {
            req = req.bearer_auth(custom_token.trim());
        }
        let resp = req
            .json(&json!({
                "inputs": prompt,
                "prompt": prompt,
                "n": 1,
                "size": "512x512",
                "response_format": "b64_json"
            }))
            .send()
            .await
            .map_err(|e| format!("Custom endpoint request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(format!("Endpoint returned HTTP {status}: {txt}"));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if content_type.contains("json") {
            let json_val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse JSON response from custom endpoint: {e}"))?;

            if let Some(b64) = json_val.get("data").and_then(|d| d.get(0)).and_then(|i| i.get("b64_json")).and_then(|s| s.as_str()) {
                BASE64_STANDARD.decode(b64).map_err(|e| format!("Invalid base64 image data: {e}"))?
            } else if let Some(img_url) = json_val.get("data").and_then(|d| d.get(0)).and_then(|i| i.get("url")).and_then(|s| s.as_str()) {
                let img_resp = client.get(img_url).send().await.map_err(|e| format!("Failed to download image from URL: {e}"))?;
                img_resp.bytes().await.map_err(|e| format!("Failed to read image bytes: {e}"))?.to_vec()
            } else if let Some(b64) = json_val.get("images").and_then(|arr| arr.get(0)).and_then(|s| s.as_str()) {
                BASE64_STANDARD.decode(b64).map_err(|e| format!("Invalid base64 from WebUI: {e}"))?
            } else {
                return Err(format!("Could not extract image from endpoint response: {json_val}"));
            }
        } else {
            resp.bytes().await.map_err(|e| format!("Failed to read image bytes: {e}"))?.to_vec()
        }
    } else {

        // High-reliability Instant Free Image Generation (No API Key Required)
        let seed = chrono::Utc::now().timestamp_subsec_millis();
        let encoded_prompt: String = prompt
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c.to_string() } else if c == ' ' { "%20".to_string() } else { format!("%{:02X}", c as u32) })
            .collect();

        let instant_url = format!(
            "https://image.pollinations.ai/prompt/{encoded_prompt}?width=512&height=512&nologo=true&seed={seed}"
        );

        let resp = client
            .get(&instant_url)
            .send()
            .await
            .map_err(|e| format!("Connection to design generator failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(format!("Design generator returned HTTP {status}"));
        }
        resp.bytes().await.map_err(|e| format!("Failed to read image data: {e}"))?.to_vec()
    };

    // Save to temp file in library/artwork
    let temp_filename = format!("ai_gen_{}.png", Uuid::new_v4());
    let temp_path = state.library_root.join("library").join("artwork").join(&temp_filename);
    let _ = fs::create_dir_all(state.library_root.join("library").join("artwork"));
    fs::write(&temp_path, &image_bytes).map_err(|e| format!("Failed to save generated image: {e}"))?;


    let b64 = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&image_bytes));

    Ok(GeneratedArtworkResult {
        image_data: b64,
        temp_path: temp_path.to_string_lossy().to_string(),
        prompt_used: prompt,
    })
}

#[tauri::command]
pub async fn test_hf_connection(
    token: Option<String>,
    model: Option<String>,
) -> Result<String, String> {
    let tok = token.unwrap_or_default().trim().to_string();
    let endpoint = model.unwrap_or_default().trim().to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Stitchflow/0.1.0 (Windows)")
        .build()
        .map_err(|e| e.to_string())?;

    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        // Test custom endpoint
        let mut req = client.get(&endpoint);
        if !tok.is_empty() {
            req = req.bearer_auth(&tok);
        }
        let resp = req.send().await.map_err(|e| format!("Network error reaching endpoint: {e}"))?;
        if resp.status().is_success() || resp.status().as_u16() == 404 || resp.status().as_u16() == 405 {
            return Ok("Connected to custom creation endpoint successfully!".into());
        }
        return Err(format!("Endpoint returned HTTP {}", resp.status()));
    }

    if !tok.is_empty() {
        // Verify Hugging Face token
        let whoami_resp = client
            .get("https://huggingface.co/api/whoami-v2")
            .bearer_auth(&tok)
            .send()
            .await
            .map_err(|e| format!("Network error reaching Hugging Face: {e}"))?;

        if whoami_resp.status().is_success() {
            let json_body: serde_json::Value = whoami_resp.json().await.unwrap_or_default();
            let username = json_body.get("name").and_then(|v| v.as_str()).unwrap_or("user");
            return Ok(format!("Connected to Hugging Face successfully! Authenticated as @{username} (Ready to generate designs)."));
        } else if whoami_resp.status().as_u16() == 401 || whoami_resp.status().as_u16() == 403 {
            return Err("Invalid Hugging Face Token (HTTP 401). Please check your token at huggingface.co/settings/tokens".into());
        }
    }

    // Default free instant service
    let resp = client
        .get("https://image.pollinations.ai/prompt/test?width=64&height=64&nologo=true")
        .send()
        .await
        .map_err(|e| format!("Connection error: {e}"))?;

    if resp.status().is_success() {
        Ok("Free Instant Design Generator is online and ready! (No setup or API key required)".into())
    } else {
        Err(format!("Service returned HTTP {}", resp.status()))
    }
}







#[tauri::command]
pub fn digitize_and_import_design(
    state: State<AppState>,
    source_image_path: String,
    title: String,
    target_format: String,
    width_mm: f64,
    height_mm: f64,
    tags: Vec<String>,
    category: Option<String>,
) -> Result<crate::models::Design, String> {
    let src_img = PathBuf::from(&source_image_path);
    if !src_img.exists() {
        return Err("Source artwork image not found on disk.".into());
    }

    let id = Uuid::new_v4().to_string();
    let raw_fmt = target_format.to_lowercase().replace('.', "");
    let fmt_clean = match raw_fmt.as_str() {
        "pes" | "dst" | "jef" | "exp" | "vp3" | "xxx" | "pec" | "u01" | "tbf" => raw_fmt,
        _ => "pes".to_string(),
    };
    let filename = format!("{title}.{fmt_clean}");


    let managed_dst = state
        .library_root
        .join("library")
        .join("designs")
        .join(format!("{id}.{fmt_clean}"));
    let preview_dst = state
        .library_root
        .join("library")
        .join("previews")
        .join(format!("{id}.png"));

    let _ = fs::create_dir_all(state.library_root.join("library").join("designs"));
    let _ = fs::create_dir_all(state.library_root.join("library").join("previews"));


    let meta = state
        .adapter
        .digitize(
            &src_img,
            &managed_dst,
            &fmt_clean,
            width_mm,
            height_mm,
            Some(&preview_dst),
        )
        .map_err(|e| format!("Auto-digitizing failed: {e}"))?;

    let checksum_val = crate::db::checksum(&managed_dst).unwrap_or_default();
    let size_bytes = managed_dst.metadata().map(|m| m.len() as i64).unwrap_or(0);
    let time_now = now();

    let dom_colors = meta
        .threads
        .iter()
        .map(|t| t.hex.clone())
        .collect::<Vec<String>>();
    let dom_colors_json = serde_json::to_string(&dom_colors).unwrap_or_default();
    let threads_json = serde_json::to_string(&meta.threads).unwrap_or_default();

    {
        let db = state.db.lock().map_err(|_| "Database is busy")?;

        db.execute(
            "INSERT INTO designs(id, title, filename, managed_path, preview_path, checksum, format, width_mm, height_mm, stitches, colors, size_bytes, source_path, status, ai_category, ai_subject, ai_style, ai_description, dominant_colors, threads_json, imported_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'active', ?14, ?15, 'Auto-Digitized AI Motif', 'Generated via Hugging Face and auto-digitized by Stitchflow', ?16, ?17, ?18)",
            params![
                id,
                title,
                filename,
                managed_dst.to_string_lossy().to_string(),
                preview_dst.to_string_lossy().to_string(),
                checksum_val,
                fmt_clean.to_uppercase(),
                meta.width_mm,
                meta.height_mm,
                meta.stitches as i64,
                meta.colors as i64,
                size_bytes,
                src_img.to_string_lossy().to_string(),
                category.as_deref().unwrap_or("AI Generated"),
                title,
                dom_colors_json,
                threads_json,
                time_now
            ],
        )
        .map_err(|e| format!("Failed to insert design: {e}"))?;

        // Insert tags
        for t in &tags {
            let clean_t = t.trim().to_lowercase().replace(' ', "-");
            if clean_t.is_empty() {
                continue;
            }
            let tag_id = Uuid::new_v4().to_string();
            let _ = db.execute(
                "INSERT INTO tags(id, name) VALUES(?1, ?2) ON CONFLICT(name) DO NOTHING",
                params![tag_id, clean_t],
            );
            let actual_tag_id: String = db
                .query_row(
                    "SELECT id FROM tags WHERE name = ?1",
                    params![clean_t],
                    |r| r.get(0),
                )
                .unwrap_or_default();

            if !actual_tag_id.is_empty() {
                let _ = db.execute(
                    "INSERT INTO design_tags(design_id, tag_id) VALUES(?1, ?2) ON CONFLICT DO NOTHING",
                    params![id, actual_tag_id],
                );
            }
        }
    }

    Ok(crate::models::Design {
        id: id.clone(),
        title,
        filename,
        format: fmt_clean.to_uppercase(),
        width_mm: Some(meta.width_mm),
        height_mm: Some(meta.height_mm),
        stitches: Some(meta.stitches as i64),
        colors: Some(meta.colors as i64),
        size_bytes,
        tags,
        collection: None,
        collection_id: None,
        job: None,
        job_id: None,
        imported_at: time_now,
        duplicate: false,
        preview_url: Some(preview_dst.to_string_lossy().to_string()),
        preview_path: Some(preview_dst.to_string_lossy().to_string()),
        managed_path: Some(managed_dst.to_string_lossy().to_string()),
        status: "active".into(),
        ai_category: category.or(Some("AI Generated".into())),
        ai_subject: None,
        ai_style: Some("Auto-Digitized AI Motif".into()),
        ai_description: Some("Generated via Hugging Face and auto-digitized by Stitchflow".into()),
        dominant_colors: dom_colors,
        threads: meta.threads,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    pub op: String,
    #[serde(default)]
    pub stop_index: Option<usize>,
    #[serde(default)]
    pub from_color: Option<String>,
    #[serde(default)]
    pub to_hex: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub factor: Option<f64>,
    #[serde(default)]
    pub angle_deg: Option<f64>,
    #[serde(default)]
    pub axis: Option<String>,
    #[serde(default)]
    pub max_dimension_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedEditResult {
    pub design_id: String,
    pub instruction: String,
    pub applied_operations: Vec<EditOperation>,
    pub density_warning: Option<String>,
    pub temp_edited_path: String,
    pub temp_preview_path: String,
    pub proposed_preview_data: String,
    pub original_width_mm: f64,
    pub original_height_mm: f64,
    pub original_stitches: usize,
    pub original_colors: usize,
    pub proposed_width_mm: f64,
    pub proposed_height_mm: f64,
    pub proposed_stitches: usize,
    pub proposed_colors: usize,
    pub proposed_threads: Vec<ThreadInfo>,
}


fn parse_deterministic_edit_ops(
    instruction: &str,
    _orig_w: f64,
    _orig_h: f64,
    threads_count: usize,
) -> Vec<EditOperation> {

    let lower = instruction.to_lowercase();
    let mut ops = Vec::new();

    // 1. Scale operations
    if lower.contains("smaller") || lower.contains("reduce") || lower.contains("shrink") {
        let mut factor = 0.80; // default 20% smaller
        if lower.contains("10%") { factor = 0.90; }
        else if lower.contains("15%") { factor = 0.85; }
        else if lower.contains("20%") { factor = 0.80; }
        else if lower.contains("25%") { factor = 0.75; }
        else if lower.contains("30%") { factor = 0.70; }
        else if lower.contains("40%") { factor = 0.60; }
        else if lower.contains("50%") || lower.contains("half") { factor = 0.50; }
        ops.push(EditOperation {
            op: "scale".into(),
            stop_index: None,
            from_color: None,
            to_hex: None,
            description: None,
            factor: Some(factor),
            angle_deg: None,
            axis: None,
            max_dimension_mm: None,
        });
    } else if lower.contains("bigger") || lower.contains("enlarge") || lower.contains("larger") || lower.contains("increase") {
        let mut factor = 1.20; // default 20% bigger
        if lower.contains("10%") { factor = 1.10; }
        else if lower.contains("15%") { factor = 1.15; }
        else if lower.contains("20%") { factor = 1.20; }
        else if lower.contains("25%") { factor = 1.25; }
        else if lower.contains("30%") { factor = 1.30; }
        else if lower.contains("50%") { factor = 1.50; }
        else if lower.contains("2x") || lower.contains("double") { factor = 2.00; }
        ops.push(EditOperation {
            op: "scale".into(),
            stop_index: None,
            from_color: None,
            to_hex: None,
            description: None,
            factor: Some(factor),
            angle_deg: None,
            axis: None,
            max_dimension_mm: None,
        });
    }

    // 2. Rotate operations (also catches 'flip 90 degrees')
    if lower.contains("rotate") || lower.contains("turn") || lower.contains("spin") || (lower.contains("flip") && (lower.contains("90") || lower.contains("180") || lower.contains("270") || lower.contains("deg"))) {
        let mut angle = 90.0;
        if lower.contains("180") { angle = 180.0; }
        else if lower.contains("270") { angle = 270.0; }
        else if lower.contains("45") { angle = 45.0; }
        else if lower.contains("90") { angle = 90.0; }
        ops.push(EditOperation {
            op: "rotate".into(),
            stop_index: None,
            from_color: None,
            to_hex: None,
            description: None,
            factor: None,
            angle_deg: Some(angle),
            axis: None,
            max_dimension_mm: None,
        });
    } else if lower.contains("flip") || lower.contains("mirror") {
        // 3. Flip / Mirror operations (only when not specified with rotation degrees)
        let axis = if lower.contains("vertical") || lower.contains("upside") || lower.contains("y") { "vertical" } else { "horizontal" };
        ops.push(EditOperation {
            op: "flip".into(),
            stop_index: None,
            from_color: None,
            to_hex: None,
            description: None,
            factor: None,
            angle_deg: None,
            axis: Some(axis.into()),
            max_dimension_mm: None,
        });
    }


    // 4. Fit hoop operations
    if lower.contains("hoop") {
        let mut max_dim = 60.0;
        if lower.contains("40") { max_dim = 40.0; }
        else if lower.contains("50") { max_dim = 50.0; }
        else if lower.contains("60") { max_dim = 60.0; }
        else if lower.contains("80") { max_dim = 80.0; }
        else if lower.contains("100") { max_dim = 100.0; }
        ops.push(EditOperation {
            op: "fit_hoop".into(),
            stop_index: None,
            from_color: None,
            to_hex: None,
            description: None,
            factor: None,
            angle_deg: None,
            axis: None,
            max_dimension_mm: Some(max_dim),
        });
    }


    // 5. Recolor operations
    let color_map = [
        ("royal blue", "#1D4ED8", "Royal Blue"),
        ("navy blue", "#1E3A8A", "Navy Blue"),
        ("navy", "#1E3A8A", "Navy Blue"),
        ("blue", "#2563EB", "Blue"),
        ("emerald green", "#059669", "Emerald Green"),
        ("forest green", "#15803D", "Forest Green"),
        ("emerald", "#059669", "Emerald Green"),
        ("green", "#16A34A", "Green"),
        ("ruby red", "#BE123C", "Ruby Red"),
        ("crimson", "#991B1B", "Crimson"),
        ("red", "#DC2626", "Red"),
        ("gold", "#EAB308", "Gold"),
        ("yellow", "#FACC15", "Yellow"),
        ("purple", "#9333EA", "Purple"),
        ("violet", "#7C3AED", "Violet"),
        ("pink", "#EC4899", "Pink"),
        ("orange", "#EA580C", "Orange"),
        ("black", "#000000", "Black"),
        ("white", "#FFFFFF", "White"),
        ("silver", "#94A3B8", "Silver"),
        ("teal", "#0D9488", "Teal"),
    ];

    // Check "change X to Y" pattern (e.g. "change red thread to royal blue")
    let mut matched_recolor = false;
    for (name_dest, hex_dest, desc_dest) in color_map {
        if lower.contains(&format!("to {name_dest}")) || lower.ends_with(name_dest) {
            // Find if a source color is mentioned (e.g. "red thread", "green parts")
            let mut from_col = None;
            for (name_src, _, _) in color_map {
                if name_src != name_dest && (lower.contains(&format!("change {name_src}")) || lower.contains(&format!("{name_src} thread")) || lower.contains(&format!("{name_src} to"))) {
                    from_col = Some(name_src.to_string());
                    break;
                }
            }

            let stop_idx = if lower.contains("stop 2") || lower.contains("thread 2") || lower.contains("second") {
                Some(1)
            } else if lower.contains("stop 3") || lower.contains("thread 3") || lower.contains("third") {
                Some(2)
            } else if from_col.is_some() {
                None
            } else {
                Some(0)
            };

            ops.push(EditOperation {
                op: "recolor_stop".into(),
                stop_index: stop_idx,
                from_color: from_col,
                to_hex: Some(hex_dest.into()),
                description: Some(desc_dest.into()),
                factor: None,
                angle_deg: None,
                axis: None,
                max_dimension_mm: None,
            });
            matched_recolor = true;
            break;
        }
    }

    if !matched_recolor {
        for (name, hex, desc) in color_map {
            if lower.contains(name) {
                let stop_idx = if lower.contains("stop 2") || lower.contains("thread 2") || lower.contains("second") {
                    1
                } else if lower.contains("stop 3") || lower.contains("thread 3") || lower.contains("third") {
                    2
                } else {
                    0
                };

                if stop_idx < threads_count || threads_count == 0 {
                    ops.push(EditOperation {
                        op: "recolor_stop".into(),
                        stop_index: Some(stop_idx),
                        from_color: None,
                        to_hex: Some(hex.into()),
                        description: Some(desc.into()),
                        factor: None,
                        angle_deg: None,
                        axis: None,
                        max_dimension_mm: None,
                    });
                    break;
                }
            }
        }
    }

    ops
}

#[tauri::command]
pub async fn propose_ai_edit(
    state: State<'_, AppState>,
    design_id: String,
    instruction: String,
) -> Result<ProposedEditResult, String> {
    let clean_inst = instruction.trim().to_string();
    if clean_inst.is_empty() {
        return Err("Please enter an editing instruction (e.g. 'Make this 20% smaller' or 'Change thread #1 to royal blue').".into());
    }

    // 1. Fetch source design metadata from DB
    let (_title, fmt, managed_path_str, width_mm, height_mm, stitches, colors, threads_json): (
        String,
        String,
        String,
        f64,
        f64,
        i64,
        i64,
        String,
    ) = {
        let db = state.db.lock().map_err(|_| "Database is busy")?;
        db.query_row(
            "SELECT title, format, managed_path, COALESCE(width_mm, 50.0), COALESCE(height_mm, 50.0), COALESCE(stitches, 1000), COALESCE(colors, 1), COALESCE(threads_json, '[]')
             FROM designs WHERE id = ?1",
            params![design_id],
            |r| Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            )),
        )
        .map_err(|e| format!("Design not found: {e}"))?
    };

    let orig_threads: Vec<ThreadInfo> = serde_json::from_str(&threads_json).unwrap_or_default();
    let src_path = PathBuf::from(&managed_path_str);
    if !src_path.exists() {
        return Err("Original embroidery file not found on disk.".into());
    }

    // 2. Translate instruction to structured operations
    let ops = parse_deterministic_edit_ops(&clean_inst, width_mm, height_mm, orig_threads.len());
    if ops.is_empty() {
        return Err("Could not determine a valid safe operation for this request. Please try: 'Make 20% smaller', 'Rotate 90 deg', 'Flip horizontal', or 'Change thread 1 to gold'.".into());
    }

    let ops_json = serde_json::to_string(&ops).map_err(|e| e.to_string())?;

    // 3. Prepare temporary files
    let edit_id = Uuid::new_v4().to_string();
    let raw_ext = fmt.to_lowercase().replace('.', "");
    let ext = match raw_ext.as_str() {
        "pes" | "dst" | "jef" | "exp" | "vp3" | "xxx" | "pec" | "u01" | "tbf" => raw_ext,
        _ => "pes".to_string(),
    };
    let temp_dir = state.library_root.join("library").join("temp");
    let _ = fs::create_dir_all(&temp_dir);


    let temp_edited_dst = temp_dir.join(format!("edit_{edit_id}.{ext}"));
    let temp_prev_dst = temp_dir.join(format!("edit_prev_{edit_id}.png"));

    // 4. Execute deterministic sidecar edit
    let meta = state
        .adapter
        .edit(&src_path, &temp_edited_dst, &ops_json, Some(&temp_prev_dst))
        .map_err(|e| format!("Embroidery editing engine failed: {e}"))?;

    // 5. Calculate stitch density warning if scaling is significant
    let density_warning = if meta.width_mm > 0.0 && width_mm > 0.0 {
        let area_ratio = (meta.width_mm * meta.height_mm) / (width_mm * height_mm);
        if area_ratio < 0.65 {
            Some(format!(
                "⚠️ Scaling down to {:.1} × {:.1} mm significantly increases stitch density (+{:.0}% density). Recommend test sewout or adjusting density in Ink/Stitch.",
                meta.width_mm,
                meta.height_mm,
                (1.0 / area_ratio - 1.0) * 100.0
            ))
        } else if area_ratio > 1.50 {
            Some(format!(
                "⚠️ Scaling up to {:.1} × {:.1} mm increases stitch spacing. Fills may appear less dense.",
                meta.width_mm,
                meta.height_mm
            ))
        } else {
            None
        }
    } else {
        None
    };

    // 6. Read generated proposed preview as base64
    let prev_bytes = fs::read(&temp_prev_dst).map_err(|e| format!("Failed to read proposed preview: {e}"))?;
    let b64_preview = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&prev_bytes));

    Ok(ProposedEditResult {
        design_id,
        instruction: clean_inst,
        applied_operations: ops,
        density_warning,
        temp_edited_path: temp_edited_dst.to_string_lossy().to_string(),
        temp_preview_path: temp_prev_dst.to_string_lossy().to_string(),
        proposed_preview_data: b64_preview,
        original_width_mm: width_mm,
        original_height_mm: height_mm,
        original_stitches: stitches as usize,
        original_colors: colors as usize,
        proposed_width_mm: meta.width_mm,
        proposed_height_mm: meta.height_mm,
        proposed_stitches: meta.stitches,
        proposed_colors: meta.colors,
        proposed_threads: meta.threads,
    })
}

#[tauri::command]

pub fn apply_proposed_edit(
    state: State<AppState>,
    design_id: String,
    temp_edited_path: String,
    temp_preview_path: String,
    save_mode: String, // "new_revision" | "new_design"
) -> Result<crate::models::Design, String> {
    let edited_p = PathBuf::from(&temp_edited_path);
    let preview_p = PathBuf::from(&temp_preview_path);

    if !edited_p.exists() {
        return Err("Temporary edited embroidery file expired or not found.".into());
    }

    let meta = state
        .adapter
        .inspect(&edited_p)
        .map_err(|e| format!("Failed to inspect edited file: {e}"))?;

    let checksum_val = crate::db::checksum(&edited_p).unwrap_or_default();
    let size_bytes = edited_p.metadata().map(|m| m.len() as i64).unwrap_or(0);
    let time_now = now();

    let dom_colors = meta
        .threads
        .iter()
        .map(|t| t.hex.clone())
        .collect::<Vec<String>>();
    let dom_colors_json = serde_json::to_string(&dom_colors).unwrap_or_default();
    let threads_json = serde_json::to_string(&meta.threads).unwrap_or_default();

    let db = state.db.lock().map_err(|_| "Database is busy")?;

    if save_mode == "new_revision" {
        // Fetch current design row
        let (title, filename, current_managed, current_fmt, current_checksum, current_size): (
            String,
            String,
            String,
            String,
            String,
            i64,
        ) = db
            .query_row(
                "SELECT title, filename, managed_path, format, checksum, size_bytes FROM designs WHERE id = ?1",
                params![design_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .map_err(|e| format!("Design not found: {e}"))?;

        // Count existing revisions
        let rev_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM design_revisions WHERE design_id = ?1",
                params![design_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let next_rev = rev_count + 1;
        let rev_id = Uuid::new_v4().to_string();

        // 1. Archive current file into revision table
        let _ = db.execute(
            "INSERT INTO design_revisions(id, design_id, revision_number, filename, managed_path, checksum, format, size_bytes, created_at, note)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Auto-archived prior to AI Edit')",
            params![
                rev_id,
                design_id,
                next_rev,
                filename,
                current_managed,
                current_checksum,
                current_fmt,
                current_size,
                time_now
            ],
        );

        // 2. Move edited file to managed library
        let target_managed = state
            .library_root
            .join("library")
            .join("designs")
            .join(format!("{design_id}.{}", current_fmt.to_lowercase()));
        let target_preview = state
            .library_root
            .join("library")
            .join("previews")
            .join(format!("{design_id}.png"));

        let _ = fs::copy(&edited_p, &target_managed);
        let _ = fs::copy(&preview_p, &target_preview);

        // 3. Update designs row
        db.execute(
            "UPDATE designs SET 
                width_mm = ?1, 
                height_mm = ?2, 
                stitches = ?3, 
                colors = ?4, 
                size_bytes = ?5, 
                checksum = ?6, 
                dominant_colors = ?7, 
                threads_json = ?8 
             WHERE id = ?9",
            params![
                meta.width_mm,
                meta.height_mm,
                meta.stitches as i64,
                meta.colors as i64,
                size_bytes,
                checksum_val,
                dom_colors_json,
                threads_json,
                design_id
            ],
        )
        .map_err(|e| format!("Failed to update design with edit: {e}"))?;

        // Return updated design
        Ok(crate::models::Design {
            id: design_id,
            title,
            filename,
            format: current_fmt,
            width_mm: Some(meta.width_mm),
            height_mm: Some(meta.height_mm),
            stitches: Some(meta.stitches as i64),
            colors: Some(meta.colors as i64),
            size_bytes,
            tags: vec![],
            collection: None,
            collection_id: None,
            job: None,
            job_id: None,
            imported_at: time_now,
            duplicate: false,
            preview_url: Some(target_preview.to_string_lossy().to_string()),
            preview_path: Some(target_preview.to_string_lossy().to_string()),
            managed_path: Some(target_managed.to_string_lossy().to_string()),
            status: "active".into(),
            ai_category: None,
            ai_subject: None,
            ai_style: None,
            ai_description: None,
            dominant_colors: dom_colors,
            threads: meta.threads,
        })
    } else {
        // Save as Brand New Design
        let new_id = Uuid::new_v4().to_string();
        let orig_title: String = db
            .query_row(
                "SELECT title FROM designs WHERE id = ?1",
                params![design_id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "Design".into());

        let new_title = format!("{orig_title} (Edited)");
        let ext = meta.format.to_lowercase();
        let new_filename = format!("{new_title}.{ext}");

        let target_managed = state
            .library_root
            .join("library")
            .join("designs")
            .join(format!("{new_id}.{ext}"));
        let target_preview = state
            .library_root
            .join("library")
            .join("previews")
            .join(format!("{new_id}.png"));

        let _ = fs::copy(&edited_p, &target_managed);
        let _ = fs::copy(&preview_p, &target_preview);

        db.execute(
            "INSERT INTO designs(id, title, filename, managed_path, preview_path, checksum, format, width_mm, height_mm, stitches, colors, size_bytes, source_path, status, ai_category, ai_subject, ai_style, ai_description, dominant_colors, threads_json, imported_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'active', 'AI Edited', ?2, 'Structured Edit', 'Edited via Stitchflow Smart Edit', ?14, ?15, ?16)",
            params![
                new_id,
                new_title,
                new_filename,
                target_managed.to_string_lossy().to_string(),
                target_preview.to_string_lossy().to_string(),
                checksum_val,
                meta.format.to_uppercase(),
                meta.width_mm,
                meta.height_mm,
                meta.stitches as i64,
                meta.colors as i64,
                size_bytes,
                edited_p.to_string_lossy().to_string(),
                dom_colors_json,
                threads_json,
                time_now
            ],
        )
        .map_err(|e| format!("Failed to insert new edited design: {e}"))?;

        Ok(crate::models::Design {
            id: new_id,
            title: new_title,
            filename: new_filename,
            format: meta.format.to_uppercase(),
            width_mm: Some(meta.width_mm),
            height_mm: Some(meta.height_mm),
            stitches: Some(meta.stitches as i64),
            colors: Some(meta.colors as i64),
            size_bytes,
            tags: vec!["ai-edited".into()],
            collection: None,
            collection_id: None,
            job: None,
            job_id: None,
            imported_at: time_now,
            duplicate: false,
            preview_url: Some(target_preview.to_string_lossy().to_string()),
            preview_path: Some(target_preview.to_string_lossy().to_string()),
            managed_path: Some(target_managed.to_string_lossy().to_string()),
            status: "active".into(),
            ai_category: Some("AI Edited".into()),
            ai_subject: None,
            ai_style: Some("Structured Edit".into()),
            ai_description: Some("Edited via Stitchflow Smart Edit".into()),
            dominant_colors: dom_colors,
            threads: meta.threads,
        })
    }
}


