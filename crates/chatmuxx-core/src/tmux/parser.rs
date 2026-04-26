use std::path::PathBuf;

use crate::{ChatMuxXError, Result};

use super::model::TmuxWindow;

const SEP: char = '\t';

pub(crate) const LIST_WINDOWS_FORMAT: &str =
    "#{session_name}\t#{window_id}\t#{pane_id}\t#{window_name}\t#{pane_current_path}\t#{window_active}";

pub(crate) fn parse_windows(output: &str) -> Result<Vec<TmuxWindow>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_window_line)
        .collect()
}

fn parse_window_line(line: &str) -> Result<TmuxWindow> {
    let fields = line.split(SEP).collect::<Vec<_>>();
    if fields.len() != 6 {
        return Err(ChatMuxXError::TmuxParse(format!(
            "expected 6 fields, got {} in {line:?}",
            fields.len()
        )));
    }

    Ok(TmuxWindow {
        session_name: fields[0].to_owned(),
        window_id: fields[1].to_owned(),
        pane_id: fields[2].to_owned(),
        name: fields[3].to_owned(),
        cwd: (!fields[4].is_empty()).then(|| PathBuf::from(fields[4])),
        active: fields[5] == "1",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_windows_reads_tmux_rows() {
        let output = "chatmuxx\t@1\t%2\tapi\t/tmp/project\t1\n";

        let windows = parse_windows(output).expect("parse windows");

        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].session_name, "chatmuxx");
        assert_eq!(windows[0].window_id, "@1");
        assert_eq!(windows[0].pane_id, "%2");
        assert_eq!(windows[0].name, "api");
        assert_eq!(windows[0].cwd, Some(PathBuf::from("/tmp/project")));
        assert!(windows[0].active);
    }

    #[test]
    fn parse_windows_rejects_bad_rows() {
        let err = parse_windows("too\tfew").expect_err("bad row");

        assert!(matches!(err, ChatMuxXError::TmuxParse(_)));
    }
}
