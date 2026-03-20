use anyhow::Result;
use reqwest::Client;

#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    client: Client,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
            model: model.to_string(),
        }
    }

    pub async fn is_healthy(&self) -> bool {
        match self
            .client
            .get(&format!("{}/api/version", self.base_url))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn pull_model(&self) -> Result<()> {
        let body = serde_json::json!({
            "name": self.model,
        });

        let resp = self
            .client
            .post(&format!("{}/api/pull", self.base_url))
            .json(&body)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to pull model: {}", resp.status()))
        }
    }

    pub async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.2,
                "num_predict": max_tokens,
            }
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        let text = resp.bytes().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_slice(&text).unwrap_or_default();
        let response = json["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(response)
    }

    pub async fn generate_streaming(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();
        let prompt_copy = prompt.to_string();

        tokio::spawn(async move {
            let body = serde_json::json!({
                "model": model,
                "prompt": prompt_copy,
                "stream": false,
                "options": {
                    "temperature": 0.2,
                    "num_predict": max_tokens,
                }
            });

            if let Ok(resp) = client
                .post(format!("{}/api/generate", base_url))
                .json(&body)
                .send()
                .await
            {
                if let Ok(bytes) = resp.bytes().await {
                    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if let Some(response_text) = json["response"].as_str() {
                            let _ = tx.send(response_text.to_string()).await;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}
