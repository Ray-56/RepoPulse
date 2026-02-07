use async_trait::async_trait;
use serde::Deserialize;

use crate::application::{AppError, AppResult, WatchProvider};
use crate::domain::{Event, EventType, Source, WatchKind, WatchTarget};

pub struct NpmLatestProvider {
    client: reqwest::Client,
}

impl NpmLatestProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NpmResp {
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
}

#[derive(Debug, Deserialize)]
struct DistTags {
    latest: String,
}

#[async_trait]
impl WatchProvider for NpmLatestProvider {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<Event>> {
        let pkg = match &target.kind {
            WatchKind::NpmLatest { package } => package,
            _ => return Ok(None),
        };

        let url = format!("https://registry.npmjs.org/{}", pkg);

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?
            .error_for_status()
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let body: NpmResp = resp
            .json()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let latest = body.dist_tags.latest;
        let subject = pkg.to_string();
        let event_id = Event::make_event_id(&EventType::NpmLatest, &subject, &latest);

        Ok(Some(Event {
            event_id,
            event_type: EventType::NpmLatest,
            source: Source::Npm,
            subject,
            old_value: None,
            new_value: latest,
            occurred_at: None,
            detected_at: now_string(),
            url: Some(format!("https://www.npmjs.com/package/{}", pkg)),
        }))
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
