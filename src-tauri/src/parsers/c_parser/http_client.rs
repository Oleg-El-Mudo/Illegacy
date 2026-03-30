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
        info!("Длина кода для отправки: {} символов", code.len());

        let payload = serde_json::json!({ "code": code });
        info!("Размер JSON payload: {} байт", payload.to_string().len());

        let response = self
            .client
            .post(&endpoint)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                info!("HTTP статус ответа: {}", status);
                
                let response_text = resp.text().await?;
                info!("Тело ответа (первые 500 символов): {}", 
                    response_text.chars().take(500).collect::<String>());
                
                if !status.is_success() {
                    warn!("HTTP ошибка: {}", status);
                    return Err(anyhow!("HTTP ошибка: {}", status));
                }

                let json: Value = serde_json::from_str(&response_text)?;
                
                // Проверяем, есть ли ошибка в ответе
                if let Some(success) = json.get("success") {
                    if !success.as_bool().unwrap_or(false) {
                        let error = json
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Неизвестная ошибка");
                        warn!("Парсер вернул ошибку: {}", error);
                        return Err(anyhow!("Ошибка парсера: {}", error));
                    }
                }
                
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
