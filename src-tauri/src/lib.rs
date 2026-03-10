// подключение модулей парсеров и генераторов
pub mod ast;
pub mod generators;
pub mod parsers;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
// чтение и запись файлов
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

use generators::{python_gen::PythonGenerator, Generator};
use parsers::{c_parser::CParser, Parser};

#[derive(Debug, Serialize, Deserialize)]
struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TranspileResult {
    success: bool,
    output: Option<String>,
    error: Option<String>,
    ast_json: Option<String>, // Для отладки
}

#[derive(Debug, Serialize, Deserialize)]
struct ParserStatus {
    docker_available: bool,
    parsers: Vec<String>,
    generators: Vec<String>,
}

#[tauri::command]
async fn open_file_with_filter(app: AppHandle, lang: String) -> Result<String, String> {
    info!("Открытие файла с фильтром для языка: {}", lang);

    // Получаем фильтры для выбранного языка
    let filters = get_filters_for_language(&lang);

    // Используем асинхронный диалог
    let file_path = open_file_async(app, filters).await?;

    // Читаем содержимое файла
    match read_file_content(&file_path) {
        Ok(content) => Ok(content),
        Err(e) => {
            error!("Ошибка чтения файла: {}", e);
            Err(format!("Не удалось прочитать файл: {}", e))
        }
    }
}

// Вспомогательная асинхронная функция для открытия файла
async fn open_file_async(app: AppHandle, filters: Vec<FileFilter>) -> Result<String, String> {
    use std::sync::Arc;
    use tokio::sync::{oneshot, Mutex};

    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let mut dialog = app.dialog().file();

    // Добавляем фильтры
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }

    // Показываем диалог и обрабатываем результат через callback
    dialog.pick_file(move |file_path| {
        let tx = tx.clone();
        // Используем tokio::spawn для отправки результата
        tauri::async_runtime::spawn(async move {
            let mut tx_guard = tx.lock().await;
            if let Some(tx) = tx_guard.take() {
                let result = match file_path {
                    Some(path) => Ok(path.to_string()),
                    None => Err("Файл не выбран".to_string()),
                };
                let _ = tx.send(result);
            }
        });
    });

    // Ждем результат с таймаутом
    match tokio::time::timeout(tokio::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Диалог был отменен".to_string()),
        Err(_) => Err("Таймаут ожидания диалога".to_string()),
    }
}

#[tauri::command]
async fn save_file_with_filter(
    app: AppHandle,
    content: String,
    lang: String,
) -> Result<String, String> {
    info!("Сохранение файла с фильтром для языка: {}", lang);

    // Получаем фильтры для выбранного выходного языка
    let filters = get_filters_for_language(&lang);

    // Используем асинхронный диалог
    let file_path = save_file_async(app, filters, &lang).await?;

    // Сохраняем содержимое в файл
    match write_file_content(&file_path, &content) {
        Ok(_) => {
            info!("Файл успешно сохранен");
            Ok(file_path)
        }
        Err(e) => {
            error!("Ошибка сохранения файла: {}", e);
            Err(format!("Не удалось сохранить файл: {}", e))
        }
    }
}

// Вспомогательная асинхронная функция для сохранения файла
async fn save_file_async(
    app: AppHandle,
    filters: Vec<FileFilter>,
    _lang: &str,
) -> Result<String, String> {
    use std::sync::Arc;
    use tokio::sync::{oneshot, Mutex};

    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let mut dialog = app.dialog().file();

    // Добавляем фильтры
    for filter in &filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }

    // Если есть фильтры, предлагаем расширение по умолчанию
    if let Some(first_filter) = filters.first() {
        if let Some(default_ext) = first_filter.extensions.first() {
            let default_file_name = format!("output.{}", default_ext);
            dialog = dialog.set_file_name(&default_file_name);
        }
    }

    // Показываем диалог сохранения
    dialog.save_file(move |file_path| {
        let tx = tx.clone();
        tauri::async_runtime::spawn(async move {
            let mut tx_guard = tx.lock().await;
            if let Some(tx) = tx_guard.take() {
                let result = match file_path {
                    Some(path) => Ok(path.to_string()),
                    None => Err("Сохранение отменено".to_string()),
                };
                let _ = tx.send(result);
            }
        });
    });

    // Ждем результат с таймаутом
    match tokio::time::timeout(tokio::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Диалог был отменен".to_string()),
        Err(_) => Err("Таймаут ожидания диалога".to_string()),
    }
}

#[tauri::command]
async fn transpile_c_to_python(code: String) -> Result<TranspileResult, String> {
    info!(
        "Запуск транспиляции C -> Python, длина кода: {} символов",
        code.len()
    );

    // Проверяем доступность Docker (неблокирующая проверка)
    let docker_available = tokio::task::spawn_blocking(|| CParser::is_available())
        .await
        .unwrap_or(false);

    if !docker_available {
        let error_msg =
            "Docker не доступен. Пожалуйста, убедитесь что Docker установлен и запущен.\n\
                        Установка Docker: https://docs.docker.com/get-docker/";
        error!("{}", error_msg);
        return Ok(TranspileResult {
            success: false,
            output: None,
            error: Some(error_msg.to_string()),
            ast_json: None,
        });
    }

    // Добавляем таймаут для всей операции
    let timeout_duration = tokio::time::Duration::from_secs(30);

    let result = tokio::time::timeout(timeout_duration, async {
        // Шаг 1: Парсим C код в AST
        info!("Парсинг C кода...");
        let ast_result = CParser::parse_async(&code).await;

        let ast = match ast_result {
            Ok(ast) => {
                info!("Парсинг успешен, получено AST");
                ast
            }
            Err(e) => {
                let error_msg = format!("Ошибка парсинга C кода: {}", e);
                error!("{}", error_msg);
                return TranspileResult {
                    success: false,
                    output: None,
                    error: Some(error_msg),
                    ast_json: None,
                };
            }
        };

        // Сохраняем AST в JSON для отладки
        let ast_json = match serde_json::to_string_pretty(&ast) {
            Ok(json) => {
                debug_ast_to_file(&json).ok();
                Some(json)
            }
            Err(e) => {
                warn!("Не удалось сериализовать AST в JSON: {}", e);
                None
            }
        };

        // Шаг 2: Генерируем Python код из AST
        info!("Генерация Python кода...");

        // Используем spawn_blocking для генерации, так как это CPU-интенсивная операция
        let python_result =
            tokio::task::spawn_blocking(move || PythonGenerator::generate(&ast)).await;

        match python_result {
            Ok(Ok(python_code)) => {
                info!(
                    "Генерация успешна, длина кода: {} символов",
                    python_code.len()
                );
                TranspileResult {
                    success: true,
                    output: Some(python_code),
                    error: None,
                    ast_json,
                }
            }
            Ok(Err(e)) => {
                let error_msg = format!("Ошибка генерации Python кода: {}", e);
                error!("{}", error_msg);
                TranspileResult {
                    success: false,
                    output: None,
                    error: Some(error_msg),
                    ast_json,
                }
            }
            Err(e) => {
                let error_msg = format!("Ошибка при выполнении генератора: {}", e);
                error!("{}", error_msg);
                TranspileResult {
                    success: false,
                    output: None,
                    error: Some(error_msg),
                    ast_json,
                }
            }
        }
    })
    .await;

    match result {
        Ok(transpile_result) => Ok(transpile_result),
        Err(_) => {
            let error_msg = "Транспиляция превысила время ожидания (30 секунд)".to_string();
            error!("{}", error_msg);
            Ok(TranspileResult {
                success: false,
                output: None,
                error: Some(error_msg),
                ast_json: None,
            })
        }
    }
}
/// Проверка статуса Docker и парсеров
#[tauri::command]
async fn check_parser_status() -> ParserStatus {
    info!("Проверка статуса парсеров");

    // Используем tokio::task::spawn_blocking для CPU-интенсивной операции
    let docker_available = tokio::task::spawn_blocking(|| CParser::is_available())
        .await
        .unwrap_or(false);

    ParserStatus {
        docker_available,
        parsers: vec!["C".to_string()],
        generators: vec!["Python".to_string()],
    }
}

/// Получение информации о поддерживаемых языках
#[tauri::command]
async fn get_supported_languages() -> serde_json::Value {
    info!("Получение списка поддерживаемых языков");

    serde_json::json!({
        "input": ["c", "fortran", "php", "cobol"],
        "output": ["python", "java", "go"],
        "pairs": [
            {"from": "c", "to": "python"},
            {"from": "c", "to": "java"},
            {"from": "c", "to": "go"}
        ]
    })
}

/// Простая транспиляция без AST (для тестирования)
#[tauri::command]
async fn simple_transpile(
    code: String,
    from_lang: String,
    to_lang: String,
) -> Result<String, String> {
    info!("Простая транспиляция из {} в {}", from_lang, to_lang);

    if from_lang == "c" && to_lang == "python" {
        // Вызываем основную функцию
        let result = transpile_c_to_python(code).await?;

        if result.success {
            Ok(result
                .output
                .unwrap_or_else(|| "# Пустой результат".to_string()))
        } else {
            Err(result
                .error
                .unwrap_or_else(|| "Неизвестная ошибка".to_string()))
        }
    } else {
        Err(format!(
            "Транспиляция из {} в {} пока не поддерживается",
            from_lang, to_lang
        ))
    }
}

/// Вспомогательная функция для сохранения AST в файл (отладка)
fn debug_ast_to_file(_ast_json: &str) -> Result<(), std::io::Error> {
    #[cfg(debug_assertions)]
    {
        use std::fs;
        use std::path::Path;

        let debug_dir = Path::new("target/debug/ast");
        if !debug_dir.exists() {
            fs::create_dir_all(debug_dir)?;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let filename = debug_dir.join(format!("ast_{}.json", timestamp));
        fs::write(filename, _ast_json)?;
    }
    Ok(())
}

fn get_filters_for_language(language: &str) -> Vec<FileFilter> {
    match language {
        "c" => vec![FileFilter {
            name: "C Files".to_string(),
            extensions: vec!["c".to_string(), "h".to_string()],
        }],
        "fortran" => vec![FileFilter {
            name: "Fortran Files".to_string(),
            extensions: vec![
                "f".to_string(),
                "for".to_string(),
                "f90".to_string(),
                "f95".to_string(),
            ],
        }],
        "php" => vec![FileFilter {
            name: "PHP Files".to_string(),
            extensions: vec!["php".to_string()],
        }],
        "cobol" => vec![FileFilter {
            name: "COBOL Files".to_string(),
            extensions: vec!["cob".to_string(), "cbl".to_string()],
        }],
        "python" => vec![FileFilter {
            name: "Python Files".to_string(),
            extensions: vec!["py".to_string()],
        }],
        "java" => vec![FileFilter {
            name: "Java Files".to_string(),
            extensions: vec!["java".to_string()],
        }],
        "go" => vec![FileFilter {
            name: "Go Files".to_string(),
            extensions: vec!["go".to_string()],
        }],
        _ => vec![FileFilter {
            name: "All Files".to_string(),
            extensions: vec!["*".to_string()],
        }],
    }
}

fn read_file_content(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Ошибка открытия файла: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Ошибка чтения файла: {}", e))?;
    Ok(content)
}

fn write_file_content(path: &str, content: &str) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| format!("Ошибка создания файла: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Ошибка записи в файл: {}", e))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Инициализация логгера
    #[cfg(debug_assertions)]
    {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
    }

    info!("Запуск Tauri приложения");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_file_with_filter,
            save_file_with_filter,
            transpile_c_to_python,
            check_parser_status,
            get_supported_languages,
            simple_transpile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
