use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;

use crate::application::{EventStore, TargetRepository};

#[derive(Clone)]
pub struct ApiState {
    pub store: Arc<dyn EventStore>,
    pub targets: Arc<dyn TargetRepository>,
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/targets", get(list_targets))
        .route("/events", get(list_events))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn list_targets(State(state): State<ApiState>) -> impl IntoResponse {
    match state.targets.list_enabled_targets().await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response(),
    }
}

#[derive(Deserialize)]
struct EventsQuery {
    limit: Option<u32>,
    since: Option<String>, // "24h" | "7d" | "3600s"
    label: Option<String>,
    r#type: Option<String>,
    subject: Option<String>,
}

async fn list_events(
    State(state): State<ApiState>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100).min(500);

    let since_epoch = match q.since.as_deref() {
        Some(v) => match parse_since_to_epoch(v) {
            Some(e) => Some(e),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "invalid since (use 24h/7d/3600s or 3600s)".to_string(),
                )
                    .into_response();
            }
        },
        None => None,
    };

    let query = crate::application::EventQuery {
        limit,
        since_epoch,
        label: q.label.clone(),
        event_type: q.r#type.as_deref().and_then(parse_type),
        subject: q.subject.clone(),
    };

    match state.store.list_events_filtered(query).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response(),
    }

    // match state.store.list_events(limit).await {
    //     Ok(v) => Json(v).into_response(),
    //     Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response(),
    // }
}

fn now_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn parse_since_to_epoch(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 2 {
        return None;
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_part.parse().ok()?;
    let seconds = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 60 * 60,
        "d" => n * 60 * 60 * 24,
        _ => return None,
    };
    Some(now_epoch().saturating_sub(seconds))
}

fn parse_type(t: &str) -> Option<crate::domain::EventType> {
    match t {
        "release" => Some(crate::domain::EventType::GitHubRelease),
        "branch" => Some(crate::domain::EventType::GitHubBranch),
        "npm" => Some(crate::domain::EventType::NpmLatest),
        "whatsapp-web" => Some(crate::domain::EventType::WhatsAppWebVersion),
        _ => None,
    }
}
