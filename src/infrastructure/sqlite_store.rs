use async_trait::async_trait;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use crate::application::{AppError, AppResult, EventStore};
use crate::domain::Event;

pub struct SqliteEventStore {
    pool: SqlitePool,
}

impl SqliteEventStore {
    /// db_url 示例
    /// - "sqlite:/data/state.db" (推荐用于 docker volume)
    /// - "sqlite:./state.db"
    pub async fn new(db_url: &str) -> AppResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> AppResult<()> {
        // seen: 幂等去重表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS seen (
              event_id TEXT PRIMARY KEY,
              seen_at TEXT NOT NULL
            );
          "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        // events: 事件明细表(为后续 API/Digest 打基础)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
              event_id TEXT PRIMARY KEY,
              event_type TEXT NOT NULL,
              source TEXT NOT NULL,
              subject TEXT NOT NULL,
              old_value TEXT,
              new_value TEXT NOT NULL,
              occurred_at TEXT,
              detected_at TEXT NOT NULL,
              url TEXT
            );
          "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notify_log (
                scope_key TEXT PRIMARY KEY,
                last_sent_at INTEGER NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn has_seen(&self, event_id: &str) -> AppResult<bool> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT event_id FROM seen WHERE event_id = ? LIMIT 1")
                .bind(event_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(row.is_some())
    }

    async fn mark_seen(&self, event_id: &str) -> AppResult<()> {
        let now = now_string();

        sqlx::query("INSERT OR IGNORE INTO seen(event_id, seen_at) VALUES(?, ?)")
            .bind(event_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn append_event(&self, event: &Event) -> AppResult<()> {
        // 事件表以 event_id 为主键，重复插入会忽略（额外兜底）
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO events(
               event_id, event_type, source, subject,
               old_value, new_value, occurred_at, detected_at, url
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_id)
        .bind(format!("{:?}", event.event_type))
        .bind(event.source.to_string())
        .bind(&event.subject)
        .bind(&event.old_value.as_deref())
        .bind(&event.new_value)
        .bind(&event.occurred_at.as_deref())
        .bind(&event.detected_at)
        .bind(event.url.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_last_notified(&self, scope_key: &str) -> AppResult<Option<i64>> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT last_sent_at FROM notify_log WHERE scope_key = ? LIMIT 1")
                .bind(scope_key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(row.map(|t| t.0))
    }

    async fn set_last_notified(&self, scope_key: &str, epoch_seconds: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO notify_log(scope_key, last_sent_at) VALUES(?, ?)
            ON CONFLICT(scope_key) DO UPDATE SET last_sent_at=excluded.last_sent_at
            "#,
        )
        .bind(scope_key)
        .bind(epoch_seconds)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        Ok(())
    }
}

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}s_since_epoch", secs)
}
