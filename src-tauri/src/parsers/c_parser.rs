use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;

use super::Parser;
use crate::ast::ASTNode;

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
            return Err(anyhow!(
                "Образ Docker не найден. Соберите его: cd docker && ./build.sh"
            ));
        }

        // Запускаем контейнер и сразу получаем вывод
        info!("Выполнение команды: docker run --rm -v {}:/input/input.c:ro c-parser:latest python app.py /input/input.c", 
              input_file.display());

        let start = std::time::Instant::now();

        let output = Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "-v",
                &format!("{}:/input/input.c:ro", input_file.display()),
                "c-parser:latest",
                "python",
                "app.py",
                "/input/input.c",
            ])
            .output()
            .await
            .context("Ошибка запуска Docker контейнера")?;

        let elapsed = start.elapsed();
        info!("Docker контейнер выполнился за {:?}", elapsed);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            error!(
                "Docker контейнер завершился с ошибкой (код: {:?})",
                output.status.code()
            );
            error!("STDERR: {}", stderr);
            if !stdout.is_empty() {
                error!("STDOUT: {}", stdout);
            }

            return Err(anyhow!("Docker контейнер завершился с ошибкой: {}", stderr));
        }

        let stdout = String::from_utf8(output.stdout)?;
        debug!(
            "Ответ от парсера (первые 500 символов): {}",
            &stdout.chars().take(500).collect::<String>()
        );

        if stdout.is_empty() {
            warn!("Получен пустой ответ от парсера");
        }

        Ok(stdout)
    }

    /// Использует HTTP API парсера (если контейнер уже запущен) - АСИНХРОННАЯ
    async fn use_http_api(&self, code: &str) -> Result<String> {
        info!("Попытка подключения к HTTP API парсера на localhost:5000");

        let response = self
            .client
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
            }
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
                let error = value
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                return Err(anyhow!("Ошибка парсера: {}", error));
            }
        }

        // Извлекаем AST
        let ast_value = value
            .get("ast")
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
                        debug!("Обработка объявления функции");

                        // 1. Проверяем прямой атрибут name
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Найдено имя функции (прямой атрибут): {}", name);
                        }

                        // 2. Проверяем вложенный declarator
                        if let Some(decl) = obj.get("decl") {
                            debug!("Найден decl: {:#?}", decl);
                            if let Some(decl_obj) = decl.as_object() {
                                if let Some(decl_name) =
                                    decl_obj.get("name").and_then(|n| n.as_str())
                                {
                                    node.attributes.insert(
                                        "name".to_string(),
                                        Value::String(decl_name.to_string()),
                                    );
                                    debug!("Найдено имя функции (через decl): {}", decl_name);
                                }

                                // 3. Ищем параметры функции в declarator
                                if let Some(type_obj) = decl_obj.get("type") {
                                    debug!("Найден type в decl: {:#?}", type_obj);
                                    if let Some(type_obj) = type_obj.as_object() {
                                        // Параметры могут быть в args прямо в type
                                        if let Some(args) = type_obj.get("args") {
                                            debug!("Найден args в type: {:#?}", args);
                                            if let Some(args_obj) = args.as_object() {
                                                if args_obj.get("__node__").and_then(|n| n.as_str())
                                                    == Some("ParamList")
                                                {
                                                    debug!("Найдены параметры функции в type.args");
                                                    self.extract_params_from_paramlist(
                                                        args_obj, &mut node,
                                                    )?;
                                                }
                                            }
                                        }
                                    }
                                }

                                // 4. Также ищем параметры в прямом атрибуте args (если есть)
                                if let Some(args) = decl_obj.get("args") {
                                    debug!("Найден прямой args в decl: {:#?}", args);
                                    if let Some(args_obj) = args.as_object() {
                                        if args_obj.get("__node__").and_then(|n| n.as_str())
                                            == Some("ParamList")
                                        {
                                            debug!("Найдены параметры функции в decl.args");
                                            self.extract_params_from_paramlist(
                                                args_obj, &mut node,
                                            )?;
                                        }
                                    }
                                }
                            }
                        }

                        // 5. Проверяем тип возврата
                        if let Some(ret_type) = obj.get("type") {
                            if let Some(ret_obj) = ret_type.as_object() {
                                if let Some(type_name) =
                                    ret_obj.get("names").and_then(|n| n.as_array())
                                {
                                    if let Some(first_name) =
                                        type_name.first().and_then(|n| n.as_str())
                                    {
                                        node.attributes.insert(
                                            "return_type".to_string(),
                                            Value::String(first_name.to_string()),
                                        );
                                    }
                                }
                            }
                        }

                        // ОТЛАДКА: выводим все атрибуты после обработки
                        debug!(
                            "Атрибуты функции {} после обработки: {:#?}",
                            node.attributes
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown"),
                            node.attributes
                        );
                    }
                    "Decl" => {
                        debug!("Обработка Decl узла");
                        debug!("Полное содержимое Decl: {:#?}", obj);

                        // Для объявлений переменных
                        let decl_name = obj.get("name").and_then(|n| n.as_str()).map(String::from);
                        debug!("Имя в Decl: {:?}", decl_name);

                        if let Some(ref name) = decl_name {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.clone()));
                        }

                        // Сначала обрабатываем инициализатор, если есть
                        let mut init_node = None;
                        if let Some(init) = obj.get("init") {
                            debug!("Инициализатор в Decl: {:#?}", init);
                            if let Ok(init_node_val) = self.parse_ast_node(init) {
                                debug!("Создан инициализатор типа: {}", init_node_val.node_type);
                                init_node = Some(init_node_val);
                            }
                        }

                        // Проверяем тип объявления
                        if let Some(type_obj) = obj.get("type") {
                            debug!("Тип в Decl: {:#?}", type_obj);

                            if let Some(type_obj_map) = type_obj.as_object() {
                                if let Some(type_node) =
                                    type_obj_map.get("__node__").and_then(|n| n.as_str())
                                {
                                    debug!("Тип узла: {}", type_node);

                                    if type_node == "ArrayDecl" {
                                        debug!("Найдено объявление массива в Decl");

                                        if let Ok(mut array_node) = self.parse_ast_node(type_obj) {
                                            if let Some(ref name) = decl_name {
                                                array_node.attributes.insert(
                                                    "name".to_string(),
                                                    Value::String(name.clone()),
                                                );
                                                debug!("Добавлено имя массива: {}", name);
                                            }

                                            // Добавляем инициализатор как ребенка массива, если он есть
                                            if let Some(init) = init_node {
                                                debug!("Добавляем инициализатор к узлу массива");
                                                array_node.children.push(init);
                                            }

                                            node.children.push(array_node);
                                        }
                                    } else {
                                        // Обычное объявление - добавляем тип и инициализатор как детей Decl
                                        if let Ok(type_decl_node) = self.parse_ast_node(type_obj) {
                                            node.children.push(type_decl_node);
                                        }
                                        // Для обычных объявлений добавляем инициализатор отдельно
                                        if let Some(init) = init_node {
                                            node.children.push(init);
                                        }
                                    }
                                }
                            }
                        } else if let Some(init) = init_node {
                            // Если нет типа, но есть инициализатор
                            node.children.push(init);
                        }

                        // ВАЖНО: НЕ добавляем init_node ещё раз здесь!
                    }
                    "IdentifierType" => {
                        // Для идентификаторов типов
                        if let Some(names) = obj.get("names") {
                            if let Some(names_array) = names.as_array() {
                                if let Some(first_name) =
                                    names_array.first().and_then(|n| n.as_str())
                                {
                                    node.attributes.insert(
                                        "name".to_string(),
                                        Value::String(first_name.to_string()),
                                    );
                                }
                            }
                        }
                    }

                    // В c_parser.rs, в обработке "Constant":
                    "Constant" => {
                        // Для констант
                        if let Some(value) = obj.get("value") {
                            // Если это строковое представление числа с суффиксом 'f', убираем суффикс
                            if let Some(value_str) = value.as_str() {
                                if value_str.ends_with('f')
                                    && value_str[..value_str.len() - 1].parse::<f64>().is_ok()
                                {
                                    let clean_value = value_str[..value_str.len() - 1].to_string();
                                    node.attributes
                                        .insert("value".to_string(), Value::String(clean_value));
                                } else {
                                    node.attributes.insert("value".to_string(), value.clone());
                                }
                            } else {
                                node.attributes.insert("value".to_string(), value.clone());
                            }
                        }
                        if let Some(type_name) = obj.get("type") {
                            node.attributes
                                .insert("type".to_string(), type_name.clone());
                        }
                    }

                    "ID" => {
                        // Для идентификаторов (имена переменных)
                        if let Some(name) = obj.get("name") {
                            node.attributes.insert("name".to_string(), name.clone());
                        }
                    }

                    "BinaryOp" => {
                        // Для бинарных операций
                        if let Some(op) = obj.get("op") {
                            node.attributes.insert("op".to_string(), op.clone());
                        }
                    }

                    "Assignment" => {
                        // Для присваиваний
                        if let Some(op) = obj.get("op") {
                            node.attributes.insert("op".to_string(), op.clone());
                        }
                    }

                    "UnaryOp" => {
                        // Для унарных операций
                        if let Some(op) = obj.get("op") {
                            node.attributes.insert("op".to_string(), op.clone());
                        }

                        // Ищем операнд (expr)
                        if let Some(expr) = obj.get("expr") {
                            debug!("Операнд унарной операции: {:#?}", expr);
                            if let Ok(expr_node) = self.parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }
                    }

                    "Cast" => {
                        debug!("Обработка приведения типа (cast)");
                        debug!("Содержимое Cast: {:#?}", obj);

                        // Сохраняем тип, к которому приводим
                        if let Some(to_type) = obj.get("to_type") {
                            node.attributes
                                .insert("to_type".to_string(), to_type.clone());
                        }

                        // Обрабатываем выражение, которое приводится
                        if let Some(expr) = obj.get("expr") {
                            if let Ok(expr_node) = self.parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }
                    }

                    "ParamList" => {
                        debug!("Обработка ParamList");

                        // Сохраняем параметры как атрибут
                        if let Some(params) = obj.get("params") {
                            node.attributes.insert("params".to_string(), params.clone());

                            // Добавляем параметры как детей
                            if let Some(params_array) = params.as_array() {
                                for param in params_array {
                                    if let Ok(param_node) = self.parse_ast_node(param) {
                                        node.children.push(param_node);
                                    }
                                }
                            }
                        }
                    }

                    "FuncCall" => {
                        // Для вызова функции
                        debug!("Обработка вызова функции");
                        debug!("Содержимое узла FuncCall: {:#?}", obj);

                        // Ищем имя функции
                        if let Some(name_obj) = obj.get("name") {
                            if let Some(name_obj) = name_obj.as_object() {
                                if let Some(name) = name_obj.get("name").and_then(|n| n.as_str()) {
                                    node.attributes.insert(
                                        "name".to_string(),
                                        Value::String(name.to_string()),
                                    );
                                    debug!("Найдено имя функции в вызове: {}", name);
                                }
                            }
                        }

                        // Ищем аргументы
                        if let Some(args) = obj.get("args") {
                            debug!("Найдены аргументы функции: {:#?}", args);
                            if let Some(args_obj) = args.as_object() {
                                if args_obj.get("__node__").and_then(|n| n.as_str())
                                    == Some("ExprList")
                                {
                                    debug!("Найдены аргументы функции (ExprList)");
                                    // Важно: парсим args как узел, чтобы получить его детей
                                    if let Ok(args_node) = self.parse_ast_node(args) {
                                        debug!(
                                            "Аргументы узел типа {} с {} детьми",
                                            args_node.node_type,
                                            args_node.children.len()
                                        );
                                        node.children.push(args_node);
                                    }
                                }
                            }
                        }
                    }

                    "ParamDecl" => {
                        debug!("Обработка ParamDecl");

                        // Ищем имя параметра
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                        }

                        // Ищем тип параметра
                        if let Some(type_obj) = obj.get("type") {
                            if let Some(type_obj) = type_obj.as_object() {
                                if let Some(names) = type_obj.get("names") {
                                    if let Some(names_array) = names.as_array() {
                                        if let Some(first_name) =
                                            names_array.first().and_then(|n| n.as_str())
                                        {
                                            node.attributes.insert(
                                                "type".to_string(),
                                                Value::String(first_name.to_string()),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    "ExprList" => {
                        debug!("Обработка ExprList");
                        debug!("Содержимое ExprList: {:#?}", obj);

                        // В pycparser ExprList может иметь поля вида "exprs[0]", "exprs[1]" и т.д.
                        let mut values = std::collections::HashMap::new();

                        for (key, value) in obj {
                            if key.starts_with("exprs[") && key.ends_with(']') {
                                if let Some(index_str) =
                                    key.strip_prefix("exprs[").and_then(|s| s.strip_suffix(']'))
                                {
                                    if let Ok(index) = index_str.parse::<usize>() {
                                        debug!("Найдено поле {} с индексом {}", key, index);
                                        values.insert(index, value);
                                    }
                                }
                            }
                        }

                        // Сортируем по индексу и добавляем детей
                        let mut indices: Vec<_> = values.keys().collect();
                        indices.sort();

                        for index in indices {
                            if let Some(value) = values.get(index) {
                                debug!("Добавляем аргумент с индексом {}: {:#?}", index, value);
                                if let Ok(expr_node) = self.parse_ast_node(value) {
                                    node.children.push(expr_node);
                                    debug!("Добавлен аргумент");
                                }
                            }
                        }

                        debug!(
                            "Итоговое количество детей в ExprList: {}",
                            node.children.len()
                        );
                    }

                    // В методе parse_ast_node, в секции match node_type
                    "If" => {
                        debug!("Обработка IF узла");
                        debug!("Содержимое IF узла: {:#?}", obj);

                        // Сохраняем координаты для отладки
                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                            debug!("IF узел координаты: {}", coord);
                        }

                        // Обрабатываем условие (обычно это первый ребенок)
                        if let Some(cond) = obj.get("cond") {
                            debug!("Условие IF: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }

                        // Обрабатываем тело if (then)
                        if let Some(iftrue) = obj.get("iftrue") {
                            debug!("Тело THEN (iftrue): {:#?}", iftrue);
                            if let Some(coord) = iftrue.get("coord").and_then(|c| c.as_str()) {
                                debug!("Тело THEN координаты: {}", coord);
                            }
                            if let Ok(body_node) = self.parse_ast_node(iftrue) {
                                node.children.push(body_node);
                            }
                        }

                        // Обрабатываем тело else (iffalse)
                        if let Some(iffalse) = obj.get("iffalse") {
                            debug!("Тело ELSE (iffalse): {:#?}", iffalse);
                            if let Some(coord) = iffalse.get("coord").and_then(|c| c.as_str()) {
                                debug!("Тело ELSE координаты: {}", coord);
                            }
                            if let Ok(body_node) = self.parse_ast_node(iffalse) {
                                node.children.push(body_node);
                            }
                        }
                    }

                    // В методе parse_ast_node, в секции match node_type добавьте:
                    "Switch" => {
                        debug!("Обработка SWITCH узла");
                        debug!("Содержимое SWITCH: {:#?}", obj);

                        // Сохраняем координаты
                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Обрабатываем выражение switch (условие)
                        if let Some(cond) = obj.get("cond") {
                            debug!("Условие SWITCH: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }

                        // Обрабатываем тело switch (список case/default)
                        if let Some(stmt) = obj.get("stmt") {
                            debug!("Тело SWITCH: {:#?}", stmt);
                            if let Ok(body_node) = self.parse_ast_node(stmt) {
                                node.children.push(body_node);
                            }
                        }
                    }

                    // В методе parse_ast_node, для "Case":
                    "Case" => {
                        debug!("Обработка CASE узла");
                        debug!("Содержимое CASE: {:#?}", obj);

                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Обрабатываем значение case (expr)
                        if let Some(expr) = obj.get("expr") {
                            debug!("Значение CASE: {:#?}", expr);
                            if let Ok(expr_node) = self.parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }

                        // Обрабатываем операторы в case (stmts)
                        // В pycparser stmts может быть массивом или отдельными полями stmts[0], stmts[1] и т.д.
                        if let Some(stmts) = obj.get("stmts") {
                            debug!("Операторы CASE: {:#?}", stmts);

                            // Создаем Compound узел для операторов
                            let mut compound_node = ASTNode::new("Compound");

                            if let Some(stmts_array) = stmts.as_array() {
                                for stmt in stmts_array {
                                    if let Ok(stmt_node) = self.parse_ast_node(stmt) {
                                        compound_node.children.push(stmt_node);
                                    }
                                }
                            } else {
                                // Может быть объект с полями stmts[0], stmts[1] и т.д.
                                let mut i = 0;
                                loop {
                                    let key = format!("stmts[{}]", i);
                                    if let Some(stmt) = obj.get(&key) {
                                        if let Ok(stmt_node) = self.parse_ast_node(stmt) {
                                            compound_node.children.push(stmt_node);
                                        }
                                        i += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }

                            node.children.push(compound_node);
                        }
                    }
                    "Default" => {
                        debug!("Обработка DEFAULT узла");
                        debug!("Содержимое DEFAULT: {:#?}", obj);

                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Обрабатываем операторы в default (stmts)
                        if let Some(stmts) = obj.get("stmts") {
                            debug!("Операторы DEFAULT: {:#?}", stmts);

                            // Создаем Compound узел для операторов
                            let mut compound_node = ASTNode::new("Compound");

                            if let Some(stmts_array) = stmts.as_array() {
                                for stmt in stmts_array {
                                    if let Ok(stmt_node) = self.parse_ast_node(stmt) {
                                        compound_node.children.push(stmt_node);
                                    }
                                }
                            } else {
                                // Может быть объект с полями stmts[0], stmts[1] и т.д.
                                let mut i = 0;
                                loop {
                                    let key = format!("stmts[{}]", i);
                                    if let Some(stmt) = obj.get(&key) {
                                        if let Ok(stmt_node) = self.parse_ast_node(stmt) {
                                            compound_node.children.push(stmt_node);
                                        }
                                        i += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }

                            node.children.push(compound_node);
                        }
                    }

                    // В методе parse_ast_node, в секции match node_type, добавьте обработку для "ArrayDecl" и "ArrayRef":
                    "ArrayDecl" => {
                        debug!("Обработка объявления массива");
                        debug!("Содержимое ArrayDecl: {:#?}", obj);

                        // Сохраняем имя массива
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Имя массива: {}", name);
                        }

                        // Обрабатываем тип элементов
                        if let Some(type_obj) = obj.get("type") {
                            debug!("Тип элементов массива: {:#?}", type_obj);
                            if let Ok(type_node) = self.parse_ast_node(type_obj) {
                                node.children.push(type_node);
                            }
                        }

                        // Обрабатываем размер массива (если указан)
                        if let Some(dim) = obj.get("dim") {
                            debug!("Размер массива: {:#?}", dim);
                            if let Ok(dim_node) = self.parse_ast_node(dim) {
                                node.children.push(dim_node);
                            }
                        }
                    }

                    // указатель
                    "PtrDecl" => {
                        debug!("Обработка объявления указателя");
                        debug!("Содержимое PtrDecl: {:#?}", obj);

                        // Сохраняем квалификаторы (const и т.д.)
                        if let Some(quals) = obj.get("quals") {
                            node.attributes.insert("quals".to_string(), quals.clone());
                        }

                        // Обрабатываем тип, на который указывает указатель
                        if let Some(type_obj) = obj.get("type") {
                            debug!("Тип, на который указывает указатель: {:#?}", type_obj);
                            if let Ok(type_node) = self.parse_ast_node(type_obj) {
                                node.children.push(type_node);
                            }
                        }
                    }

                    "InitList" => {
                        debug!("Обработка InitList");
                        debug!("Содержимое InitList: {:#?}", obj);

                        // В pycparser InitList может иметь поля вида "exprs[0]", "exprs[1]" и т.д.
                        // Собираем все значения в вектор, чтобы избежать дублирования
                        let mut values = std::collections::HashMap::new();

                        for (key, value) in obj {
                            if key.starts_with("exprs[") && key.ends_with(']') {
                                // Извлекаем индекс из ключа "exprs[0]" -> "0"
                                if let Some(index_str) =
                                    key.strip_prefix("exprs[").and_then(|s| s.strip_suffix(']'))
                                {
                                    if let Ok(index) = index_str.parse::<usize>() {
                                        debug!("Найдено поле {} с индексом {}", key, index);
                                        values.insert(index, value);
                                    }
                                }
                            }
                        }

                        // Сортируем по индексу и добавляем детей
                        let mut indices: Vec<_> = values.keys().collect();
                        indices.sort();

                        for index in indices {
                            if let Some(value) = values.get(index) {
                                debug!("Добавляем элемент с индексом {}: {:#?}", index, value);
                                if let Ok(expr_node) = self.parse_ast_node(value) {
                                    node.children.push(expr_node);
                                    debug!("Добавлен элемент инициализации");
                                }
                            }
                        }

                        // Также проверяем наличие поля "exprs" (может быть массивом в других версиях)
                        if node.children.is_empty() {
                            if let Some(exprs) = obj.get("exprs") {
                                debug!("Найдено поле exprs в InitList");
                                if let Some(exprs_array) = exprs.as_array() {
                                    debug!(
                                        "Количество выражений в InitList: {}",
                                        exprs_array.len()
                                    );
                                    for (i, expr) in exprs_array.iter().enumerate() {
                                        debug!("  InitList[{}]: {:#?}", i, expr);
                                        if let Ok(expr_node) = self.parse_ast_node(expr) {
                                            node.children.push(expr_node);
                                        }
                                    }
                                }
                            }
                        }

                        debug!(
                            "Итоговое количество детей в InitList: {}",
                            node.children.len()
                        );
                    }

                    // В методе parse_ast_node, в секции match node_type, добавьте после обработки "InitList":
                    "NamedInitializer" => {
                        debug!("Обработка NamedInitializer (именованного инициализатора)");
                        debug!("Содержимое NamedInitializer: {:#?}", obj);

                        // Обрабатываем имя поля (может быть в виде "name[0]" или просто "name")
                        let mut name_indices = Vec::new();
                        for (key, value) in obj {
                            if key.starts_with("name[") && key.ends_with(']') {
                                if let Some(index_str) =
                                    key.strip_prefix("name[").and_then(|s| s.strip_suffix(']'))
                                {
                                    if let Ok(index) = index_str.parse::<usize>() {
                                        name_indices.push((index, value));
                                    }
                                }
                            }
                        }

                        // Сортируем по индексу и сохраняем
                        name_indices.sort_by_key(|(i, _)| *i);
                        for (i, (_, value)) in name_indices.iter().enumerate() {
                            node.attributes
                                .insert(format!("name[{}]", i), (*value).clone());
                        }

                        // Если нет индексированных имен, проверяем прямое поле "name"
                        if name_indices.is_empty() {
                            if let Some(name) = obj.get("name") {
                                node.attributes.insert("name".to_string(), name.clone());
                            }
                        }

                        // Обрабатываем выражение
                        if let Some(expr) = obj.get("expr") {
                            node.attributes.insert("expr".to_string(), expr.clone());
                            if let Ok(expr_node) = self.parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }
                    }
                    "ArrayRef" => {
                        debug!("Обработка обращения к элементу массива");
                        debug!("Содержимое ArrayRef: {:#?}", obj);

                        // Имя массива
                        if let Some(name_obj) = obj.get("name") {
                            if let Ok(name_node) = self.parse_ast_node(name_obj) {
                                node.children.push(name_node);
                            }
                        }

                        // Индекс
                        if let Some(subscript) = obj.get("subscript") {
                            if let Ok(index_node) = self.parse_ast_node(subscript) {
                                node.children.push(index_node);
                            }
                        }
                    }

                    // В методе parse_ast_node, в секции match node_type добавьте:
                    "TernaryOp" => {
                        debug!("Обработка TernaryOp узла");
                        debug!("Содержимое TernaryOp: {:#?}", obj);

                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Обрабатываем условие (cond)
                        if let Some(cond) = obj.get("cond") {
                            debug!("Условие TernaryOp: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }

                        // Обрабатываем значение если true (iftrue)
                        if let Some(iftrue) = obj.get("iftrue") {
                            debug!("Значение если true: {:#?}", iftrue);
                            if let Ok(true_node) = self.parse_ast_node(iftrue) {
                                node.children.push(true_node);
                            }
                        }

                        // Обрабатываем значение если false (iffalse)
                        if let Some(iffalse) = obj.get("iffalse") {
                            debug!("Значение если false: {:#?}", iffalse);
                            if let Ok(false_node) = self.parse_ast_node(iffalse) {
                                node.children.push(false_node);
                            }
                        }
                    }
                    // В методе parse_ast_node, в секции match node_type, добавьте:
                    "Return" => {
                        debug!("Обработка RETURN узла");
                        debug!("Содержимое Return: {:#?}", obj);

                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Ищем выражение для возврата (может быть в поле "expr")
                        if let Some(expr) = obj.get("expr") {
                            debug!("Выражение return: {:#?}", expr);
                            if let Ok(expr_node) = self.parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }
                    }

                    "Compound" => {
                        debug!("Обработка Compound");

                        // В pycparser Compound может иметь поле "block_items" с массивом операторов
                        if let Some(block_items) = obj.get("block_items") {
                            debug!("Найдены block_items в Compound");
                            if let Some(items_array) = block_items.as_array() {
                                debug!("Количество операторов в Compound: {}", items_array.len());
                                for (i, item) in items_array.iter().enumerate() {
                                    debug!("  Оператор {} в Compound: {:#?}", i, item);
                                    if let Ok(stmt_node) = self.parse_ast_node(item) {
                                        node.children.push(stmt_node);
                                    }
                                }
                            }
                        } else {
                            // Альтернативный формат: могут быть отдельные поля
                            debug!("Ищем отдельные поля в Compound");
                            let mut i = 0;
                            loop {
                                let key = format!("block_items[{}]", i);
                                if let Some(item) = obj.get(&key) {
                                    debug!("Найдено поле {} в Compound", key);
                                    if let Ok(stmt_node) = self.parse_ast_node(item) {
                                        node.children.push(stmt_node);
                                    }
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                        }

                        debug!(
                            "Compound обработан, добавлено {} детей",
                            node.children.len()
                        );
                    }

                    // В методе parse_ast_node, в секции match node_type добавьте:
                    "While" => {
                        debug!("Обработка WHILE узла");
                        debug!("Содержимое WHILE: {:#?}", obj);

                        // Сохраняем координаты
                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Обрабатываем условие (cond)
                        if let Some(cond) = obj.get("cond") {
                            debug!("Условие WHILE: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }

                        // Обрабатываем тело (stmt)
                        if let Some(stmt) = obj.get("stmt") {
                            debug!("Тело WHILE: {:#?}", stmt);
                            if let Ok(body_node) = self.parse_ast_node(stmt) {
                                node.children.push(body_node);
                            }
                        }
                    }

                    // В методе parse_ast_node, в секции match node_type добавьте:
                    "DoWhile" => {
                        debug!("Обработка DO-WHILE узла");
                        debug!("Содержимое DO-WHILE: {:#?}", obj);

                        if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                            node.coord = Some(coord.to_string());
                        }

                        // Тело цикла (stmt)
                        if let Some(stmt) = obj.get("stmt") {
                            debug!("Тело DO-WHILE: {:#?}", stmt);
                            if let Ok(body_node) = self.parse_ast_node(stmt) {
                                node.children.push(body_node);
                            }
                        }

                        // Условие (cond)
                        if let Some(cond) = obj.get("cond") {
                            debug!("Условие DO-WHILE: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }
                    }

                    // В методе parse_ast_node, в секции match node_type добавьте:
                    "For" => {
                        debug!("Обработка FOR узла");
                        debug!("ПОЛНОЕ содержимое FOR узла: {:#?}", obj);

                        // Сохраняем все атрибуты для отладки
                        for (key, value) in obj {
                            debug!("  FOR атрибут {}: {:#?}", key, value);
                        }

                        // Обрабатываем все поля как детей
                        if let Some(init) = obj.get("init") {
                            debug!("FOR init: {:#?}", init);
                            if let Ok(init_node) = self.parse_ast_node(init) {
                                node.children.push(init_node);
                            }
                        }

                        if let Some(cond) = obj.get("cond") {
                            debug!("FOR cond: {:#?}", cond);
                            if let Ok(cond_node) = self.parse_ast_node(cond) {
                                node.children.push(cond_node);
                            }
                        }

                        if let Some(next) = obj.get("next") {
                            debug!("FOR next: {:#?}", next);
                            if let Ok(next_node) = self.parse_ast_node(next) {
                                node.children.push(next_node);
                            }
                        }

                        if let Some(stmt) = obj.get("stmt") {
                            debug!("FOR stmt: {:#?}", stmt);
                            if let Ok(stmt_node) = self.parse_ast_node(stmt) {
                                node.children.push(stmt_node);
                            }
                        }
                    }

                    "Struct" => {
                        debug!("Обработка STRUCT узла");
                        debug!("ПОЛНОЕ содержимое STRUCT: {:#?}", obj);

                        // Сохраняем имя структуры
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Имя структуры: {}", name);
                        }

                        // Сохраняем всё JSON представление для дальнейшего использования
                        node.attributes
                            .insert("_json".to_string(), Value::Object(obj.clone()));

                        // Обрабатываем поля структуры (decls)
                        if let Some(decls) = obj.get("decls") {
                            debug!("Поля структуры: {:#?}", decls);

                            if let Some(decls_array) = decls.as_array() {
                                for decl in decls_array {
                                    if let Ok(decl_node) = self.parse_ast_node(decl) {
                                        node.children.push(decl_node);
                                    }
                                }
                            }
                        }
                    }
                    "StructDecl" => {
                        debug!("Обработка STRUCT DECL (объявление переменной типа структуры)");

                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                        }

                        if let Some(type_obj) = obj.get("type") {
                            if let Ok(type_node) = self.parse_ast_node(type_obj) {
                                node.children.push(type_node);
                            }
                        }
                    }

                    "StructRef" => {
                        debug!("Обработка доступа к полю структуры");

                        // Имя структуры (обычно ID)
                        if let Some(name_obj) = obj.get("name") {
                            if let Ok(name_node) = self.parse_ast_node(name_obj) {
                                node.children.push(name_node);
                            }
                        }

                        // Имя поля
                        if let Some(field) = obj.get("field") {
                            if let Ok(field_node) = self.parse_ast_node(field) {
                                node.children.push(field_node);
                            }
                        }
                    }

                    // Добавьте этот блок в match после обработки "Decl" или перед "_"
                    "TypeDecl" => {
                        debug!("Обработка TypeDecl узла");
                        debug!("Содержимое TypeDecl: {:#?}", obj);

                        // Сохраняем имя объявления (имя переменной/параметра)
                        if let Some(declname) = obj.get("declname").and_then(|v| v.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(declname.to_string()));
                            debug!("Имя в TypeDecl: {}", declname);
                        }

                        // ВАЖНО: Сохраняем информацию о типе из вложенного узла
                        if let Some(type_obj) = obj.get("type") {
                            debug!("Тип в TypeDecl: {:#?}", type_obj);

                            // Парсим вложенный тип как отдельный узел и добавляем как ребенка
                            if let Ok(type_node) = self.parse_ast_node(type_obj) {
                                node.children.push(type_node);
                            }

                            // Также сохраняем JSON представление типа как атрибут
                            // Это критически важно для определения структурных переменных!
                            node.attributes.insert("type".to_string(), type_obj.clone());

                            // Проверяем, является ли тип структурой
                            if let Some(type_obj_map) = type_obj.as_object() {
                                if let Some(node_type) =
                                    type_obj_map.get("__node__").and_then(|n| n.as_str())
                                {
                                    if node_type == "Struct" {
                                        debug!("TypeDecl указывает на структуру!");
                                        if let Some(struct_name) =
                                            type_obj_map.get("name").and_then(|n| n.as_str())
                                        {
                                            node.attributes.insert(
                                                "struct_type".to_string(),
                                                Value::String(struct_name.to_string()),
                                            );
                                            debug!("Имя структуры: {}", struct_name);
                                        }
                                    } else if node_type == "Union" {
                                        if let Some(union_name) =
                                            type_obj_map.get("name").and_then(|n| n.as_str())
                                        {
                                            node.attributes.insert(
                                                "union_type".to_string(),
                                                Value::String(union_name.to_string()),
                                            );
                                            debug!("Имя объединения: {}", union_name);
                                        }
                                    }
                                }
                            }
                        }

                        // Сохраняем квалификаторы типа (const, volatile и т.д.)
                        if let Some(quals) = obj.get("quals") {
                            node.attributes.insert("quals".to_string(), quals.clone());
                        }
                    }

                    // В методе parse_ast_node, в секции "Union":
                    "Union" => {
                        debug!("Обработка UNION узла");
                        debug!("ПОЛНОЕ содержимое UNION: {:#?}", obj);

                        // Сохраняем имя объединения
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Имя объединения: {}", name);
                        }

                        // Сохраняем всё JSON представление для дальнейшего использования
                        node.attributes
                            .insert("_json".to_string(), Value::Object(obj.clone()));

                        // Сохраняем информацию о полях для вложенных структур
                        let mut field_types = serde_json::Map::new();

                        // Обрабатываем поля объединения (decls)
                        if let Some(decls) = obj.get("decls") {
                            debug!("Поля объединения: {:#?}", decls);

                            if let Some(decls_array) = decls.as_array() {
                                for decl in decls_array {
                                    if let Ok(decl_node) = self.parse_ast_node(decl) {
                                        // Сохраняем информацию о типе поля
                                        if let Some(field_name) = decl_node
                                            .attributes
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                        {
                                            if let Some(type_attr) =
                                                decl_node.attributes.get("type")
                                            {
                                                field_types.insert(
                                                    field_name.to_string(),
                                                    type_attr.clone(),
                                                );
                                            }
                                        }
                                        node.children.push(decl_node);
                                    }
                                }
                            }
                        }

                        if !field_types.is_empty() {
                            node.attributes
                                .insert("field_types".to_string(), Value::Object(field_types));
                        }
                    }
                    "UnionDecl" => {
                        debug!("Обработка UNION DECL (объявление переменной типа объединения)");

                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                        }

                        if let Some(type_obj) = obj.get("type") {
                            if let Ok(type_node) = self.parse_ast_node(type_obj) {
                                node.children.push(type_node);
                            }
                        }
                    }

                    "Enum" => {
                        debug!("Обработка ENUM узла");
                        debug!("ПОЛНОЕ содержимое ENUM: {:#?}", obj);

                        // Сохраняем имя перечисления
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Имя перечисления: {}", name);
                        }

                        // Сохраняем всё JSON представление для дальнейшего использования
                        node.attributes
                            .insert("_json".to_string(), Value::Object(obj.clone()));

                        // Обрабатываем значения перечисления (values)
                        // В pycparser это может быть массив или объект с полями
                        if let Some(values) = obj.get("values") {
                            debug!("Значения enum: {:#?}", values);

                            if let Some(values_array) = values.as_array() {
                                for value in values_array {
                                    if let Ok(enumerator_node) = self.parse_ast_node(value) {
                                        node.children.push(enumerator_node);
                                    }
                                }
                            }
                        }
                    }

                    "Enumerator" => {
                        debug!("Обработка ENUMERATOR узла");
                        debug!("Содержимое ENUMERATOR: {:#?}", obj);

                        // Сохраняем имя элемента перечисления
                        if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                            node.attributes
                                .insert("name".to_string(), Value::String(name.to_string()));
                            debug!("Имя элемента enum: {}", name);
                        }

                        // Сохраняем значение, если указано явно (например, RED=5)
                        if let Some(value) = obj.get("value") {
                            node.attributes.insert("value".to_string(), value.clone());
                            debug!("Значение элемента enum: {:?}", value);
                        }
                    }

                    _ => {
                        // Для остальных узлов просто копируем все атрибуты
                        debug!("Обработка узла типа: {}", node_type);
                    }
                }

                // Обрабатываем все остальные поля для детей
                for (key, val) in obj {
                    if key == "__node__"
        || key == "coord"
        || key == "name"
        || key == "type"
        || key == "decl"
        || key == "params"
        || key == "init"
        || key == "args"
        || key == "cond"        // Добавлено для While и If
        || key == "iftrue"
        || key == "iffalse"
        || key == "stmt"        // Добавлено для While
        || key == "value"
        || key == "stmts"
        || key == "expr"
        || key == "dim"
        || key == "subscript"
        || key.starts_with("exprs")
        || key.starts_with("block_items")
        || key.starts_with("params[")
                    {
                        continue;
                    }

                    // Если значение - объект с __node__, это дочерний узел
                    if val.is_object() {
                        if val.get("__node__").is_some() {
                            // Добавляем проверку, чтобы не добавлять уже существующие узлы
                            let new_node = self.parse_ast_node(val)?;

                            // Проверяем, не добавлен ли уже такой узел
                            let mut already_exists = false;
                            for child in &node.children {
                                if child.node_type == new_node.node_type
                                    && child.coord == new_node.coord
                                {
                                    already_exists = true;
                                    debug!("Предотвращено дублирование узла типа {} с координатами {:?}", 
                           child.node_type, child.coord);
                                    break;
                                }
                            }

                            if !already_exists {
                                node.children.push(new_node);
                            }
                        }
                    }
                    // Если значение - массив, проверяем элементы
                    else if val.is_array() {
                        if let Some(arr) = val.as_array() {
                            for item in arr {
                                if item.is_object() && item.get("__node__").is_some() {
                                    let new_node = self.parse_ast_node(item)?;

                                    // Проверяем, не добавлен ли уже такой узел
                                    let mut already_exists = false;
                                    for child in &node.children {
                                        if child.node_type == new_node.node_type
                                            && child.coord == new_node.coord
                                        {
                                            already_exists = true;
                                            break;
                                        }
                                    }

                                    if !already_exists {
                                        node.children.push(new_node);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(node)
            } else {
                // Обычный объект (не узел)
                Ok(ASTNode::new("Object").with_attr("value", value.clone()))
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
            Ok(ASTNode::new("Value").with_attr("value", value.clone()))
        }
    }

    /// Извлекает параметры из узла ParamList и добавляет их в узел функции
    fn extract_params_from_paramlist(
        &self,
        paramlist_obj: &serde_json::Map<String, Value>,
        node: &mut ASTNode,
    ) -> Result<()> {
        let mut param_names = Vec::new();
        let mut param_nodes = Vec::new();

        // Собираем все параметры (они могут быть с ключами params[0], params[1] и т.д.)
        for (key, value) in paramlist_obj {
            if key.starts_with("params[") || key == "params" {
                debug!("Обработка параметра с ключом {}: {:#?}", key, value);

                if let Some(param_obj) = value.as_object() {
                    // Сохраняем узел параметра
                    if let Ok(param_node) = self.parse_ast_node(value) {
                        // Извлекаем имя параметра перед добавлением в param_nodes
                        if let Some(param_name) = param_obj.get("name").and_then(|n| n.as_str()) {
                            param_names.push(Value::String(param_name.to_string()));
                            debug!("Найдено имя параметра: {}", param_name);
                        }
                        param_nodes.push(param_node);
                    }
                }
            }
        }

        // Сохраняем параметры как атрибуты
        if !param_names.is_empty() {
            node.attributes
                .insert("param_names".to_string(), Value::Array(param_names));
            debug!("Добавлен атрибут param_names");
        }

        if !param_nodes.is_empty() {
            let mut param_list_node = ASTNode::new("ParamList");
            // Клонируем param_nodes для использования здесь
            param_list_node.children = param_nodes.clone();
            node.children.push(param_list_node);
            debug!("Добавлен ParamList с {} параметрами", param_nodes.len());
        }

        Ok(())
    }

    /// Асинхронный метод парсинга
    pub async fn parse_async(code: &str) -> Result<ASTNode> {
        let parser = Self::new();

        if !parser.docker_available {
            return Err(anyhow!(
                "Docker не доступен. Запустите Docker и перезапустите приложение."
            ));
        }

        // Пробуем сначала HTTP API (если контейнер уже запущен)
        match parser.use_http_api(code).await {
            Ok(response) => {
                info!("Использован HTTP API парсера");
                parser.convert_json_to_ast(&response)
            }
            Err(http_err) => {
                // Если HTTP не работает, запускаем разовый контейнер
                info!(
                    "HTTP API не доступен ({}), запускаем разовый контейнер",
                    http_err
                );

                match parser.run_docker_container_clean(code).await {
                    Ok(response) => {
                        info!("Docker контейнер успешно выполнен");
                        parser.convert_json_to_ast(&response)
                    }
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
    fn parse(_code: &str) -> Result<ASTNode> {
        Err(anyhow!("Используйте parse_async в асинхронном контексте"))
    }

    fn is_available() -> bool {
        Self::check_docker()
    }
}
