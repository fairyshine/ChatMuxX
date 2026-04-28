use super::footer::is_terminal_footer_line;

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

pub(crate) fn normalize_agent_pane_text(
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

pub(super) fn is_common_agent_noise_line(line: &str) -> bool {
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
        .strip_prefix(is_braille_spinner_char)
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

pub(super) fn is_inline_ui_hint_line(line: &str) -> bool {
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

pub(super) fn strip_parenthesized_segments(
    text: &str,
    should_strip: impl Fn(&str) -> bool,
) -> String {
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

pub(super) fn parenthesized_segment_is_ui_chrome(text: &str) -> bool {
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

pub(super) fn is_agent_numbered_option_line(line: &str) -> bool {
    strip_agent_selection_marker(line).is_some_and(looks_like_numbered_option)
}

pub(super) fn strip_agent_selection_marker(line: &str) -> Option<&str> {
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
