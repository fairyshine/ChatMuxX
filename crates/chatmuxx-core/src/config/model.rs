use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub owner: OwnerConfig,
    #[serde(default)]
    pub wechat: WeChatConfig,
    #[serde(default)]
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
    #[serde(default)]
    pub wechat_user_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeChatConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_wechat_base_url")]
    pub base_url: String,
    #[serde(default = "default_wechat_bot_type")]
    pub bot_type: String,
    #[serde(default = "default_wechat_long_poll_timeout_ms")]
    pub long_poll_timeout_ms: u64,
}

impl Default for WeChatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: default_wechat_base_url(),
            bot_type: default_wechat_bot_type(),
            long_poll_timeout_ms: default_wechat_long_poll_timeout_ms(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_wechat_base_url() -> String {
    "https://ilinkai.weixin.qq.com".to_owned()
}

fn default_wechat_bot_type() -> String {
    "3".to_owned()
}

fn default_wechat_long_poll_timeout_ms() -> u64 {
    38_000
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfigs {
    #[serde(default = "default_codex_provider")]
    pub codex: ProviderConfig,
    #[serde(default = "default_claude_provider")]
    pub claude: ProviderConfig,
    #[serde(default = "default_shell_provider")]
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

fn default_codex_provider() -> ProviderConfig {
    ProviderConfig::new("codex")
}

fn default_claude_provider() -> ProviderConfig {
    ProviderConfig::new("claude")
}

fn default_shell_provider() -> ProviderConfig {
    ProviderConfig::new("bash")
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
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

    #[cfg(test)]
    pub(crate) fn new_for_test(command: impl Into<String>) -> Self {
        Self::new(command)
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
