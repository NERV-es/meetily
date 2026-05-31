use super::{DictionaryEntry, DictionaryState};
use chrono::Utc;
use tauri::{AppHandle, Manager, Runtime};
use uuid::Uuid;

/// Tauri command: get all dictionary entries
#[tauri::command]
pub async fn get_dictionary(
    app: AppHandle<impl Runtime>,
) -> Result<Vec<DictionaryEntry>, String> {
    let state = app.state::<DictionaryState>();
    let entries = state.read().await;
    Ok(entries.clone())
}

/// Tauri command: add a dictionary entry
#[tauri::command]
pub async fn add_dictionary_entry(
    app: AppHandle<impl Runtime>,
    display: String,
    aliases: Vec<String>,
) -> Result<DictionaryEntry, String> {
    let state = app.state::<DictionaryState>();
    let entry = DictionaryEntry {
        id: Uuid::new_v4().to_string(),
        display,
        aliases,
        source: "meetily".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    };

    {
        let mut entries = state.write().await;
        entries.push(entry.clone());
    }

    super::save_dictionary(&state).await?;
    Ok(entry)
}

/// Tauri command: remove a dictionary entry by ID
#[tauri::command]
pub async fn remove_dictionary_entry(
    app: AppHandle<impl Runtime>,
    id: String,
) -> Result<(), String> {
    let state = app.state::<DictionaryState>();
    {
        let mut entries = state.write().await;
        entries.retain(|e| e.id != id);
    }
    super::save_dictionary(&state).await?;
    Ok(())
}

/// Tauri command: update a dictionary entry
#[tauri::command]
pub async fn update_dictionary_entry(
    app: AppHandle<impl Runtime>,
    id: String,
    display: Option<String>,
    aliases: Option<Vec<String>>,
) -> Result<DictionaryEntry, String> {
    let state = app.state::<DictionaryState>();
    let updated;
    {
        let mut entries = state.write().await;
        let entry = entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("Entry not found")?;

        if let Some(d) = display {
            entry.display = d;
        }
        if let Some(a) = aliases {
            entry.aliases = a;
        }
        entry.updated_at = Utc::now().to_rfc3339();
        updated = entry.clone();
    }
    super::save_dictionary(&state).await?;
    Ok(updated)
}

/// Tauri command: import dictionary from VoiceInk format
#[tauri::command]
pub async fn import_voiceink_dictionary(
    app: AppHandle<impl Runtime>,
    path: String,
) -> Result<usize, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read VoiceInk dictionary: {}", e))?;

    // VoiceInk stores dictionary as JSON array with {word, replacement} pairs
    #[derive(serde::Deserialize)]
    struct VoiceInkEntry {
        word: Option<String>,
        replacement: Option<String>,
        #[serde(rename = "originalWord")]
        original_word: Option<String>,
    }

    let voiceink_entries: Vec<VoiceInkEntry> =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let state = app.state::<DictionaryState>();
    let mut count = 0;
    {
        let mut entries = state.write().await;
        for ve in voiceink_entries {
            let display = ve.replacement.or(ve.word.clone()).unwrap_or_default();
            let alias = ve.original_word.or(ve.word).unwrap_or_default();
            if !display.is_empty() && !alias.is_empty() {
                entries.push(DictionaryEntry {
                    id: Uuid::new_v4().to_string(),
                    display,
                    aliases: vec![alias],
                    source: "voiceink-import".to_string(),
                    updated_at: Utc::now().to_rfc3339(),
                });
                count += 1;
            }
        }
    }
    super::save_dictionary(&state).await?;
    Ok(count)
}
