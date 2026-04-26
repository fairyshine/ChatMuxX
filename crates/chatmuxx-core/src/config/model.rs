use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub owner: OwnerConfig,
    pub wechat: WeChatConfig,
    pub providers: ProviderConfigs,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DaemonConfig {
    pub tmux_session: String,
    pub poll_interval_ms: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            tmux_session: "chatmuxx".to_owned(),
            poll_interval_ms: 1500,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OwnerConfig {
    pub wechat_user_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatConfig {
    pub enabled: bool,
}

impl Default for WeChatConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfigs {
    pub codex: ProviderConfig,
    pub claude: ProviderConfig,
    pub shell: ProviderConfig,
}

impl Default for ProviderConfigs {
    fn default() -> Self {
        Self {
            codex: ProviderConfig::new("codex"),
            claude: ProviderConfig::new("claude"),
            shell: ProviderConfig::new("bash"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl ProviderConfig {
    fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_v0_1_conventions() {
        let config = Config::default();

        assert_eq!(config.daemon.tmux_session, "chatmuxx");
        assert_eq!(config.daemon.poll_interval_ms, 1500);
        assert!(config.wechat.enabled);
        assert_eq!(config.providers.codex.command, "codex");
        assert_eq!(config.providers.claude.command, "claude");
        assert_eq!(config.providers.shell.command, "bash");
    }

    #[test]
    fn default_config_serializes_to_expected_sections() {
        let text = toml::to_string_pretty(&Config::default()).expect("serialize config");

        assert!(text.contains("[daemon]"));
        assert!(text.contains("[owner]"));
        assert!(text.contains("[wechat]"));
        assert!(text.contains("[providers.codex]"));
        assert!(text.contains("[providers.claude]"));
        assert!(text.contains("[providers.shell]"));
    }
}
