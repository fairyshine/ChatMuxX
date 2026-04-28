use super::{claude, codex, normalize};
use crate::provider::ProviderKind;

const MODEL_FIELD_LABELS: &[&str] = &["model", "模型"];
const STATUS_LABEL: &str = "Status";

pub(crate) fn extract_terminal_footer(text: &str, provider: ProviderKind) -> Option<String> {
    match provider {
        ProviderKind::Shell => None,
        ProviderKind::Codex => {
            let status = codex::extract_status(text);
            let context = extract_provider_footer_context(text);
            format_terminal_footer(status.as_deref(), context.as_deref())
        }
        ProviderKind::Claude => {
            let status = claude::extract_status(text);
            let context = extract_provider_footer_context(text);
            format_terminal_footer(status.as_deref(), context.as_deref())
        }
    }
}

pub(crate) fn footer_for_display(
    pending_footer: Option<&str>,
    last_footer: Option<&str>,
) -> Option<String> {
    pending_footer
        .and_then(normalize_footer)
        .or_else(|| last_footer.and_then(normalize_footer))
}

pub(crate) fn normalize_footer(footer: &str) -> Option<String> {
    let footer = footer.trim();
    if footer.is_empty() {
        None
    } else {
        Some(footer.to_owned())
    }
}

pub(crate) fn format_display_message(body: String, footer: Option<&str>) -> String {
    let body = body.trim_matches('\n').trim_end();
    match footer.map(str::trim).filter(|footer| !footer.is_empty()) {
        Some(footer) if !body.is_empty() => format!("{body}\n\n——\n{footer}"),
        Some(footer) => footer.to_owned(),
        None => body.to_owned(),
    }
}

fn format_terminal_footer(status: Option<&str>, context: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
        parts.push(format!("{STATUS_LABEL}: {}", status.trim()));
    }
    if let Some(context) = context.filter(|value| !value.trim().is_empty()) {
        parts.push(context.trim().to_owned());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

pub(crate) fn extract_footer_value(footer: &str, label: &str) -> Option<String> {
    footer.split('·').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(label)
            .and_then(|value| {
                value
                    .trim()
                    .strip_prefix(':')
                    .or_else(|| value.trim().strip_prefix('：'))
                    .or(Some(value.trim()))
            })
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn extract_provider_footer_context(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .take(12)
        .find_map(extract_footer_context_from_line)
}

fn extract_footer_context_from_line(line: &str) -> Option<String> {
    let cleaned = clean_footer_line(line);
    if cleaned.is_empty() {
        return None;
    }

    if is_terminal_footer_line(&cleaned) {
        if let Some(model) = extract_model_from_line(&cleaned) {
            return Some(model);
        }

        if cleaned.contains('·') {
            return Some(cleaned);
        }
    }

    extract_model_from_line(&cleaned)
}

pub(super) fn clean_footer_line(line: &str) -> String {
    line.trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '│' | '┃' | '┆' | '┊' | '║' | '╎' | '╏' | '─' | '━' | ' ' | '\t'
            )
        })
        .trim()
        .to_owned()
}

fn extract_model_from_line(line: &str) -> Option<String> {
    if let Some(value) = extract_labeled_value(line, MODEL_FIELD_LABELS) {
        return Some(value);
    }

    if !is_terminal_footer_line(line) {
        return None;
    }

    let cleaned = clean_footer_line(line);
    if looks_like_model_context(&cleaned) {
        return Some(cleaned);
    }

    line.split('·')
        .map(clean_footer_line)
        .find(|part| looks_like_model_context(part))
}

fn extract_labeled_value(line: &str, labels: &[&str]) -> Option<String> {
    for label in labels {
        let lower = line.to_ascii_lowercase();
        let Some(start) = lower.find(&label.to_ascii_lowercase()) else {
            continue;
        };
        let after_label = &line[start + label.len()..];
        let value = after_label
            .trim_start()
            .trim_start_matches([':', '：', '=', '-'])
            .trim_start();
        let value = value
            .split(['·', '|', '│'])
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if !value.is_empty() {
            return Some(value.to_owned());
        }
    }

    None
}

pub(super) fn looks_like_model_context(text: &str) -> bool {
    let text = clean_footer_line(text);
    let lower = text.to_ascii_lowercase();
    !text.is_empty()
        && text.chars().count() <= 120
        && !looks_like_path_only(&text)
        && (lower.contains("model")
            || lower.contains("模型")
            || (text.contains('·') && text.chars().any(|ch| ch.is_ascii_digit())))
}

pub(super) fn is_terminal_footer_line(line: &str) -> bool {
    let cleaned = clean_footer_line(line);
    if cleaned.is_empty()
        || normalize::is_agent_numbered_option_line(&cleaned)
        || normalize::looks_like_numbered_option(&cleaned)
        || looks_like_path_only(&cleaned)
    {
        return false;
    }

    let lower = cleaned.to_ascii_lowercase();
    contains_labeled_value(&lower)
        || lower.contains("token")
        || lower.contains("context")
        || lower.contains("上下文")
        || (cleaned.contains('·')
            && cleaned.chars().any(|ch| ch.is_ascii_digit())
            && !cleaned.contains('?'))
        || normalize::is_mostly_ui_symbols(&cleaned)
}

fn contains_labeled_value(lower: &str) -> bool {
    lower.contains("model") || lower.contains("模型")
}

pub(super) fn looks_like_path_only(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.chars().any(char::is_whitespace)
        && (text.starts_with('/') || text.starts_with("~/") || text.contains("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn footer_is_added_to_display_message() {
        let text = format_display_message("done".to_owned(), Some("Status: thinking · gpt-5.5"));

        assert_eq!(text, "done\n\n——\nStatus: thinking · gpt-5.5");
    }

    #[test]
    fn unchanged_footer_is_included_with_display_body() {
        let footer = footer_for_display(Some("Status: thinking"), Some("Status: thinking"));

        assert_eq!(footer, Some("Status: thinking".to_owned()));
    }

    #[test]
    fn last_footer_is_reused_when_current_pane_has_no_footer() {
        let footer = footer_for_display(None, Some("gpt-5.5 high · ~/Code/ChatMuxX"));

        assert_eq!(footer, Some("gpt-5.5 high · ~/Code/ChatMuxX".to_owned()));
    }

    #[test]
    fn terminal_footer_includes_status_and_model() {
        let footer = extract_terminal_footer(
            "answer\n⠋ thinking\nmodel: gpt-5.1-codex",
            ProviderKind::Codex,
        );

        assert_eq!(footer, Some("Status: thinking · gpt-5.1-codex".to_owned()));
    }

    #[test]
    fn terminal_footer_keeps_codex_bottom_bar_context() {
        let footer = extract_terminal_footer(
            "answer\n\n› Use /skills to list available skills\n\n  gpt-5.5 high · ~/Code/ChatMuxX",
            ProviderKind::Codex,
        );

        assert_eq!(footer, Some("gpt-5.5 high · ~/Code/ChatMuxX".to_owned()));
    }

    #[test]
    fn terminal_footer_keeps_claude_startup_context() {
        let version = "v42.0.7";
        let footer = extract_terminal_footer(
            &format!("╭─── Claude Code {version} ─────────╮\n│   Sonnet 4.6 · API Usage Billing   │\n│          ~/Code/ChatMuxX           │\n❯\n  ? for shortcuts              ● high · /effort"),
            ProviderKind::Claude,
        );

        assert_eq!(footer, Some("Sonnet 4.6 · API Usage Billing".to_owned()));
    }

    #[test]
    fn terminal_footer_includes_claude_status() {
        let footer = extract_terminal_footer(
            "⏺ 让我先看一下。\n✻ Pondering… (8s · thinking)\n⎿ README.md\n  Sonnet 4.6 · API Usage Billing",
            ProviderKind::Claude,
        );

        assert_eq!(
            footer,
            Some(
                "Status: ✻ Pondering… (8s · thinking) · Sonnet 4.6 · API Usage Billing".to_owned()
            )
        );
    }

    #[test]
    fn terminal_footer_includes_status_and_bottom_bar_context() {
        let footer = extract_terminal_footer(
            "answer\n⠋ thinking\n  gpt-5.5 high · ~/Code/ChatMuxX",
            ProviderKind::Codex,
        );

        assert_eq!(
            footer,
            Some("Status: thinking · gpt-5.5 high · ~/Code/ChatMuxX".to_owned())
        );
    }
}
