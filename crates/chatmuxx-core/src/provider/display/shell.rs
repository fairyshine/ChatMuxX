pub(super) fn normalize_pane_text(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !is_noisy_shell_line(line))
        .filter(|line| !is_shell_prompt_or_echo_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_noisy_shell_line(line: &str) -> bool {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();
    is_shell_login_notice(&lower) || is_support_url_notice(&lower)
}

fn is_shell_login_notice(lower: &str) -> bool {
    lower.chars().count() <= 120
        && (lower.contains("default interactive shell")
            || lower.contains("update your account")
            || lower.contains("chsh -s"))
}

fn is_support_url_notice(lower: &str) -> bool {
    lower.starts_with("for more details") && lower.contains("http")
}

fn is_shell_prompt_or_echo_line(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }

    if line.starts_with("cmx> ") || line == "cmx>" {
        return true;
    }

    if starts_with_prompt_marker(line) {
        return true;
    }

    if let Some(index) = prompt_marker_index(line) {
        let prefix = line[..index].trim();
        return !prefix.is_empty()
            && prefix.chars().count() <= 80
            && looks_like_shell_prompt_prefix(prefix);
    }

    let Some(last) = line.chars().last() else {
        return false;
    };
    matches!(last, '$' | '%' | '#')
        && line.chars().count() <= 80
        && looks_like_shell_prompt_prefix(line.trim_end_matches(['$', '%', '#']).trim())
}

fn starts_with_prompt_marker(line: &str) -> bool {
    ["$ ", "% ", "# ", "> "]
        .iter()
        .any(|marker| line.starts_with(marker))
}

fn prompt_marker_index(line: &str) -> Option<usize> {
    ["$ ", "% ", "# ", "> "]
        .iter()
        .filter_map(|marker| line.find(marker))
        .min()
}

fn looks_like_shell_prompt_prefix(prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    prefix.starts_with("bash-")
        || prefix.starts_with("sh-")
        || prefix.contains('@')
        || prefix.contains(':')
        || prefix.contains('~')
        || prefix.contains('/')
        || prefix.ends_with('>')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_normalization_filters_prompts_and_command_echo() {
        let text = normalize_pane_text(
            "The default interactive shell is now zsh.\nTo update your account to use zsh, please run `chsh -s /bin/zsh`.\nFor more details, please visit https://support.apple.com/kb/HT208050.\nbash-3.2$ pwd\n/Users/wumengsong/Code/ChatMuxX\nbash-3.2$",
        );

        assert_eq!(text, "/Users/wumengsong/Code/ChatMuxX");
    }

    #[test]
    fn pane_normalization_handles_zsh_style_prompts() {
        let text = normalize_pane_text(
            "wumengsong@Mac ChatMuxX % ls\nREADME.md\nCargo.toml\nwumengsong@Mac ChatMuxX %",
        );

        assert_eq!(text, "README.md\nCargo.toml");
    }
}
