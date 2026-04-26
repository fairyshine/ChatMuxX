use crate::{
    config::ProviderConfig,
    provider::{
        model::{
            LaunchMode, OutputSource, ProviderCapabilities, ProviderKind, ProviderLaunchCommand,
            ProviderLaunchRequest,
        },
        registry::{merge_config_args, ProviderAdapter},
    },
    ChatMuxXError, Result,
};

#[derive(Clone, Debug)]
pub struct CodexProvider {
    config: ProviderConfig,
}

impl CodexProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ProviderAdapter for CodexProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            structured_output: false,
            resume: false,
            continue_last: false,
            prompt_detection: false,
        }
    }

    fn launch_command(&self, req: &ProviderLaunchRequest) -> Result<ProviderLaunchCommand> {
        if req.launch_mode != LaunchMode::Fresh {
            return Err(ChatMuxXError::UnsupportedLaunchMode {
                provider: ProviderKind::Codex,
                mode: req.launch_mode.as_str(),
            });
        }

        let (program, args, env) = merge_config_args(&self.config, &req.extra_args);
        Ok(ProviderLaunchCommand {
            program,
            args,
            env,
            display_name: "codex".to_owned(),
        })
    }

    fn output_source(&self) -> OutputSource {
        OutputSource::PaneOnly
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn codex_launch_uses_config_and_extra_args() {
        let provider = CodexProvider::new(ProviderConfig {
            command: "codex".to_owned(),
            args: vec!["--ask-for-approval".to_owned(), "never".to_owned()],
            env: Default::default(),
        });

        let command = provider
            .launch_command(&ProviderLaunchRequest {
                workspace: PathBuf::from("/tmp/project"),
                launch_mode: LaunchMode::Fresh,
                extra_args: vec!["--model".to_owned(), "gpt-5.5".to_owned()],
            })
            .expect("launch command");

        assert_eq!(command.program, "codex");
        assert_eq!(
            command.args,
            vec!["--ask-for-approval", "never", "--model", "gpt-5.5"]
        );
    }
}
