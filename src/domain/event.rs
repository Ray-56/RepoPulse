use serde::{Deserialize, Serialize};

use super::Source;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    GitHubRelease,
    GitHubBranch,
    NpmLatest,
    WhatsAppWebVersion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String, // idempotency key
    pub event_type: EventType,
    pub source: Source,
    pub subject: String, // "owner/repo" or "pkg"
    pub old_value: Option<String>,
    pub new_value: String,
    /// TODO: upgrade to chrono datetime for better timezone handling.
    pub occurred_at: Option<String>, // upstream time if known (RFC3339 string for now)
    pub detected_at: String, // local time (RFC3339 string for now)
    pub url: Option<String>,
}

impl Event {
    /// A simple deterministic id key. v1 uses a naive schema; can be upgraded to hashing later.
    pub fn make_event_id(event_type: &EventType, subject: &str, new_value: &str) -> String {
        format!("{:?}|{}|{}", event_type, subject, new_value)
    }
}
