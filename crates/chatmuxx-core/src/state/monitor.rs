use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::provider::{OutputSourceKind, ProviderKind};

use super::SessionId;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MonitorState {
    pub schema_version: u32,
    pub sessions: BTreeMap<SessionId, MonitorSessionState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MonitorSessionState {
    pub provider: ProviderKind,
    pub source: Option<MonitorSourceState>,
    pub last_pane_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MonitorSourceState {
    pub kind: OutputSourceKind,
    pub path: Option<PathBuf>,
    pub offset: Option<u64>,
    pub last_seen_id: Option<String>,
}
