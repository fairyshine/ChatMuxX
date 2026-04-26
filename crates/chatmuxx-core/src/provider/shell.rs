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
pub struct ShellProvider {
    config: ProviderConfig,
}

impl ShellProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ProviderAdapter for ShellProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Shell
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
                provider: ProviderKind::Shell,
                mode: req.launch_mode.as_str(),
            });
        }

        let (program, args, env) = merge_config_args(&self.config, &req.extra_args);
        Ok(ProviderLaunchCommand {
            program,
            args,
            env,
            display_name: "shell".to_owned(),
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
    fn shell_launch_uses_configured_command_and_appends_extra_args() {
        let mut env = BTreeMap::new();
        env.insert("CHATMUXX_TEST".to_owned(), "1".to_owned());
        let provider = ShellProvider::new(ProviderConfig {
            command: "zsh".to_owned(),
            args: vec!["-l".to_owned()],
            env,
        });

        let command = provider
            .launch_command(&request(vec!["-i".to_owned()]))
            .expect("launch command");

        assert_eq!(command.program, "zsh");
        assert_eq!(command.args, vec!["-l", "-i"]);
        assert_eq!(command.env["CHATMUXX_TEST"], "1");
        assert_eq!(command.display_name, "shell");
    }

    #[test]
    fn shell_rejects_resume_modes() {
        let provider = ShellProvider::new(ProviderConfig::new_for_test("bash"));
        let err = provider
            .launch_command(&ProviderLaunchRequest {
                workspace: PathBuf::from("/tmp/project"),
                launch_mode: LaunchMode::Continue,
                extra_args: Vec::new(),
            })
            .unwrap_err();

        assert!(err.to_string().contains("unsupported launch mode"));
    }
}
