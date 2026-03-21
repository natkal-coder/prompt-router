use crate::cloud::adapter::CloudBackend;
use crate::session::ContextPayload;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::fs;
use std::io::Write;

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

        // Write to temp file (stdin piping doesn't work reliably with claude)
        let temp_path = format!("/tmp/lokahi_claude_{}.txt", uuid::Uuid::new_v4());
        fs::write(&temp_path, &prompt)?;

        let output = Command::new(&self.command)
            .arg("-p")
            .stdin(std::fs::File::open(&temp_path)?)
            .output()
            .await?;

        // Clean up
        let _ = fs::remove_file(&temp_path);

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
        let prompt = self.build_prompt(context);
        let command = self.command.clone();

        // Test if command exists
        match Command::new(&command).arg("-p").arg("").output().await {
            Err(e) => return Err(anyhow::anyhow!("Claude command failed: {}", e)),
            Ok(_) => {} // Command exists
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let temp_path = format!("/tmp/lokahi_claude_{}.txt", uuid::Uuid::new_v4());
            if let Ok(_) = fs::write(&temp_path, &prompt) {
                if let Ok(stdin_file) = std::fs::File::open(&temp_path) {
                    match Command::new(&command)
                        .arg("-p")
                        .stdin(stdin_file)
                        .stdout(Stdio::piped())
                        .spawn()
                    {
                        Ok(mut child) => {
                            if let Some(stdout) = child.stdout.take() {
                                let reader = BufReader::new(stdout);
                                let mut lines = reader.lines();

                                while let Ok(Some(line)) = lines.next_line().await {
                                    let _ = tx.send(line).await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to spawn claude: {}", e);
                        }
                    }
                }
                let _ = fs::remove_file(&temp_path);
            }
        });

        Ok(rx)
    }

    fn name(&self) -> &str {
        "claude"
    }
}
