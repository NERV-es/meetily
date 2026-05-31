use super::export_meeting_markdown;
use tauri::{AppHandle, Runtime};

/// Tauri command: export a meeting transcript to markdown
#[tauri::command]
pub async fn export_meeting<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<super::ExportedMeeting, String> {
    export_meeting_markdown(&app, &meeting_id).await
}

/// Tauri command: get export directory path
#[tauri::command]
pub async fn get_export_dir() -> Result<String, String> {
    let config = super::ExportConfig::default();
    Ok(config.export_dir.to_string_lossy().to_string())
}
