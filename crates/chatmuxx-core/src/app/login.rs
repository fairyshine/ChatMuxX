use std::time::{SystemTime, UNIX_EPOCH};

use qrcode::{render::unicode, QrCode};

use crate::{
    channel::wechat::{QrLoginStatus, WeChatClient},
    config::{default_config_path, load_config, Config},
    state::{
        accounts::{AccountState, WeChatAccountRecord},
        atomic::{load_json_or_default, save_json},
        files::{ensure_state_dir, StatePaths},
        sessions::{AppState, OwnerId, OwnerState},
    },
    ChatMuxXError, Result,
};

pub async fn wechat() -> Result<()> {
    let paths = StatePaths::from_default_root()?;
    ensure_state_dir(&paths.root).await?;
    let config = load_config_or_default().await?;
    let client = WeChatClient::new(&config.wechat.base_url);

    println!("Starting WeChat login through iLink...");
    let start = client.get_bot_qrcode(&config.wechat.bot_type).await?;
    println!("Scan this QR code with WeChat:");
    render_qr(&start.url);
    println!("QR URL:");
    println!("{}", start.url);
    println!("Waiting for confirmation...");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(ChatMuxXError::WeChatProtocol(
                "QR login timed out after 5 minutes".to_owned(),
            ));
        }

        match client.poll_qrcode_status(&start.qrcode).await? {
            QrLoginStatus::Waiting => {
                print!(".");
                sleep_one_second().await;
            }
            QrLoginStatus::Scanned => {
                println!("\nScanned. Confirm login in WeChat...");
                sleep_one_second().await;
            }
            QrLoginStatus::Expired => {
                return Err(ChatMuxXError::WeChatProtocol(
                    "QR code expired; run `cmux login wechat` again".to_owned(),
                ));
            }
            QrLoginStatus::Cancelled => {
                return Err(ChatMuxXError::WeChatProtocol(
                    "QR login was cancelled".to_owned(),
                ));
            }
            QrLoginStatus::Confirmed(credentials) => {
                persist_login(&paths, credentials, &config).await?;
                println!("\nWeChat login saved.");
                return Ok(());
            }
        }
    }
}

async fn persist_login(
    paths: &StatePaths,
    credentials: crate::channel::wechat::WeChatLoginCredentials,
    config: &Config,
) -> Result<()> {
    let mut accounts: AccountState = load_json_or_default(&paths.accounts).await?;
    accounts.schema_version = 1;
    let now = now_string();

    if let Some(existing) = accounts
        .wechat
        .iter_mut()
        .find(|account| account.account_id == credentials.account_id)
    {
        existing.bot_token = credentials.bot_token;
        existing.bot_user_id = credentials.user_id.clone();
        existing.base_url = credentials.base_url;
        existing.updated_at = now.clone();
    } else {
        accounts.wechat.push(WeChatAccountRecord {
            account_id: credentials.account_id.clone(),
            bot_user_id: credentials.user_id.clone(),
            bot_token: credentials.bot_token,
            base_url: credentials.base_url,
            get_updates_buf: None,
            context_tokens: Default::default(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }
    save_json(&paths.accounts, &accounts).await?;

    let mut state: AppState = load_json_or_default(&paths.state).await?;
    state.schema_version = 1;
    if state.owner.is_none() {
        let wechat_user_id = config.owner.wechat_user_id.clone().or(credentials.user_id);
        state.owner = Some(OwnerState {
            id: OwnerId("owner-local".to_owned()),
            wechat_user_id,
        });
        save_json(&paths.state, &state).await?;
    }

    Ok(())
}

async fn load_config_or_default() -> Result<Config> {
    let path = default_config_path()?;
    match load_config(&path).await {
        Ok(config) => Ok(config),
        Err(ChatMuxXError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Config::default())
        }
        Err(err) => Err(err),
    }
}

async fn sleep_one_second() {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn render_qr(text: &str) {
    match QrCode::new(text.as_bytes()) {
        Ok(code) => {
            let rendered = code.render::<unicode::Dense1x2>().quiet_zone(false).build();
            println!("{rendered}");
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to render QR code");
        }
    }
}
