use anyhow::{Result, anyhow, Context};
use serde_json::Value;
use tokio::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use reqwest::Client;
use log::{info, error, debug, warn};

use crate::ast::ASTNode;
use super::Parser;

/// Клиент для взаимодействия с Docker контейнером парсера C
pub struct CParser {
    client: Client,
    docker_available: bool,
}

impl CParser {
    /// Создает новый экземпляр парсера
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            docker_available: Self::check_docker(),
        }
    }
    
    /// Проверяет наличие Docker (синхронная, вызывается при инициализации)
    fn check_docker() -> bool {
        let output = std::process::Command::new("docker")
            .args(&["info"])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                info!("Docker доступен");
                true
            }
            _ => {
                error!("Docker не найден. Убедитесь, что Docker установлен и запущен.");
                false
            }
        }
    }
    
    /// Запускает Docker контейнер и сразу удаляет после использования
    async fn run_docker_container_clean(&self, code: &str) -> Result<String> {
        // Создаем временный файл с кодом
        let temp_dir = tempfile::tempdir()?;
        let input_file = temp_dir.path().join("input.c");
        tokio::fs::write(&input_file, code).await?;
        
        info!("Запуск Docker контейнера (без имени) для парсинга C кода");
        debug!("Путь к временному файлу: {:?}", input_file);
        debug!("Содержимое файла:\n{}", code);
        
        // Проверяем, существует ли образ
        let check_image = Command::new("docker")
            .args(&["image", "inspect", "c-parser:latest"])
            .output()
            .await?;
        
        if !check_image.status.success() {
            error!("Образ c-parser:latest не найден. Запустите ./build.sh в папке docker");
            return Err(anyhow!("Образ Docker не найден. Соберите его: cd docker && ./build.sh"));
        }
        
        // Запускаем контейнер и сразу получаем вывод
        info!("Выполнение команды: docker run --rm -v {}:/input/input.c:ro c-parser:latest python app.py /input/input.c", 
              input_file.display());
        
        let start = std::time::Instant::now();
        
        let output = Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "-v", &format!("{}:/input/input.c:ro", input_file.display()),
                "c-parser:latest",
                "python", "app.py", "/input/input.c"
            ])
            .output()
            .await
            .context("Ошибка запуска Docker контейнера")?;
        
        let elapsed = start.elapsed();
        info!("Docker контейнер выполнился за {:?}", elapsed);
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            error!("Docker контейнер завершился с ошибкой (код: {:?})", output.status.code());
            error!("STDERR: {}", stderr);
            if !stdout.is_empty() {
                error!("STDOUT: {}", stdout);
            }
            
            return Err(anyhow!("Docker контейнер завершился с ошибкой: {}", stderr));
        }
        
        let stdout = String::from_utf8(output.stdout)?;
        debug!("Ответ от парсера (первые 500 символов): {}", &stdout.chars().take(500).collect::<String>());
        
        if stdout.is_empty() {
            warn!("Получен пустой ответ от парсера");
        }
        
        Ok(stdout)
    }
    
    /// Использует HTTP API парсера (если контейнер уже запущен) - АСИНХРОННАЯ
    async fn use_http_api(&self, code: &str) -> Result<String> {
        info!("Попытка подключения к HTTP API парсера на localhost:5000");
        
        let response = self.client
            .post("http://localhost:5000/parse")
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
            },
            Err(e) => {
                debug!("HTTP API не доступен: {}", e);
                Err(anyhow!("HTTP API не доступен: {}", e))
            }
        }
    }
    
    /// Преобразует JSON от парсера в наш ASTNode
    fn convert_json_to_ast(&self, json_str: &str) -> Result<ASTNode> {
        debug!("Парсинг JSON ответа, длина: {} символов", json_str.len());
        
        let value: Value = serde_json::from_str(json_str)?;
        
        // Проверяем успешность
        if let Some(success) = value.get("success") {
            if !success.as_bool().unwrap_or(false) {
                let error = value.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                return Err(anyhow!("Ошибка парсера: {}", error));
            }
        }
        
        // Извлекаем AST
        let ast_value = value.get("ast")
            .ok_or_else(|| anyhow!("Нет поля ast в ответе: {:?}", value))?;
        
        self.parse_ast_node(ast_value)
    }
    
    /// Рекурсивно парсит JSON в ASTNode
fn parse_ast_node(&self, value: &Value) -> Result<ASTNode> {
    if let Some(obj) = value.as_object() {
        // Проверяем, является ли это узлом
        if let Some(node_type) = obj.get("__node__").and_then(|v| v.as_str()) {
            let mut node = ASTNode::new(node_type);
            
            // Обрабатываем координаты
            if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                node.coord = Some(coord.to_string());
            }
            
            // СПЕЦИАЛЬНАЯ ОБРАБОТКА ДЛЯ РАЗНЫХ ТИПОВ УЗЛОВ
            match node_type {
                "FuncDef" | "FuncDecl" => {
                    // Для определения функции - ищем имя в разных местах
                    
                    // 1. Проверяем прямой атрибут name
                    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
                        debug!("Найдено имя функции (прямой атрибут): {}", name);
                    }
                    
                    // 2. Проверяем вложенный declarator
                    if let Some(decl) = obj.get("decl") {
                        if let Some(decl_obj) = decl.as_object() {
                            if let Some(decl_name) = decl_obj.get("name").and_then(|n| n.as_str()) {
                                node.attributes.insert("name".to_string(), Value::String(decl_name.to_string()));
                                debug!("Найдено имя функции (через decl): {}", decl_name);
                            }
                            
                            // 3. Проверяем type -> decl -> name
                            if let Some(type_obj) = decl_obj.get("type") {
                                if let Some(type_obj) = type_obj.as_object() {
                                    if let Some(type_decl) = type_obj.get("decl") {
                                        if let Some(type_decl_obj) = type_decl.as_object() {
                                            if let Some(type_decl_name) = type_decl_obj.get("name").and_then(|n| n.as_str()) {
                                                node.attributes.insert("name".to_string(), Value::String(type_decl_name.to_string()));
                                                debug!("Найдено имя функции (через type->decl): {}", type_decl_name);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // 4. Проверяем тип возврата
                    if let Some(ret_type) = obj.get("type") {
                        if let Some(ret_obj) = ret_type.as_object() {
                            if let Some(type_name) = ret_obj.get("names").and_then(|n| n.as_array()) {
                                if let Some(first_name) = type_name.first().and_then(|n| n.as_str()) {
                                    node.attributes.insert("return_type".to_string(), Value::String(first_name.to_string()));
                                }
                            }
                        }
                    }
                },
                
                "Decl" => {
                    // Для объявлений переменных
                    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
                    }
                    
                    // Проверяем инициализатор
                    if let Some(init) = obj.get("init") {
                        node.children.push(self.parse_ast_node(init)?);
                    }
                },
                
                "IdentifierType" => {
                    // Для идентификаторов типов
                    if let Some(names) = obj.get("names") {
                        if let Some(names_array) = names.as_array() {
                            if let Some(first_name) = names_array.first().and_then(|n| n.as_str()) {
                                node.attributes.insert("name".to_string(), Value::String(first_name.to_string()));
                            }
                        }
                    }
                },
                
                "Constant" => {
                    // Для констант
                    if let Some(value) = obj.get("value") {
                        node.attributes.insert("value".to_string(), value.clone());
                    }
                    if let Some(type_name) = obj.get("type") {
                        node.attributes.insert("type".to_string(), type_name.clone());
                    }
                },
                
                "ID" => {
                    // Для идентификаторов (имена переменных)
                    if let Some(name) = obj.get("name") {
                        node.attributes.insert("name".to_string(), name.clone());
                    }
                },
                
                "BinaryOp" => {
                    // Для бинарных операций
                    if let Some(op) = obj.get("op") {
                        node.attributes.insert("op".to_string(), op.clone());
                    }
                },
                
                "Assignment" => {
                    // Для присваиваний
                    if let Some(op) = obj.get("op") {
                        node.attributes.insert("op".to_string(), op.clone());
                    }
                },
                
                "UnaryOp" => {
                    // Для унарных операций
                    if let Some(op) = obj.get("op") {
                        node.attributes.insert("op".to_string(), op.clone());
                    }
                },
                
                "ParamList" => {
                    // Для списка параметров
                    if let Some(params) = obj.get("params") {
                        if let Some(params_array) = params.as_array() {
                            for param in params_array {
                                node.children.push(self.parse_ast_node(param)?);
                            }
                        }
                    }
                },

                // В методе parse_ast_node добавьте обработку для FuncCall:

"FuncCall" => {
    // Для вызова функции
    debug!("Обработка вызова функции");
    
    // Ищем имя функции
    if let Some(name_obj) = obj.get("name") {
        if let Some(name_obj) = name_obj.as_object() {
            if let Some(name) = name_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes.insert("name".to_string(), Value::String(name.to_string()));
                debug!("Найдено имя функции в вызове: {}", name);
            }
        }
    }
    
    // Ищем аргументы
    if let Some(args) = obj.get("args") {
        if let Some(args_obj) = args.as_object() {
            if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ExprList") {
                node.children.push(self.parse_ast_node(args)?);
            }
        }
    }
},
                
                _ => {
                    // Для остальных узлов просто копируем все атрибуты
                    debug!("Обработка узла типа: {}", node_type);
                }
            }
            
            // Обрабатываем все остальные поля для детей
            for (key, val) in obj {
                // Пропускаем уже обработанные специальные поля
                if key == "__node__" || key == "coord" || 
                   key == "name" || key == "type" || key == "decl" ||
                   key == "params" || key == "init" {
                    continue;
                }
                
                // Если значение - объект с __node__, это дочерний узел
                if val.is_object() {
                    if val.get("__node__").is_some() {
                        node.children.push(self.parse_ast_node(val)?);
                    }
                } 
                // Если значение - массив, проверяем элементы
                else if val.is_array() {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            if item.is_object() && item.get("__node__").is_some() {
                                node.children.push(self.parse_ast_node(item)?);
                            }
                        }
                    }
                }
            }
            
            Ok(node)
        } else {
            // Обычный объект (не узел)
            Ok(ASTNode::new("Object")
                .with_attr("value", value.clone()))
        }
    } else if let Some(arr) = value.as_array() {
        // Массив узлов
        let mut children = Vec::new();
        for item in arr {
            if item.is_object() && item.get("__node__").is_some() {
                children.push(self.parse_ast_node(item)?);
            }
        }
        Ok(ASTNode::new("Array").with_attr("count", children.len()))
    } else {
        // Примитивное значение
        Ok(ASTNode::new("Value")
            .with_attr("value", value.clone()))
    }
}
    
    /// Асинхронный метод парсинга
    pub async fn parse_async(code: &str) -> Result<ASTNode> {
        let parser = Self::new();
        
        if !parser.docker_available {
            return Err(anyhow!("Docker не доступен. Запустите Docker и перезапустите приложение."));
        }
        
        // Пробуем сначала HTTP API (если контейнер уже запущен)
        match parser.use_http_api(code).await {
            Ok(response) => {
                info!("Использован HTTP API парсера");
                parser.convert_json_to_ast(&response)
            },
            Err(http_err) => {
                // Если HTTP не работает, запускаем разовый контейнер
                info!("HTTP API не доступен ({}), запускаем разовый контейнер", http_err);
                
                match parser.run_docker_container_clean(code).await {
                    Ok(response) => {
                        info!("Docker контейнер успешно выполнен");
                        parser.convert_json_to_ast(&response)
                    },
                    Err(docker_err) => {
                        error!("Ошибка запуска Docker контейнера: {}", docker_err);
                        Err(docker_err)
                    }
                }
            }
        }
    }
}

impl Parser for CParser {
    fn parse(code: &str) -> Result<ASTNode> {
        Err(anyhow!("Используйте parse_async в асинхронном контексте"))
    }
    
    fn is_available() -> bool {
        Self::check_docker()
    }
}