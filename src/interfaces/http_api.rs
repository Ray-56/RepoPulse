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
use crate::domain::{Event, WatchTarget};

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
}

async fn list_events(
    State(state): State<ApiState>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100).min(500);

    match state.store.list_events(limit).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response(),
    }
}
