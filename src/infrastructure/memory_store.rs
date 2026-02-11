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

    async fn append_event_record(&self, record: &crate::application::EventRecord) -> AppResult<()> {
        // v1: in-memory 只保存 event 本地即可
        self.append_event(&record.event).await
    }

    async fn list_events_filtered(
        &self,
        query: crate::application::EventQuery,
    ) -> AppResult<Vec<Event>> {
        // v1: 为了先过编译与跑通 API, in-memory 退化为 limit 查询
        let limit = query.limit.min(500);
        self.list_events(limit).await
    }

    async fn list_event_records_filtered(
        &self,
        query: crate::application::EventQuery,
    ) -> AppResult<Vec<crate::application::EventRecord>> {
        let events = self.list_events(query.limit).await?;
        let out = events
            .into_iter()
            .map(|e| crate::application::EventRecord {
                detected_at_epoch: 0,
                target_id: "".to_string(),
                labels: vec![],
                event: e,
            })
            .collect();
        Ok(out)
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
        let inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        let mut v = inner.events.clone();
        v.reverse(); // newest first (since we push at end)
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn upsert_event_record_return_rowid(
        &self,
        record: &crate::application::EventRecord,
    ) -> AppResult<i64> {
        // in-memory 没 rowid：用一个伪 rowid（events 长度）即可保证单调
        self.append_event(&record.event).await?;
        let inner = self
            .inner
            .lock()
            .map_err(|_| AppError::Storage("lock poisoned".into()))?;
        Ok(inner.events.len() as i64)
    }

    async fn list_event_records_cursor(
        &self,
        query: crate::application::EventRecordQuery,
    ) -> AppResult<Vec<(i64, crate::application::EventRecord)>> {
        let events = self.list_events(query.limit).await?;
        let mut out = vec![];
        for (i, e) in events.into_iter().enumerate() {
            out.push((
                i as i64,
                crate::application::EventRecord {
                    event: e,
                    target_id: "".into(),
                    labels: vec![],
                    detected_at_epoch: 0,
                },
            ));
        }
        Ok(out)
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
