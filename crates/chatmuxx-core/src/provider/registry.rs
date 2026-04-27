use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    config::{ProviderConfig, ProviderConfigs},
    provider::{
        model::{
            OutputSource, ProviderCapabilities, ProviderKind, ProviderLaunchCommand,
            ProviderLaunchRequest,
        },
        ClaudeProvider, CodexProvider, ShellProvider,
    },
    ChatMuxXError, Result,
};

pub trait ProviderAdapter: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn capabilities(&self) -> ProviderCapabilities;
    fn launch_command(&self, req: &ProviderLaunchRequest) -> Result<ProviderLaunchCommand>;

    fn output_source(&self) -> OutputSource {
        OutputSource::PaneOnly
    }
}

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: BTreeMap<ProviderKind, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new(providers: impl IntoIterator<Item = Arc<dyn ProviderAdapter>>) -> Self {
        Self {
            providers: providers
                .into_iter()
                .map(|provider| (provider.kind(), provider))
                .collect(),
        }
    }

    pub fn from_configs(configs: &ProviderConfigs) -> Self {
        Self::new([
            Arc::new(CodexProvider::new(configs.codex.clone())) as Arc<dyn ProviderAdapter>,
            Arc::new(ClaudeProvider::new(configs.claude.clone())) as Arc<dyn ProviderAdapter>,
            Arc::new(ShellProvider::new(configs.shell.clone())) as Arc<dyn ProviderAdapter>,
        ])
    }

    pub fn get(&self, kind: ProviderKind) -> Result<Arc<dyn ProviderAdapter>> {
        self.providers
            .get(&kind)
            .cloned()
            .ok_or(ChatMuxXError::UnknownProvider(kind))
    }

    pub fn list(&self) -> Vec<ProviderKind> {
        self.providers.keys().copied().collect()
    }
}

pub(crate) fn merge_config_args(
    config: &ProviderConfig,
    extra_args: &[String],
) -> (String, Vec<String>, BTreeMap<String, String>) {
    let mut args = config.args.clone();
    args.extend(extra_args.iter().cloned());
    (config.command.clone(), args, config.env.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfigs;

    #[test]
    fn registry_contains_builtin_providers_from_default_config() {
        let registry = ProviderRegistry::from_configs(&ProviderConfigs::default());

        assert_eq!(
            registry.list(),
            vec![
                ProviderKind::Codex,
                ProviderKind::Claude,
                ProviderKind::Shell
            ]
        );
        assert_eq!(
            registry.get(ProviderKind::Claude).unwrap().kind(),
            ProviderKind::Claude
        );
        assert_eq!(
            registry.get(ProviderKind::Shell).unwrap().kind(),
            ProviderKind::Shell
        );
        assert_eq!(
            registry.get(ProviderKind::Codex).unwrap().kind(),
            ProviderKind::Codex
        );
    }

    #[test]
    fn registry_rejects_unregistered_provider() {
        let registry = ProviderRegistry::new([]);
        let Err(err) = registry.get(ProviderKind::Codex) else {
            panic!("empty registry should reject codex");
        };

        assert!(err.to_string().contains("unknown provider"));
    }
}
