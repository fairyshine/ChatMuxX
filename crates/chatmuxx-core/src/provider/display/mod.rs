mod claude;
mod codex;
mod shell;

use crate::provider::ProviderKind;

const MODEL_FIELD_LABELS: &[&str] = &["model", "模型"];
const STATUS_LABEL: &str = "Status";

pub(crate) fn split_for_chat(text: &str) -> Vec<String> {
    const MAX_CHARS: usize = 1800;
    let text = if text.trim().is_empty() {
        "(empty)"
    } else {
        text
    };
    let mut parts = Vec::new();
    let mut rest = text.trim_matches('\n').trim_end().to_owned();
    while rest.chars().count() > MAX_CHARS {
        let split_at = natural_split_index(&rest, MAX_CHARS);
        let head = rest[..split_at].trim_matches('\n').trim_end().to_owned();
        if !head.is_empty() {
            parts.push(head);
        }
        rest = rest[split_at..]
            .trim_start_matches('\n')
            .trim_start()
            .to_owned();
    }
    if !rest.is_empty() {
        parts.push(rest);
    }
    parts
}

fn natural_split_index(text: &str, max_chars: usize) -> usize {
    let char_indices = text.char_indices().collect::<Vec<_>>();
    if char_indices.len() <= max_chars {
        return text.len();
    }

    let hard = char_indices[max_chars].0;
    let search_start_char = max_chars.saturating_sub(500);
    let search_start = char_indices
        .get(search_start_char)
        .map(|(index, _)| *index)
        .unwrap_or(0);
    let window = &text[search_start..hard];

    if let Some((offset, ch)) = window.char_indices().rev().find(|(_, ch)| *ch == '\n') {
        return search_start + offset + ch.len_utf8();
    }
    if let Some((offset, ch)) = window
        .char_indices()
        .rev()
        .find(|(_, ch)| matches!(ch, '。' | '！' | '？' | '.' | '!' | '?' | ';' | '；'))
    {
        return search_start + offset + ch.len_utf8();
    }
    if let Some((offset, ch)) = window.char_indices().rev().find(|(_, ch)| *ch == ' ') {
        return search_start + offset + ch.len_utf8();
    }

    hard
}

pub(crate) fn trim_for_chat(text: &str) -> String {
    const MAX_LINES: usize = 60;
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(MAX_LINES);
    lines[start..].join("\n")
}

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

fn looks_like_model_context(text: &str) -> bool {
    let text = clean_footer_line(text);
    let lower = text.to_ascii_lowercase();
    !text.is_empty()
        && text.chars().count() <= 120
        && !looks_like_path_only(&text)
        && (lower.contains("model")
            || lower.contains("模型")
            || (text.contains('·') && text.chars().any(|ch| ch.is_ascii_digit())))
}

fn is_terminal_footer_line(line: &str) -> bool {
    let cleaned = clean_footer_line(line);
    if cleaned.is_empty()
        || is_agent_numbered_option_line(&cleaned)
        || looks_like_numbered_option(&cleaned)
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
        || is_mostly_ui_symbols(&cleaned)
}

fn contains_labeled_value(lower: &str) -> bool {
    lower.contains("model") || lower.contains("模型")
}

fn looks_like_path_only(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.chars().any(char::is_whitespace)
        && (text.starts_with('/') || text.starts_with("~/") || text.contains("/"))
}

pub(super) fn is_mostly_ui_symbols(text: &str) -> bool {
    let chars = text.chars().filter(|ch| !ch.is_whitespace()).count();
    if chars == 0 {
        return false;
    }
    let content_chars = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .filter(|ch| ch.is_alphanumeric())
        .count();
    let ui_chars = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .filter(|ch| is_separator_char(*ch) || matches!(ch, '●' | '○' | '◐' | '◑' | '◒' | '◓'))
        .count();
    ui_chars * 3 >= chars * 2 && content_chars * 3 <= chars
}

pub(super) fn normalize_agent_pane_text(
    text: &str,
    clean_content_line: impl Fn(&str) -> String,
    is_provider_noise_line: impl Fn(&str) -> bool,
) -> String {
    let raw_lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let footer_start = raw_lines.len().saturating_sub(8);
    let lines = raw_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let cleaned = clean_content_line(line);
            if index >= footer_start
                && cleaned.trim() == line.trim()
                && is_terminal_footer_line(line)
            {
                None
            } else {
                Some(cleaned)
            }
        })
        .flat_map(|line| {
            line.lines()
                .map(str::trim_end)
                .filter(|line| !line.trim().is_empty())
                .filter(|line| !is_common_agent_noise_line(line))
                .filter(|line| !is_provider_noise_line(line))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    lines.join("\n")
}

fn is_common_agent_noise_line(line: &str) -> bool {
    let trimmed = line.trim();
    if is_agent_numbered_option_line(trimmed) || looks_like_numbered_option(trimmed) {
        return false;
    }

    is_spinner_status_line(trimmed)
        || trimmed.starts_with("› ")
        || trimmed == "›"
        || trimmed.starts_with("❯")
        || is_inline_ui_hint_line(trimmed)
        || trimmed
            .chars()
            .all(|ch| is_separator_char(ch) || ch.is_whitespace())
}

pub(super) fn is_spinner_status_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().next().is_some_and(is_braille_spinner_char)
}

pub(super) fn clean_status_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let without_spinner = trimmed
        .strip_prefix(|ch| is_braille_spinner_char(ch))
        .unwrap_or(trimmed)
        .trim();
    if without_spinner.is_empty() {
        None
    } else {
        Some(without_spinner.to_owned())
    }
}

fn is_braille_spinner_char(ch: char) -> bool {
    matches!(ch, '\u{2800}'..='\u{28ff}')
}

fn is_inline_ui_hint_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let line = line.trim();
    if line.starts_with('?') && lower.contains("shortcut") {
        return true;
    }

    line.chars().count() <= 160
        && (lower.contains("ctrl") || lower.contains("interrupt"))
        && (lower.contains("+") || lower.contains("to ") || lower.contains("interrupt"))
}

pub(super) fn strip_inline_control_hints(text: &str) -> String {
    strip_parenthesized_segments(text, parenthesized_segment_is_ui_chrome)
}

pub(super) fn contains_control_hint(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("ctrl") || lower.contains("esc") || lower.contains("interrupt")
}

pub(super) fn contains_prompt_marker(text: &str) -> bool {
    text.contains(" › ") || text.contains(" ❯ ")
}

pub(super) fn looks_like_ascii_status_label(text: &str) -> bool {
    let label = text
        .split(['(', '·', '›', '❯'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(['•', '-', '*'])
        .trim();

    !label.is_empty()
        && label.chars().count() <= 32
        && label
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch.is_whitespace())
}

fn strip_parenthesized_segments(text: &str, should_strip: impl Fn(&str) -> bool) -> String {
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('(') {
        let Some(end) = rest[start..].find(')') else {
            break;
        };
        let end = start + end + 1;
        let candidate = &rest[start..end];
        if should_strip(candidate) {
            output.push_str(&rest[..start]);
            rest = &rest[end..];
        } else {
            output.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    output.push_str(rest);
    output.trim_end().to_owned()
}

fn parenthesized_segment_is_ui_chrome(text: &str) -> bool {
    contains_control_hint(text)
        && !text
            .trim_matches(['(', ')'])
            .trim()
            .eq_ignore_ascii_case("esc")
}

pub(super) fn clean_selected_option_or_line(line: &str) -> Option<String> {
    let trimmed_end = line.trim_end();
    strip_agent_selection_marker(trimmed_end)
        .filter(|rest| looks_like_numbered_option(rest))
        .map(|rest| rest.trim_end().to_owned())
}

pub(super) fn split_inline_separators(text: &str) -> String {
    let mut output = String::new();
    let mut separator_run = String::new();

    for ch in text.chars() {
        if is_separator_char(ch) {
            separator_run.push(ch);
            continue;
        }

        if separator_run.chars().count() >= 8 && !output.ends_with('\n') {
            output.push('\n');
        } else if !separator_run.is_empty() {
            output.push_str(&separator_run);
        }
        separator_run.clear();
        output.push(ch);
    }

    if separator_run.chars().count() >= 8 && !output.ends_with('\n') {
        output.push('\n');
    } else {
        output.push_str(&separator_run);
    }

    output
}

pub(super) fn strip_standalone_prompt_markers(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !matches!(line.trim(), "›" | "❯"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_agent_numbered_option_line(line: &str) -> bool {
    strip_agent_selection_marker(line).is_some_and(looks_like_numbered_option)
}

fn strip_agent_selection_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix('›')
        .or_else(|| trimmed.strip_prefix('❯'))
        .map(str::trim_start)
}

pub(super) fn looks_like_numbered_option(text: &str) -> bool {
    let mut chars = text.chars();
    let mut digits = 0;
    while matches!(chars.clone().next(), Some(ch) if ch.is_ascii_digit()) {
        digits += 1;
        chars.next();
        if digits > 3 {
            return false;
        }
    }
    if digits == 0 {
        return false;
    }

    if !matches!(chars.next(), Some('.' | ')' | '、' | '．')) {
        return false;
    }

    match chars.next() {
        None => true,
        Some(ch) => ch.is_whitespace(),
    }
}

pub(crate) fn is_separator_char(ch: char) -> bool {
    matches!(
        ch,
        '─' | '━'
            | '-'
            | '—'
            | '═'
            | '│'
            | '┃'
            | '┆'
            | '┊'
            | '║'
            | '╎'
            | '╏'
            | '╭'
            | '╮'
            | '╰'
            | '╯'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_text_is_split_by_char_count() {
        let parts = split_for_chat(&"a".repeat(1801));

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].chars().count(), 1800);
    }

    #[test]
    fn chat_text_split_prefers_sentence_boundary() {
        let text = format!("{}。{}", "a".repeat(1700), "b".repeat(200));
        let parts = split_for_chat(&text);

        assert_eq!(parts.len(), 2);
        assert!(parts[0].ends_with('。'));
        assert!(parts[1].starts_with('b'));
    }

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
