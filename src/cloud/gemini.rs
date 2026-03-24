use crate::cloud::adapter::CloudBackend;
use crate::session::ContextPayload;
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone)]
pub struct GeminiBackend {
    command: String,
}

impl GeminiBackend {
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
impl CloudBackend for GeminiBackend {
    async fn send(&self, context: &ContextPayload) -> Result<String> {
        let prompt = self.build_prompt(context);

        let output = Command::new(&self.command)
            .arg("--prompt")
            .arg(&prompt)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Gemini command failed: {}",
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

        // Test spawn to check if command exists
        match Command::new(&command).arg("--help").stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            Err(e) => return Err(anyhow::anyhow!("Gemini command failed: {}", e)),
            Ok(mut child) => {
                let _ = child.wait().await;
            }
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            match Command::new(&command)
                .arg("--prompt")
                .arg(&prompt)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();

                        while let Ok(Some(line)) = lines.next_line().await {
                            if let Err(_) = tx.send(line).await {
                                break; // Receiver dropped, stop reading
                            }
                        }
                    }

                    // Wait for child with timeout
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(20),
                        child.wait()
                    ).await {
                        Ok(Ok(status)) if !status.success() => {
                            tracing::error!("Gemini process failed with status: {}", status);
                        }
                        Ok(Err(e)) => {
                            tracing::error!("Failed to wait for gemini process: {}", e);
                        }
                        Err(_) => {
                            tracing::error!("Gemini process timed out after 20s");
                            let _ = child.kill().await;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to spawn gemini command: {}", e);
                }
            }
        });

        Ok(rx)
    }

    fn name(&self) -> &str {
        "gemini"
    }
}
