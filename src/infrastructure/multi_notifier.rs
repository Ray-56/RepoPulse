use async_trait::async_trait;

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
        // 单个渠道失败不影响其它渠道：这里选择“尽量发”
        // v1: 简单处理，后续可以记录 notify_log
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
    }
}
