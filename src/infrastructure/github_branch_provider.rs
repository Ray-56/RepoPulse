use async_trait::async_trait;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;

use crate::application::{AppError, AppResult, WatchProvider};
use crate::domain::{Event, EventType, Source, WatchKind, WatchTarget};

pub struct GitHubBranchProvider {
    client: reqwest::Client,
    token: Option<String>,
}

impl GitHubBranchProvider {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
        }
    }
}

#[derive(Debug, Deserialize)]
struct BranchResp {
    commit: CommitObj,
    _links: LinksObj,
}

#[derive(Debug, Deserialize)]
struct CommitObj {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct LinksObj {
    html: Option<String>,
}

#[async_trait]
impl WatchProvider for GitHubBranchProvider {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<Event>> {
        let (repo, branch) = match &target.kind {
            WatchKind::GitHubBranch { repo, branch } => (repo, branch),
            _ => return Ok(None),
        };

        let url = format!(
            "https://api.github.com/repos/{}/branches/{}",
            repo.as_str(),
            branch,
        );

        let mut req = self
            .client
            .get(url)
            .header(USER_AGENT, "repopulse")
            .header(ACCEPT, "application/vnd.github+json");

        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?
            .error_for_status()
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let body: BranchResp = resp
            .json()
            .await
            .map_err(|e| AppError::Provider(e.to_string()))?;

        let sha = body.commit.sha;
        let subject = format!("{}#{}", repo.as_str(), branch);

        let event_id = Event::make_event_id(&EventType::GitHubBranch, &subject, &sha);

        Ok(Some(Event {
            event_id,
            event_type: EventType::GitHubBranch,
            source: Source::GitHub,
            subject,
            old_value: None,
            new_value: sha,
            occurred_at: None,
            detected_at: now_string(),
            url: body._links.html,
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
