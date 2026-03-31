/// Модуль управления Docker интеграцией
/// Проверка статусов сервисов и выполнение скрипта refresh.sh

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use tauri::Emitter;

/// Статус всех Docker сервисов
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DockerServicesStatus {
    pub c_parser: bool,
    pub f2c_service: bool,
    pub python_service: bool,
    pub c_service: bool,
}

/// Статус выполнения скрипта
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshProgress {
    pub message: String,
    pub progress: f64, // 0.0 to 1.0
    pub is_error: bool,
}

/// Проверка статуса Docker сервиса по health endpoint
fn check_service_health(url: &str, service_name: &str) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build();

    match client {
        Ok(client) => {
            match client.get(url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Сервис {} доступен по адресу {}", service_name, url);
                        true
                    } else {
                        warn!("Сервис {} вернул статус: {}", service_name, response.status());
                        false
                    }
                }
                Err(e) => {
                    warn!("Сервис {} не доступен: {}", service_name, e);
                    false
                }
            }
        }
        Err(e) => {
            error!("Ошибка создания HTTP клиента для {}: {}", service_name, e);
            false
        }
    }
}

/// Проверка статуса C parser сервиса
fn check_c_parser_status() -> bool {
    check_service_health("http://localhost:5000/health", "c-parser")
}

/// Проверка статуса f2c сервиса
fn check_f2c_service_status() -> bool {
    check_service_health("http://localhost:5001/health", "f2c-service")
}

/// Проверка статуса Python сервиса
fn check_python_service_status() -> bool {
    check_service_health("http://localhost:5002/health", "python-service")
}

/// Проверка статуса C сервиса
fn check_c_service_status() -> bool {
    check_service_health("http://localhost:5003/health", "c-service")
}

/// Проверка статусов всех Docker сервисов
#[tauri::command]
pub fn check_docker_services_status() -> Result<DockerServicesStatus, String> {
    info!("Проверка статусов Docker сервисов");

    let c_parser = check_c_parser_status();
    let f2c_service = check_f2c_service_status();
    let python_service = check_python_service_status();
    let c_service = check_c_service_status();

    info!(
        "Статусы сервисов - C Parser: {}, F2C: {}, Python: {}, C Service: {}",
        c_parser, f2c_service, python_service, c_service
    );

    Ok(DockerServicesStatus {
        c_parser,
        f2c_service,
        python_service,
        c_service,
    })
}

/// Получить путь к директории docker
fn get_docker_dir() -> Result<PathBuf, String> {
    // Путь относительно текущей рабочей директории (где запущено приложение)
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Ошибка получения текущего каталога: {}", e))?;
    
    // Пробуем несколько вариантов расположения docker директории
    let docker_dir = current_dir.join("docker");
    
    if docker_dir.exists() && docker_dir.is_dir() {
        Ok(docker_dir)
    } else {
        // Пробуем относительно src-tauri
        let parent_dir = current_dir.parent()
            .ok_or_else(|| "Не удалось получить родительский каталог".to_string())?;
        let docker_dir = parent_dir.join("docker");
        
        if docker_dir.exists() && docker_dir.is_dir() {
            Ok(docker_dir)
        } else {
            Err(format!("Docker директория не найдена. Текущий каталог: {:?}", current_dir))
        }
    }
}

/// Выполнить скрипт refresh.sh с передачей логов через events
#[tauri::command]
pub async fn run_refresh_script(
    app: tauri::AppHandle,
) -> Result<String, String> {
    info!("Запуск скрипта refresh.sh");

    let docker_dir = get_docker_dir()?;
    let refresh_script = docker_dir.join("refresh.sh");

    if !refresh_script.exists() {
        error!("Скрипт refresh.sh не найден по пути: {:?}", refresh_script);
        return Err("Скрипт refresh.sh не найден".to_string());
    }

    // Отправляем событие начала процесса
    let _ = app.emit("refresh-progress", RefreshProgress {
        message: "Запуск процесса переустановки зависимостей...".to_string(),
        progress: 0.0,
        is_error: false,
    });

    // Делаем скрипт исполняемым
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&refresh_script) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(&refresh_script, permissions);
        }
    }

    // Запускаем скрипт
    let mut child = Command::new("bash")
        .arg(&refresh_script)
        .current_dir(&docker_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Ошибка запуска скрипта: {}", e))?;

    info!("Скрипт запущен, PID: {:?}", child.id());

    // Читаем вывод скрипта в реальном времени
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Обрабатываем stdout в отдельном потоке
    let app_clone = app.clone();
    let stdout_handle = std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    // Сначала определяем прогресс
                    let (progress, message) = parse_progress_from_line(&line_content);
                    
                    info!("[refresh.sh] {}", line_content);
                    let _ = app_clone.emit("refresh-log", line_content);
                    
                    // Отправляем прогресс только для значимых строк
                    if progress > 0.0 {
                        let _ = app_clone.emit("refresh-progress", RefreshProgress {
                            message,
                            progress,
                            is_error: false,
                        });
                    }
                }
            }
        }
    });

    // Обрабатываем stderr в отдельном потоке
    let app_clone_stderr = app.clone();
    let stderr_handle = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    warn!("[refresh.sh] {}", line_content);
                    let _ = app_clone_stderr.emit("refresh-log", line_content);
                }
            }
        }
    });

    // Ждем завершения скрипта
    let result = child.wait()
        .map_err(|e| format!("Ошибка ожидания завершения скрипта: {}", e))?;

    // Ждем завершения потоков обработки вывода
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    if result.success() {
        info!("Скрипт refresh.sh успешно завершен");

        let _ = app.emit("refresh-progress", RefreshProgress {
            message: "Переустановка зависимостей завершена успешно!".to_string(),
            progress: 1.0,
            is_error: false,
        });

        Ok("Переустановка зависимостей завершена успешно!".to_string())
    } else {
        let error_msg = format!("Скрипт завершился с кодом: {:?}", result.code());
        error!("{}", error_msg);

        let _ = app.emit("refresh-progress", RefreshProgress {
            message: format!("Ошибка: {}", error_msg),
            progress: 1.0,
            is_error: true,
        });

        Err(error_msg)
    }
}

/// Парсинг прогресса из строки вывода скрипта
fn parse_progress_from_line(line: &str) -> (f64, String) {
    // Этапы выполнения и соответствующий им прогресс
    if line.contains("=== Шаг 1: Остановка и удаление контейнеров ===") {
        return (0.1, "Остановка контейнеров...".to_string());
    } else if line.contains("=== Шаг 2: Удаление старых образов ===") {
        return (0.3, "Удаление образов...".to_string());
    } else if line.contains("=== Шаг 3: Сборка новых образов ===") {
        return (0.5, "Сборка новых образов...".to_string());
    } else if line.contains("Сборка c-parser") {
        return (0.55, "Сборка c-parser...".to_string());
    } else if line.contains("Сборка f2c-service") {
        return (0.65, "Сборка f2c-service...".to_string());
    } else if line.contains("Сборка python-service") {
        return (0.75, "Сборка python-service...".to_string());
    } else if line.contains("Сборка c-service") {
        return (0.85, "Сборка c-service...".to_string());
    } else if line.contains("=== Шаг 4: Запуск сервисов ===") {
        return (0.9, "Запуск сервисов...".to_string());
    } else if line.contains("Перезагрузка завершена") {
        return (1.0, "Завершено!".to_string());
    } else if line.contains("✓ Контейнер остановлен") || line.contains("✓ Контейнер удалён") {
        return (0.2, "Остановка контейнеров...".to_string());
    } else if line.contains("✓ Образ удалён") {
        return (0.4, "Удаление образов...".to_string());
    } else if line.contains("Запуск C парсера") || line.contains("Запуск f2c сервиса") || 
              line.contains("Запуск Python сервиса") || line.contains("Запуск C service") {
        return (0.95, "Запуск сервисов...".to_string());
    }
    
    // Возвращаем текущее сообщение без изменения прогресса
    (0.0, line.to_string())
}

/// Инициализация Docker модуля
pub fn init_docker_module() {
    info!("Docker модуль инициализирован");
}
