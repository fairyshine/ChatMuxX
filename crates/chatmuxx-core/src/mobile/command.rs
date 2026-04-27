use std::path::PathBuf;
use std::str::FromStr;

use crate::{provider::ProviderKind, state::sessions::SessionId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MobileCommand {
    Help,
    New(NewSessionArgs),
    Sessions,
    Switch { session_id: Option<SessionId> },
    Close { session_id: Option<SessionId> },
    Provider { provider: Option<ProviderKind> },
    Screenshot,
    Esc,
    Interrupt,
    Enter,
    Recover { session_id: Option<SessionId> },
    Unknown { name: String, args: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSessionArgs {
    pub workspace: Option<PathBuf>,
    pub provider: Option<ProviderKind>,
    pub extra_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedInbound {
    BridgeCommand(MobileCommand),
    ProviderSlashCommand(String),
    PlainText(String),
    FlowReply(String),
}

pub fn parse_mobile_text(text: &str, flow_active: bool) -> ParsedInbound {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ParsedInbound::PlainText(String::new());
    }

    if is_bridge_command(trimmed) {
        return ParsedInbound::BridgeCommand(parse_bridge_command(trimmed));
    }

    if trimmed.starts_with('/') {
        return ParsedInbound::ProviderSlashCommand(trimmed.to_owned());
    }

    if flow_active {
        ParsedInbound::FlowReply(trimmed.to_owned())
    } else {
        ParsedInbound::PlainText(text.to_owned())
    }
}

fn is_bridge_command(text: &str) -> bool {
    text == "cmux" || text.strip_prefix("cmux ").is_some()
}

fn parse_bridge_command(text: &str) -> MobileCommand {
    let tokens = split_words(text);
    let mut args = tokens.into_iter();
    let _command_prefix = args.next();
    let Some(name) = args.next() else {
        return MobileCommand::Help;
    };
    let rest = args.collect::<Vec<_>>();

    match name.as_str() {
        "help" | "h" => MobileCommand::Help,
        "new" => MobileCommand::New(parse_new_args(rest)),
        "sessions" | "windows" | "ls" => MobileCommand::Sessions,
        "switch" | "sw" => MobileCommand::Switch {
            session_id: first_session_id(rest),
        },
        "close" | "rm" => MobileCommand::Close {
            session_id: first_session_id(rest),
        },
        "provider" | "use" => MobileCommand::Provider {
            provider: rest
                .first()
                .and_then(|value| ProviderKind::from_str(value).ok()),
        },
        "screenshot" | "shot" => MobileCommand::Screenshot,
        "esc" => MobileCommand::Esc,
        "interrupt" | "ctrl-c" | "stop" => MobileCommand::Interrupt,
        "enter" => MobileCommand::Enter,
        "recover" => MobileCommand::Recover {
            session_id: first_session_id(rest),
        },
        other => MobileCommand::Unknown {
            name: other.to_owned(),
            args: rest,
        },
    }
}

fn parse_new_args(args: Vec<String>) -> NewSessionArgs {
    let mut workspace = None;
    let mut provider = None;
    let mut extra_args = Vec::new();
    let mut after_delimiter = false;

    for arg in args {
        if after_delimiter {
            extra_args.push(arg);
            continue;
        }

        if arg == "--" {
            after_delimiter = true;
        } else if provider.is_none() {
            if let Ok(kind) = ProviderKind::from_str(&arg) {
                provider = Some(kind);
            } else if workspace.is_none() {
                workspace = Some(PathBuf::from(arg));
            } else {
                extra_args.push(arg);
            }
        } else if workspace.is_none() {
            workspace = Some(PathBuf::from(arg));
        } else {
            extra_args.push(arg);
        }
    }

    NewSessionArgs {
        workspace,
        provider,
        extra_args,
    }
}

fn first_session_id(args: Vec<String>) -> Option<SessionId> {
    args.into_iter().next().map(SessionId)
}

fn split_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match (quote, ch) {
            (_, '\\') => escaped = true,
            (Some(active), ch) if ch == active => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmux_without_subcommand_becomes_help() {
        assert_eq!(
            parse_mobile_text("cmux", false),
            ParsedInbound::BridgeCommand(MobileCommand::Help)
        );
    }

    #[test]
    fn slash_command_is_forwarded_to_provider() {
        assert_eq!(
            parse_mobile_text("/model gpt-5.5", false),
            ParsedInbound::ProviderSlashCommand("/model gpt-5.5".to_owned())
        );
    }

    #[test]
    fn plain_text_is_not_claimed_by_bridge() {
        assert_eq!(
            parse_mobile_text("hello codex", false),
            ParsedInbound::PlainText("hello codex".to_owned())
        );
    }

    #[test]
    fn active_flow_claims_plain_text_replies() {
        assert_eq!(
            parse_mobile_text("1", true),
            ParsedInbound::FlowReply("1".to_owned())
        );
    }

    #[test]
    fn new_command_parses_workspace_provider_and_extra_args() {
        assert_eq!(
            parse_mobile_text("cmux new '/tmp/my project' claude -- --model opus", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                workspace: Some(PathBuf::from("/tmp/my project")),
                provider: Some(ProviderKind::Claude),
                extra_args: vec!["--model".to_owned(), "opus".to_owned()],
            }))
        );
    }

    #[test]
    fn provider_command_accepts_provider_name() {
        assert_eq!(
            parse_mobile_text("cmux provider codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::Provider {
                provider: Some(ProviderKind::Codex)
            })
        );
    }

    #[test]
    fn close_command_accepts_optional_session_id() {
        assert_eq!(
            parse_mobile_text("cmux close sess-1", false),
            ParsedInbound::BridgeCommand(MobileCommand::Close {
                session_id: Some(SessionId("sess-1".to_owned()))
            })
        );
    }

    #[test]
    fn split_words_handles_basic_quotes() {
        assert_eq!(
            split_words("cmux new \"/tmp/a b\" shell"),
            vec!["cmux", "new", "/tmp/a b", "shell"]
        );
    }
}
