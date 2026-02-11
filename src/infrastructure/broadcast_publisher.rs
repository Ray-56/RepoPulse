use async_trait::async_trait;

use crate::application::{AppResult, EventRecord, EventRecordPublisher};
use crate::infrastructure::event_bus::EventBus;

pub struct BroadcastPublisher {
    bus: EventBus,
}

impl BroadcastPublisher {
    pub fn new(bus: EventBus) -> Self {
        Self { bus }
    }
}

#[async_trait]
impl EventRecordPublisher for BroadcastPublisher {
    async fn publish(&self, rowid: i64, record: &EventRecord) -> AppResult<()> {
        self.bus.publish(rowid, record.clone());
        Ok(())
    }
}
