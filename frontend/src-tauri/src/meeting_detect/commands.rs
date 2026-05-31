use super::{DetectionConfig, DetectionState};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;

/// Tauri command: enable/disable meeting auto-detection
#[tauri::command]
pub async fn set_meeting_detection(
    app: AppHandle<impl Runtime>,
    enabled: bool,
) -> Result<(), String> {
    let config = app.state::<Arc<RwLock<DetectionConfig>>>();
    let mut cfg = config.write().await;
    cfg.enabled = enabled;
    Ok(())
}

/// Tauri command: get detection state
#[tauri::command]
pub async fn get_detection_state(
    app: AppHandle<impl Runtime>,
) -> Result<DetectionState, String> {
    let config = app.state::<Arc<RwLock<DetectionConfig>>>();
    let cfg = config.read().await;
    Ok(DetectionState {
        is_monitoring: cfg.enabled,
        meeting_detected: false, // TODO: pull from detection loop state
        detection_reason: None,
        auto_recording_active: false,
    })
}

/// Tauri command: update meeting app bundle IDs to monitor
#[tauri::command]
pub async fn set_meeting_apps(
    app: AppHandle<impl Runtime>,
    bundle_ids: Vec<String>,
) -> Result<(), String> {
    let config = app.state::<Arc<RwLock<DetectionConfig>>>();
    let mut cfg = config.write().await;
    cfg.meeting_app_bundles = bundle_ids;
    Ok(())
}

/// Tauri command: set silence timeout for auto-stop
#[tauri::command]
pub async fn set_silence_timeout(
    app: AppHandle<impl Runtime>,
    seconds: u64,
) -> Result<(), String> {
    let config = app.state::<Arc<RwLock<DetectionConfig>>>();
    let mut cfg = config.write().await;
    cfg.silence_timeout_seconds = seconds;
    Ok(())
}
