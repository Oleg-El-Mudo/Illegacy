use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// HTTP клиент для взаимодействия с парсером C
pub struct ParserHttpClient {
    client: Client,
}

impl ParserHttpClient {
    /// Создает новый HTTP клиент с таймаутом
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .unwrap(),
        }
    }

    /// Создает клиента с таймаутом по умолчанию (30 секунд)
    pub fn default() -> Self {
        Self::new(30)
    }

    /// Пытается получить ответ от HTTP API парсера
    pub async fn parse(&self, base_url: &str, code: &str) -> Result<String> {
        let endpoint = format!("{}/parse", base_url);
        info!("Попытка подключения к HTTP API парсера на {}", endpoint);

        let response = self
            .client
            .post(&endpoint)
            .json(&serde_json::json!({ "code": code }))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    warn!("HTTP ошибка: {}", resp.status());
                    return Err(anyhow!("HTTP ошибка: {}", resp.status()));
                }

                let json: Value = resp.json().await?;
                info!("HTTP API успешно ответил");
                Ok(json.to_string())
            }
            Err(e) => {
                debug!("HTTP API не доступен: {}", e);
                Err(anyhow!("HTTP API не доступен: {}", e))
            }
        }
    }
}

impl Default for ParserHttpClient {
    fn default() -> Self {
        Self::default()
    }
}
