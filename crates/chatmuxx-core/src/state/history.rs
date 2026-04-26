use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    error::{IoContext, Result},
    security::redact::redact_key_value,
};

use super::SessionId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistoryLine {
    pub schema_version: u32,
    pub event: HistoryEvent,
}

impl HistoryLine {
    pub fn new(event: HistoryEvent) -> Self {
        Self {
            schema_version: 1,
            event: event.redacted(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryEvent {
    UserInput {
        conversation_id: String,
        session_id: Option<SessionId>,
        text: String,
        at: String,
    },
    ProviderOutput {
        session_id: SessionId,
        text: String,
        at: String,
    },
    SessionEvent {
        session_id: SessionId,
        event: SessionHistoryKind,
        at: String,
    },
    BridgeCommand {
        conversation_id: String,
        command: String,
        at: String,
    },
}

impl HistoryEvent {
    fn redacted(self) -> Self {
        match self {
            Self::UserInput {
                conversation_id,
                session_id,
                text,
                at,
            } => Self::UserInput {
                conversation_id,
                session_id,
                text: redact_text(&text),
                at,
            },
            Self::ProviderOutput {
                session_id,
                text,
                at,
            } => Self::ProviderOutput {
                session_id,
                text: redact_text(&text),
                at,
            },
            Self::SessionEvent {
                session_id,
                event,
                at,
            } => Self::SessionEvent {
                session_id,
                event,
                at,
            },
            Self::BridgeCommand {
                conversation_id,
                command,
                at,
            } => Self::BridgeCommand {
                conversation_id,
                command: redact_text(&command),
                at,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHistoryKind {
    Created,
    Bound,
    Switched,
    Closed,
    Recovered,
    ProviderReplaced,
}

pub async fn append_history(path: &Path, event: HistoryEvent) -> Result<()> {
    if let Some(parent) = path.parent() {
        super::files::ensure_state_dir(parent).await?;
    }

    let line = HistoryLine::new(event);
    let mut bytes =
        serde_json::to_vec(&line).map_err(|source| crate::ChatMuxXError::JsonSerialize {
            path: path.to_path_buf(),
            source,
        })?;
    bytes.push(b'\n');

    let mut options = tokio::fs::OpenOptions::new();
    options.create(true).append(true);
    let mut file = options.open(path).await.at(path)?;

    use tokio::io::AsyncWriteExt;
    file.write_all(&bytes).await.at(path)?;
    file.flush().await.at(path)
}

fn redact_text(text: &str) -> String {
    let mut redacted = text.to_owned();
    for key in [
        "authorization",
        "bot_token",
        "context_token",
        "typing_ticket",
        "upload_url",
    ] {
        redacted = redact_assignments(&redacted, key);
    }
    redacted
}

fn redact_assignments(text: &str, _key: &str) -> String {
    text.split_whitespace()
        .map(|part| {
            let Some((left, right)) = part.split_once('=') else {
                return part.to_owned();
            };
            format!(
                "{left}={}",
                redact_key_value(key_from_assignment(left), right)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn key_from_assignment(left: &str) -> &str {
    left.trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '{' || ch == ',')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn append_history_writes_jsonl() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.jsonl");

        append_history(
            &path,
            HistoryEvent::BridgeCommand {
                conversation_id: "conv_1".to_owned(),
                command: "cmx help".to_owned(),
                at: "2026-04-26T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("append history");

        let text = tokio::fs::read_to_string(&path)
            .await
            .expect("read history");
        assert_eq!(text.lines().count(), 1);
        assert!(text.contains("\"bridge_command\""));
    }

    #[test]
    fn history_line_redacts_sensitive_assignments() {
        let line = HistoryLine::new(HistoryEvent::UserInput {
            conversation_id: "conv_1".to_owned(),
            session_id: None,
            text: "bot_token=abcdefghijklmnop ok=yes".to_owned(),
            at: "2026-04-26T00:00:00Z".to_owned(),
        });

        let json = serde_json::to_string(&line).expect("history json");
        assert!(!json.contains("abcdefghijklmnop"));
        assert!(json.contains("abcd"));
        assert!(json.contains("ok=yes"));
    }
}
