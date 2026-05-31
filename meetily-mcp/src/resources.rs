// MCP Resources — read-only data endpoints

use serde_json::{json, Value};
use std::path::PathBuf;

pub fn list_resources() -> Value {
    json!({
        "resources": [
            {
                "uri": "meetily://meetings/latest",
                "name": "Latest Meeting",
                "description": "The most recently completed meeting transcript",
                "mimeType": "text/markdown"
            },
            {
                "uri": "meetily://dictionary",
                "name": "Shared Dictionary",
                "description": "Unified dictionary shared across Meetily, VoiceInk, and Raycast",
                "mimeType": "application/json"
            }
        ]
    })
}

pub fn read_resource(uri: &str) -> anyhow::Result<Value> {
    match uri {
        "meetily://meetings/latest" => {
            let notify_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local")
                .join("share")
                .join("meetily")
                .join("last-export.json");

            if !notify_path.exists() {
                return Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/plain",
                        "text": "No meetings exported yet."
                    }]
                }));
            }

            let meta: Value = serde_json::from_str(&std::fs::read_to_string(&notify_path)?)?;
            let content = if let Some(path) = meta.get("transcript_path").and_then(|v| v.as_str()) {
                std::fs::read_to_string(path).unwrap_or_else(|_| "Transcript file not found".to_string())
            } else {
                "No transcript path in export metadata".to_string()
            };

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/markdown",
                    "text": content
                }]
            }))
        }
        "meetily://dictionary" => {
            let path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".config")
                .join("unified-dictionary")
                .join("dictionary.json");

            let content = if path.exists() {
                std::fs::read_to_string(&path)?
            } else {
                "[]".to_string()
            };

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }))
        }
        _ => Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": format!("Unknown resource: {}", uri)
            }]
        })),
    }
}
