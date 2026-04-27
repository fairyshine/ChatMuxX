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
pub struct ClaudeProvider {
    config: ProviderConfig,
}

impl ClaudeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ProviderAdapter for ClaudeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
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
                provider: ProviderKind::Claude,
                mode: req.launch_mode.as_str(),
            });
        }

        let (program, args, env) = merge_config_args(&self.config, &req.extra_args);
        Ok(ProviderLaunchCommand {
            program,
            args,
            env,
            display_name: "claude".to_owned(),
        })
    }

    fn output_source(&self) -> OutputSource {
        OutputSource::PaneOnly
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use super::*;

    fn request(extra_args: Vec<String>) -> ProviderLaunchRequest {
        ProviderLaunchRequest {
            workspace: PathBuf::from("/tmp/project"),
            launch_mode: LaunchMode::Fresh,
            extra_args,
        }
    }

    #[test]
    fn claude_launch_uses_config_and_extra_args() {
        let mut env = BTreeMap::new();
        env.insert(
            "ANTHROPIC_BASE_URL".to_owned(),
            "https://example.test".to_owned(),
        );
        let provider = ClaudeProvider::new(ProviderConfig {
            command: "claude".to_owned(),
            args: vec!["--model".to_owned(), "sonnet".to_owned()],
            env,
        });

        let command = provider
            .launch_command(&request(vec!["--dangerously-skip-permissions".to_owned()]))
            .expect("launch command");

        assert_eq!(command.program, "claude");
        assert_eq!(
            command.args,
            vec!["--model", "sonnet", "--dangerously-skip-permissions"]
        );
        assert_eq!(command.env["ANTHROPIC_BASE_URL"], "https://example.test");
        assert_eq!(command.display_name, "claude");
    }

    #[test]
    fn claude_rejects_resume_modes() {
        let provider = ClaudeProvider::new(ProviderConfig::new_for_test("claude"));
        let err = provider
            .launch_command(&ProviderLaunchRequest {
                workspace: PathBuf::from("/tmp/project"),
                launch_mode: LaunchMode::Resume {
                    provider_session_id: "abc".to_owned(),
                },
                extra_args: Vec::new(),
            })
            .unwrap_err();

        assert!(err.to_string().contains("unsupported launch mode"));
    }
}
