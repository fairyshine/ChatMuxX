use std::path::PathBuf;

use crate::{
    provider::{LaunchMode, ProviderKind},
    state::sessions::{OwnerId, SessionId, SessionStatus},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSessionRequest {
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub launch_mode: LaunchMode,
    pub extra_args: Vec<String>,
    pub owner: OwnerId,
    pub conversation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseReason {
    UserRequested,
    ProviderReplacement,
    RecoveryReplacement,
    DeadSessionCleanup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionSummary {
    pub id: SessionId,
    pub provider: ProviderKind,
    pub workspace: PathBuf,
    pub status: SessionStatus,
    pub display_name: String,
}
