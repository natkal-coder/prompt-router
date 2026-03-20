use crate::session::ContextPayload;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CloudBackend: Send + Sync {
    async fn send(&self, context: &ContextPayload) -> Result<String>;
    async fn send_streaming(
        &self,
        context: &ContextPayload,
    ) -> Result<tokio::sync::mpsc::Receiver<String>>;
    fn name(&self) -> &str;
}
