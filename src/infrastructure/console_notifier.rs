use async_trait::async_trait;

use crate::application::{AppResult, Notifier};
use crate::domain::Event;

pub struct ConsoleNotifier;

impl ConsoleNotifier {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Notifier for ConsoleNotifier {
    async fn notify(&self, event: &Event) -> AppResult<()> {
        println!(
            "NOTIFY: type={:?} subject={} {} -> {} url={}",
            event.event_type,
            event.subject,
            event.old_value.clone().unwrap_or_else(|| "(none".into()),
            event.new_value,
            event.url.clone().unwrap_or_else(|| "(none)".into())
        );
        Ok(())
    }
}
