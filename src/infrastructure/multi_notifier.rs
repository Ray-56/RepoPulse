use async_trait::async_trait;
use tracing::warn;

use crate::application::{AppResult, Notifier};
use crate::domain::Event;

pub struct MultiNotifier {
    notifiers: Vec<Box<dyn Notifier>>,
}

impl MultiNotifier {
    pub fn new(notifiers: Vec<Box<dyn Notifier>>) -> Self {
        Self { notifiers }
    }
}

#[async_trait]
impl Notifier for MultiNotifier {
    async fn notify(&self, event: &Event) -> AppResult<()> {
        for (idx, n) in self.notifiers.iter().enumerate() {
            if let Err(e) = n.notify(event).await {
                warn!(
                    notifier_index = idx,
                    event_id = %event.event_id,
                    err = %format!("{e}"),
                    "notifier failed"
                );
            }
        }
        Ok(())
    }
    /* async fn notify(&self, event: &Event) -> AppResult<()> {
        let mut last_err = None;

        for n in &self.notifiers {
            if let Err(e) = n.notify(event).await {
                last_err = Some(e);
            }
        }

        if let Some(e) = last_err {
            return Err(e);
        }

        Ok(())
    } */
}
