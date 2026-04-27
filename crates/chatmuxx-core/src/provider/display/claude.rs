use super::{
    clean_footer_line, clean_selected_option_or_line, normalize_agent_pane_text,
    split_inline_separators, strip_standalone_prompt_markers,
};

const STATUS_SYMBOLS: &[char] = &['✢', '✳', '✶', '✻', '✽', '✼', '✺'];
const TIP_MARKER: &str = "⎿ Tip:";
const CONTENT_RESUME_MARKERS: &[&str] = &["⏺", "• ", "Read ", "Reading ", "Searched ", "\n"];
const STARTUP_NOISE_SUBSTRINGS: &[&str] = &[
    "claude code",
    "tips for getting started",
    "welcome back",
    "run /init to create",
    "recent activity",
    "no recent activity",
    "api usage billing",
    "/effort",
];
const STARTUP_NOISE_EXACT: &[&str] = &["esc to cancel"];

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
        .find_map(|(index, ch)| is_status_symbol(ch).then_some(index))?;
    let status = &line[start..];
    if !looks_like_status_fragment(status, line.len().saturating_sub(start)) {
        return None;
    }
    let end = next_meaningful_agent_marker(&status[status.chars().next()?.len_utf8()..])
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
    while let Some(start) = output.find(TIP_MARKER) {
        let after_start = start + TIP_MARKER.len();
        let tail = &output[after_start..];
        let next = next_meaningful_agent_marker(tail).map(|index| after_start + index);
        match next {
            Some(end) => output.replace_range(start..end, ""),
            None => {
                output.truncate(start);
                break;
            }
        }
    }
    output
}

fn strip_inline_status(text: &str) -> String {
    let mut output = text.to_owned();
    for &symbol in STATUS_SYMBOLS {
        while let Some(index) = output.find(symbol) {
            let tail = &output[index..];
            if !looks_like_status_fragment(tail, output.len().saturating_sub(index)) {
                break;
            }
            if let Some(next) = next_meaningful_agent_marker(&tail[symbol.len_utf8()..]) {
                let end = index + symbol.len_utf8() + next;
                output.replace_range(index..end, "");
            } else {
                output.truncate(index);
                break;
            }
        }
    }
    output
}

fn next_meaningful_agent_marker(text: &str) -> Option<usize> {
    CONTENT_RESUME_MARKERS
        .iter()
        .filter_map(|marker| text.find(marker))
        .min()
}

fn is_status_symbol(ch: char) -> bool {
    STATUS_SYMBOLS.contains(&ch)
}

fn looks_like_status_fragment(text: &str, chars_after_symbol: usize) -> bool {
    text.chars().next().is_some_and(is_status_symbol)
        && (chars_after_symbol <= 160
            || next_meaningful_agent_marker(text)
                .is_some_and(|index| text[..index].chars().count() <= 160))
}

fn is_claude_noise_line(line: &str) -> bool {
    let cleaned = clean_footer_line(line);
    let lower = cleaned.to_ascii_lowercase();
    STARTUP_NOISE_SUBSTRINGS
        .iter()
        .any(|fragment| lower.contains(fragment))
        || STARTUP_NOISE_EXACT.iter().any(|value| lower == *value)
        || (cleaned.starts_with("~/") && !cleaned.chars().any(char::is_whitespace))
        || cleaned
            .chars()
            .any(|ch| matches!(ch, '▐' | '▛' | '█' | '▜' | '▌' | '▝' | '▘' | '▗'))
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
