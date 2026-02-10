use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};

use crate::application::{EventQuery, EventStore, TargetRepository};
use crate::domain::EventType;

/// Minimal MCP-like server over stdio:
/// - tools/list
/// - tools/call
pub struct McpServer {
    pub store: Arc<dyn EventStore>,
    pub targets: Arc<dyn TargetRepository>,
    pub api_token: Option<String>,
}

impl McpServer {
    pub async fn serve(self) -> anyhow::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = io::BufReader::new(stdin).lines();
        let mut out = io::BufWriter::new(stdout);

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let req: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    self.write_error(&mut out, None, format!("invalid json: {e}"))
                        .await?;
                    continue;
                }
            };

            let id = req.get("id").cloned();
            let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");

            match method {
                "tools/list" => {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "tools": [
                                {
                                    "name": "health",
                                    "description": "Health check for RepoPulse",
                                    "inputSchema": { "type": "object", "properties": {} }
                                },
                                {
                                    "name": "list_targets",
                                    "description": "List enabled watch targets",
                                    "inputSchema": { "type": "object", "properties": {} }
                                },
                                {
                                    "name": "get_events",
                                    "description": "Query recent events with filters",
                                    "inputSchema": {
                                      "type": "object",
                                      "properties": {
                                        "since": { "type": "string", "description": "e.g. 24h, 7d, 3600s" },
                                        "label": { "type": "string" },
                                        "type": { "type": "string", "description": "release|branch|npm|waweb" },
                                        "subject": { "type": "string" },
                                        "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                                      }
                                    }
                                }
                            ]
                        }
                    });
                    self.write_json(&mut out, resp).await?;
                }
                "tools/call" => {
                    let params = req.get("params").cloned().unwrap_or(json!({}));
                    let tool = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let args = params.get("arguments").cloned().unwrap_or(json!({}));

                    if let Some(expected) = &self.api_token {
                        let provided = args.get("token").and_then(|v| v.as_str()).unwrap_or("");
                        if provided != expected {
                            self.write_error(&mut out, id, "unauthorized".to_string())
                                .await?;
                            continue;
                        }
                    }

                    let result = match tool {
                        "health" => json!({"content": [{ "type": "text", "text": "ok"}]}),

                        "list_targets" => match self.targets.list_enabled_targets().await {
                            Ok(t) => json!({ "content": [ { "type": "json", "json": t } ] }),
                            Err(e) => {
                                self.write_error(&mut out, id, format!("list_targets failed: {e}"))
                                    .await?;
                                continue;
                            }
                        },
                        "get_events" => {
                            let limit = args
                                .get("limit")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(100)
                                .min(500) as u32;
                            let since_epoch = args
                                .get("since")
                                .and_then(|v| v.as_str())
                                .and_then(parse_since_to_epoch);
                            let label = args
                                .get("label")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let subject = args
                                .get("subject")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let event_type = args
                                .get("type")
                                .and_then(|v| v.as_str())
                                .and_then(parse_type);

                            let q = EventQuery {
                                since_epoch,
                                limit,
                                label,
                                event_type,
                                subject,
                            };

                            match self.store.list_events_filtered(q).await {
                                Ok(items) => {
                                    json!({ "content": [ { "type": "json", "json": { "items": items } } ] })
                                }
                                Err(e) => {
                                    self.write_error(
                                        &mut out,
                                        id,
                                        format!("get_events failed: {e}"),
                                    )
                                    .await?;
                                    continue;
                                }
                            }
                        }

                        _ => {
                            let msg = format!("unknown tool: {tool}");
                            self.write_error(&mut out, id, msg).await?;
                            continue;
                        }
                    };

                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result
                    });
                    self.write_json(&mut out, resp).await?;
                }

                _ => {
                    self.write_error(&mut out, id, format!("unknown method: {method}"))
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn write_json(
        &self,
        out: &mut io::BufWriter<io::Stdout>,
        v: Value,
    ) -> anyhow::Result<()> {
        out.write_all(v.to_string().as_bytes()).await?;
        out.write_all(b"\n").await?;
        out.flush().await?;
        Ok(())
    }

    async fn write_error(
        &self,
        out: &mut io::BufWriter<io::Stdout>,
        id: Option<Value>,
        msg: String,
    ) -> anyhow::Result<()> {
        let resp = json!({
          "jsonrpc": "2.0",
          "id": id,
          "error": {
            "code": -32600,
            "message": msg,
          }
        });
        self.write_json(out, resp).await
    }
}

fn now_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn parse_since_to_epoch(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 2 {
        return None;
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_part.parse().ok()?;
    let seconds = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86400,
        _ => return None,
    };
    Some(now_epoch().saturating_sub(seconds))
}

fn parse_type(t: &str) -> Option<EventType> {
    match t {
        "release" => Some(EventType::GitHubRelease),
        "branch" => Some(EventType::GitHubBranch),
        "npm" => Some(EventType::NpmLatest),
        "waweb" => Some(EventType::WhatsAppWebVersion),
        _ => None,
    }
}
