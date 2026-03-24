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

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error ({}): {}", status, text));
        }

        let text = resp.bytes().await?;
        let json: serde_json::Value = serde_json::from_slice(&text)?;
        let response = json["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response field in Ollama reply"))?
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

            match client
                .post(format!("{}/api/generate", base_url))
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!("Ollama API error: {}", resp.status());
                        if let Ok(text) = resp.text().await {
                            eprintln!("  Response: {}", text);
                        }
                        return;
                    }

                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if let Some(response_text) = json["response"].as_str() {
                                let _ = tx.send(response_text.to_string()).await;
                            } else {
                                eprintln!("No response field in Ollama reply");
                            }
                        } else {
                            eprintln!("Failed to parse Ollama JSON response");
                        }
                    } else {
                        eprintln!("Failed to read Ollama response bytes");
                    }
                }
                Err(e) => {
                    eprintln!("Ollama request failed: {}", e);
                }
            }
        });

        Ok(rx)
    }
}
