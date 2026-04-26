use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct CreateTmuxWindow {
    pub session_name: String,
    pub window_name: String,
    pub cwd: PathBuf,
    pub command: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TmuxWindow {
    pub session_name: String,
    pub window_id: String,
    pub pane_id: String,
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TmuxKey {
    Enter,
    Escape,
    CtrlC,
    Tab,
}

impl TmuxKey {
    pub(crate) fn as_tmux_key(self) -> &'static str {
        match self {
            Self::Enter => "Enter",
            Self::Escape => "Escape",
            Self::CtrlC => "C-c",
            Self::Tab => "Tab",
        }
    }
}
