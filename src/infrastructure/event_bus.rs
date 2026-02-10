use tokio::sync::broadcast;

use crate::application::EventRecord;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<EventRecord>,
}

impl EventBus {
    pub fn new(buffer: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventRecord> {
        self.tx.subscribe()
    }

    pub fn publish(&self, record: EventRecord) {
        // ignore lag errors; consumers many miss some events if slow
        let _ = self.tx.send(record);
    }
}
