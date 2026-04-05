/// Модуль управления Docker интеграцией
/// Проверка статусов сервисов и выполнение скриптов refresh (кроссплатформенно)

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use tauri::path::BaseDirectory;
use std::sync::OnceLock;

/// Кэшированный путь к Docker директории
static DOCKER_DIR_CACHE: OnceLock<PathBuf> = OnceLock::new();

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
/// Приоритет: 1) Ресурсы bundle (с копированием в temp), 2) Относительно текущей директории (dev режим)
fn get_docker_dir(app: Option<&tauri::AppHandle>) -> Result<PathBuf, String> {
    // Проверяем кэш
    if let Some(cached) = DOCKER_DIR_CACHE.get() {
        return Ok(cached.clone());
    }

    let docker_dir;

    // Вариант 1: Пробуем получить путь из ресурсов Tauri (для скомпилированного приложения)
    if let Some(app_handle) = app {
        if let Ok(resolved_path) = app_handle.path().resolve("docker", BaseDirectory::Resource) {
            if resolved_path.exists() && resolved_path.is_dir() {
                info!("Docker директория найдена в ресурсах: {:?}", resolved_path);

                // Копируем во временную директорию для записи (build.sh создаёт контейнеры)
                let temp_docker = copy_docker_to_temp(&resolved_path)?;
                let _ = DOCKER_DIR_CACHE.set(temp_docker.clone());
                return Ok(temp_docker);
            }
        }
    }

    // Вариант 2: Пробуем относительно текущей рабочей директории (dev режим)
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Ошибка получения текущего каталога: {}", e))?;

    // Пробуем ./docker
    let docker_dir_dev = current_dir.join("docker");
    if docker_dir_dev.exists() && docker_dir_dev.is_dir() {
        info!("Docker директория найдена в текущем каталоге: {:?}", docker_dir_dev);
        docker_dir = docker_dir_dev;
        let _ = DOCKER_DIR_CACHE.set(docker_dir.clone());
        return Ok(docker_dir);
    }

    // Пробуем ../docker (если мы в src-tauri/)
    if let Some(parent) = current_dir.parent() {
        let docker_dir_parent = parent.join("docker");
        if docker_dir_parent.exists() && docker_dir_parent.is_dir() {
            info!("Docker директория найдена в родительском каталоге: {:?}", docker_dir_parent);
            docker_dir = docker_dir_parent;
            let _ = DOCKER_DIR_CACHE.set(docker_dir.clone());
            return Ok(docker_dir);
        }
    }

    Err(format!("Docker директория не найдена. Текущий каталог: {:?}", current_dir))
}

/// Копировать Docker директорию во временное хранилище
fn copy_docker_to_temp(source: &PathBuf) -> Result<PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("illegacy-docker");

    // Если директория уже существует, удаляем её для чистой копии
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    copy_dir_all(source, &temp_dir)?;

    // Делаем скрипты исполняемыми (только Unix)
    #[cfg(unix)]
    make_scripts_executable(&temp_dir)?;

    info!("Docker директория скопирована во временный каталог: {:?}", temp_dir);
    Ok(temp_dir)
}

/// Рекурсивное копирование директории
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("Ошибка создания директории {:?}: {}", dst, e))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Ошибка чтения директории {:?}: {}", src, e))? {
        let entry = entry.map_err(|e| format!("Ошибка чтения entry: {}", e))?;
        let file_type = entry.file_type()
            .map_err(|e| format!("Ошибка получения типа файла: {}", e))?;
        let dest_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)
                .map_err(|e| format!("Ошибка копирования {:?} -> {:?}: {}", entry.path(), dest_path, e))?;
        }
    }

    Ok(())
}

/// Сделать shell скрипты исполняемыми
fn make_scripts_executable(dir: &PathBuf) -> Result<(), String> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("Ошибка чтения директории {:?}: {}", dir, e))? {
        let entry = entry.map_err(|e| format!("Ошибка чтения entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("sh") {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)
                    .map_err(|e| format!("Ошибка чтения метаданных {:?}: {}", path, e))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms)
                    .map_err(|e| format!("Ошибка установки прав для {:?}: {}", path, e))?;
            }
        }
    }

    Ok(())
}

/// Получить путь к скрипту refresh в зависимости от платформы
fn get_refresh_script_path(docker_dir: &PathBuf) -> PathBuf {
    if cfg!(windows) {
        docker_dir.join("refresh.bat")
    } else {
        docker_dir.join("refresh.sh")
    }
}

/// Выполнить скрипт refresh с передачей логов через events
#[tauri::command]
pub async fn run_refresh_script(
    app: tauri::AppHandle,
) -> Result<String, String> {
    info!("Запуск скрипта refresh");

    let docker_dir = get_docker_dir(Some(&app))?;
    let refresh_script = get_refresh_script_path(&docker_dir);

    if !refresh_script.exists() {
        error!("Скрипт refresh не найден по пути: {:?}", refresh_script);
        return Err("Скрипт refresh не найден".to_string());
    }

    // Отправляем событие начала процесса
    let _ = app.emit("refresh-progress", RefreshProgress {
        message: "Запуск процесса переустановки зависимостей...".to_string(),
        progress: 0.0,
        is_error: false,
    });

    // Делаем скрипт исполняемым (только Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&refresh_script) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(&refresh_script, permissions);
        }
    }

    // Запускаем скрипт в зависимости от платформы
    let mut child = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(&refresh_script)
            .current_dir(&docker_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Ошибка запуска скрипта: {}", e))?
    } else {
        Command::new("bash")
            .arg(&refresh_script)
            .current_dir(&docker_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Ошибка запуска скрипта: {}", e))?
    };

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
                    // Очищаем от ANSI-кодов для UI
                    let clean_line = strip_ansi_codes(&line_content);
                    
                    // Сначала определяем прогресс
                    let (progress, message) = parse_progress_from_line(&line_content);
                    
                    info!("[refresh.sh] {}", line_content);
                    // Отправляем очищенную строку в лог
                    let _ = app_clone.emit("refresh-log", clean_line);
                    
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
                    // Очищаем от ANSI-кодов для UI
                    let clean_line = strip_ansi_codes(&line_content);
                    warn!("[refresh.sh] {}", line_content);
                    let _ = app_clone_stderr.emit("refresh-log", clean_line);
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
        info!("Скрипт refresh успешно завершен");

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
    let clean_line = strip_ansi_codes(line);

    if clean_line.contains("=== Шаг 1: Остановка и удаление контейнеров ===") {
        return (0.1, "Остановка контейнеров...".to_string());
    } else if clean_line.contains("=== Шаг 2: Удаление старых образов ===") {
        return (0.3, "Удаление образов...".to_string());
    } else if clean_line.contains("=== Шаг 3: Сборка новых образов ===") {
        return (0.5, "Сборка новых образов...".to_string());
    } else if clean_line.contains("Сборка c-parser") {
        return (0.55, "Сборка c-parser...".to_string());
    } else if clean_line.contains("Сборка f2c-service") {
        return (0.65, "Сборка f2c-service...".to_string());
    } else if clean_line.contains("Сборка python-service") {
        return (0.75, "Сборка python-service...".to_string());
    } else if clean_line.contains("Сборка c-service") {
        return (0.85, "Сборка c-service...".to_string());
    } else if clean_line.contains("=== Шаг 4: Запуск сервисов ===") {
        return (0.9, "Запуск сервисов...".to_string());
    } else if clean_line.contains("Перезагрузка завершена") {
        return (1.0, "Завершено!".to_string());
    } else if clean_line.contains("Контейнер остановлен") || clean_line.contains("Контейнер удалён") ||
              clean_line.contains("[OK] Контейнер остановлен") || clean_line.contains("[OK] Контейнер удалён") {
        return (0.2, "Остановка контейнеров...".to_string());
    } else if clean_line.contains("Образ удалён") || clean_line.contains("[OK] Образ удалён") {
        return (0.4, "Удаление образов...".to_string());
    } else if clean_line.contains("Запуск C парсера") || clean_line.contains("Запуск f2c сервиса") ||
              clean_line.contains("Запуск Python сервиса") || clean_line.contains("Запуск C service") {
        return (0.95, "Запуск сервисов...".to_string());
    }

    (0.0, line.to_string())
}

/// Очистка строки от ANSI-кодов
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Начало ANSI-последовательности
            if chars.next() == Some('[') {
                // Пропускаем всё до буквы
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Инициализация Docker модуля
pub fn init_docker_module() {
    info!("Docker модуль инициализирован");
}
