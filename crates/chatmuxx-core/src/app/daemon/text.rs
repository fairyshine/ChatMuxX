use crate::{provider::display::is_separator_char, state::monitor::MonitorSessionState};

pub(super) const DISPLAY_FLUSH_INTERVAL_MS: u64 = 3_000;
const COMPACT_DEDUPE_MIN_CHARS: usize = 30;

pub(super) fn is_redundant_outbound(candidate: &str, recent: &[String]) -> bool {
    let candidate = normalize_for_history_dedupe(candidate);
    if candidate.is_empty() {
        return true;
    }
    let candidate_compact = compact_for_history_dedupe(&candidate);

    recent.iter().any(|previous| {
        let previous = normalize_for_history_dedupe(previous);
        if previous == candidate
            || previous.contains(&candidate)
            || (candidate.contains(&previous) && previous.chars().count() > 80)
        {
            return true;
        }

        let previous_compact = compact_for_history_dedupe(&previous);
        candidate_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
            && (previous_compact == candidate_compact
                || previous_compact.contains(&candidate_compact)
                || (candidate_compact.contains(&previous_compact)
                    && previous_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS))
    })
}

pub(super) fn suppress_recent_outbound_text(candidate: &str, recent: &[String]) -> Option<String> {
    let mut text = candidate.trim_matches('\n').trim_end().to_owned();
    for previous in recent {
        let next = remove_recent_overlap(&text, previous);
        if normalize_for_history_dedupe(&next).is_empty() {
            return None;
        }
        text = next;
    }

    Some(text)
}

fn remove_recent_overlap(candidate: &str, previous: &str) -> String {
    let candidate_lines = normalized_history_lines(candidate);
    let previous_lines = normalized_history_lines(previous);
    if candidate_lines.is_empty() || previous_lines.is_empty() {
        return candidate.trim_matches('\n').trim_end().to_owned();
    }

    let candidate_compact = compact_lines(&candidate_lines);
    let previous_compact = compact_lines(&previous_lines);
    if candidate_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
        && previous_compact.contains(&candidate_compact)
    {
        return String::new();
    }

    if contains_line_window(&previous_lines, &candidate_lines) {
        return String::new();
    }

    if let Some(start) = find_line_window(&candidate_lines, &previous_lines) {
        let mut kept = Vec::new();
        kept.extend(candidate_lines[..start].iter().cloned());
        kept.extend(
            candidate_lines[start + previous_lines.len()..]
                .iter()
                .cloned(),
        );
        return kept.join("\n");
    }

    let max_overlap = candidate_lines.len().min(previous_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == candidate_lines[..overlap] {
            return candidate_lines[overlap..].join("\n");
        }
    }

    for prefix_len in (1..candidate_lines.len()).rev() {
        if contains_line_window(&previous_lines, &candidate_lines[..prefix_len]) {
            return candidate_lines[prefix_len..].join("\n");
        }

        let prefix_compact = compact_lines(&candidate_lines[..prefix_len]);
        if prefix_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
            && previous_compact.contains(&prefix_compact)
        {
            return candidate_lines[prefix_len..].join("\n");
        }
    }

    candidate.trim_matches('\n').trim_end().to_owned()
}

fn normalized_history_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn contains_line_window(haystack: &[String], needle: &[String]) -> bool {
    find_line_window(haystack, needle).is_some()
}

fn find_line_window(haystack: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(super) fn merge_pending_text(previous: Option<String>, delta: &str) -> Option<String> {
    let delta = delta.trim_matches('\n').trim_end();
    if delta.trim().is_empty() {
        return previous;
    }

    let Some(previous) = previous.filter(|text| !text.trim().is_empty()) else {
        return Some(delta.to_owned());
    };

    Some(merge_overlapping_text(&previous, delta))
}

fn merge_overlapping_text(previous: &str, next: &str) -> String {
    let previous = previous.trim_matches('\n').trim_end();
    let next = next.trim_matches('\n').trim_end();

    if previous.is_empty() {
        return next.to_owned();
    }
    if next.is_empty() || previous == next || previous.contains(next) {
        return previous.to_owned();
    }
    if next.contains(previous) {
        return next.to_owned();
    }

    let previous_lines = previous.lines().collect::<Vec<_>>();
    let next_lines = next.lines().collect::<Vec<_>>();
    let max_overlap = previous_lines.len().min(next_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == next_lines[..overlap] {
            let suffix = next_lines[overlap..].join("\n");
            if suffix.trim().is_empty() {
                return previous.to_owned();
            }
            return format!("{previous}\n{suffix}");
        }
    }

    let previous_chars = previous.chars().collect::<Vec<_>>();
    let next_chars = next.chars().collect::<Vec<_>>();
    let max_char_overlap = previous_chars.len().min(next_chars.len()).min(500);
    for overlap in (20..=max_char_overlap).rev() {
        if previous_chars[previous_chars.len() - overlap..] == next_chars[..overlap] {
            let suffix = next_chars[overlap..].iter().collect::<String>();
            if suffix.trim().is_empty() {
                return previous.to_owned();
            }
            return format!("{previous}{suffix}");
        }
    }

    format!("{previous}\n{next}")
}

pub(super) fn should_flush_pending(state: &MonitorSessionState, now_ms: u64) -> bool {
    if state
        .pending_display_text
        .as_deref()
        .is_none_or(|text| text.trim().is_empty())
    {
        return false;
    }

    state.pending_since_ms.is_some_and(|pending_since| {
        now_ms.saturating_sub(pending_since) >= DISPLAY_FLUSH_INTERVAL_MS
    })
}

fn normalize_for_history_dedupe(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_for_history_dedupe(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_whitespace() && !is_separator_char(*ch))
        .collect()
}

fn compact_lines(lines: &[String]) -> String {
    compact_for_history_dedupe(&lines.join("\n"))
}

pub(super) fn text_delta(
    previous: Option<&str>,
    current: &str,
    normalize: impl Fn(&str) -> String,
) -> Option<String> {
    let current = normalize(current);
    if current.trim().is_empty() {
        return None;
    }

    let Some(previous) = previous else {
        return Some(current);
    };
    let previous = normalize(previous);

    if current == previous {
        return None;
    }
    let current_compact = compact_for_history_dedupe(&current);
    let previous_compact = compact_for_history_dedupe(&previous);
    if current_compact.chars().count() > COMPACT_DEDUPE_MIN_CHARS
        && (current_compact == previous_compact || previous_compact.contains(&current_compact))
    {
        return None;
    }
    if let Some(delta) = current.strip_prefix(&previous) {
        return non_empty_delta(delta);
    }

    let previous_lines = previous.lines().collect::<Vec<_>>();
    let current_lines = current.lines().collect::<Vec<_>>();
    let max_overlap = previous_lines.len().min(current_lines.len());
    for overlap in (1..=max_overlap).rev() {
        if previous_lines[previous_lines.len() - overlap..] == current_lines[..overlap] {
            return non_empty_delta(&current_lines[overlap..].join("\n"));
        }
    }

    if let Some(prefix_len) =
        longest_current_prefix_seen_in_previous(&previous_lines, &current_lines)
    {
        return non_empty_delta(&current_lines[prefix_len..].join("\n"));
    }

    Some(current)
}

fn longest_current_prefix_seen_in_previous(
    previous_lines: &[&str],
    current_lines: &[&str],
) -> Option<usize> {
    let max_len = previous_lines.len().min(current_lines.len());
    for len in (1..=max_len).rev() {
        let prefix = &current_lines[..len];
        let prefix_compact = compact_for_history_dedupe(&prefix.join("\n"));
        if len < 3 && prefix_compact.chars().count() < COMPACT_DEDUPE_MIN_CHARS {
            continue;
        }
        if previous_lines.windows(len).any(|window| window == prefix) {
            return Some(len);
        }
    }

    None
}

fn non_empty_delta(delta: &str) -> Option<String> {
    let delta = delta.trim_matches('\n').trim_end();
    if delta.trim().is_empty() {
        None
    } else {
        Some(delta.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{provider::ProviderKind, state::monitor::MonitorSessionState};

    fn pane_delta(previous: Option<&str>, current: &str, provider: ProviderKind) -> Option<String> {
        text_delta(previous, current, |text| {
            crate::provider::display::normalize_pane_text(text, provider)
        })
    }

    #[test]
    fn pane_delta_sends_only_appended_text() {
        let delta = pane_delta(
            Some("first\nsecond"),
            "first\nsecond\nthird",
            ProviderKind::Codex,
        );

        assert_eq!(delta, Some("third".to_owned()));
    }

    #[test]
    fn pane_delta_handles_scrolled_overlap() {
        let delta = pane_delta(Some("a\nb\nc"), "b\nc\nd", ProviderKind::Codex);

        assert_eq!(delta, Some("d".to_owned()));
    }

    #[test]
    fn pane_delta_handles_current_tail_embedded_in_previous() {
        let previous = [
            "startup banner",
            "old warning",
            "• 明白，这个仓库 /Users/wumengsong/Code/cmux_test 是专门用来测试 cmux 的。",
            "  你接下来想让我在这里做什么？",
        ]
        .join("\n");
        let current = [
            "old warning",
            "• 明白，这个仓库 /Users/wumengsong/Code/cmux_test 是专门用来测试 cmux 的。",
            "  你接下来想让我在这里做什么？",
            "• 我来快速列一下当前目录内容。",
            "• 当前目录 /Users/wumengsong/Code/cmux_test 里没有项目文件，只有：",
            "  - . 当前目录",
            "  - .. 上级目录",
        ]
        .join("\n");

        let delta = pane_delta(Some(&previous), &current, ProviderKind::Codex);

        assert_eq!(
            delta,
            Some(
                [
                    "• 我来快速列一下当前目录内容。",
                    "• 当前目录 /Users/wumengsong/Code/cmux_test 里没有项目文件，只有：",
                    "  - . 当前目录",
                    "  - .. 上级目录",
                ]
                .join("\n")
            )
        );
    }

    #[test]
    fn pane_delta_suppresses_repeated_text() {
        let delta = pane_delta(Some("same"), "same", ProviderKind::Codex);

        assert_eq!(delta, None);
    }

    #[test]
    fn pane_delta_suppresses_terminal_reflow_only_changes() {
        let delta = pane_delta(
            Some("README.zh-CN.md\n列出会话、创建本地 shell 测试会话"),
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话",
            ProviderKind::Codex,
        );

        assert_eq!(delta, None);
    }

    #[test]
    fn pending_text_waits_for_flush_interval() {
        let state = MonitorSessionState {
            provider: ProviderKind::Codex,
            source: None,
            last_pane_hash: None,
            last_pane_text: None,
            last_status_text: None,
            last_footer_text: None,
            pending_display_text: Some("hello".to_owned()),
            pending_footer_text: None,
            pending_since_ms: Some(1_000),
            last_delivery_at_ms: None,
        };

        assert!(!should_flush_pending(&state, 3_999));
        assert!(should_flush_pending(&state, 4_000));
    }

    #[test]
    fn pending_text_appends_with_newline() {
        let pending = merge_pending_text(Some("one".to_owned()), "two");

        assert_eq!(pending, Some("one\ntwo".to_owned()));
    }

    #[test]
    fn pending_text_drops_contained_duplicate() {
        let pending = merge_pending_text(Some("alpha\nbeta\ngamma".to_owned()), "beta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn pending_text_merges_line_overlap() {
        let pending = merge_pending_text(Some("alpha\nbeta".to_owned()), "beta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn pending_text_replaces_when_next_contains_previous() {
        let pending = merge_pending_text(Some("beta".to_owned()), "alpha\nbeta\ngamma");

        assert_eq!(pending, Some("alpha\nbeta\ngamma".to_owned()));
    }

    #[test]
    fn redundant_outbound_detects_exact_recent_message() {
        assert!(is_redundant_outbound(
            "hello\nworld",
            &[" hello \n\n world ".to_owned()]
        ));
    }

    #[test]
    fn redundant_outbound_detects_candidate_inside_recent_message() {
        assert!(is_redundant_outbound(
            "world",
            &["hello\nworld\nagain".to_owned()]
        ));
    }

    #[test]
    fn redundant_outbound_allows_new_text() {
        assert!(!is_redundant_outbound(
            "new output",
            &["old output".to_owned()]
        ));
    }

    #[test]
    fn recent_outbound_suppression_keeps_only_suffix_after_previous_message() {
        let text = suppress_recent_outbound_text("alpha\nbeta\ngamma", &["alpha\nbeta".to_owned()]);

        assert_eq!(text, Some("gamma".to_owned()));
    }

    #[test]
    fn recent_outbound_suppression_drops_fully_repeated_message() {
        let text = suppress_recent_outbound_text("alpha\nbeta", &["alpha\nbeta\ngamma".to_owned()]);

        assert_eq!(text, None);
    }

    #[test]
    fn recent_outbound_suppression_removes_scrolled_overlap() {
        let text =
            suppress_recent_outbound_text("beta\ngamma\ndelta", &["alpha\nbeta\ngamma".to_owned()]);

        assert_eq!(text, Some("delta".to_owned()));
    }

    #[test]
    fn recent_outbound_suppression_drops_reflowed_repeat() {
        let text = suppress_recent_outbound_text(
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build",
            &[
                "README.zh-CN.md\n列出会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build"
                    .to_owned(),
            ],
        );

        assert_eq!(text, None);
    }

    #[test]
    fn recent_outbound_suppression_keeps_suffix_after_reflowed_prefix() {
        let text = suppress_recent_outbound_text(
            "README.zh-C N.md\n列出 会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build\n新增内容",
            &[
                "README.zh-CN.md\n列出会话、创建本地 shell 测试会话\n这样新用户不只知道怎么 build"
                    .to_owned(),
            ],
        );

        assert_eq!(text, Some("新增内容".to_owned()));
    }
}
