mod claude;
mod codex;
mod footer;
mod normalize;
mod shell;
mod split;

use crate::provider::ProviderKind;

pub(crate) use footer::{
    extract_footer_value, extract_terminal_footer, footer_for_display, format_display_message,
    normalize_footer,
};
pub(crate) use normalize::is_separator_char;
pub(crate) use split::{split_for_chat, trim_for_chat};

pub(crate) fn normalize_pane_text(text: &str, provider: ProviderKind) -> String {
    match provider {
        ProviderKind::Codex => codex::normalize_pane_text(text),
        ProviderKind::Claude => claude::normalize_pane_text(text),
        ProviderKind::Shell => shell::normalize_pane_text(text),
    }
}

pub(crate) fn normalize_raw_pane_text(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
