use std::process::Stdio;

use tokio::process::Command;

use crate::{ChatMuxXError, Result};

use super::{
    model::{CreateTmuxWindow, TmuxKey, TmuxWindow},
    parser::{parse_windows, LIST_WINDOWS_FORMAT},
};

#[derive(Clone, Debug, Default)]
pub struct TmuxClient;

impl TmuxClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn ensure_managed_session(&self, name: &str) -> Result<()> {
        if self.has_session(name).await? {
            return Ok(());
        }

        self.run([
            "new-session",
            "-d",
            "-s",
            name,
            "-n",
            "__main__",
            "sh",
            "-lc",
            "sleep infinity",
        ])
        .await?;
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
        let full_command = if env_prefix.is_empty() {
            command
        } else {
            format!("{env_prefix} {command}")
        };
        args.push(full_command);

        let output = self.run_owned(args).await?;
        parse_windows(&output)?
            .into_iter()
            .next()
            .ok_or_else(|| ChatMuxXError::TmuxParse("new-window returned no rows".to_owned()))
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
        self.run(["capture-pane", "-p", "-t", pane_id]).await
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
}

fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| shell_escape(part))
        .collect::<Vec<_>>()
        .join(" ")
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

    #[tokio::test]
    async fn tmux_integration_create_capture_and_close() {
        if std::env::var("CHATMUXX_TEST_TMUX").ok().as_deref() != Some("1") {
            return;
        }

        let client = TmuxClient::new();
        let session = format!("chatmuxx-test-{}", std::process::id());
        client
            .ensure_managed_session(&session)
            .await
            .expect("ensure session");

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
