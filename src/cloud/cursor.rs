use crate::cloud::adapter::CloudBackend;
use crate::session::ContextPayload;
use anyhow::Result;
use async_trait::async_trait;

pub struct CursorBackend {
    command: String,
}

impl CursorBackend {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
        }
    }
}

#[async_trait]
impl CloudBackend for CursorBackend {
    async fn send(&self, _context: &ContextPayload) -> Result<String> {
        // TODO: Implement Cursor Agent integration in Phase 2
        Err(anyhow::anyhow!("Cursor Agent integration not yet implemented"))
    }

    async fn send_streaming(
        &self,
        _context: &ContextPayload,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        // TODO: Implement streaming for Cursor Agent in Phase 2
        Err(anyhow::anyhow!("Cursor Agent streaming not yet implemented"))
    }

    fn name(&self) -> &str {
        "cursor"
    }
}
