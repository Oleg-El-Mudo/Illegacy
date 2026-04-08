/// Модуль управления Ollama интеграцией
/// Проверка статуса, управление моделями (скачивание, удаление, выбор)

use log::{error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::Emitter;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const OLLAMA_TIMEOUT: u64 = 30;
// Таймаут для скачивания - 1 час (модели могут быть большими)
const OLLAMA_PULL_TIMEOUT: u64 = 3600;

/// Информация о модели Ollama
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

/// Доступная модель для скачивания
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AvailableModel {
    pub name: String,
    pub description: String,
    pub size: String,
    pub category: String,
    pub parameters: Vec<String>, // Доступные варианты параметров (например: "7b", "13b", "34b")
    pub default_parameter: String, // Параметр по умолчанию
}

/// Статус Ollama сервиса
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaStatus {
    pub available: bool,
    pub version: Option<String>,
    pub models: Vec<OllamaModel>,
}

/// Прогресс скачивания модели
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullProgress {
    pub model: String,
    pub status: String,
    pub progress: f64, // 0.0 to 1.0
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub message: String,
}

/// Результат операции с моделью
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelOperationResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

/// Создание HTTP клиента для Ollama
fn create_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(OLLAMA_TIMEOUT))
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))
}

/// Проверка доступности Ollama сервиса
#[tauri::command]
pub async fn check_ollama_status() -> Result<OllamaStatus, String> {
    info!("Проверка статуса Ollama сервиса");

    let client = create_client()?;

    // Проверяем health endpoint
    match client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let models: Vec<OllamaModel> = json
                            .get("models")
                            .and_then(|m| m.as_array())
                            .map(|models_array| {
                                models_array
                                    .iter()
                                    .filter_map(|model| {
                                        Some(OllamaModel {
                                            name: model.get("name")?.as_str()?.to_string(),
                                            size: model.get("size")?.as_u64()?,
                                            digest: model.get("digest")?.as_str()?.to_string(),
                                            modified_at: model
                                                .get("modified_at")?
                                                .as_str()?
                                                .to_string(),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        info!("Ollama сервис доступен, моделей: {}", models.len());

                        Ok(OllamaStatus {
                            available: true,
                            version: Some("latest".to_string()),
                            models,
                        })
                    }
                    Err(e) => {
                        warn!("Ошибка парсинга ответа Ollama: {}", e);
                        Ok(OllamaStatus {
                            available: true,
                            version: None,
                            models: vec![],
                        })
                    }
                }
            } else {
                warn!("Ollama вернул статус: {}", response.status());
                Ok(OllamaStatus {
                    available: false,
                    version: None,
                    models: vec![],
                })
            }
        }
        Err(e) => {
            warn!("Ollama сервис не доступен: {}", e);
            Ok(OllamaStatus {
                available: false,
                version: None,
                models: vec![],
            })
        }
    }
}

/// Получить список популярных моделей для скачивания
#[tauri::command]
pub async fn get_available_models() -> Result<Vec<AvailableModel>, String> {
    info!("Получение списка доступных моделей");

    // Список code-oriented моделей для легаси-кода
    let models = vec![
        AvailableModel {
            name: "codellama".to_string(),
            description: "Code Llama - специализированная модель Meta для генерации и понимания кода".to_string(),
            size: "~3.8-19 GB".to_string(),
            category: "Code".to_string(),
            parameters: vec!["7b".to_string(), "13b".to_string(), "34b".to_string(), "70b".to_string()],
            default_parameter: "7b".to_string(),
        },
        AvailableModel {
            name: "deepseek-coder".to_string(),
            description: "DeepSeek Coder - модель с превосходной производительностью для кода".to_string(),
            size: "~1.2-32 GB".to_string(),
            category: "Code".to_string(),
            parameters: vec!["1.3b".to_string(), "6.7b".to_string(), "33b".to_string()],
            default_parameter: "6.7b".to_string(),
        },
        AvailableModel {
            name: "qwen2.5-coder".to_string(),
            description: "Qwen Coder - современная модель для работы с кодом от Alibaba".to_string(),
            size: "~1.8-14 GB".to_string(),
            category: "Code".to_string(),
            parameters: vec!["0.5b".to_string(), "1.5b".to_string(), "3b".to_string(), "7b".to_string(), "14b".to_string()],
            default_parameter: "3b".to_string(),
        },
        AvailableModel {
            name: "starcoder2".to_string(),
            description: "StarCoder2 - модель для генерации кода от BigCode".to_string(),
            size: "~3-14 GB".to_string(),
            category: "Code".to_string(),
            parameters: vec!["3b".to_string(), "7b".to_string(), "15b".to_string()],
            default_parameter: "7b".to_string(),
        },
        AvailableModel {
            name: "phi3".to_string(),
            description: "Phi-3 - компактная, но мощная модель от Microsoft для кода".to_string(),
            size: "~1.5-7 GB".to_string(),
            category: "Code".to_string(),
            parameters: vec!["mini".to_string(), "medium".to_string(), "small".to_string()],
            default_parameter: "mini".to_string(),
        },
        AvailableModel {
            name: "llama3.1".to_string(),
            description: "Llama 3.1 - универсальная модель с хорошими способностями к коду".to_string(),
            size: "~2-70 GB".to_string(),
            category: "Code/General".to_string(),
            parameters: vec!["8b".to_string(), "70b".to_string()],
            default_parameter: "8b".to_string(),
        },
        AvailableModel {
            name: "mistral".to_string(),
            description: "Mistral - эффективная модель общего назначения с поддержкой кода".to_string(),
            size: "~4.1 GB".to_string(),
            category: "General".to_string(),
            parameters: vec!["7b".to_string()],
            default_parameter: "7b".to_string(),
        },
        AvailableModel {
            name: "gemma2".to_string(),
            description: "Gemma 2 - легковесная модель от Google с поддержкой кода".to_string(),
            size: "~2-9 GB".to_string(),
            category: "General".to_string(),
            parameters: vec!["2b".to_string(), "9b".to_string()],
            default_parameter: "2b".to_string(),
        },
    ];

    Ok(models)
}

/// Скачать модель из Ollama с прогрессом через events
#[tauri::command]
pub async fn pull_ollama_model(
    app: tauri::AppHandle,
    model_name: String,
) -> Result<ModelOperationResult, String> {
    info!("=== НАЧАЛО СКАЧИВАНИЯ МОДЕЛИ: {} ===", model_name);

    // Создаём клиент с увеличенным таймаутом для скачивания
    let client = Client::builder()
        .timeout(Duration::from_secs(OLLAMA_PULL_TIMEOUT))
        .build()
        .map_err(|e| {
            error!("Ошибка создания HTTP клиента: {}", e);
            format!("Ошибка создания HTTP клиента: {}", e)
        })?;

    info!("Отправка запроса к Ollama: POST {}/api/pull", OLLAMA_BASE_URL);
    info!("Параметры: name={}, stream=true", model_name);

    // Отправляем событие начала скачивания
    let _ = app.emit("ollama-pull-progress", PullProgress {
        model: model_name.clone(),
        status: "starting".to_string(),
        progress: 0.0,
        completed: None,
        total: None,
        message: "Отправка запроса к Ollama...".to_string(),
    });

    // Запускаем скачивание модели с streaming
    let response = client
        .post(format!("{}/api/pull", OLLAMA_BASE_URL))
        .json(&serde_json::json!({
            "name": &model_name,
            "stream": true
        }))
        .send()
        .await;

    info!("Получен ответ от Ollama: {:?}", response.as_ref().map(|r| r.status()));

    match response {
        Ok(response) => {
            let status_code = response.status();
            info!("HTTP статус ответа: {}", status_code);

            if !status_code.is_success() {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Неизвестная ошибка".to_string());
                let error_msg = format!("Ошибка скачивания модели (HTTP {}): {}", status_code, error_body);
                error!("{}", error_msg);

                let _ = app.emit("ollama-pull-progress", PullProgress {
                    model: model_name.clone(),
                    status: "error".to_string(),
                    progress: 0.0,
                    completed: None,
                    total: None,
                    message: error_msg.clone(),
                });

                return Ok(ModelOperationResult {
                    success: false,
                    message: String::new(),
                    error: Some(error_msg),
                });
            }

            info!("Ответ успешен, начинаем чтение streaming данных...");

            // Читаем streaming ответ по частям (chunk by chunk)
            let mut stream = response.bytes_stream();
            use futures_util::StreamExt;

            let mut buffer = Vec::new();
            let mut line_count = 0;
            let mut last_progress = 0.0;

            // Читаем каждый chunk
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        info!("Получен chunk данных: {} байт", chunk.len());
                        buffer.extend_from_slice(&chunk);

                        // Парсим все полные строки из буфера
                        while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                            let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<_>>();
                            let line = String::from_utf8_lossy(&line_bytes);
                            let line = line.trim();

                            if line.is_empty() {
                                continue;
                            }

                            line_count += 1;
                            info!("=== Строка #{} от Ollama: {} ===", line_count, line);

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                // Проверяем на ошибку
                                if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
                                    let error_msg = format!("Ошибка Ollama: {}", error);
                                    error!("{}", error_msg);

                                    let _ = app.emit("ollama-pull-progress", PullProgress {
                                        model: model_name.clone(),
                                        status: "error".to_string(),
                                        progress: 0.0,
                                        completed: None,
                                        total: None,
                                        message: error_msg.clone(),
                                    });

                                    return Ok(ModelOperationResult {
                                        success: false,
                                        message: String::new(),
                                        error: Some(error_msg),
                                    });
                                }

                                // Парсим прогресс
                                let status = json
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("downloading")
                                    .to_string();

                                let completed = json.get("completed").and_then(|v| v.as_u64());
                                let total = json.get("total").and_then(|v| v.as_u64());

                                info!("Статус: {}, completed: {:?}, total: {:?}", status, completed, total);

                                let progress = match (completed, total) {
                                    (Some(c), Some(t)) if t > 0 => {
                                        let p = c as f64 / t as f64;
                                        info!("Прогресс: {:.2}%", p * 100.0);
                                        p
                                    }
                                    _ => last_progress,
                                };

                                last_progress = progress;

                                // Формируем сообщение
                                let message = if status == "success" {
                                    "Скачивание завершено!".to_string()
                                } else if let (Some(c), Some(t)) = (completed, total) {
                                    let pct = (progress * 100.0).round() as u32;
                                    let completed_mb = c as f64 / 1024.0 / 1024.0;
                                    let total_mb = t as f64 / 1024.0 / 1024.0;
                                    format!("{:.1} MB / {:.1} MB ({}%)", completed_mb, total_mb, pct)
                                } else {
                                    status.clone()
                                };

                                info!("Сообщение для UI: {}", message);

                                // Отправляем событие прогресса
                                let _ = app.emit("ollama-pull-progress", PullProgress {
                                    model: model_name.clone(),
                                    status: status.clone(),
                                    progress,
                                    completed,
                                    total,
                                    message,
                                });

                                // Если скачивание завершено
                                if status == "success" {
                                    info!("=== СКАЧИВАНИЕ МОДЕЛИ {} УСПЕШНО ЗАВЕРШЕНО ===", model_name);
                                    return Ok(ModelOperationResult {
                                        success: true,
                                        message: format!("Модель {} успешно скачана", model_name),
                                        error: None,
                                    });
                                }
                            } else {
                                warn!("Не удалось распарсить JSON: {}", line);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Ошибка чтения streaming данных: {}", e);
                        error!("{}", error_msg);

                        let _ = app.emit("ollama-pull-progress", PullProgress {
                            model: model_name.clone(),
                            status: "error".to_string(),
                            progress: 0.0,
                            completed: None,
                            total: None,
                            message: error_msg.clone(),
                        });

                        return Ok(ModelOperationResult {
                            success: false,
                            message: String::new(),
                            error: Some(error_msg),
                        });
                    }
                }
            }

            // Обрабатываем оставшиеся данные в буфере
            if !buffer.is_empty() {
                let remaining = String::from_utf8_lossy(&buffer);
                let remaining = remaining.trim();
                if !remaining.is_empty() {
                    info!("Оставшиеся данные: {}", remaining);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(remaining) {
                        if json.get("status").and_then(|v| v.as_str()) == Some("success") {
                            info!("=== СКАЧИВАНИЕ МОДЕЛИ {} УСПЕШНО ЗАВЕРШЕНО (из буфера) ===", model_name);
                            return Ok(ModelOperationResult {
                                success: true,
                                message: format!("Модель {} успешно скачана", model_name),
                                error: None,
                            });
                        }
                    }
                }
            }

            // Если дошли сюда, значит streaming не вернул success
            warn!("=== СКАЧИВАНИЕ МОДЕЛИ {} ЗАВЕРШИЛОСЬ БЕЗ СТАТУСА SUCCESS ===", model_name);
            warn!("Всего обработано строк: {}", line_count);
            Ok(ModelOperationResult {
                success: false,
                message: String::new(),
                error: Some("Скачивание не завершилось успешно (нет статуса success)".to_string()),
            })
        }
        Err(e) => {
            let error_msg = format!("Ошибка запроса к Ollama: {}", e);
            error!("=== ОШИБКА ЗАПРОСА К OLLAMA: {} ===", error_msg);

            let _ = app.emit("ollama-pull-progress", PullProgress {
                model: model_name.clone(),
                status: "error".to_string(),
                progress: 0.0,
                completed: None,
                total: None,
                message: error_msg.clone(),
            });

            Ok(ModelOperationResult {
                success: false,
                message: String::new(),
                error: Some(error_msg),
            })
        }
    }
}

/// Удалить модель из Ollama
#[tauri::command]
pub async fn delete_ollama_model(model_name: String) -> Result<ModelOperationResult, String> {
    info!("Удаление модели: {}", model_name);

    let client = create_client()?;

    match client
        .delete(format!("{}/api/delete", OLLAMA_BASE_URL))
        .json(&serde_json::json!({
            "name": model_name
        }))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                info!("Модель {} успешно удалена", model_name);
                Ok(ModelOperationResult {
                    success: true,
                    message: format!("Модель {} успешно удалена", model_name),
                    error: None,
                })
            } else {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Неизвестная ошибка".to_string());
                let error_msg = format!("Ошибка удаления модели: {}", error_body);
                error!("{}", error_msg);
                Ok(ModelOperationResult {
                    success: false,
                    message: String::new(),
                    error: Some(error_msg),
                })
            }
        }
        Err(e) => {
            let error_msg = format!("Ошибка запроса к Ollama: {}", e);
            error!("{}", error_msg);
            Ok(ModelOperationResult {
                success: false,
                message: String::new(),
                error: Some(error_msg),
            })
        }
    }
}

/// Выбрать активную модель для использования
#[tauri::command]
pub async fn set_active_ollama_model(model_name: String) -> Result<ModelOperationResult, String> {
    info!("Выбор активной модели: {}", model_name);

    // Проверяем, что модель существует
    let status = check_ollama_status().await?;

    if !status.available {
        return Ok(ModelOperationResult {
            success: false,
            message: String::new(),
            error: Some("Ollama сервис не доступен".to_string()),
        });
    }

    let model_exists = status.models.iter().any(|m| m.name == model_name);

    if !model_exists {
        return Ok(ModelOperationResult {
            success: false,
            message: String::new(),
            error: Some(format!("Модель {} не найдена", model_name)),
        });
    }

    // Сохраняем выбранную модель в localStorage через Tauri
    // В реальном приложении здесь можно сохранить в файл конфигурации
    info!("Модель {} выбрана как активная", model_name);

    Ok(ModelOperationResult {
        success: true,
        message: format!("Модель {} выбрана как активная", model_name),
        error: None,
    })
}

/// Получить текущую активную модель
#[tauri::command]
pub async fn get_active_ollama_model() -> Result<Option<String>, String> {
    // В реальном приложении здесь можно прочитать из файла конфигурации
    // Пока возвращаем None - модель не выбрана
    Ok(None)
}

/// Результат оптимизации кода
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeOptimizationResult {
    pub success: bool,
    pub optimized_code: Option<String>,
    pub error: Option<String>,
    pub removed_patterns: Option<u32>,
    pub optimized_lines: Option<u32>,
}

/// Оптимизация кода через Ollama LLM
#[tauri::command]
pub async fn optimize_code_with_ollama(
    code: String,
    target_lang: String,
) -> Result<CodeOptimizationResult, String> {
    info!(
        "Запуск оптимизации кода через Ollama, язык: {}, длина: {} символов",
        target_lang,
        code.len()
    );

    // Проверяем доступность Ollama
    let status = check_ollama_status().await?;

    if !status.available {
        return Ok(CodeOptimizationResult {
            success: false,
            optimized_code: None,
            error: Some("Ollama сервис не доступен".to_string()),
            removed_patterns: None,
            optimized_lines: None,
        });
    }

    if status.models.is_empty() {
        return Ok(CodeOptimizationResult {
            success: false,
            optimized_code: None,
            error: Some("Нет установленных моделей Ollama".to_string()),
            removed_patterns: None,
            optimized_lines: None,
        });
    }

    // Получаем активную модель
    let active_model = status
        .models
        .first()
        .ok_or("Нет доступных моделей")?
        .name
        .clone();

    info!("Используем модель: {}", active_model);
    
    // Формируем промпт для оптимизации
    let prompt = format!(
        r#"You are a code optimization expert for {lang} programming language.
Your task is to optimize the following code that was automatically translated from another language.

Optimization goals:
1. Remove unnecessary variable declarations (e.g., `x = None` before `x = value`)
2. Remove empty/redundant statements
3. Simplify overly complex constructions
4. Remove unused variables
5. Optimize control flow where possible
6. Keep the code functionally identical but cleaner

IMPORTANT RULES:
- DO NOT change the logic or functionality
- DO NOT add new features or change behavior
- ONLY output the optimized code, no explanations
- Preserve all comments in the original code
- Keep the same code structure where it makes sense
- DO NOT omit important parts of the code under comments like "the same code ..."
- If there were several similar constructs in the code, optimize EACH of them

Here is the code to optimize:

```{lang}
{code}
```

Output only the optimized {lang} code:"#,
        lang = target_lang,
        code = code
    );

    // Создаём HTTP клиент с таймаутом
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120)) // минуты на оптимизацию
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))?;

    info!("Отправка запроса к Ollama API для оптимизации...");

    // Отправляем запрос к Ollama API
    let response = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&serde_json::json!({
            "model": active_model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.2, // Низкая температура для детерминированности
                "num_predict": 8192 // Максимальная длина ответа
            }
        }))
        .send()
        .await;

    match response {
        Ok(response) => {
            let status_code = response.status();
            info!("HTTP статус от Ollama: {}", status_code);

            if !status_code.is_success() {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Неизвестная ошибка".to_string());
                let error_msg = format!("Ошибка Ollama API (HTTP {}): {}", status_code, error_body);
                error!("{}", error_msg);

                return Ok(CodeOptimizationResult {
                    success: false,
                    optimized_code: None,
                    error: Some(error_msg),
                    removed_patterns: None,
                    optimized_lines: None,
                });
            }

            // Парсим ответ
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Ошибка парсинга ответа Ollama: {}", e))?;

            // Извлекаем сгенерированный код
            let optimized_code = result
                .get("response")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "Ollama вернула пустой ответ".to_string())?;

            info!("Получен ответ от Ollama, длина: {} символов", optimized_code.len());

            // Очищаем код от маркеров языковых блоков (```python ... ```)
            let cleaned_code = clean_code_blocks(&optimized_code);

            // Считаем статистику
            let original_lines = code.lines().count() as u32;
            let optimized_lines = cleaned_code.lines().count() as u32;
            let removed_patterns = original_lines.saturating_sub(optimized_lines);

            info!(
                "Оптимизация завершена: {} -> {} строк (удалено: {})",
                original_lines, optimized_lines, removed_patterns
            );

            Ok(CodeOptimizationResult {
                success: true,
                optimized_code: Some(cleaned_code),
                error: None,
                removed_patterns: Some(removed_patterns),
                optimized_lines: Some(optimized_lines),
            })
        }
        Err(e) => {
            let error_msg = format!("Ошибка запроса к Ollama: {}", e);
            error!("{}", error_msg);

            Ok(CodeOptimizationResult {
                success: false,
                optimized_code: None,
                error: Some(error_msg),
                removed_patterns: None,
                optimized_lines: None,
            })
        }
    }
}

/// Очистка кода от markdown блоков
fn clean_code_blocks(code: &str) -> String {
    let mut result = code.to_string();

    // Удаляем открывающие маркеры ```language
    for line in code.lines() {
        if line.starts_with("```") && line.len() > 3 {
            let lang_marker = &line[3..];
            result = result.replace(&format!("```{}", lang_marker), "");
        }
    }

    // Удаляем все оставшиеся ```
    result = result.replace("```", "");

    // Удаляем пустые строки в начале и конце
    result.trim().to_string()
}

/// Инициализация Ollama модуля
pub fn init_ollama_module() {
    info!("Ollama модуль инициализирован");
}
