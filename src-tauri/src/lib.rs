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
struct F2CResult {
    success: bool,
    c_code: Option<String>,
    error: Option<String>,
    original_length: Option<usize>,
    converted_length: Option<usize>,
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

#[derive(Debug, Serialize, Deserialize)]
struct F2CStatus {
    available: bool,
    f2c_available: bool,
}

/// Проверка статуса f2c-сервиса (Fortran -> C)
#[tauri::command]
async fn check_f2c_status() -> Result<F2CStatus, String> {
    info!("Проверка статуса f2c-сервиса");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    match client
        .get("http://localhost:5001/health")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => Ok(F2CStatus {
                        available: true,
                        f2c_available: json
                            .get("f2c_available")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    }),
                    Err(_) => Ok(F2CStatus {
                        available: true,
                        f2c_available: false,
                    }),
                }
            } else {
                Ok(F2CStatus {
                    available: false,
                    f2c_available: false,
                })
            }
        }
        Err(_) => Ok(F2CStatus {
            available: false,
            f2c_available: false,
        }),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PythonStatus {
    available: bool,
    python_available: bool,
}

/// Проверка статуса Python-сервиса
#[tauri::command]
async fn check_python_status() -> Result<PythonStatus, String> {
    info!("Проверка статуса Python-сервиса");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    match client
        .get("http://localhost:5002/health")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => Ok(PythonStatus {
                        available: true,
                        python_available: json
                            .get("python_available")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    }),
                    Err(_) => Ok(PythonStatus {
                        available: true,
                        python_available: false,
                    }),
                }
            } else {
                Ok(PythonStatus {
                    available: false,
                    python_available: false,
                })
            }
        }
        Err(_) => Ok(PythonStatus {
            available: false,
            python_available: false,
        }),
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
            {"from": "fortran", "to": "python"},
            {"from": "c", "to": "java"},
            {"from": "c", "to": "go"}
        ]
    })
}

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

/// Конвертация Fortran -> C через f2c сервис
#[tauri::command]
async fn transpile_fortran_to_c(fortran_code: String) -> Result<F2CResult, String> {
    info!(
        "Запуск конвертации Fortran -> C, длина кода: {} символов",
        fortran_code.len()
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    // Отправляем запрос на f2c сервис
    let response = client
        .post("http://localhost:5001/convert")
        .json(&serde_json::json!({
            "code": fortran_code,
            "clean": true  // Очищаем от заголовков f2c
        }))
        .send()
        .await
        .map_err(|e| format!("Ошибка запроса к f2c сервису: {}", e))?;

    let status = response.status();
    
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_else(|_| "Неизвестная ошибка".to_string());
        return Ok(F2CResult {
            success: false,
            c_code: None,
            error: Some(format!("Ошибка f2c сервиса ({}): {}", status, error_body)),
            original_length: None,
            converted_length: None,
        });
    }

    let result: F2CResult = response
        .json()
        .await
        .map_err(|e| format!("Ошибка парсинга ответа f2c: {}", e))?;

    if !result.success {
        info!("Конвертация Fortran -> C не удалась: {:?}", result.error);
    } else {
        info!("Конвертация Fortran -> C успешна");
    }

    Ok(result)
}

/// Полная цепочка Fortran -> C -> Python
#[tauri::command]
async fn transpile_fortran_to_python(code: String) -> Result<TranspileResult, String> {
    info!(
        "Запуск транспиляции Fortran -> Python, длина кода: {} символов",
        code.len()
    );

    // Шаг 1: Конвертируем Fortran -> C
    info!("Шаг 1: Конвертация Fortran -> C...");
    let f2c_result = transpile_fortran_to_c(code).await?;

    if !f2c_result.success {
        return Ok(TranspileResult {
            success: false,
            output: None,
            error: f2c_result.error,
            ast_json: None,
        });
    }

    let c_code = f2c_result.c_code.unwrap_or_default();
    info!(
        "Шаг 1 завершён: получено {} символов C кода",
        c_code.len()
    );

    // Шаг 2: Очищаем C код от комментариев и #include (как в текущей реализации)
    info!("Шаг 2: Очистка C кода...");
    let cleaned_c_code = clean_c_code_for_transpilation(&c_code);
    info!("Шаг 2 завершён: {} символов после очистки", cleaned_c_code.len());

    // Если после очистки код стал пустым
    if cleaned_c_code.trim().is_empty() {
        return Ok(TranspileResult {
            success: false,
            output: None,
            error: Some("Код пуст после очистки".to_string()),
            ast_json: None,
        });
    }

    // Шаг 3: Парсим C код в AST и генерируем Python
    info!("Шаг 3: Парсинг C -> AST и генерация Python...");
    let python_result = transpile_c_to_python(cleaned_c_code).await?;

    if python_result.success {
        info!("Транспиляция Fortran -> Python успешна");
    } else {
        error!("Ошибка транспиляции Fortran -> Python: {:?}", python_result.error);
    }

    Ok(python_result)
}

/// Очистка C кода от комментариев, директив #include и специфичного мусора f2c
fn clean_c_code_for_transpilation(code: &str) -> String {
    let mut result = String::new();
    let mut chars = code.chars().peekable();

    while let Some(ch) = chars.next() {
        // Проверяем начало однострочного комментария //
        if ch == '/' && chars.peek() == Some(&'/') {
            // Пропускаем до конца строки
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
            }
            continue;
        }

        // Проверяем начало многострочного комментария /*
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next(); // пропускаем *
            // Ищем закрывающий */
            while let Some(&c) = chars.peek() {
                if c == '*' {
                    chars.next();
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                } else {
                    chars.next();
                }
            }
            continue;
        }

        // Проверяем директиву #include
        if ch == '#' {
            let mut directive = String::from("#");
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                directive.push(c);
                chars.next();
            }
            // Если это не #include, сохраняем директиву
            if !directive.trim_start().starts_with("#include") {
                result.push_str(&directive);
            }
            continue;
        }

        result.push(ch);
    }

    // Удаляем специфичные типы f2c
    let result = result
        .replace("static integer ", "int ")
        .replace("static doublereal ", "double ")
        .replace("static real ", "float ")
        .replace("static logical ", "int ")
        .replace("static char ", "char ")
        .replace("ftnlen ", "")
        .replace("cilist ", "int ")
        .replace("integer ", "int ")
        .replace("doublereal ", "double ")
        .replace("real ", "float ")
        .replace("logical ", "int ");

    // Удаляем объявления внешних функций и переменных
    let lines: Vec<&str> = result.lines().collect();
    let mut filtered_lines = Vec::new();
    
    for line in lines {
        let trimmed = line.trim();
        
        // Пропускаем extern объявления
        if trimmed.starts_with("extern") {
            continue;
        }
        
        // Пропускаем объявления функций f2c и специфичные строки
        if trimmed.contains("s_wsle(") 
            || trimmed.contains("do_lio(")
            || trimmed.contains("e_wsle(")
            || trimmed.contains("s_copy(")
            || trimmed.contains("s_stop(")
            || trimmed.starts_with("\"")
            || trimmed.starts_with("',")
            || trimmed.starts_with("\",")
        {
            continue;
        }
        
        // Заменяем вызовы функций вывода на printf
        let line = line
            // Заменяем s_wsle(&xxx); на пустоту (начало вывода)
            .replace("s_wsle(&io___2);", "")
            .replace("s_wsle(&io___1);", "")
            .replace("s_wsle(&io___3);", "")
            .replace("s_wsle(&io___4);", "")
            .replace("s_wsle(&io___5);", "")
            // Заменяем e_wsle(); на пустоту (конец вывода)  
            .replace("e_wsle();", "")
            // Заменяем do_lio на printf с аргументом
            .replace("do_lio(&c__9, &c__1,", "printf(\"%s\",")
            .replace("do_lio(&c__1, &c__9,", "printf(\"%d\",")
            // Удаляем хвост вызова do_lio
            ;
        
        // Удаляем остаточные вызовы f2c и пустые строки
        if line.trim().is_empty() || 
           line.trim() == ";" ||
           line.trim().contains("s_copy(") ||
           line.trim().contains("s_stop")
        {
            continue;
        }
        
        filtered_lines.push(line);
    }

    filtered_lines.join("\n")
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

/// Результат выполнения Python кода
#[derive(Debug, Serialize, Deserialize)]
struct PythonExecutionResult {
    success: bool,
    stdout: Option<String>,
    stderr: Option<String>,
    return_code: Option<i32>,
}

/// Выполнение Python кода
#[tauri::command]
async fn execute_python_code(code: String) -> Result<PythonExecutionResult, String> {
    info!(
        "Запуск выполнения Python кода, длина кода: {} символов",
        code.len()
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    let response = client
        .post("http://localhost:5002/execute")
        .json(&serde_json::json!({
            "code": code
        }))
        .send()
        .await
        .map_err(|e| format!("Ошибка запроса к Python сервису: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_else(|_| "Неизвестная ошибка".to_string());
        return Ok(PythonExecutionResult {
            success: false,
            stdout: None,
            stderr: Some(format!("Ошибка Python сервиса ({}): {}", status, error_body)),
            return_code: None,
        });
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Ошибка парсинга ответа Python: {}", e))?;

    Ok(PythonExecutionResult {
        success: result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        stdout: result
            .get("stdout")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        stderr: result
            .get("stderr")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        return_code: result
            .get("return_code")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    })
}

/// Остановка выполнения Python кода
#[tauri::command]
async fn stop_python_execution() -> Result<String, String> {
    info!("Остановка выполнения Python кода");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    let response = client
        .post("http://localhost:5002/stop")
        .send()
        .await
        .map_err(|e| format!("Ошибка запроса остановки: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_else(|_| "Неизвестная ошибка".to_string());
        return Err(format!("Ошибка остановки ({}): {}", status, error_body));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Ошибка парсинга ответа: {}", e))?;

    Ok(result
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Выполнено")
        .to_string())
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
            transpile_fortran_to_c,
            transpile_fortran_to_python,
            check_parser_status,
            check_f2c_status,
            check_python_status,
            execute_python_code,
            stop_python_execution,
            get_supported_languages,
            simple_transpile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
