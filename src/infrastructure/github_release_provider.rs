use async_trait::async_trait;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

use crate::application::{AppError, AppResult, WatchProvider};
use crate::domain::{Event, EventType, Source, WatchKind, WatchTarget};

pub struct GitHubReleaseProvider {
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHubReleaseProvider {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReleaseResp {
    tag_name: Option<String>,
    html_url: Option<String>,
    published_at: Option<String>,
}

#[async_trait]
impl WatchProvider for GitHubReleaseProvider {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<Event>> {
        let repo = match &target.kind {
            WatchKind::GitHubRelease { repo } => repo,
            _ => return Ok(None),
        };

        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            repo.as_str(),
        );

        let mut req = self
            .client
            .get(url)
            .header(USER_AGENT, "repopulse")
            .header(ACCEPT, "application/vnd.github.v3+json");

        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?;

        // 没有 release (404) 不是错误
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let resp = resp
            .error_for_status()
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let body: ReleaseResp = resp
            .json()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let tag = match body.tag_name {
            Some(t) => t,
            None => return Ok(None),
        };

        let subject = repo.as_str();
        let event_id = Event::make_event_id(&EventType::GitHubRelease, &subject, &tag);

        Ok(Some(Event {
            event_id,
            event_type: EventType::GitHubRelease,
            source: Source::GitHub,
            subject,
            old_value: None, // v1: 不在 provider 里算 old
            new_value: tag,
            occurred_at: body.published_at,
            detected_at: chrono_now_rfc3339(),
            url: body.html_url,
        }))
    }
}

fn chrono_now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}s_since_epoch", secs)
}
