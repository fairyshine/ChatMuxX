pub mod claude;
pub mod codex;
pub(crate) mod display;
pub mod model;
pub mod registry;
pub mod shell;

pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use model::{
    LaunchMode, OutputSource, OutputSourceKind, ProviderCapabilities, ProviderCursor,
    ProviderEvent, ProviderKind, ProviderLaunchCommand, ProviderLaunchRequest, ProviderReadResult,
    ProviderStatus,
};
pub use registry::{ProviderAdapter, ProviderRegistry};
pub use shell::ShellProvider;
