use crate::domain::{Event, WatchTarget};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub event: crate::domain::Event,
    pub target_id: String,
    pub labels: Vec<String>,
    pub detected_at_epoch: i64,
}

#[derive(Clone, Debug, Default)]
pub struct EventQuery {
    pub since_epoch: Option<i64>,
    pub limit: u32,
    pub label: Option<String>,
    pub event_type: Option<crate::domain::EventType>,
    pub subject: Option<String>,
}

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

    // 旧接口: 保留(可以内部转调 append_event_record)
    async fn append_event(&self, event: &Event) -> AppResult<()>;

    // 新接口: 带 target_id / labels / epoch
    async fn append_event_record(&self, record: &EventRecord) -> AppResult<()>;

    // 旧接口: 保留(给 /events?limit 简单用)
    async fn list_events(&self, limit: u32) -> AppResult<Vec<Event>>;

    // 新接口: 用于过滤查询
    async fn list_events_filtered(&self, query: EventQuery) -> AppResult<Vec<Event>>;

    async fn list_event_records_filtered(&self, query: EventQuery) -> AppResult<Vec<EventRecord>>;

    async fn get_last_notified(&self, scope_key: &str) -> AppResult<Option<i64>>;
    async fn set_last_notified(&self, scope_key: &str, epoch_seconds: i64) -> AppResult<()>;
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

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, record: &EventRecord) -> AppResult<()>;
}
