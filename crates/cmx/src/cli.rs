use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cmx", version, about = "ChatMuxX command line interface")]
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
    /// List known ChatMuxX sessions.
    List,
    /// Close a managed session/window.
    Close { session_id: String },
}
