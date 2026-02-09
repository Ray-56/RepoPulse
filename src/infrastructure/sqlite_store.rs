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
              url TEXT,
              target_id TEXT,
              labels TEXT,
              detected_at_epoch INTEGER NOT NULL DEFAULT 0
            );
          "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        // add target_id
        let _ = sqlx::query("ALTER TABLE events ADD COLUMN target_id TEXT")
            .execute(&self.pool)
            .await;
        // add labels
        let _ = sqlx::query("ALTER TABLE events ADD COLUMN labels TEXT")
            .execute(&self.pool)
            .await;
        // add detected_at_epoch
        let _ = sqlx::query(
            "ALTER TABLE events ADD COLUMN detected_at_epoch INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&self.pool)
        .await;

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
        let record = crate::application::EventRecord {
            event: event.clone(),
            target_id: "".to_string(),
            labels: vec![],
            detected_at_epoch: 0,
        };
        self.append_event_record(&record).await
        // // 事件表以 event_id 为主键，重复插入会忽略（额外兜底）
        // sqlx::query(
        //     r#"
        //     INSERT OR IGNORE INTO events(
        //        event_id, event_type, source, subject,
        //        old_value, new_value, occurred_at, detected_at, url
        //     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        //     "#,
        // )
        // .bind(&event.event_id)
        // .bind(format!("{:?}", event.event_type))
        // .bind(event.source.to_string())
        // .bind(&event.subject)
        // .bind(&event.old_value.as_deref())
        // .bind(&event.new_value)
        // .bind(&event.occurred_at.as_deref())
        // .bind(&event.detected_at)
        // .bind(event.url.as_deref())
        // .execute(&self.pool)
        // .await
        // .map_err(|e| AppError::Storage(e.to_string()))?;

        // Ok(())
    }

    async fn append_event_record(&self, record: &crate::application::EventRecord) -> AppResult<()> {
        let e = &record.event;
        let labels_joined = if record.labels.is_empty() {
            None
        } else {
            Some(record.labels.join(","))
        };

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO events(
                event_id, event_type, source, subject,
                old_value, new_value, occurred_at, detected_at, url,
                target_id, labels, detected_at_epoch
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&e.event_id)
        .bind(format!("{:?}", e.event_type))
        .bind(e.source.to_string())
        .bind(&e.subject)
        .bind(e.old_value.as_deref())
        .bind(&e.detected_at)
        .bind(e.url.as_deref())
        .bind(&record.target_id)
        .bind(labels_joined.as_deref())
        .bind(record.detected_at_epoch)
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

    async fn list_events(&self, limit: u32) -> AppResult<Vec<Event>> {
        // 用 rowid 倒序拉最新（不依赖 detected_at 的格式）
        let limit_i64 = i64::from(limit);
        let rows = sqlx::query!(
            r#"
            SELECT
              event_id,
              event_type,
              source,
              subject,
              old_value,
              new_value,
              occurred_at,
              detected_at,
              url
            FROM events
            ORDER BY rowid DESC
            LIMIT ?
            "#,
            limit_i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let event_type = match r.event_type.as_str() {
                "GitHubRelease" => crate::domain::EventType::GitHubRelease,
                "GitHubBranch" => crate::domain::EventType::GitHubBranch,
                "NpmLatest" => crate::domain::EventType::NpmLatest,
                "WhatsAppWebVersion" => crate::domain::EventType::WhatsAppWebVersion,
                _ => crate::domain::EventType::GitHubRelease, // fallback（也可改成 Err）
            };

            let source = match r.source.as_str() {
                "github" => crate::domain::Source::GitHub,
                "npm" => crate::domain::Source::Npm,
                "whatsapp-web" => crate::domain::Source::WhatsAppWeb,
                _ => crate::domain::Source::GitHub,
            };

            out.push(crate::domain::Event {
                event_id: r.event_id.unwrap_or_default(),
                event_type,
                source,
                subject: r.subject,
                old_value: r.old_value,
                new_value: r.new_value,
                occurred_at: r.occurred_at,
                detected_at: r.detected_at,
                url: r.url,
            });
        }

        Ok(out)
    }

    async fn list_events_filtered(
        &self,
        query: crate::application::EventQuery,
    ) -> AppResult<Vec<Event>> {
        // let mut sql = String::from(
        //     r#"
        //     SELECT
        //         event_id, event_type, source, subject, old_value, new_value,
        //         occurred_at, detected_at, url
        //     FROM events
        //     WHERE 1=1
        //     "#,
        // );

        use sqlx::QueryBuilder;
        let mut qb = QueryBuilder::new(
            r#"
            SELECT
                event_id, event_type, source, subject, old_value, new_value,
                occurred_at, detected_at, url
            FROM events
            WHERE 1=1
            "#,
        );

        if let Some(since) = query.since_epoch {
            qb.push(" AND detected_at_epoch >=");
            qb.push_bind(since);
        }

        if let Some(label) = query.label {
            qb.push(" AND labels LIKE ");
            qb.push_bind(format!("%{}%", label));
        }

        if let Some(event_type) = query.event_type {
            qb.push(" AND event_type = ");
            qb.push_bind(format!("{:?}", event_type));
        }

        if let Some(subject) = query.subject {
            qb.push(" AND subject = ");
            qb.push_bind(subject);
        }

        qb.push(" ORDER BY detected_at_epoch DESC, rowid DESC LIMIT ");
        qb.push_bind(query.limit.min(500) as i64);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());

        for row in rows {
            // 区列（用 Row API)
            use sqlx::Row;
            let event_id = row
                .try_get("event_id")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let event_type_s: String = row
                .try_get("event_type")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let source_s: String = row
                .try_get("source")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let subject = row
                .try_get("subject")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let old_value = row
                .try_get("old_value")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let new_value = row
                .try_get("new_value")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let occurred_at = row
                .try_get("occurred_at")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let detected_at = row
                .try_get("detected_at")
                .map_err(|e| AppError::Storage(e.to_string()))?;
            let url = row
                .try_get("url")
                .map_err(|e| AppError::Storage(e.to_string()))?;

            let event_type = match event_type_s.as_str() {
                "GitHubRelease" => crate::domain::EventType::GitHubRelease,
                "GitHubBranch" => crate::domain::EventType::GitHubBranch,
                "NpmLatest" => crate::domain::EventType::NpmLatest,
                "WhatsAppWebVersion" => crate::domain::EventType::WhatsAppWebVersion,
                _ => crate::domain::EventType::GitHubRelease,
            };

            let source = match source_s.as_str() {
                "github" => crate::domain::Source::GitHub,
                "npm" => crate::domain::Source::Npm,
                "whatsapp-web" => crate::domain::Source::WhatsAppWeb,
                _ => crate::domain::Source::GitHub,
            };

            out.push(crate::domain::Event {
                event_id,
                event_type,
                source,
                subject,
                old_value,
                new_value,
                occurred_at,
                detected_at,
                url,
            });
        }

        Ok(out)
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
