use std::process::Stdio;

use tokio::process::Command;

use crate::{ChatMuxXError, Result};

use super::{
    model::{CreateTmuxWindow, TmuxKey, TmuxWindow},
    parser::{parse_windows, LIST_WINDOWS_FORMAT},
};

const MANAGED_WINDOW_WIDTH: &str = "240";
const MANAGED_WINDOW_HEIGHT: &str = "80";
const MANAGED_HISTORY_LIMIT: &str = "10000";
const CAPTURE_SCROLLBACK_LINES: &str = "-5000";
const LAUNCH_AFTER_RESIZE_DELAY_SECONDS: &str = "0.10";
const MANAGED_PLACEHOLDER_COMMAND: &str = "while :; do sleep 3600; done";

#[derive(Clone, Debug, Default)]
pub struct TmuxClient;

impl TmuxClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn ensure_managed_session(&self, name: &str) -> Result<()> {
        if self.has_session(name).await? {
            self.configure_managed_session(name).await?;
            return Ok(());
        }

        self.run([
            "new-session",
            "-d",
            "-s",
            name,
            "-x",
            MANAGED_WINDOW_WIDTH,
            "-y",
            MANAGED_WINDOW_HEIGHT,
            "-n",
            "__main__",
            "sh",
            "-lc",
            MANAGED_PLACEHOLDER_COMMAND,
        ])
        .await?;
        self.configure_managed_session(name).await?;
        Ok(())
    }

    pub async fn has_session(&self, name: &str) -> Result<bool> {
        let output = Command::new("tmux")
            .args(["has-session", "-t", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match output {
            Ok(status) => Ok(status.success()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Err(ChatMuxXError::CommandNotFound("tmux".to_owned()))
            }
            Err(err) => Err(ChatMuxXError::Io {
                path: "tmux".into(),
                source: err,
            }),
        }
    }

    pub async fn has_pane(&self, pane_id: &str) -> Result<bool> {
        let output = Command::new("tmux")
            .args(["display-message", "-p", "-t", pane_id, "#{pane_id}"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match output {
            Ok(status) => Ok(status.success()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Err(ChatMuxXError::CommandNotFound("tmux".to_owned()))
            }
            Err(err) => Err(ChatMuxXError::Io {
                path: "tmux".into(),
                source: err,
            }),
        }
    }

    pub async fn list_windows(&self, session_name: &str) -> Result<Vec<TmuxWindow>> {
        let output = self
            .run([
                "list-windows",
                "-t",
                session_name,
                "-F",
                LIST_WINDOWS_FORMAT,
            ])
            .await?;
        parse_windows(&output)
    }

    pub async fn create_window(&self, req: CreateTmuxWindow) -> Result<TmuxWindow> {
        let target = req.session_name.clone();
        let mut args = vec![
            "new-window".to_owned(),
            "-P".to_owned(),
            "-F".to_owned(),
            LIST_WINDOWS_FORMAT.to_owned(),
            "-t".to_owned(),
            target,
            "-n".to_owned(),
            req.window_name,
            "-c".to_owned(),
            req.cwd.display().to_string(),
        ];

        let command = shell_join(&req.command);
        let env_prefix = req
            .env
            .iter()
            .map(|(key, value)| format!("{}={}", shell_escape(key), shell_escape(value)))
            .collect::<Vec<_>>()
            .join(" ");
        let full_command = wrap_launch_command(&command, &env_prefix);
        args.push(full_command);

        let output = self.run_owned(args).await?;
        let window = parse_windows(&output)?
            .into_iter()
            .next()
            .ok_or_else(|| ChatMuxXError::TmuxParse("new-window returned no rows".to_owned()))?;
        self.resize_window(&window.window_id).await?;
        Ok(window)
    }

    pub async fn send_text(&self, pane_id: &str, text: &str) -> Result<()> {
        self.run(["send-keys", "-t", pane_id, "-l", text]).await?;
        Ok(())
    }

    pub async fn send_key(&self, pane_id: &str, key: TmuxKey) -> Result<()> {
        self.run(["send-keys", "-t", pane_id, key.as_tmux_key()])
            .await?;
        Ok(())
    }

    pub async fn capture_pane(&self, pane_id: &str) -> Result<String> {
        self.resize_window(pane_id).await?;
        self.run([
            "capture-pane",
            "-p",
            "-J",
            "-S",
            CAPTURE_SCROLLBACK_LINES,
            "-t",
            pane_id,
        ])
        .await
    }

    pub async fn close_window(&self, window_id: &str) -> Result<()> {
        self.run(["kill-window", "-t", window_id]).await?;
        Ok(())
    }

    async fn run<const N: usize>(&self, args: [&str; N]) -> Result<String> {
        self.run_owned(args.into_iter().map(str::to_owned).collect())
            .await
    }

    async fn run_owned(&self, args: Vec<String>) -> Result<String> {
        let output = Command::new("tmux")
            .args(&args)
            .output()
            .await
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    ChatMuxXError::CommandNotFound("tmux".to_owned())
                } else {
                    ChatMuxXError::Io {
                        path: "tmux".into(),
                        source: err,
                    }
                }
            })?;

        if !output.status.success() {
            return Err(ChatMuxXError::TmuxCommandFailed {
                command: format!("tmux {}", args.join(" ")),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    }

    async fn configure_managed_session(&self, name: &str) -> Result<()> {
        self.run([
            "set-option",
            "-t",
            name,
            "history-limit",
            MANAGED_HISTORY_LIMIT,
        ])
        .await?;
        self.run(["set-window-option", "-t", name, "window-size", "manual"])
            .await?;
        for window in self.list_windows(name).await? {
            self.resize_window(&window.window_id).await?;
        }
        Ok(())
    }

    async fn resize_window(&self, window_id: &str) -> Result<()> {
        self.run([
            "resize-window",
            "-t",
            window_id,
            "-x",
            MANAGED_WINDOW_WIDTH,
            "-y",
            MANAGED_WINDOW_HEIGHT,
        ])
        .await?;
        Ok(())
    }
}

fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| shell_escape(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap_launch_command(command: &str, env_prefix: &str) -> String {
    let exec_command = if env_prefix.is_empty() {
        format!("exec {command}")
    } else {
        format!("{env_prefix} exec {command}")
    };
    format!("sleep {LAUNCH_AFTER_RESIZE_DELAY_SECONDS}; {exec_command}")
}

fn shell_escape(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_leaves_simple_values_alone() {
        assert_eq!(shell_escape("codex"), "codex");
        assert_eq!(shell_escape("/tmp/project"), "/tmp/project");
    }

    #[test]
    fn shell_escape_quotes_spaces() {
        assert_eq!(shell_escape("/tmp/my project"), "'/tmp/my project'");
    }

    #[test]
    fn shell_join_escapes_each_part() {
        let parts = vec!["echo".to_owned(), "hello world".to_owned()];

        assert_eq!(shell_join(&parts), "echo 'hello world'");
    }

    #[test]
    fn launch_command_waits_for_managed_resize() {
        assert_eq!(wrap_launch_command("codex", ""), "sleep 0.10; exec codex");
    }

    #[test]
    fn launch_command_preserves_environment_assignments() {
        assert_eq!(
            wrap_launch_command("codex", "FOO=bar BAZ=qux"),
            "sleep 0.10; FOO=bar BAZ=qux exec codex"
        );
    }

    #[test]
    fn managed_placeholder_command_is_portable_shell() {
        assert_eq!(
            MANAGED_PLACEHOLDER_COMMAND,
            "while :; do sleep 3600; done"
        );
    }

    #[tokio::test]
    async fn tmux_integration_create_capture_and_close() {
        if std::env::var("CHATMUXX_TEST_TMUX").ok().as_deref() != Some("1") {
            return;
        }

        let client = TmuxClient::new();
        let session = format!("chatmuxx-test-{}", std::process::id());
        let _ = client.run(["kill-session", "-t", &session]).await;
        client.ensure_managed_session(&session).await.expect("ensure session");
        assert!(client.has_session(&session).await.expect("has session"));

        let window = client
            .create_window(CreateTmuxWindow {
                session_name: session.clone(),
                window_name: "test".to_owned(),
                cwd: std::env::temp_dir(),
                command: vec!["sh".to_owned()],
                env: Default::default(),
            })
            .await
            .expect("create window");

        client
            .send_text(&window.pane_id, "printf hello")
            .await
            .expect("send text");
        client
            .send_key(&window.pane_id, TmuxKey::Enter)
            .await
            .expect("enter");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let pane = client
            .capture_pane(&window.pane_id)
            .await
            .expect("capture pane");
        assert!(pane.contains("hello"));

        client
            .close_window(&window.window_id)
            .await
            .expect("close window");
        let _ = client.run(["kill-session", "-t", &session]).await;
    }
}
