use super::{
    clean_selected_option_or_line, clean_status_line, contains_control_hint,
    contains_prompt_marker, is_spinner_status_line, looks_like_ascii_status_label,
    looks_like_numbered_option, normalize_agent_pane_text, split_inline_separators,
    strip_inline_control_hints, strip_standalone_prompt_markers,
};

pub(super) fn normalize_pane_text(text: &str) -> String {
    normalize_agent_pane_text(text, clean_content_line, is_codex_noise_line)
}

pub(super) fn extract_status(text: &str) -> Option<String> {
    text.lines()
        .rev()
        .find(|line| is_spinner_status_line(line))
        .and_then(clean_status_line)
}

fn is_codex_noise_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('⚠')
}

fn clean_content_line(line: &str) -> String {
    if let Some(option) = clean_selected_option_or_line(line) {
        return option;
    }

    let text = split_inline_separators(line.trim_end());
    let text = strip_inline_codex_ui(&text);
    strip_standalone_prompt_markers(&text)
}

fn strip_inline_codex_ui(text: &str) -> String {
    let mut output = strip_inline_warning_tail(text);
    output = strip_inline_activity_tail(&output);
    output = strip_inline_codex_prompt_tail(&output);
    strip_inline_control_hints(&output)
}

fn strip_inline_warning_tail(text: &str) -> String {
    match text.find(" ⚠") {
        Some(start) => text[..start].trim_end().to_owned(),
        None => text.to_owned(),
    }
}

fn strip_inline_activity_tail(text: &str) -> String {
    if text.trim_start().starts_with('•') && is_activity_status_tail(text) {
        return String::new();
    }

    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(" • ") {
        let start = search_from + relative_start;
        let tail = &text[start..];
        if contains_control_hint(tail) || contains_prompt_marker(tail) {
            return text[..start].trim_end().to_owned();
        }
        search_from = start + " • ".len();
    }
    text.to_owned()
}

fn is_activity_status_tail(text: &str) -> bool {
    looks_like_ascii_status_label(text) && contains_control_hint(text) && has_status_metadata(text)
}

fn has_status_metadata(text: &str) -> bool {
    text.contains('(') || text.contains('·')
}

fn strip_inline_codex_prompt_tail(text: &str) -> String {
    let Some((start, tail)) = find_prompt_tail(text) else {
        return text.to_owned();
    };
    if looks_like_numbered_option(tail) {
        return text.to_owned();
    }
    text[..start].trim_end().to_owned()
}

fn find_prompt_tail(text: &str) -> Option<(usize, &str)> {
    let marker = " › ";
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(marker) {
        let start = search_from + relative_start;
        let line_start = text[..start].rfind('\n').map_or(0, |index| index + 1);
        let before_marker = text[line_start..start].trim();
        let tail = text[start + marker.len()..].trim_start();
        if before_marker.is_empty()
            || before_marker.starts_with('•')
            || before_marker.starts_with('└')
        {
            return Some((start, tail));
        }
        search_from = start + marker.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_normalization_filters_spinner_status() {
        let text = normalize_pane_text("hello\n⠋ thinking\nworld");

        assert_eq!(text, "hello\nworld");
    }

    #[test]
    fn pane_normalization_filters_prompt_echo_and_ui_hints() {
        let text = normalize_pane_text(
            "answer\n› user prompt\n• Working (1s • esc to interrupt)\n› Use /skills to list available skills",
        );

        assert_eq!(text, "answer");
    }

    #[test]
    fn pane_normalization_keeps_selected_numbered_option() {
        let text = normalize_pane_text(
            "Would you like to make the following edits?\n› 1. Yes, proceed (y)\n  2. Yes, and don't ask again for these files (a)\n  3. No, and tell Codex what to do differently (esc)\nPress enter to confirm or esc to cancel",
        );

        assert_eq!(
            text,
            "Would you like to make the following edits?\n1. Yes, proceed (y)\n  2. Yes, and don't ask again for these files (a)\n  3. No, and tell Codex what to do differently (esc)\nPress enter to confirm or esc to cancel"
        );
    }

    #[test]
    fn pane_normalization_keeps_inline_selected_numbered_option() {
        let text = normalize_pane_text(
            "cat > file <<'EOF'\nbody\nEOF › 1. Yes, proceed (y) 2. No, and tell Codex what to do differently (esc)",
        );

        assert_eq!(
            text,
            "cat > file <<'EOF'\nbody\nEOF › 1. Yes, proceed (y) 2. No, and tell Codex what to do differently (esc)"
        );
    }

    #[test]
    fn pane_normalization_keeps_body_before_inline_status() {
        let text = normalize_pane_text(
            "• 找到了 ChatMuxX；我会检查它的插件结构和规则，再放入同一个可选插件。 • Working (1m 57s • esc to interrupt) › Improve documentation in @filename",
        );

        assert_eq!(
            text,
            "• 找到了 ChatMuxX；我会检查它的插件结构和规则，再放入同一个可选插件。"
        );
    }

    #[test]
    fn pane_normalization_removes_inline_prompt_tail() {
        let text = normalize_pane_text(
            "• 已按你的要求收敛，只保留 VS Code Port Forwarding / devtunnels.ms 方案。 已更新 › Improve documentation in @filename",
        );

        assert_eq!(
            text,
            "• 已按你的要求收敛，只保留 VS Code Port Forwarding / devtunnels.ms 方案。 已更新"
        );
    }

    #[test]
    fn pane_normalization_filters_repeated_environment_warnings() {
        let text = normalize_pane_text(
            "answer\n⚠ Skipped loading 1 skill(s) due to invalid SKILL.md files.\n⚠ /Users/me/SKILL.md: missing YAML frontmatter delimited by ---\n⚠ Model metadata for `gpt-5.5` not found. Defaulting to fallback metadata; this can degrade performance and cause issues.",
        );

        assert_eq!(text, "answer");
    }

    #[test]
    fn pane_normalization_strips_transcript_hints_without_dropping_output() {
        let text = normalize_pane_text(
            "• Ran find . -maxdepth 3 -print\n└ ./README.md ./package.json (ctrl + t to view transcript)",
        );

        assert_eq!(
            text,
            "• Ran find . -maxdepth 3 -print\n└ ./README.md ./package.json"
        );
    }

    #[test]
    fn status_is_extracted_from_spinner_line() {
        let status = extract_status("hello\n⠋ thinking");

        assert_eq!(status, Some("thinking".to_owned()));
    }

    #[test]
    fn pane_normalization_filters_bottom_model_line() {
        let text = normalize_pane_text("answer\nmodel: gpt-5.1-codex");

        assert_eq!(text, "answer");
    }
}
