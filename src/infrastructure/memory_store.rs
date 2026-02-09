use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::application::{AppError, AppResult, EventStore, TargetRepository};
use crate::domain::{Event, WatchTarget};

#[derive(Clone, Default)]
pub struct InMemoryEventStore {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    seen: HashSet<String>,
    events: Vec<Event>,
    // 预留：cooldown、notify_log 等
    meta: HashMap<String, String>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn has_seen(&self, event_id: &str) -> AppResult<bool> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        Ok(inner.seen.contains(event_id))
    }

    async fn mark_seen(&self, event_id: &str) -> AppResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        inner.seen.insert(event_id.to_string());
        Ok(())
    }

    async fn append_event(&self, event: &Event) -> AppResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        inner.events.push(event.clone());
        Ok(())
    }

    async fn get_last_notified(&self, scope_key: &str) -> AppResult<Option<i64>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        Ok(inner.meta.get(scope_key).and_then(|v| v.parse().ok()))
    }

    async fn set_last_notified(&self, scope_key: &str, epoch_seconds: i64) -> AppResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        inner
            .meta
            .insert(scope_key.to_string(), epoch_seconds.to_string());
        Ok(())
    }

    async fn list_events(&self, limit: u32) -> AppResult<Vec<Event>> {
        let inner = self.inner.lock().map_err(|_| AppError::Storage("lock poisoned".into()))?;
        let mut v = inner.events.clone();
        v.reverse(); // newest first (since we push at end)
        v.truncate(limit as usize);
        Ok(v)
    }
}

#[derive(Clone)]
pub struct InMemoryTargetRepository {
    targets: Arc<Vec<WatchTarget>>,
}

impl InMemoryTargetRepository {
    pub fn new(targets: Vec<WatchTarget>) -> Self {
        Self {
            targets: Arc::new(targets),
        }
    }
}

#[async_trait]
impl TargetRepository for InMemoryTargetRepository {
    async fn list_enabled_targets(&self) -> AppResult<Vec<WatchTarget>> {
        Ok(self.targets.iter().cloned().filter(|t| t.enabled).collect())
    }
}
