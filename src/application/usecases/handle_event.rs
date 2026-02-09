use crate::application::{AppResult, EventStore, Notifier};
use crate::domain::Event;

pub struct HandleEventUseCase<'a> {
    pub store: &'a dyn EventStore,
    pub notifier: &'a dyn Notifier,
    pub cooldown_seconds: u64, // 0 means disabled
}

impl<'a> HandleEventUseCase<'a> {
    pub async fn execute(
        &self,
        event: &Event,
        target_id: &str,
        labels: &[String],
    ) -> AppResult<()> {
        // 1) dedup by event_id
        if self.store.has_seen(&event.event_id).await? {
            return Ok(());
        }

        // 2) persist event & seen
        let now_epoch = epoch_seconds();
        let record = crate::application::EventRecord {
            event: event.clone(),
            target_id: target_id.to_string(),
            labels: labels.to_vec(),
            detected_at_epoch: now_epoch,
        };
        self.store.append_event_record(&record).await?;
        self.store.mark_seen(&event.event_id).await?;

        // 3) cooldown policy (ByTargetAndType)
        if self.cooldown_seconds > 0 {
            let scope_key = format!("{}|{:?}", target_id, event.event_type);
            let now = epoch_seconds();

            if let Some(last) = self.store.get_last_notified(&scope_key).await? {
                let elapsed = now.saturating_sub(last);
                if elapsed < self.cooldown_seconds as i64 {
                    // within cooldown: skip notifying
                    return Ok(());
                }
            }

            // send + record
            self.notifier.notify(event).await?;
            self.store.set_last_notified(&scope_key, now).await?;
            return Ok(());
        }

        // no cooldown
        self.notifier.notify(event).await?;
        Ok(())
    }
}

fn epoch_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
