use async_trait::async_trait;

use crate::application::{AppResult, WatchProvider};
use crate::domain::{Event, EventType, WatchTarget};

pub struct FakeWatchProvider;

impl FakeWatchProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WatchProvider for FakeWatchProvider {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<Event>> {
        // v1: 造一个确定性的事件 (subject 来源于 target)
        let event_type = match &target.kind {
            crate::domain::WatchKind::GitHubRelease { .. } => EventType::GitHubRelease,
            crate::domain::WatchKind::GitHubBranch { .. } => EventType::GitHubBranch,
            crate::domain::WatchKind::NpmLatest { .. } => EventType::NpmLatest,
            crate::domain::WatchKind::WhatsappWebVersion {} => EventType::WhatsappWebVersion,
        };

        let subject = target.kind.subject();
        let new_value = "v1.0.0".to_string();
        let event_id = Event::make_event_id(&event_type, &subject, &new_value);

        Ok(Some(Event {
            event_id,
            event_type,
            source: target.kind.source(),
            subject,
            old_value: Some("v0.9.0".to_string()),
            new_value,
            occurred_at: None,
            detected_at: "2026-02-04T00:00:00+08:00".to_string(),
            url: Some("https://example.com".to_string()),
        }))
    }
}
