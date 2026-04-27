use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cmux", version, about = "ChatMuxX command line interface")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the ChatMuxX daemon in the foreground.
    Daemon(DaemonArgs),
    /// Login to a chat channel.
    Login(LoginArgs),
    /// Validate local configuration and dependencies.
    Doctor,
    /// Manage local configuration.
    Config(ConfigArgs),
    /// Inspect and manage ChatMuxX sessions.
    Sessions(SessionsArgs),
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    /// Optional config file path.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[command(subcommand)]
    pub channel: LoginChannel,
}

#[derive(Debug, Subcommand)]
pub enum LoginChannel {
    /// Login to WeChat through the iLink HTTP API.
    Wechat,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Create an initial config.toml.
    Init(ConfigInitArgs),
}

#[derive(Debug, Args)]
pub struct ConfigInitArgs {
    /// Optional config path. Defaults to ~/.chatmuxx/config.toml.
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SessionsArgs {
    #[command(subcommand)]
    pub command: SessionsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommand {
    /// Create a managed tmux window for local development/testing.
    New(SessionsNewArgs),
    /// List known ChatMuxX sessions.
    List,
    /// Close a managed session/window.
    Close { session_id: String },
    /// Rename a session id.
    Rename { session_id: String, new_id: String },
    /// Delete Dead and Closed session records from local state.
    Prune,
    /// Send literal text to a session pane.
    Send(SessionsSendArgs),
    /// Capture the current text from a session pane.
    Capture { session_id: String },
}

#[derive(Debug, Args)]
pub struct SessionsNewArgs {
    /// Optional human-friendly session id, for example `main`.
    #[arg(long)]
    pub id: Option<String>,
    /// Workspace directory where the provider should start.
    pub workspace: PathBuf,
    /// Provider to start: shell, codex, or claude. Only shell is wired first.
    #[arg(default_value = "shell")]
    pub provider: String,
    /// Extra provider arguments after `--`.
    #[arg(last = true)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Args)]
pub struct SessionsSendArgs {
    /// ChatMuxX session id.
    pub session_id: String,
    /// Literal text to send.
    pub text: String,
    /// Press Enter after sending the literal text.
    #[arg(long)]
    pub enter: bool,
}
