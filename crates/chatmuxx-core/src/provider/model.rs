use std::{collections::BTreeMap, fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::state::sessions::SessionId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Codex,
    Claude,
    Shell,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Shell => "shell",
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProviderKind {
    type Err = ProviderKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "codex" => Ok(Self::Codex),
            "claude" | "claude-code" | "cc" => Ok(Self::Claude),
            "shell" | "sh" => Ok(Self::Shell),
            other => Err(ProviderKindParseError {
                value: other.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderKindParseError {
    pub value: String,
}

impl fmt::Display for ProviderKindParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown provider kind: {}", self.value)
    }
}

impl std::error::Error for ProviderKindParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCapabilities {
    pub structured_output: bool,
    pub resume: bool,
    pub continue_last: bool,
    pub prompt_detection: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LaunchMode {
    Fresh,
    Continue,
    Resume { provider_session_id: String },
}

impl LaunchMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Continue => "continue",
            Self::Resume { .. } => "resume",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderLaunchRequest {
    pub workspace: PathBuf,
    pub launch_mode: LaunchMode,
    pub extra_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderLaunchCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub display_name: String,
}

impl ProviderLaunchCommand {
    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone());
        argv.extend(self.args.clone());
        argv
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputSource {
    Structured {
        kind: OutputSourceKind,
        path: PathBuf,
    },
    PaneOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputSourceKind {
    CodexJsonl,
    ClaudeTranscript,
    TmuxPane,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProviderCursor {
    pub offset: Option<u64>,
    pub last_seen_id: Option<String>,
    pub pane_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReadResult {
    pub events: Vec<ProviderEvent>,
    pub next_cursor: ProviderCursor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderEvent {
    AssistantMessage {
        session_id: SessionId,
        text: String,
        source: OutputSourceKind,
    },
    StatusChanged {
        session_id: SessionId,
        status: ProviderStatus,
    },
    CommandOutput {
        session_id: SessionId,
        text: String,
    },
    SessionFinished {
        session_id: SessionId,
    },
    SessionFailed {
        session_id: SessionId,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderStatus {
    Starting,
    Running,
    WaitingInput,
    Done,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_parses_mobile_names() {
        assert_eq!(
            "codex".parse::<ProviderKind>().unwrap(),
            ProviderKind::Codex
        );
        assert_eq!(
            "CLAUDE".parse::<ProviderKind>().unwrap(),
            ProviderKind::Claude
        );
        assert_eq!(
            "claude-code".parse::<ProviderKind>().unwrap(),
            ProviderKind::Claude
        );
        assert_eq!("cc".parse::<ProviderKind>().unwrap(), ProviderKind::Claude);
        assert_eq!("sh".parse::<ProviderKind>().unwrap(), ProviderKind::Shell);
    }

    #[test]
    fn provider_kind_serializes_as_stable_snake_case() {
        let text = serde_json::to_string(&ProviderKind::Shell).unwrap();

        assert_eq!(text, "\"shell\"");
    }

    #[test]
    fn launch_command_argv_keeps_program_first() {
        let command = ProviderLaunchCommand {
            program: "bash".to_owned(),
            args: vec!["-l".to_owned()],
            env: BTreeMap::new(),
            display_name: "shell".to_owned(),
        };

        assert_eq!(command.argv(), vec!["bash", "-l"]);
    }
}
