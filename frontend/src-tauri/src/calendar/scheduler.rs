// Calendar scheduler — background task that polls ICS and maintains event cache

use super::{ics::fetch_and_parse_ics, CalendarConfig, CalendarState};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Start the calendar polling background task
pub fn start_calendar_poller(
    config: Arc<RwLock<CalendarConfig>>,
    state: CalendarState,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let cfg = config.read().await.clone();

            if let Some(ref url) = cfg.ics_url {
                match fetch_and_parse_ics(url).await {
                    Ok(events) => {
                        info!("📅 Calendar sync: {} events loaded", events.len());
                        let mut state_guard = state.write().await;
                        *state_guard = events;
                    }
                    Err(e) => {
                        error!("📅 Calendar sync failed: {}", e);
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(
                cfg.poll_interval_minutes * 60,
            ))
            .await;
        }
    })
}

/// Get events that are active right now
pub async fn get_active_events(state: &CalendarState, config: &CalendarConfig) -> Vec<super::CalendarEvent> {
    let events = state.read().await;
    events
        .iter()
        .filter(|e| {
            e.is_active_now(
                config.pre_meeting_buffer_seconds,
                config.post_meeting_buffer_seconds,
            )
        })
        .cloned()
        .collect()
}
