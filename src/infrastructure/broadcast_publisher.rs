use async_trait::async_trait;

use crate::application::{AppResult, EventPublisher, EventRecord};
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
impl EventPublisher for BroadcastPublisher {
    async fn publish(&self, record: &EventRecord) -> AppResult<()> {
        self.bus.publish(record.clone());
        Ok(())
    }
}
