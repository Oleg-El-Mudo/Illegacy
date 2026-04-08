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

// Базовые настройки для оптимизации кода
const OPTIMIZATION_BASE_TIMEOUT: u64 = 60; // Базовый таймаут 60 секунд
const OPTIMIZATION_TIMEOUT_PER_CHAR: u64 = 50; // 50 мс на каждый символ кода (0.05 сек)
const OPTIMIZATION_MAX_TIMEOUT: u64 = 600; // Максимальный таймаут 10 минут
const OPTIMIZATION_MAX_TOKENS_BASE: u32 = 4096; // Базовое количество токенов
const OPTIMIZATION_MAX_TOKENS_PER_LINE: u32 = 50; // Дополнительные токены на строку кода
const OPTIMIZATION_MAX_TOKENS_CAP: u32 = 32768; // Максимальный лимит токенов

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

/// Расчет оптимального таймаута на основе размера кода
fn calculate_optimization_timeout(code_length: usize) -> u64 {
    // Базовый таймаут + время на каждый символ
    let calculated_timeout = OPTIMIZATION_BASE_TIMEOUT + ((code_length as u64 * OPTIMIZATION_TIMEOUT_PER_CHAR) / 1000);
    
    // Ограничиваем минимальным и максимальным значениями
    let timeout = calculated_timeout.clamp(OPTIMIZATION_BASE_TIMEOUT, OPTIMIZATION_MAX_TIMEOUT);
    
    info!("Расчет таймаута: код {} символов -> {} секунд (базовый: {} с, на символы: {} мс)", 
          code_length, 
          timeout, 
          OPTIMIZATION_BASE_TIMEOUT,
          (code_length as u64 * OPTIMIZATION_TIMEOUT_PER_CHAR) / 1000);
    
    timeout
}

/// Расчет максимального количества токенов для ответа
fn calculate_max_tokens(code: &str) -> u32 {
    let line_count = code.lines().count() as u32;
    
    // Базовые токены + токены на строку
    let calculated_tokens = OPTIMIZATION_MAX_TOKENS_BASE + (line_count * OPTIMIZATION_MAX_TOKENS_PER_LINE);
    
    // Ограничиваем максимальным значением
    let tokens = calculated_tokens.min(OPTIMIZATION_MAX_TOKENS_CAP);
    
    info!("Расчет токенов: {} строк -> {} токенов (базовые: {}, на строки: {}, максимум: {})", 
          line_count, 
          tokens, 
          OPTIMIZATION_MAX_TOKENS_BASE,
          line_count * OPTIMIZATION_MAX_TOKENS_PER_LINE,
          OPTIMIZATION_MAX_TOKENS_CAP);
    
    tokens
}

/// Извлечение размера модели из имени (например: "codellama:7b" -> 7)
fn extract_model_parameter_size(model_name: &str) -> f32 {
    // Ищем паттерн типа "7b", "13b", "70b" и т.д.
    if let Some(colon_pos) = model_name.find(':') {
        let param_part = &model_name[colon_pos + 1..];
        // Извлекаем числовую часть до 'b'
        if let Some(b_pos) = param_part.find('b') {
            let num_part = &param_part[..b_pos];
            if let Ok(size) = num_part.parse::<f32>() {
                return size;
            }
        }
    }
    // По умолчанию возвращаем 7 (средний размер)
    7.0
}

/// Оптимизация кода через Ollama LLM
#[tauri::command]
pub async fn optimize_code_with_ollama(
    code: String,
    target_lang: String,
) -> Result<CodeOptimizationResult, String> {
    info!("================================================================");
    info!("ЗАПУСК ОПТИМИЗАЦИИ КОДА ЧЕРЕЗ OLLAMA");
    info!("================================================================");
    info!("Целевой язык: {}", target_lang);
    info!("Длина кода: {} символов", code.len());
    info!("Количество строк: {}", code.lines().count());
    info!("================================================================");

    // Проверяем доступность Ollama
    info!("Шаг 1: Проверка доступности Ollama сервиса...");
    let status = check_ollama_status().await?;

    if !status.available {
        error!("Ollama сервис НЕ доступен!");
        return Ok(CodeOptimizationResult {
            success: false,
            optimized_code: None,
            error: Some("Ollama сервис не доступен".to_string()),
            removed_patterns: None,
            optimized_lines: None,
        });
    }
    info!("✓ Ollama сервис доступен");

    if status.models.is_empty() {
        error!("Нет установленных моделей Ollama!");
        return Ok(CodeOptimizationResult {
            success: false,
            optimized_code: None,
            error: Some("Нет установленных моделей Ollama".to_string()),
            removed_patterns: None,
            optimized_lines: None,
        });
    }
    info!("✓ Найдено моделей: {}", status.models.len());

    // Получаем активную модель
    let active_model = status
        .models
        .first()
        .ok_or("Нет доступных моделей")?
        .name
        .clone();

    let model_size = extract_model_parameter_size(&active_model);
    info!("✓ Активная модель: {} (примерный размер: {}B параметров)", active_model, model_size);

    // Рассчитываем оптимальные параметры
    info!("Шаг 2: Расчет оптимальных параметров запроса...");
    let timeout_secs = calculate_optimization_timeout(code.len());
    let max_tokens = calculate_max_tokens(&code);
    
    info!("Таймаут: {} секунд ({} минут)", timeout_secs, timeout_secs / 60);
    info!("Максимум токенов: {}", max_tokens);

    // Формируем промпт для оптимизации
    info!("Шаг 3: Формирование промпта...");
    let prompt = format!(
    r#"ROLE: You are a {lang} code refactoring tool. Your ONLY function is to output optimized code — no text, no explanations, no comments about changes.

STRICT OUTPUT RULES:
1. Output ONLY the cleaned {lang} code in a single code block.
2. Start with the first line of code immediately — no greetings, headers, or disclaimers.
3. End with the last line of code — no closing remarks.
4. Do NOT include ANY text outside the code block.
5. Do NOT add comments, notes, or explanations of any kind.
6. Start output with ```{lang} and end with ```. Do not include ANY text outside these markers.

ALLOWED OPTIMIZATIONS:
- Remove unused variables and unused imports.
- Eliminate redundant initializations (e.g., `x = None` before `x = value` → keep only `x = value`).
- Delete empty statements and no‑op lines.
- Simplify trivial redundancies:
  * `x = x + 0` → remove line
  * `x += 0` → remove line
  * `x = x * 1` → remove line
- Merge consecutive assignments: `a = 1; a = 2` → `a = 2`.
- Remove duplicate imports.

MUST PRESERVE:
- All logic, calculations, and control flow (if/elif/else, for, while, try/except).
- Function signatures (names, parameters, return types).
- Overall code structure and nesting levels.
- All existing comments — copy them exactly as‑is.
- Required imports (only remove duplicates).
- Variable and function names — do NOT rename anything.
- I/O operations (print, logging, file writes, etc.).
- Exception handling blocks — keep try/except/finally intact.
- All enum definitions and class structures.
- All method implementations in classes.

INPUT CODE:
```{lang}
{code}
```


OUTPUT: The optimized {lang} code only. Begin with code immediately. No other text."#,
lang = target_lang,
code = code
);
    
    info!("Длина промпта: {} символов", prompt.len());
    info!("Промпт сформирован ✓");

    // Создаём HTTP клиент с рассчитанным таймаутом
    info!("Шаг 4: Создание HTTP клиента с таймаутом {} секунд...", timeout_secs);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| {
            error!("✗ Ошибка создания HTTP клиента: {}", e);
            format!("Ошибка создания HTTP клиента: {}", e)
        })?;
    info!("✓ HTTP клиент создан успешно");

    // Формируем JSON запроса
    info!("Шаг 5: Формирование JSON запроса...");
    let request_body = serde_json::json!({
        "model": active_model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.1,
            "num_predict": max_tokens,
            "top_p": 0.9,
            "top_k": 40,
            "repeat_penalty": 1.1,
            "stop": ["\n\n\n"]
        }
    });
    
    info!("JSON запрос сформирован");
    info!("Размер JSON: {} байт", serde_json::to_string(&request_body).map(|s| s.len()).unwrap_or(0));

    // Отправляем запрос к Ollama API
    info!("Шаг 6: Отправка запроса к Ollama API...");
    info!("URL: POST {}/api/generate", OLLAMA_BASE_URL);
    info!("Модель: {}", active_model);
    info!("Таймаут: {} секунд", timeout_secs);
    
    let request_start = std::time::Instant::now();
    
    let response = client
        .post(format!("{}/api/generate", OLLAMA_BASE_URL))
        .json(&request_body)
        .send()
        .await;

    let elapsed = request_start.elapsed();
    info!("Время ожидания ответа: {:.2} секунд", elapsed.as_secs_f64());

    match response {
        Ok(response) => {
            let status_code = response.status();
            info!("✓ Получен ответ от Ollama");
            info!("HTTP статус: {}", status_code);
            info!("Заголовки ответа: {:?}", response.headers());

            if !status_code.is_success() {
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Неизвестная ошибка".to_string());
                
                error!("✗ Ollama вернул ошибку HTTP {}", status_code);
                error!("Тело ошибки: {}", error_body);
                
                let error_msg = format!("Ошибка Ollama API (HTTP {}): {}", status_code, error_body);

                return Ok(CodeOptimizationResult {
                    success: false,
                    optimized_code: None,
                    error: Some(error_msg),
                    removed_patterns: None,
                    optimized_lines: None,
                });
            }

            // Парсим ответ
            info!("Шаг 7: Парсинг JSON ответа...");
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| {
                    error!("✗ Ошибка парсинга JSON ответа: {}", e);
                    format!("Ошибка парсинга ответа Ollama: {}", e)
                })?;
            
            info!("✓ JSON успешно распарсен");
            info!("Полный ответ Ollama (первые 500 символов):");
            info!("{:.500}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| "Не удалось форматировать".to_string()));

            // Проверяем наличие ошибки в ответе
            if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                error!("✗ Ollama вернула ошибку в ответе: {}", error);
                return Ok(CodeOptimizationResult {
                    success: false,
                    optimized_code: None,
                    error: Some(format!("Ollama error: {}", error)),
                    removed_patterns: None,
                    optimized_lines: None,
                });
            }

            // Извлекаем сгенерированный код
            info!("Шаг 8: Извлечение оптимизированного кода...");
            let optimized_code = result
                .get("response")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if optimized_code.is_none() {
                warn!("⚠ Ollama вернула пустой ответ или отсутствует поле 'response'");
                warn!("Доступные поля в ответе: {:?}", result.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
                
                // Проверяем есть ли done_reason (может указывать на проблему)
                if let Some(done_reason) = result.get("done_reason").and_then(|v| v.as_str()) {
                    warn!("Причина завершения: {}", done_reason);
                }
                
                return Ok(CodeOptimizationResult {
                    success: false,
                    optimized_code: None,
                    error: Some("Ollama вернула пустой ответ".to_string()),
                    removed_patterns: None,
                    optimized_lines: None,
                });
            }

            let optimized_code = optimized_code.unwrap();
            info!("✓ Получен ответ, длина: {} символов", optimized_code.len());
            info!("Количество строк в ответе: {}", optimized_code.lines().count());

            // Очищаем код от маркеров языковых блоков
            info!("Шаг 9: Очистка кода от markdown маркеров...");
            let cleaned_code = clean_code_blocks(&optimized_code);
            info!("✓ Код очищен, финальная длина: {} символов", cleaned_code.len());
            info!("Финальное количество строк: {}", cleaned_code.lines().count());

            // Считаем статистику
            let original_lines = code.lines().count() as u32;
            let optimized_lines = cleaned_code.lines().count() as u32;
            let removed_patterns = original_lines.saturating_sub(optimized_lines);

            info!("================================================================");
            info!("ОПТИМИЗАЦИЯ ЗАВЕРШЕНА УСПЕШНО");
            info!("================================================================");
            info!("Оригинальных строк: {}", original_lines);
            info!("Оптимизированных строк: {}", optimized_lines);
            info!("Удалено паттернов: {}", removed_patterns);
            info!("================================================================");

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
            error!("================================================================");
            error!("✗ ОШИБКА ЗАПРОСА К OLLAMA");
            error!("================================================================");
            error!("Тип ошибки: {:?}", e);
            error!("Сообщение: {}", error_msg);
            
            // Дополнительная диагностика
            if e.is_timeout() {
                error!(">>> ДИАГНОСТИКА: Таймаут запроса!");
                error!(">>> Текущий таймаут: {} секунд", timeout_secs);
                error!(">>> Возможно, модель {} не успела обработать запрос за отведенное время", active_model);
                error!(">>> Рекомендации:");
                error!("    - Увеличьте OPTIMIZATION_MAX_TIMEOUT в коде");
                error!("    - Используйте модель с меньшим количеством параметров");
                error!("    - Уменьшите размер входного кода");
                error!("    - Проверьте производительность системы (CPU/RAM)");
            } else if e.is_connect() {
                error!(">>> ДИАГНОСТИКА: Ошибка подключения!");
                error!(">>> Не удалось соединиться с Ollama по адресу {}", OLLAMA_BASE_URL);
                error!(">>> Проверьте, что Ollama запущен: ollama serve");
            } else if e.is_request() {
                error!(">>> ДИАГНОСТИКА: Ошибка формирования запроса!");
                error!(">>> Проблема с JSON или параметрами запроса");
            }
            
            error!("================================================================");

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
    
    // Удаляем ```python, ```rust и т.д.
    result = result
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n");
    
    result.trim().to_string()
}

/// Инициализация Ollama модуля
pub fn init_ollama_module() {
    info!("Ollama модуль инициализирован");
}
