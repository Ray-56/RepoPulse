use async_trait::async_trait;
use tracing::info;

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
        info!(
          event_type = ?event.event_type,
          subject = %event.subject,
          old = %event.old_value.clone().unwrap_or_else(|| "(none)".into()),
          new = %event.new_value,
          url = %event.url.clone().unwrap_or_else(|| "(none)".into()),
          "notify"
        );
        Ok(())
    }
}
