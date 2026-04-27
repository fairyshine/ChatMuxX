use super::{
    clean_footer_line, clean_selected_option_or_line, contains_prompt_marker, is_mostly_ui_symbols,
    looks_like_ascii_status_label, normalize_agent_pane_text, split_inline_separators,
    strip_standalone_prompt_markers,
};

pub(super) fn normalize_pane_text(text: &str) -> String {
    normalize_agent_pane_text(text, clean_content_line, is_claude_noise_line)
}

pub(super) fn extract_status(text: &str) -> Option<String> {
    text.lines().rev().find_map(extract_status_from_line)
}

fn clean_content_line(line: &str) -> String {
    if let Some(option) = clean_selected_option_or_line(line) {
        return option;
    }

    let text = split_inline_separators(line.trim_end());
    let text = strip_inline_tips(&text);
    let text = strip_inline_status(&text);
    strip_standalone_prompt_markers(&text)
}

fn extract_status_from_line(line: &str) -> Option<String> {
    let line = line.trim();
    let start = line
        .char_indices()
        .find_map(|(index, ch)| is_symbol_status_char(ch).then_some(index))?;
    let status = &line[start..];
    if !looks_like_status_tail(&status[status.chars().next()?.len_utf8()..]) {
        return None;
    }
    let end = next_claude_content_marker(&status[status.chars().next()?.len_utf8()..])
        .map(|index| status.chars().next().unwrap().len_utf8() + index)
        .unwrap_or(status.len());
    let cleaned = status[..end]
        .trim()
        .trim_end_matches(['─', '━', ' '])
        .trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_owned())
    }
}

fn strip_inline_tips(text: &str) -> String {
    let mut output = text.to_owned();
    while let Some(start) = find_tip_marker(&output) {
        let tail = &output[start..];
        let end = next_claude_content_marker(&tail['⎿'.len_utf8()..])
            .map(|index| '⎿'.len_utf8() + index)
            .unwrap_or(tail.len());
        output.replace_range(start..start + end, "");
    }
    output
}

fn find_tip_marker(text: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find('⎿') {
        let start = search_from + relative_start;
        if marker_tail_has_label(&text[start + '⎿'.len_utf8()..]) {
            return Some(start);
        }
        search_from = start + '⎿'.len_utf8();
    }
    None
}

fn marker_tail_has_label(text: &str) -> bool {
    let head = text.trim_start();
    let label = head
        .split([':', '：'])
        .next()
        .unwrap_or_default()
        .trim();
    !label.is_empty()
        && label.chars().count() <= 32
        && label
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch.is_whitespace())
        && (head.contains(':') || head.contains('：'))
}

fn strip_inline_status(text: &str) -> String {
    let mut output = text.to_owned();
    while let Some((start, marker)) = find_marker_char(&output, &is_symbol_status_char) {
        let tail = &output[start + marker.len_utf8()..];
        if !looks_like_status_tail(tail) {
            break;
        }
        match next_claude_content_marker(tail) {
            Some(end) => output.replace_range(start..start + marker.len_utf8() + end, ""),
            None => {
                output.truncate(start);
                break;
            }
        }
    }
    output
}

fn find_marker_char(text: &str, is_marker: &impl Fn(char) -> bool) -> Option<(usize, char)> {
    text.char_indices()
        .find_map(|(index, ch)| is_marker(ch).then_some((index, ch)))
}

fn next_claude_content_marker(text: &str) -> Option<usize> {
    text.char_indices()
        .find(|(_, ch)| matches!(*ch, '⏺' | '•' | '└'))
        .map(|(index, _)| index)
        .or_else(|| text.find('\n'))
}

fn looks_like_status_tail(text: &str) -> bool {
    let head = text.split('\n').next().unwrap_or_default();
    head.chars().count() <= 180 || next_claude_content_marker(head).is_some()
}

fn is_symbol_status_char(ch: char) -> bool {
    matches!(ch, '✢' | '✳' | '✶' | '✻' | '✽' | '✼' | '✺')
}

fn is_claude_noise_line(line: &str) -> bool {
    let cleaned = clean_footer_line(line);
    let lower = cleaned.to_ascii_lowercase();
    looks_like_box_chrome(&cleaned)
        || looks_like_version_title_line(&cleaned)
        || looks_like_ascii_status_label(&cleaned) && contains_prompt_marker(&cleaned)
        || is_mostly_ui_symbols(&cleaned)
        || looks_like_path_only(&cleaned)
        || lower.starts_with("esc ") && lower.contains("cancel")
}

fn looks_like_version_title_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower
        .split_whitespace()
        .any(|token| token.starts_with('v') && token.chars().skip(1).next().is_some_and(|ch| ch.is_ascii_digit()))
        && line.chars().count() <= 120
        && line.chars().filter(|ch| ch.is_ascii_alphabetic()).count() >= 12
}

fn looks_like_box_chrome(line: &str) -> bool {
    line.contains('│') || line.contains('╭') || line.contains('╰') || line.contains('╮') || line.contains('╯')
}

fn looks_like_path_only(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && !text.chars().any(char::is_whitespace) && (text.starts_with("~/") || text.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_normalization_filters_welcome_screen() {
        let text = normalize_pane_text(
            "╭─── Claude Code v2.1.119 ─────────────────────────╮\n│            Welcome back!           │ Tips for getting started\n│               ▐▛███▜▌              │ Run /init to create a CLAUDE.md file\n│   Sonnet 4.6 · API Usage Billing   │ Recent activity\n│          ~/Code/ChatMuxX           │ No recent activity\n╰──────────────────────────────────────────────────╯\n❯\n  ? for shortcuts                             ● high · /effort",
        );

        assert_eq!(text, "");
    }

    #[test]
    fn pane_normalization_keeps_help_body_without_modal_chrome() {
        let text = normalize_pane_text(
            "❯ /help\n────────────────────────\nClaude Code v2.1.119 general commands custom-commands\nClaude understands your codebase, makes edits with your permission.\nShortcuts\n! for bash mode\nEsc to cancel",
        );

        assert_eq!(
            text,
            "Claude understands your codebase, makes edits with your permission.\nShortcuts\n! for bash mode"
        );
    }

    #[test]
    fn pane_normalization_removes_inline_ui_noise() {
        let text = normalize_pane_text(
            "你想从哪个方向开始优化？ ✻ Baked for 44s ───────────────────────────────────────────────────────── ❯ ───────────────────────────────────────────────────────── ? for shortcuts",
        );

        assert_eq!(text, "你想从哪个方向开始优化？");
    }

    #[test]
    fn pane_normalization_removes_symbol_status_without_matching_english_text() {
        let text = normalize_pane_text(
            "已经完成第一步。 ✳ 正在整理结果 12s ───────────── ❯ ? for shortcuts",
        );

        assert_eq!(text, "已经完成第一步。");
    }

    #[test]
    fn pane_normalization_removes_inline_tip_but_keeps_answer() {
        let text = normalize_pane_text(
            "✽ Hyperspacing… ⎿ Tip: Name your conversations with /rename to find them easily in /resume later ⏺ 让我先把所有文件都读一遍再做分析。",
        );

        assert_eq!(text, "⏺ 让我先把所有文件都读一遍再做分析。");
    }

    #[test]
    fn pane_normalization_keeps_non_tip_result_lines() {
        let text = normalize_pane_text(
            "⏺ Searching for 2 patterns, reading 2 files…\n⎿ README.md\n⎿ plugins/mobile-web-debugging/SKILL.md\n⎿ Tip: Use /statusline to customize the footer",
        );

        assert_eq!(
            text,
            "⏺ Searching for 2 patterns, reading 2 files…\n⎿ README.md\n⎿ plugins/mobile-web-debugging/SKILL.md"
        );
    }

    #[test]
    fn pane_normalization_keeps_selected_numbered_option() {
        let text = normalize_pane_text(
            "Choose a mode:\n❯ 1. Accept once\n  2. Always accept\n  3. Cancel\n? for shortcuts",
        );

        assert_eq!(
            text,
            "Choose a mode:\n1. Accept once\n  2. Always accept\n  3. Cancel"
        );
    }
}
