// подключение модулей парсеров и генераторов
pub mod ast;
pub mod parsers;
pub mod generators;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt};
// чтение и запись файлов
use std::fs::File;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use log::{info, error, warn};

use parsers::{Parser, c_parser::CParser};
use generators::{Generator, python_gen::PythonGenerator};

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
    
    // Создаем диалог
    let mut dialog = app.dialog().file();
    
    // Добавляем фильтры
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }
    
    // Открываем диалог и получаем путь
    let file_path = dialog.blocking_pick_file();
    
    // Проверяем, выбран ли файл
    match file_path {
        Some(path) => {
            // Преобразуем FilePath в строку пути
            let path_str = path.to_string();
            info!("Выбран файл: {}", path_str);
            
            // Читаем содержимое файла
            match read_file_content(&path_str) {
                Ok(content) => Ok(content),
                Err(e) => {
                    error!("Ошибка чтения файла: {}", e);
                    Err(format!("Не удалось прочитать файл: {}", e))
                }
            }
        },
        None => {
            warn!("Файл не выбран");
            Err("Файл не выбран".to_string())
        }
    }
}

#[tauri::command]
async fn save_file_with_filter(app: AppHandle, content: String, lang: String) -> Result<String, String> {
    info!("Сохранение файла с фильтром для языка: {}", lang);
    
    // Получаем фильтры для выбранного выходного языка
    let filters = get_filters_for_language(&lang);
    
    // Создаем диалог для сохранения
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
    
    // Открываем диалог сохранения
    let file_path = dialog.blocking_save_file();
    
    // Проверяем, выбран ли путь для сохранения
    match file_path {
        Some(path) => {
            // Преобразуем FilePath в строку пути
            let path_str = path.to_string();
            info!("Сохранение в файл: {}", path_str);
            
            // Сохраняем содержимое в файл
            match write_file_content(&path_str, &content) {
                Ok(_) => {
                    info!("Файл успешно сохранен");
                    Ok(path_str)
                },
                Err(e) => {
                    error!("Ошибка сохранения файла: {}", e);
                    Err(format!("Не удалось сохранить файл: {}", e))
                }
            }
        },
        None => {
            warn!("Сохранение отменено");
            Err("Сохранение отменено".to_string())
        }
    }
}

#[tauri::command]
async fn transpile_c_to_python(code: String) -> Result<TranspileResult, String> {
    info!("Запуск транспиляции C -> Python, длина кода: {} символов", code.len());
    
    // Проверяем доступность Docker
    if !CParser::is_available() {
        let error_msg = "Docker не доступен. Пожалуйста, убедитесь что Docker установлен и запущен.\n\
                        Установка Docker: https://docs.docker.com/get-docker/";
        error!("{}", error_msg);
        return Ok(TranspileResult {
            success: false,
            output: None,
            error: Some(error_msg.to_string()),
            ast_json: None,
        });
    }
    
    // Шаг 1: Парсим C код в AST (используем асинхронную версию)
    info!("Парсинг C кода...");
    let ast_result = CParser::parse_async(&code).await;  // ← добавили .await
    
    let ast = match ast_result {
        Ok(ast) => {
            info!("Парсинг успешен, получено AST");
            ast
        },
        Err(e) => {
            let error_msg = format!("Ошибка парсинга C кода: {}", e);
            error!("{}", error_msg);
            return Ok(TranspileResult {
                success: false,
                output: None,
                error: Some(error_msg),
                ast_json: None,
            });
        }
    };
    
    // Сохраняем AST в JSON для отладки (опционально)
    let ast_json = match serde_json::to_string_pretty(&ast) {
        Ok(json) => {
            debug_ast_to_file(&json).ok();
            Some(json)
        },
        Err(e) => {
            warn!("Не удалось сериализовать AST в JSON: {}", e);
            None
        }
    };
    
    // Шаг 2: Генерируем Python код из AST
    info!("Генерация Python кода...");
    let python_result = PythonGenerator::generate(&ast);
    
    match python_result {
        Ok(python_code) => {
            info!("Генерация успешна, длина кода: {} символов", python_code.len());
            Ok(TranspileResult {
                success: true,
                output: Some(python_code),
                error: None,
                ast_json,
            })
        },
        Err(e) => {
            let error_msg = format!("Ошибка генерации Python кода: {}", e);
            error!("{}", error_msg);
            Ok(TranspileResult {
                success: false,
                output: None,
                error: Some(error_msg),
                ast_json,
            })
        }
    }
}

/// НОВАЯ ФУНКЦИЯ: Проверка статуса Docker и парсеров
#[tauri::command]
async fn check_parser_status() -> ParserStatus {
    info!("Проверка статуса парсеров");
    
    ParserStatus {
        docker_available: CParser::is_available(),
        parsers: vec!["C".to_string()],
        generators: vec!["Python".to_string()],
    }
}

/// НОВАЯ ФУНКЦИЯ: Получение информации о поддерживаемых языках
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

/// НОВАЯ ФУНКЦИЯ: Простая транспиляция без AST (для тестирования)
#[tauri::command]
async fn simple_transpile(code: String, from_lang: String, to_lang: String) -> Result<String, String> {
    info!("Простая транспиляция из {} в {}", from_lang, to_lang);
    
    if from_lang == "c" && to_lang == "python" {
        // Вызываем основную функцию
        let result = transpile_c_to_python(code).await?;
        
        if result.success {
            Ok(result.output.unwrap_or_else(|| "# Пустой результат".to_string()))
        } else {
            Err(result.error.unwrap_or_else(|| "Неизвестная ошибка".to_string()))
        }
    } else {
        Err(format!("Транспиляция из {} в {} пока не поддерживается", from_lang, to_lang))
    }
}

/// Вспомогательная функция для сохранения AST в файл (отладка)
fn debug_ast_to_file(ast_json: &str) -> Result<(), std::io::Error> {
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
        fs::write(filename, ast_json)?;
    }
    Ok(())
}

fn get_filters_for_language(language: &str) -> Vec<FileFilter> {
    match language {
        "c" => vec![
            FileFilter { 
                name: "C Files".to_string(), 
                extensions: vec!["c".to_string(), "h".to_string()] 
            }
        ],
        "fortran" => vec![
            FileFilter { 
                name: "Fortran Files".to_string(), 
                extensions: vec![
                    "f".to_string(), 
                    "for".to_string(), 
                    "f90".to_string(), 
                    "f95".to_string()
                ] 
            }
        ],
        "php" => vec![
            FileFilter { 
                name: "PHP Files".to_string(), 
                extensions: vec!["php".to_string()] 
            }
        ],
        "cobol" => vec![
            FileFilter { 
                name: "COBOL Files".to_string(), 
                extensions: vec!["cob".to_string(), "cbl".to_string()] 
            }
        ],
        "python" => vec![
            FileFilter { 
                name: "Python Files".to_string(), 
                extensions: vec!["py".to_string()] 
            }
        ],
        "java" => vec![
            FileFilter { 
                name: "Java Files".to_string(), 
                extensions: vec!["java".to_string()] 
            }
        ],
        "go" => vec![
            FileFilter { 
                name: "Go Files".to_string(), 
                extensions: vec!["go".to_string()] 
            }
        ],
        _ => vec![
            FileFilter { 
                name: "All Files".to_string(), 
                extensions: vec!["*".to_string()] 
            }
        ]
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