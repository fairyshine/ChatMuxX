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
}
