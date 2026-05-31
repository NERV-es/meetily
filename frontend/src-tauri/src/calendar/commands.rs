use super::{CalendarConfig, CalendarEvent, CalendarState};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;

/// Tauri command: set ICS calendar URL
#[tauri::command]
pub async fn set_calendar_url(
    app: AppHandle<impl Runtime>,
    url: String,
) -> Result<(), String> {
    let config = app.state::<Arc<RwLock<CalendarConfig>>>();
    let mut cfg = config.write().await;
    cfg.ics_url = Some(url);
    Ok(())
}

/// Tauri command: get upcoming calendar events
#[tauri::command]
pub async fn get_calendar_events(
    app: AppHandle<impl Runtime>,
) -> Result<Vec<CalendarEvent>, String> {
    let state = app.state::<CalendarState>();
    let events = state.read().await;
    Ok(events.clone())
}

/// Tauri command: get events happening now
#[tauri::command]
pub async fn get_active_calendar_events(
    app: AppHandle<impl Runtime>,
) -> Result<Vec<CalendarEvent>, String> {
    let state = app.state::<CalendarState>();
    let config = app.state::<Arc<RwLock<CalendarConfig>>>();
    let cfg = config.read().await;
    let active = super::scheduler::get_active_events(&*state, &*cfg).await;
    Ok(active)
}

/// Tauri command: toggle auto-record from calendar
#[tauri::command]
pub async fn set_auto_record(
    app: AppHandle<impl Runtime>,
    enabled: bool,
) -> Result<(), String> {
    let config = app.state::<Arc<RwLock<CalendarConfig>>>();
    let mut cfg = config.write().await;
    cfg.auto_record_enabled = enabled;
    Ok(())
}

/// Tauri command: manually refresh calendar
#[tauri::command]
pub async fn refresh_calendar(
    app: AppHandle<impl Runtime>,
) -> Result<usize, String> {
    let config = app.state::<Arc<RwLock<CalendarConfig>>>();
    let cfg = config.read().await;
    let url = cfg.ics_url.as_ref().ok_or("No calendar URL configured")?;
    let events = super::ics::fetch_and_parse_ics(url).await?;
    let count = events.len();
    let state = app.state::<CalendarState>();
    let mut state_guard = state.write().await;
    *state_guard = events;
    Ok(count)
}
