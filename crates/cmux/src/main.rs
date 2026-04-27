mod cli;

use clap::Parser;
use cli::{Cli, Command, ConfigCommand, LoginChannel, SessionsCommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;

    let cli = Cli::parse();
    match cli.command {
        Command::Daemon(args) => chatmuxx_core::app::daemon::run(args.config).await?,
        Command::Login(args) => match args.channel {
            LoginChannel::Wechat => chatmuxx_core::app::login::wechat().await?,
        },
        Command::Doctor => chatmuxx_core::app::doctor::run().await?,
        Command::Config(args) => match args.command {
            ConfigCommand::Init(init) => chatmuxx_core::app::config_init::run(init.path).await?,
        },
        Command::Sessions(args) => match args.command {
            SessionsCommand::New(new) => {
                chatmuxx_core::app::sessions::new(
                    new.id,
                    new.workspace,
                    new.provider,
                    new.extra_args,
                )
                .await?
            }
            SessionsCommand::List => chatmuxx_core::app::sessions::list().await?,
            SessionsCommand::Close { session_id } => {
                chatmuxx_core::app::sessions::close(session_id).await?
            }
            SessionsCommand::Rename { session_id, new_id } => {
                chatmuxx_core::app::sessions::rename(session_id, new_id).await?
            }
            SessionsCommand::Prune => chatmuxx_core::app::sessions::prune().await?,
            SessionsCommand::Send(send) => {
                chatmuxx_core::app::sessions::send(send.session_id, send.text, send.enter).await?
            }
            SessionsCommand::Capture { session_id } => {
                chatmuxx_core::app::sessions::capture(session_id).await?
            }
        },
    }

    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialize tracing subscriber: {err}"))?;

    Ok(())
}
