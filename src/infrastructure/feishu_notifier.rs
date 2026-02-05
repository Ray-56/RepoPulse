use async_trait::async_trait;
use serde::Serialize;

use crate::application::{AppError, AppResult, Notifier};
use crate::domain::Event;

pub struct FeishuNotifier {
    client: reqwest::Client,
    webhook: String,
}

impl FeishuNotifier {
    pub fn new(webhook: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            webhook,
        }
    }
}

#[derive(Debug, Serialize)]
struct FeishuTextMsg<'a> {
    msg_type: &'a str,
    content: FeishuTextContent<'a>,
}

#[derive(Debug, Serialize)]
struct FeishuTextContent<'a> {
    text: &'a str,
}

#[async_trait]
impl Notifier for FeishuNotifier {
    async fn notify(&self, event: &Event) -> AppResult<()> {
        let text = format_event_text(event);

        let payload = FeishuTextMsg {
            msg_type: "text",
            content: FeishuTextContent { text: &text },
        };

        self.client
            .post(&self.webhook)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Notifier(e.to_string()))?
            .error_for_status()
            .map_err(|e| AppError::Notifier(e.to_string()))?;

        Ok(())
    }
}

fn format_event_text(event: &Event) -> String {
    let mut lines = vec![];

    lines.push(format!("🔔 RepoPulse 检测到更新"));
    lines.push(format!("📢 事件类型: {:?}", event.event_type));
    lines.push(format!("🎯 对象: {}", event.subject));

    if let Some(old) = &event.old_value {
        lines.push(format!("变化: {} -> {}", old, event.new_value));
    } else {
        lines.push(format!("新值: {}", event.new_value));
    }

    if let Some(t) = &event.occurred_at {
        lines.push(format!("发生时间: {}", t));
    }
    lines.push(format!("检测时间: {}", event.detected_at));

    if let Some(url) = &event.url {
        lines.push(format!("详情: {}", url));
    }

    lines.join("\n")
}
