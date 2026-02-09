use crate::domain::{Event, WatchTarget};
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("notifier error: {0}")]
    Notifier(String),
    #[error("invalid config: {0}")]
    Config(String),
}

pub type AppResult<T> = Result<T, AppError>;

/// Produce an Event if a change is detected for a target.
#[async_trait]
pub trait WatchProvider: Send + Sync {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<Event>>;
}

/// Persist events + idempotency + query.
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn has_seen(&self, event_id: &str) -> AppResult<bool>;
    async fn mark_seen(&self, event_id: &str) -> AppResult<()>;
    async fn append_event(&self, event: &Event) -> AppResult<()>;
    async fn get_last_notified(&self, scope_key: &str) -> AppResult<Option<i64>>;
    async fn set_last_notified(&self, scope_key: &str, epoch_seconds: i64) -> AppResult<()>;
    async fn list_events(&self, limit: u32) -> AppResult<Vec<Event>>;
}

/// Provide list of targets (from config/DB)
#[async_trait]
pub trait TargetRepository: Send + Sync {
    async fn list_enabled_targets(&self) -> AppResult<Vec<WatchTarget>>;
}

/// Deliver notifications.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, event: &Event) -> AppResult<()>;
}
