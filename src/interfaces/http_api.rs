use std::sync::Arc;
use std::{convert::Infallible, time::Duration};

use async_stream::stream as async_stream;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse,
        sse::{Event as SseEvent, Sse},
    },
    routing::get,
};
use serde::Deserialize;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    application::{EventStore, TargetRepository},
    infrastructure::event_bus::EventBus,
};

#[derive(Clone)]
pub struct ApiState {
    pub store: Arc<dyn EventStore>,
    pub targets: Arc<dyn TargetRepository>,
    pub api_token: Option<String>,
    pub event_bus: Option<EventBus>,
}

pub fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/targets", get(list_targets))
        .route("/events", get(list_events))
        .route("/events/stream", get(stream_events))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn list_targets(State(state): State<ApiState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err((code, msg)) = check_auth(&headers, &state.api_token) {
        return (code, msg).into_response();
    }
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
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err((code, msg)) = check_auth(&headers, &state.api_token) {
        return (code, msg).into_response();
    }
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

    let event_type = match q.r#type.as_deref() {
        Some(t) => match parse_type(t) {
            Some(et) => Some(et),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "invalid type (release/branch/npm/waweb)".to_string(),
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
        event_type,
        subject: q.subject.clone(),
    };

    match state.store.list_events_filtered(query).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response(),
    }
}

#[derive(Deserialize, Clone)]
struct StreamQuery {
    replay: Option<u32>,   // e.g. 20
    since: Option<String>, // e.g. "24h" | "7d" | "3600s"
    label: Option<String>,
    r#type: Option<String>, // e.g. "release" | "branch" | "npm" | "waweb"
    subject: Option<String>,
}

async fn stream_events(
    State(state): State<ApiState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<StreamQuery>,
) -> impl IntoResponse {
    if let Err((code, msg)) = check_auth(&headers, &state.api_token) {
        return (code, msg).into_response();
    }

    let after_rowid = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok());

    let Some(bus) = state.event_bus.clone() else {
        return (
            StatusCode::NOT_IMPLEMENTED,
            "event stream not enabled".to_string(),
        )
            .into_response();
    };

    let replay = q.replay.unwrap_or(20).min(200);
    let mut since_epoch = match q.since.as_deref() {
        Some(v) => match parse_since_to_epoch(v) {
            Some(e) => Some(e),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "invalid since (use 24h/7d/3600s)".to_string(),
                )
                    .into_response();
            }
        },
        None => None,
    };

    let event_type = match q.r#type.as_deref() {
        Some(t) => match parse_type(t) {
            Some(et) => Some(et),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "invalid type (release/branch/npm/waweb)".to_string(),
                )
                    .into_response();
            }
        },
        None => None,
    };
    let last_event_id = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_epoch_from_header: Option<i64> = last_event_id
        .as_deref()
        .and_then(|id| id.split(":").next())
        .and_then(|s| s.parse::<i64>().ok());

    // If client provided  Last-Event-ID, prefer it (more precise resume point)
    if let Some(last_epoch) = last_epoch_from_header {
        // +1 avoids replaying the same second again (simple dedup strategy)
        since_epoch = Some(last_epoch.saturating_add(1));
    }

    // 1) 先查历史
    let history_query = crate::application::EventRecordQuery {
        since_epoch,
        after_rowid,
        limit: replay,
        label: q.label.clone(),
        event_type,
        subject: q.subject.clone(),
    };

    let history = match state.store.list_event_records_cursor(history_query).await {
        Ok(items) => items,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("error: {e}")).into_response();
        }
    };

    // 2) 再订阅实时
    let rx = bus.subscribe();
    let label_filter = q.label.clone();
    let subject_filter = q.subject.clone();
    let type_filter = q.r#type.clone();

    let live = BroadcastStream::new(rx).filter_map(move |msg| {
        let (rowid, record) = match msg {
            Ok(r) => r,
            Err(_) => return None, // lagged/closed
        };

        // label filter
        if let Some(label) = &label_filter {
            if !record.labels.iter().any(|l| l == label) {
                return None;
            }
        }
        // subject filter
        if let Some(subj) = &subject_filter {
            if record.event.subject != *subj {
                return None;
            }
        }
        // type filter
        if let Some(t) = &type_filter {
            let ok = match (t.as_str(), &record.event.event_type) {
                ("release", crate::domain::EventType::GitHubRelease) => true,
                ("branch", crate::domain::EventType::GitHubBranch) => true,
                ("npm", crate::domain::EventType::NpmLatest) => true,
                ("waweb", crate::domain::EventType::WhatsAppWebVersion) => true,
                _ => false,
            };
            if !ok {
                return None;
            }
        }

        let data = serde_json::to_string(&record).ok()?;
        Some(Ok::<SseEvent, Infallible>(
            SseEvent::default()
                .event("event")
                .id(rowid.to_string())
                .data(data),
        ))
    });

    // 3) "历史 + 实时" 合并，拼成 SSE stream
    let out_stream = async_stream! {
        // history as "replay"
        for (rowid, r) in history {
            let data = serde_json::to_string(&r).unwrap_or_else(|_| "{}".to_string());
            yield Ok::<SseEvent, Infallible>(SseEvent::default().event("replay").id(rowid.to_string()).data(data));
        }

        // live events
        tokio::pin!(live);

        // heartbeat every 15s
        let mut ticker = tokio::time::interval(Duration::from_secs(15));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let data = now_epoch().to_string();
                    yield Ok::<SseEvent, Infallible>(
                        SseEvent::default()
                            .event("ping")
                            .data(data)
                    );
                }
                maybe = live.next() => {
                    match maybe {
                        Some(item) => {
                            yield item;
                        }
                        None => break,
                    }
                }
            }
        }

        // while let Some(item) = live.next().await {
        //     yield item;
        // }
    };

    return Sse::new(out_stream).into_response();
}

fn check_auth(headers: &HeaderMap, token: &Option<String>) -> Result<(), (StatusCode, String)> {
    let Some(expected) = token else {
        return Ok(());
    }; // 未设置 token, 则不鉴权（可选策略）
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let ok = auth == format!("Bearer {}", expected);
    if ok {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "unauthorized".to_string()))
    }
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
        "waweb" => Some(crate::domain::EventType::WhatsAppWebVersion),
        _ => None,
    }
}
