use crate::cloud::adapter::CloudBackend;
use crate::session::ContextPayload;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[derive(Clone)]
pub struct ClaudeBackend {
    command: String,
}

impl ClaudeBackend {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
        }
    }

    fn build_prompt(&self, context: &ContextPayload) -> String {
        format!(
            "CONTEXT (Tier 1 - System):\n{}\n\n\
             CONTEXT (Tier 2 - History):\n{}\n\n\
             CONTEXT (Tier 3 - Recent):\n{}\n\n\
             CONTEXT (Tier 4 - Code):\n{}\n\n\
             USER REQUEST:\n{}",
            context.tier1_system,
            context.tier2_summaries,
            context.tier3_recent_turns,
            context.tier4_code_context,
            context.tier5_prompt,
        )
    }
}

#[async_trait]
impl CloudBackend for ClaudeBackend {
    async fn send(&self, context: &ContextPayload) -> Result<String> {
        let prompt = self.build_prompt(context);

        let mut child = Command::new(&self.command)
            .arg("--print")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Claude command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    async fn send_streaming(
        &self,
        context: &ContextPayload,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let prompt = self.build_prompt(context);
        let command = self.command.clone();

        tokio::spawn(async move {
            match Command::new(&command)
                .arg("--print")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(prompt.as_bytes()).await;
                    }

                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = tx.send(line).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to spawn claude command: {}", e);
                }
            }
        });

        Ok(rx)
    }

    fn name(&self) -> &str {
        "claude"
    }
}
