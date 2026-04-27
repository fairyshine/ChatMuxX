use std::path::PathBuf;
use std::str::FromStr;

use crate::{provider::ProviderKind, state::sessions::SessionId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MobileCommand {
    Help,
    New(NewSessionArgs),
    Sessions,
    Switch {
        session_id: Option<SessionId>,
    },
    Close {
        session_id: Option<SessionId>,
    },
    Rename {
        session_id: Option<SessionId>,
        new_id: Option<SessionId>,
    },
    Prune,
    Provider {
        provider: Option<ProviderKind>,
    },
    Screenshot,
    Esc,
    Interrupt,
    Enter,
    Recover {
        session_id: Option<SessionId>,
    },
    Unknown {
        name: String,
        args: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSessionArgs {
    pub id: Option<SessionId>,
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
    text == "cmx" || text.strip_prefix("cmx ").is_some()
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
        "new" | "n" => MobileCommand::New(parse_new_args(rest)),
        "sessions" | "windows" | "s" => parse_sessions_command(rest),
        "list" | "ls" => MobileCommand::Sessions,
        "switch" | "sw" => MobileCommand::Switch {
            session_id: first_session_id(rest),
        },
        "close" | "rm" => MobileCommand::Close {
            session_id: first_session_id(rest),
        },
        "rename" | "mv" => parse_rename_args(rest),
        "prune" | "clean" | "cleanup" => MobileCommand::Prune,
        "provider" | "use" => MobileCommand::Provider {
            provider: rest
                .first()
                .and_then(|value| ProviderKind::from_str(value).ok()),
        },
        "capture" | "cap" | "screenshot" | "shot" | "ss" => MobileCommand::Screenshot,
        "esc" => MobileCommand::Esc,
        "interrupt" | "ctrl-c" | "stop" | "i" => MobileCommand::Interrupt,
        "enter" | "e" => MobileCommand::Enter,
        "recover" | "re" => MobileCommand::Recover {
            session_id: first_session_id(rest),
        },
        other => MobileCommand::Unknown {
            name: other.to_owned(),
            args: rest,
        },
    }
}

fn parse_sessions_command(args: Vec<String>) -> MobileCommand {
    let mut args = args.into_iter();
    let Some(name) = args.next() else {
        return MobileCommand::Sessions;
    };
    let rest = args.collect::<Vec<_>>();

    match name.as_str() {
        "new" | "n" => MobileCommand::New(parse_new_args(rest)),
        "list" | "ls" => MobileCommand::Sessions,
        "switch" | "sw" => MobileCommand::Switch {
            session_id: first_session_id(rest),
        },
        "close" | "rm" => MobileCommand::Close {
            session_id: first_session_id(rest),
        },
        "rename" | "mv" => parse_rename_args(rest),
        "prune" | "clean" | "cleanup" => MobileCommand::Prune,
        "capture" | "cap" | "screenshot" | "shot" | "ss" => MobileCommand::Screenshot,
        other => MobileCommand::Unknown {
            name: other.to_owned(),
            args: rest,
        },
    }
}

fn parse_new_args(args: Vec<String>) -> NewSessionArgs {
    let mut id = None;
    let mut workspace = None;
    let mut provider = None;
    let mut extra_args = Vec::new();
    let mut after_delimiter = false;
    let mut id_pending = false;

    for arg in args {
        if after_delimiter {
            extra_args.push(arg);
            continue;
        }

        if id_pending {
            id = Some(SessionId(arg));
            id_pending = false;
        } else if arg == "--id" {
            id_pending = true;
        } else if let Some(value) = arg.strip_prefix("--id=") {
            id = Some(SessionId(value.to_owned()));
        } else if arg == "--" {
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
        id,
        workspace,
        provider,
        extra_args,
    }
}

fn first_session_id(args: Vec<String>) -> Option<SessionId> {
    args.into_iter().next().map(SessionId)
}

fn parse_rename_args(args: Vec<String>) -> MobileCommand {
    match args.as_slice() {
        [] => MobileCommand::Rename {
            session_id: None,
            new_id: None,
        },
        [new_id] => MobileCommand::Rename {
            session_id: None,
            new_id: Some(SessionId(new_id.clone())),
        },
        [session_id, new_id, ..] => MobileCommand::Rename {
            session_id: Some(SessionId(session_id.clone())),
            new_id: Some(SessionId(new_id.clone())),
        },
    }
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
    fn cmx_without_subcommand_becomes_help() {
        assert_eq!(
            parse_mobile_text("cmx", false),
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
            parse_mobile_text("cmx new '/tmp/my project' claude -- --model opus", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: None,
                workspace: Some(PathBuf::from("/tmp/my project")),
                provider: Some(ProviderKind::Claude),
                extra_args: vec!["--model".to_owned(), "opus".to_owned()],
            }))
        );
    }

    #[test]
    fn new_command_parses_custom_session_id() {
        assert_eq!(
            parse_mobile_text("cmx new --id main /tmp/project codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: Some(SessionId("main".to_owned())),
                workspace: Some(PathBuf::from("/tmp/project")),
                provider: Some(ProviderKind::Codex),
                extra_args: Vec::new(),
            }))
        );
    }

    #[test]
    fn new_command_parses_custom_session_id_after_provider() {
        assert_eq!(
            parse_mobile_text("cmx new /tmp/project codex --id main", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: Some(SessionId("main".to_owned())),
                workspace: Some(PathBuf::from("/tmp/project")),
                provider: Some(ProviderKind::Codex),
                extra_args: Vec::new(),
            }))
        );
    }

    #[test]
    fn sessions_new_command_parses_like_new() {
        assert_eq!(
            parse_mobile_text("cmx sessions new --id main /tmp/project codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: Some(SessionId("main".to_owned())),
                workspace: Some(PathBuf::from("/tmp/project")),
                provider: Some(ProviderKind::Codex),
                extra_args: Vec::new(),
            }))
        );
        assert_eq!(
            parse_mobile_text("cmx s n --id main /tmp/project codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: Some(SessionId("main".to_owned())),
                workspace: Some(PathBuf::from("/tmp/project")),
                provider: Some(ProviderKind::Codex),
                extra_args: Vec::new(),
            }))
        );
    }

    #[test]
    fn short_aliases_parse_to_mobile_commands() {
        assert_eq!(
            parse_mobile_text("cmx n /tmp/project codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::New(NewSessionArgs {
                id: None,
                workspace: Some(PathBuf::from("/tmp/project")),
                provider: Some(ProviderKind::Codex),
                extra_args: Vec::new(),
            }))
        );
        assert_eq!(
            parse_mobile_text("cmx ss", false),
            ParsedInbound::BridgeCommand(MobileCommand::Screenshot)
        );
        assert_eq!(
            parse_mobile_text("cmx i", false),
            ParsedInbound::BridgeCommand(MobileCommand::Interrupt)
        );
        assert_eq!(
            parse_mobile_text("cmx e", false),
            ParsedInbound::BridgeCommand(MobileCommand::Enter)
        );
    }

    #[test]
    fn provider_command_accepts_provider_name() {
        assert_eq!(
            parse_mobile_text("cmx provider codex", false),
            ParsedInbound::BridgeCommand(MobileCommand::Provider {
                provider: Some(ProviderKind::Codex)
            })
        );
    }

    #[test]
    fn sessions_list_command_is_a_sessions_command() {
        assert_eq!(
            parse_mobile_text("cmx sessions list", false),
            ParsedInbound::BridgeCommand(MobileCommand::Sessions)
        );
        assert_eq!(
            parse_mobile_text("cmx list", false),
            ParsedInbound::BridgeCommand(MobileCommand::Sessions)
        );
    }

    #[test]
    fn prune_command_is_parsed() {
        assert_eq!(
            parse_mobile_text("cmx prune", false),
            ParsedInbound::BridgeCommand(MobileCommand::Prune)
        );
    }

    #[test]
    fn capture_aliases_parse_to_screenshot() {
        assert_eq!(
            parse_mobile_text("cmx cap", false),
            ParsedInbound::BridgeCommand(MobileCommand::Screenshot)
        );
        assert_eq!(
            parse_mobile_text("cmx sessions capture", false),
            ParsedInbound::BridgeCommand(MobileCommand::Screenshot)
        );
    }

    #[test]
    fn rename_command_with_one_arg_renames_active_session() {
        assert_eq!(
            parse_mobile_text("cmx rename main", false),
            ParsedInbound::BridgeCommand(MobileCommand::Rename {
                session_id: None,
                new_id: Some(SessionId("main".to_owned())),
            })
        );
    }

    #[test]
    fn rename_command_with_two_args_renames_given_session() {
        assert_eq!(
            parse_mobile_text("cmx rename old main", false),
            ParsedInbound::BridgeCommand(MobileCommand::Rename {
                session_id: Some(SessionId("old".to_owned())),
                new_id: Some(SessionId("main".to_owned())),
            })
        );
    }

    #[test]
    fn close_command_accepts_optional_session_id() {
        assert_eq!(
            parse_mobile_text("cmx close sess-1", false),
            ParsedInbound::BridgeCommand(MobileCommand::Close {
                session_id: Some(SessionId("sess-1".to_owned()))
            })
        );
    }

    #[test]
    fn split_words_handles_basic_quotes() {
        assert_eq!(
            split_words("cmx new \"/tmp/a b\" shell"),
            vec!["cmx", "new", "/tmp/a b", "shell"]
        );
    }
}
