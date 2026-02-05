use crate::application::{AppResult, EventStore, Notifier};
use crate::domain::Event;

pub struct HandleEventUseCase<'a> {
    pub store: &'a dyn EventStore,
    pub notifier: &'a dyn Notifier,
}

impl<'a> HandleEventUseCase<'a> {
    pub async fn execute(&self, event: &Event) -> AppResult<()> {
        // v1: only dedup; cooldown later in v1 step 7
        if self.store.has_seen(&event.event_id).await? {
            return Ok(());
        }

        self.store.append_event(event).await?;
        self.store.mark_seen(&event.event_id).await?;
        self.notifier.notify(event).await?;

        Ok(())
    }
}
