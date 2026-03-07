use anyhow::Result;
use log::{debug, warn};
use std::collections::HashSet;

use super::Generator;
use crate::ast::ASTNode;

pub struct PythonGenerator {
    indent_level: usize,
    indent_size: usize,
    symbols: HashSet<String>,
    in_expression: bool,
    current_struct_type: Option<String>,
    struct_info: std::collections::HashMap<String, Vec<String>>,
    enum_info: std::collections::HashMap<String, Vec<String>>,
    // Новое: отслеживание переменных-указателей
    pointer_vars: HashSet<String>,
    // Новое: отслеживание типов указателей (имя -> тип элемента)
    pointer_types: std::collections::HashMap<String, String>,
    // Новое: отслеживание адресов переменных
    address_taken: HashSet<String>,
    array_vars: HashSet<String>,
}

impl PythonGenerator {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            indent_size: 4,
            symbols: HashSet::new(),
            in_expression: false,
            current_struct_type: None,
            struct_info: std::collections::HashMap::new(),
            enum_info: std::collections::HashMap::new(),
            pointer_vars: HashSet::new(),
            pointer_types: std::collections::HashMap::new(),
            address_taken: HashSet::new(),
            array_vars: HashSet::new(),
        }
    }

    /// Возвращает текущий отступ
    fn indent(&self) -> String {
        " ".repeat(self.indent_level * self.indent_size)
    }

    /// Добавляет отступ и строку
    fn line(&self, content: &str) -> String {
        format!("{}{}\n", self.indent(), content)
    }

    /// Генерирует код функции
    fn generate_function(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        // Получаем имя функции из разных мест
        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация функции: {}", name);

        // ОТЛАДКА: выводим все атрибуты узла
        debug!("Атрибуты узла {}: {:#?}", name, node.attributes);

        // ОТЛАДКА: выводим всех детей узла
        debug!("Дети узла {} в функции:", name);
        for (i, child) in node.children.iter().enumerate() {
            debug!("  Дитя {}: тип={}", i, child.node_type);
            if child.node_type == "Compound" {
                debug!("    Compound дети:");
                for (j, stmt) in child.children.iter().enumerate() {
                    debug!("      Оператор {}: тип={}", j, stmt.node_type);
                }
            }
        }

        // Собираем параметры функции
        let mut params = Vec::new();

        // 1. Проверяем атрибут param_names (добавлен в c_parser)
        if let Some(param_names) = node.attributes.get("param_names") {
            debug!("Найден атрибут param_names: {:#?}", param_names);
            if let Some(param_array) = param_names.as_array() {
                for param in param_array {
                    if let Some(param_name) = param.as_str() {
                        debug!("Найден параметр в param_names: {}", param_name);
                        params.push(param_name.to_string());
                        self.symbols.insert(param_name.to_string());
                    }
                }
            }
        }

        // 2. Проверяем атрибут params
        if params.is_empty() {
            if let Some(params_attr) = node.attributes.get("params") {
                debug!("Найден атрибут params: {:#?}", params_attr);
                if let Some(params_array) = params_attr.as_array() {
                    for param in params_array {
                        if let Some(param_obj) = param.as_object() {
                            if let Some(param_name) = param_obj.get("name").and_then(|v| v.as_str())
                            {
                                debug!("Найден параметр в attributes.params: {}", param_name);
                                params.push(param_name.to_string());
                                self.symbols.insert(param_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 3. Ищем параметры в детях (для FuncDef)
        if params.is_empty() {
            for child in &node.children {
                if child.node_type == "ParamList" {
                    debug!("Найден ParamList с {} детьми", child.children.len());
                    for param in &child.children {
                        if let Some(param_name) = self.extract_param_name(param) {
                            debug!("Найден параметр в ParamList: {}", param_name);
                            params.push(param_name.clone());
                            self.symbols.insert(param_name);
                        }
                    }
                }
            }
        }

        // 4. Ищем параметры в детях типа "Decl" (объявления переменных могут быть параметрами)
        if params.is_empty() {
            for child in &node.children {
                if child.node_type == "Decl" {
                    if let Some(param_name) = child.attributes.get("name").and_then(|v| v.as_str())
                    {
                        debug!("Найден параметр в Decl: {}", param_name);
                        params.push(param_name.to_string());
                        self.symbols.insert(param_name.to_string());
                    }
                }
            }
        }

        debug!("Параметры функции {}: {:?}", name, params);

        // Генерируем определение функции
        output.push_str(&self.line(&format!("def {}({}):", name, params.join(", "))));

        self.indent_level += 1;

        // Генерируем тело функции - просто проходим по всем операторам в порядке их следования
        let mut _has_return = false;
        let mut has_body = false;
        let mut body_output = String::new();

        for child in &node.children {
            if child.node_type == "Compound" {
                has_body = true;
                debug!("Обработка Compound с {} детьми", child.children.len());
                for (i, stmt) in child.children.iter().enumerate() {
                    debug!("  Оператор {} в Compound: тип={}", i, stmt.node_type);
                    let stmt_code = self.generate_statement(stmt)?;
                    body_output.push_str(&stmt_code);
                    if stmt.node_type == "Return" {
                        _has_return = true;
                    }
                }
            }
        }

        // Добавляем тело функции
        output.push_str(&body_output);

        // Если нет тела или тело пустое, добавляем pass
        if !has_body {
            output.push_str(&self.line("    pass"));
        } else if body_output.trim().is_empty() {
            output.push_str(&self.line("    pass"));
        }

        self.indent_level -= 1;
        output.push_str(&self.line(""));

        Ok(output)
    }

    // Извлекает имя параметра из узла
    fn extract_param_name(&self, node: &ASTNode) -> Option<String> {
        // Прямой атрибут name
        if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
            return Some(name.to_string());
        }

        // Проверяем детей
        for child in &node.children {
            if child.node_type == "ID" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    return Some(name.to_string());
                }
            }
        }

        None
    }

    /// Генерирует оператор
    fn generate_statement(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация оператора типа: {}", node.node_type);
        match node.node_type.as_str() {
            "Return" => self.generate_return(node),
            "If" => self.generate_if(node),
            "While" => self.generate_while(node),
            "DoWhile" => self.generate_dowhile(node),
            "For" => self.generate_for(node),
            "Assignment" => self.generate_assignment(node),
            "Decl" => {
                // Проверяем, является ли это объявлением массива
                let mut is_array = false;
                for child in &node.children {
                    if child.node_type == "ArrayDecl" {
                        is_array = true;
                        break;
                    }
                }

                if is_array {
                    // Ищем узел ArrayDecl среди детей
                    for child in &node.children {
                        if child.node_type == "ArrayDecl" {
                            return self.generate_array_decl(child);
                        }
                    }
                    Ok(String::new())
                } else {
                    self.generate_declaration(node)
                }
            }
            "Switch" => self.generate_switch(node),
            "FuncCall" => {
                let expr = self.generate_expression(node)?;
                Ok(self.line(&expr))
            }
            "UnaryOp" => {
                // Для унарных операторов как операторов (например, *ptr = 5)
                if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
                    if op == "*" && node.children.len() == 1 {
                        // Это разыменование указателя как левая часть присваивания
                        // Будет обработано в Assignment
                        return self.generate_unary_stmt(node);
                    }
                }
                self.generate_unary_stmt(node)
            }
            "Compound" => {
                let mut output = String::new();

                // Просто генерируем операторы в том порядке, в котором они идут в AST
                for stmt in &node.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }

                Ok(output)
            }
            "Break" => Ok(self.line("break")),
            "Continue" => Ok(self.line("continue")),
            "Dowhile" => self.generate_dowhile(node),
            "DeclList" => {
                let mut output = String::new();
                for decl in &node.children {
                    output.push_str(&self.generate_statement(decl)?);
                }
                Ok(output)
            }
            "Struct" => self.generate_struct(node),
            "StructDecl" => self.generate_struct_decl(node),
            "Union" => self.generate_union(node),
            "UnionDecl" => self.generate_union_decl(node),
            _ => {
                // Пытаемся обработать как выражение
                let expr = self.generate_expression(node)?;
                if !expr.is_empty() && expr != "None" {
                    Ok(self.line(&expr))
                } else {
                    debug!("Пропуск неизвестного оператора: {}", node.node_type);
                    Ok(String::new())
                }
            }
        }
    }

    /// Генерирует do-while цикл
    fn generate_dowhile(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация DO-WHILE цикла");
        debug!("Детей у do-while: {}", node.children.len());

        if node.children.len() >= 2 {
            // Первый ребенок - тело цикла
            let body = &node.children[0];
            // Второй ребенок - условие
            let condition = &node.children[1];

            // Генерируем условие
            let cond_code = self.generate_expression(condition)?;
            debug!("Условие do-while: {}", cond_code);

            // Генерируем Python код: while True: тело; if not условие: break
            output.push_str(&self.line("while True:"));

            self.indent_level += 1;

            // Генерируем тело цикла
            if body.node_type == "Compound" {
                for stmt in &body.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }
            } else {
                output.push_str(&self.generate_statement(body)?);
            }

            // Добавляем проверку условия для выхода
            output.push_str(&self.line(&format!("if not ({}):", cond_code)));

            self.indent_level += 1;
            output.push_str(&self.line("break"));
            self.indent_level -= 1;

            self.indent_level -= 1;
        } else {
            debug!("do-while имеет недостаточно детей: {}", node.children.len());
        }

        Ok(output)
    }

    /// Генерирует выражение
    fn generate_expression(&mut self, node: &ASTNode) -> Result<String> {
        self.in_expression = true;
        let result = self.generate_expression_internal(node);
        self.in_expression = false;
        result
    }

    /// Внутренняя рекурсивная генерация выражения
    fn generate_expression_internal(&mut self, node: &ASTNode) -> Result<String> {
        match node.node_type.as_str() {
            // В методе generate_expression_internal, добавьте обработку для строковых литералов
            "Constant" => {
                if let Some(value) = node.attributes.get("value") {
                    if let Some(s) = value.as_str() {
                        // Проверяем, является ли это строковым литералом в двойных кавычках
                        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                            // Это строковый литерал
                            Ok(s.to_string())
                        }
                        // Проверяем, является ли это символом в одинарных кавычках
                        else if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                            // Это символ, оставляем как есть
                            Ok(s.to_string())
                        }
                        // Проверяем, является ли строка числом
                        else if s.parse::<i32>().is_ok() || s.parse::<f64>().is_ok() {
                            Ok(s.to_string())
                        } else {
                            // Это идентификатор или что-то другое
                            Ok(s.to_string())
                        }
                    } else if let Some(n) = value.as_i64() {
                        Ok(n.to_string())
                    } else if let Some(n) = value.as_f64() {
                        Ok(n.to_string())
                    } else if let Some(b) = value.as_bool() {
                        Ok(b.to_string())
                    } else {
                        Ok(value.to_string())
                    }
                } else {
                    Ok("None".to_string())
                }
            }
            "ID" => {
                if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    // ПРОВЕРЯЕМ, НЕ ЯВЛЯЕТСЯ ЛИ ЭТО ЗНАЧЕНИЕМ ENUM
                    if let Some(enum_name) = self.is_enum_value(name) {
                        // Если это enum значение, квалифицируем его именем класса
                        Ok(format!("{}.{}", enum_name, name))
                    } else {
                        // Обычный идентификатор
                        Ok(name.to_string())
                    }
                } else {
                    Ok("unknown".to_string())
                }
            }

            "BinaryOp" => {
                let op = node
                    .attributes
                    .get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                // Специальная обработка для арифметики указателей
                if op == "+" && node.children.len() == 2 {
                    let left = &node.children[0];
                    let right = &node.children[1];

                    // Если левая часть - идентификатор и это указатель
                    if left.node_type == "ID" {
                        if let Some(var_name) = left.attributes.get("name").and_then(|v| v.as_str())
                        {
                            if self.pointer_vars.contains(var_name) {
                                // Это ptr + i
                                let index = self.generate_expression_internal(right)?;

                                // Если это в контексте разыменования, вернем как есть
                                // Иначе вернем как выражение для индексации
                                if self.in_expression {
                                    return Ok(format!("{}[{}]", var_name, index));
                                } else {
                                    return Ok(format!("{} + {}", var_name, index));
                                }
                            }
                        }
                    }
                }

                // Обычная бинарная операция
                let py_op = match op {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "%" => op,
                    "&&" => "and",
                    "||" => "or",
                    _ => op,
                };

                let left = if let Some(child) = node.children.get(0) {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                let right = if let Some(child) = node.children.get(1) {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                debug!("Бинарная операция: {} {} {}", left, py_op, right);

                let left_with_parens = self.wrap_if_needed(&left, node.children.get(0));
                let right_with_parens = self.wrap_if_needed(&right, node.children.get(1));

                Ok(format!(
                    "{} {} {}",
                    left_with_parens, py_op, right_with_parens
                ))
            }

            "UnaryOp" => {
                let op = node
                    .attributes
                    .get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                debug!("Унарная операция: {} с {} детьми", op, node.children.len());

                // Сначала получаем выражение для операнда
                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                // Вспомогательная функция для подсчета уровней разыменования
                fn count_dereferences(node: &ASTNode) -> usize {
                    if node.node_type == "UnaryOp" {
                        if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
                            if op == "*" && !node.children.is_empty() {
                                return 1 + count_dereferences(&node.children[0]);
                            }
                        }
                    }
                    0
                }

                match op {
                    "&" => {
                        debug!("Операция взятия адреса: &{}", expr);
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    self.address_taken.insert(var_name.to_string());
                                }
                            }
                        }
                        Ok(format!("Reference({})", expr))
                    }
                    "*" => {
                        debug!("Разыменование указателя: *{}", expr);

                        // Подсчитываем количество разыменований
                        let deref_count = count_dereferences(node);

                        if let Some(child) = node.children.first() {
                            // Случай: *(ptr + i) - арифметика указателей
                            if child.node_type == "BinaryOp" {
                                if let Some(op) =
                                    child.attributes.get("op").and_then(|v| v.as_str())
                                {
                                    if op == "+" && child.children.len() == 2 {
                                        let left = &child.children[0];
                                        let right = &child.children[1];

                                        if left.node_type == "ID" {
                                            if let Some(var_name) =
                                                left.attributes.get("name").and_then(|v| v.as_str())
                                            {
                                                let index =
                                                    self.generate_expression_internal(right)?;
                                                // ВАЖНО: преобразуем *(ptr + i) в ptr[i]
                                                return Ok(format!("{}[{}]", var_name, index));
                                            }
                                        }
                                    }
                                }
                            }

                            // Случай: *ptr - простое разыменование
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    // Проверяем, является ли это указателем на массив
                                    if self.pointer_vars.contains(var_name)
                                        && self.array_vars.contains(var_name)
                                    {
                                        // Для указателя на массив *ptr эквивалентно ptr[0]
                                        return Ok(format!("{}[0]", var_name));
                                    }

                                    // Множественное разыменование **ptr
                                    let mut result = var_name.to_string();
                                    for _ in 0..deref_count {
                                        result = format!("{}.value", result);
                                    }
                                    return Ok(result);
                                }
                            }
                        }

                        Ok(format!("{}.value", expr))
                    }
                    "++" | "p++" | "post++" => {
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    if self.pointer_vars.contains(var_name) {
                                        // Для указателей: ptr = ptr + 1 (арифметика указателей)
                                        return Ok(format!("{} + 1", var_name));
                                    }
                                }
                            }
                        }
                        Ok(format!("({} + 1)", expr))
                    }
                    "--" | "p--" | "post--" => {
                        if let Some(child) = node.children.first() {
                            if child.node_type == "ID" {
                                if let Some(var_name) =
                                    child.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    if self.pointer_vars.contains(var_name) {
                                        return Ok(format!("{} - 1", var_name));
                                    }
                                }
                            }
                        }
                        Ok(format!("({} - 1)", expr))
                    }
                    "-" => {
                        if expr.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            Ok(format!("-{}", expr))
                        } else if expr.starts_with('-') {
                            Ok(expr)
                        } else {
                            Ok(format!("-({})", expr))
                        }
                    }
                    "+" => Ok(format!("+{}", expr)),
                    "!" => Ok(format!("not {}", expr)),
                    _ => {
                        debug!("Неизвестный унарный оператор: {}", op);
                        Ok(expr)
                    }
                }
            }
            "FuncCall" => {
                // Ищем имя функции
                let mut name = None;
                let mut args_exprs = Vec::new();

                // Проверяем прямой атрибут name
                if let Some(name_attr) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    name = Some(name_attr.to_string());
                }

                // Обрабатываем детей
                for child in &node.children {
                    match child.node_type.as_str() {
                        "ID" => {
                            if let Some(id_name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                // Это имя функции
                                if name.is_none() {
                                    name = Some(id_name.to_string());
                                }
                            }
                        }
                        "ExprList" => {
                            // Это список аргументов
                            debug!("Обработка ExprList с {} детьми", child.children.len());

                            // Проходим по всем детям ExprList
                            for arg in &child.children {
                                let arg_expr = self.generate_expression_internal(arg)?;
                                if !arg_expr.is_empty() && arg_expr != "None" {
                                    debug!("Аргумент: {}", arg_expr);
                                    args_exprs.push(arg_expr);
                                }
                            }
                        }
                        _ => {
                            debug!("Игнорируем ребенка типа {} в FuncCall", child.node_type);
                        }
                    }
                }

                let name = name.unwrap_or_else(|| {
                    warn!("Не удалось определить имя функции");
                    "unknown".to_string()
                });

                debug!("Вызов функции: {} с аргументами: {:?}", name, args_exprs);
                Ok(format!("{}({})", name, args_exprs.join(", ")))
            }

            "ExprList" => {
                // Это отдельное выражение (не как аргумент функции)
                let mut exprs = Vec::new();
                debug!(
                    "Генерация ExprList как отдельного выражения с {} детьми",
                    node.children.len()
                );

                for child in &node.children {
                    let expr = self.generate_expression_internal(child)?;
                    if !expr.is_empty() {
                        debug!("  Выражение в ExprList: {}", expr);
                        exprs.push(expr);
                    }
                }
                Ok(exprs.join(", "))
            }

            "ArrayRef" => self.generate_array_ref(node),

            "InitList" => {
                let mut values = Vec::new();
                let node_id = node.coord.as_deref().unwrap_or("unknown");

                debug!(
                    "Генерация InitList (id: {}) с {} детьми, контекст структуры: {:?}",
                    node_id,
                    node.children.len(),
                    self.current_struct_type
                );

                for (i, child) in node.children.iter().enumerate() {
                    if child.node_type == "InitList" {
                        debug!("InitList содержит другой InitList как ребенка!");
                        let nested_values = self.generate_expression_internal(child)?;
                        values.push(nested_values);
                    } else if child.node_type == "NamedInitializer" {
                        // Обрабатываем именованные инициализаторы (C99 designated initializers)
                        if let Some(expr) = child.attributes.get("expr") {
                            if let Some(_expr_value) = expr.as_object() {
                                // Парсим выражение из атрибута
                                let temp_node =
                                    ASTNode::new("Value").with_attr("value", expr.clone());
                                let value = self.generate_expression_internal(&temp_node)?;
                                values.push(value);
                            } else {
                                values.push("None".to_string());
                            }
                        } else {
                            values.push("None".to_string());
                        }
                    } else {
                        let value = self.generate_expression_internal(child)?;
                        debug!(
                            "  InitList {} ребенок {}: тип={}, значение={}",
                            node_id, i, child.node_type, value
                        );
                        values.push(value);
                    }
                }

                debug!(
                    "InitList {} итоговые значения ({} шт): {:?}",
                    node_id,
                    values.len(),
                    values
                );

                // Если это инициализация структуры, создаем вызов конструктора класса
                if let Some(struct_name) = &self.current_struct_type {
                    // Для структур возвращаем вызов конструктора с параметрами
                    // Например: Point(30, 40)
                    Ok(format!("{}({})", struct_name, values.join(", ")))
                } else {
                    // Иначе возвращаем как список (для массивов)
                    Ok(format!("[{}]", values.join(", ")))
                }
            }
            "TernaryOp" => {
                debug!("Генерация TernaryOp");

                // В Python тернарный оператор имеет синтаксис: value_if_true if condition else value_if_false
                // В C: condition ? value_if_true : value_if_false
                // Поэтому порядок детей: [cond, iftrue, iffalse]

                if node.children.len() >= 3 {
                    let cond = self.generate_expression_internal(&node.children[0])?;
                    let iftrue = self.generate_expression_internal(&node.children[1])?;
                    let iffalse = self.generate_expression_internal(&node.children[2])?;

                    debug!(
                        "Тернарный оператор: cond={}, true={}, false={}",
                        cond, iftrue, iffalse
                    );
                    Ok(format!("{} if {} else {}", iftrue, cond, iffalse))
                } else {
                    debug!(
                        "TernaryOp имеет недостаточно детей: {}",
                        node.children.len()
                    );
                    Ok("None".to_string())
                }
            }

            "StructRef" => self.generate_struct_ref(node),
            "StructDecl" => {
                // Если StructDecl используется как выражение (например, в инициализации)
                self.generate_struct_decl(node)
            }

            "Cast" => {
                debug!("Обработка приведения типа");

                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                // В Python приведение типов обычно не нужно, но можно добавить
                // аннотации или преобразования для特定ных случаев
                if let Some(to_type) = node.attributes.get("to_type") {
                    if let Some(type_str) = to_type.as_str() {
                        // Для числовых типов используем соответствующие функции
                        match type_str {
                            "int" => Ok(format!("int({})", expr)),
                            "float" => Ok(format!("float({})", expr)),
                            "double" => Ok(format!("float({})", expr)),
                            "char" => Ok(format!(
                                "chr({}) if isinstance({}, int) else {}",
                                expr, expr, expr
                            )),
                            _ => Ok(expr),
                        }
                    } else {
                        Ok(expr)
                    }
                } else {
                    Ok(expr)
                }
            }

            "PtrDecl" => {
                // Обработка объявления указателя в выражении
                // Обычно это не используется напрямую в выражениях
                Ok("None".to_string())
            }
            _ => {
                debug!("Неизвестное выражение: {}", node.node_type);
                Ok("None".to_string())
            }
        }
    }

    /// Генерирует унарный оператор как оператор
    fn generate_unary_stmt(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация унарного оператора как стейтмента");

        if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
            if let Some(child) = node.children.first() {
                // Для операций с переменными
                if child.node_type == "ID" {
                    if let Some(var_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                        match op {
                            "++" | "p++" | "post++" => {
                                if self.pointer_vars.contains(var_name) {
                                    // Для указателей: ptr = ptr + 1 (арифметика указателей)
                                    return Ok(
                                        self.line(&format!("{} = {} + 1", var_name, var_name))
                                    );
                                } else {
                                    return Ok(self.line(&format!("{} += 1", var_name)));
                                }
                            }
                            "--" | "p--" | "post--" => {
                                if self.pointer_vars.contains(var_name) {
                                    return Ok(
                                        self.line(&format!("{} = {} - 1", var_name, var_name))
                                    );
                                } else {
                                    return Ok(self.line(&format!("{} -= 1", var_name)));
                                }
                            }
                            "*" => {
                                // Это разыменование указателя как выражение
                                // В Python это будет обработано в Assignment
                                return Ok(self.line(&format!("{}", var_name)));
                            }
                            _ => {}
                        }
                    }
                }

                // Для *(ptr + i) - разыменование с арифметикой
                if op == "*" && child.node_type == "BinaryOp" {
                    let inner_expr = self.generate_expression(child)?;
                    return Ok(self.line(&inner_expr));
                }
            }
        }

        let expr = self.generate_expression(node)?;
        Ok(self.line(&expr))
    }

    /// Генерирует return
    fn generate_return(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация RETURN узла");
        debug!("Детей у return: {}", node.children.len());

        if let Some(expr) = node.children.first() {
            let expr_code = self.generate_expression(expr)?;
            debug!("Возвращаемое значение: {}", expr_code);

            if expr_code.is_empty() || expr_code == "None" {
                Ok(self.line("return"))
            } else {
                Ok(self.line(&format!("return {}", expr_code)))
            }
        } else {
            debug!("return без выражения");
            Ok(self.line("return"))
        }
    }

    fn wrap_if_needed(&self, expr: &str, child: Option<&ASTNode>) -> String {
        if let Some(child_node) = child {
            // Если ребенок - бинарная операция, оборачиваем в скобки
            if child_node.node_type == "BinaryOp" {
                return format!("({})", expr);
            }
        }
        expr.to_string()
    }

    /// Генерирует if с поддержкой elif
    fn generate_if(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        let mut current_node = node;
        let mut is_first = true;

        debug!("Генерация IF/ELIF цепочки");

        // Функция для проверки, является ли узел "else if" конструкцией
        fn is_else_if(node: &ASTNode) -> bool {
            if node.node_type == "If" {
                return true;
            }
            // Проверяем, может ли это быть Compound с одним If внутри
            if node.node_type == "Compound" && node.children.len() == 1 {
                if let Some(first_child) = node.children.first() {
                    return first_child.node_type == "If";
                }
            }
            false
        }

        loop {
            if let Some(cond) = current_node.children.first() {
                let cond_code = self.generate_expression(cond)?;

                if is_first {
                    output.push_str(&self.line(&format!("if {}:", cond_code)));
                    is_first = false;
                } else {
                    output.push_str(&self.line(&format!("elif {}:", cond_code)));
                }

                self.indent_level += 1;

                // Генерируем тело текущего if
                if let Some(if_body) = current_node.children.get(1) {
                    debug!("Тело IF: тип={}", if_body.node_type);

                    if if_body.node_type == "Compound" {
                        for stmt in &if_body.children {
                            output.push_str(&self.generate_statement(stmt)?);
                        }
                    } else {
                        // Если не Compound, возможно это одиночный оператор
                        output.push_str(&self.generate_statement(if_body)?);
                    }
                }
                self.indent_level -= 1;

                // Проверяем наличие else части
                if current_node.children.len() > 2 {
                    let else_part = current_node.children.get(2).unwrap();
                    debug!("Часть ELSE: тип={}", else_part.node_type);

                    // Проверяем, является ли else часть "else if"
                    if is_else_if(else_part) {
                        // Это else if - переходим к следующей итерации для генерации elif
                        if else_part.node_type == "If" {
                            current_node = else_part;
                        } else if else_part.node_type == "Compound"
                            && !else_part.children.is_empty()
                        {
                            // Извлекаем if из compound
                            if let Some(inner_if) = else_part.children.first() {
                                current_node = inner_if;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        // Это обычный else
                        output.push_str(&self.line("else:"));
                        self.indent_level += 1;

                        if else_part.node_type == "Compound" {
                            for stmt in &else_part.children {
                                output.push_str(&self.generate_statement(stmt)?);
                            }
                        } else {
                            output.push_str(&self.generate_statement(else_part)?);
                        }
                        self.indent_level -= 1;
                        break; // Завершаем цикл после else
                    }
                } else {
                    break; // Нет else части, завершаем
                }
            } else {
                break;
            }
        }

        Ok(output)
    }

    /// Генерирует switch как match в Python
    fn generate_switch(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация SWITCH узла");
        debug!("Детей у switch: {}", node.children.len());

        // Первый ребенок - условие (выражение в switch)
        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            debug!("Условие switch: {}", cond_code);
            output.push_str(&self.line(&format!("match {}:", cond_code)));

            self.indent_level += 1;

            // Второй ребенок - тело switch (содержит case и default)
            if let Some(body) = node.children.get(1) {
                debug!("Тело switch тип: {}", body.node_type);
                if body.node_type == "Compound" {
                    debug!(
                        "Количество операторов в теле switch: {}",
                        body.children.len()
                    );
                    for (i, stmt) in body.children.iter().enumerate() {
                        debug!("  Оператор {} в switch: тип={}", i, stmt.node_type);
                        match stmt.node_type.as_str() {
                            "Case" => {
                                output.push_str(&self.generate_case(stmt)?);
                            }
                            "Default" => {
                                output.push_str(&self.generate_default(stmt)?);
                            }
                            _ => {
                                debug!("Неизвестный оператор в теле switch: {}", stmt.node_type);
                            }
                        }
                    }
                }
            }

            self.indent_level -= 1;
        } else {
            debug!("Нет условия в switch!");
        }

        Ok(output)
    }

    /// Генерирует case как паттерн в match
    fn generate_case(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация CASE узла");
        debug!("Детей у case: {}", node.children.len());
        for (i, child) in node.children.iter().enumerate() {
            debug!(
                "  Ребенок {}: тип={}, атрибуты={:?}",
                i, child.node_type, child.attributes
            );
        }

        // Первый ребенок - значение case
        if let Some(value_node) = node.children.first() {
            let value = self.generate_expression(value_node)?;
            debug!("Значение case: '{}'", value);
            output.push_str(&self.line(&format!("case {}:", value)));

            self.indent_level += 1;

            // Остальные дети - операторы в case (может быть несколько)
            for stmt in node.children.iter().skip(1) {
                if stmt.node_type == "Break" {
                    // В Python match не требует break, просто пропускаем
                    continue;
                }
                let stmt_code = self.generate_statement(stmt)?;
                output.push_str(&stmt_code);
            }

            self.indent_level -= 1;
        } else {
            debug!("Нет значения в case!");
        }

        Ok(output)
    }

    /// Генерирует default в match
    fn generate_default(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация DEFAULT узла");
        debug!("Детей у default: {}", node.children.len());
        for (i, child) in node.children.iter().enumerate() {
            debug!(
                "  Ребенок {}: тип={}, атрибуты={:?}",
                i, child.node_type, child.attributes
            );
        }

        output.push_str(&self.line("case _:"));

        self.indent_level += 1;

        // Операторы в default (все дети)
        for stmt in node.children.iter() {
            if stmt.node_type == "Break" {
                continue;
            }
            let stmt_code = self.generate_statement(stmt)?;
            output.push_str(&stmt_code);
        }

        self.indent_level -= 1;

        Ok(output)
    }

    /// Генерирует while
    fn generate_while(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация WHILE узла");
        debug!("Детей у while: {}", node.children.len());

        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            debug!("Условие while: {}", cond_code);
            output.push_str(&self.line(&format!("while {}:", cond_code)));

            self.indent_level += 1;
            if let Some(body) = node.children.get(1) {
                debug!("Тело while тип: {}", body.node_type);
                debug!("Количество операторов в теле: {}", body.children.len());

                if body.node_type == "Compound" {
                    for stmt in &body.children {
                        let stmt_code = self.generate_statement(stmt)?;
                        debug!("Оператор в теле: {}", stmt_code.trim());
                        output.push_str(&stmt_code);
                    }
                } else {
                    // Одиночный оператор без {}
                    let stmt_code = self.generate_statement(body)?;
                    debug!("Одиночный оператор: {}", stmt_code.trim());
                    output.push_str(&stmt_code);
                }
            } else {
                debug!("НЕТ ТЕЛА У WHILE!");
            }
            self.indent_level -= 1;
        } else {
            debug!("НЕТ УСЛОВИЯ У WHILE!");
        }

        if output.is_empty() {
            debug!("ВНИМАНИЕ: while цикл не сгенерирован!");
        }

        Ok(output)
    }

    fn generate_declaration(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
            self.symbols.insert(name.to_string());

            debug!("Анализ объявления переменной: {}", name);
            debug!("  Тип узла: {}", node.node_type);
            debug!("  Атрибуты: {:#?}", node.attributes);
            debug!("  Количество детей: {}", node.children.len());

            // Проверяем, является ли это указателем
            let mut is_pointer = false;
            let mut _pointed_type = String::new();

            // Проверяем детей на наличие PtrDecl
            for child in &node.children {
                if child.node_type == "PtrDecl" {
                    is_pointer = true;
                    debug!("  Найден указатель: {}", name);

                    // Пытаемся определить тип, на который указывает
                    for grandchild in &child.children {
                        if grandchild.node_type == "TypeDecl"
                            || grandchild.node_type == "IdentifierType"
                        {
                            if let Some(type_name) = self.extract_type_name(grandchild) {
                                _pointed_type = type_name;
                            }
                        }
                    }
                    break;
                }
            }

            // Сначала проверяем, не является ли это определением enum (Decl с Enum внутри)
            for child in &node.children {
                if child.node_type == "Enum" {
                    debug!("  Найдено определение ENUM внутри Decl");
                    // Генерируем класс enum и возвращаем, не обрабатывая как переменную
                    return self.generate_enum(child);
                }
            }

            // Определяем тип объявления
            let mut is_struct_var = false;
            let mut is_union_var = false;
            let mut is_enum_var = false;
            let mut struct_type = None;
            let mut union_type = None;
            let mut enum_type = None;

            // 1. Проверяем атрибут type самого узла Decl
            if let Some(type_attr) = node.attributes.get("type") {
                debug!("  Атрибут type узла: {:#?}", type_attr);
                if let Some(type_obj) = type_attr.as_object() {
                    match type_obj.get("__node__").and_then(|v| v.as_str()) {
                        Some("Struct") => {
                            is_struct_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                struct_type = Some(name.to_string());
                                debug!("  Найдена структура в атрибуте type узла: {}", name);
                            }
                        }
                        Some("Union") => {
                            is_union_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                union_type = Some(name.to_string());
                                debug!("  Найдено объединение в атрибуте type узла: {}", name);
                            }
                        }
                        Some("Enum") => {
                            is_enum_var = true;
                            if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                enum_type = Some(name.to_string());
                                debug!("  Найдено перечисление в атрибуте type узла: {}", name);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // 2. Проверяем детей, если еще не определили тип
            if !is_struct_var && !is_union_var && !is_enum_var {
                for child in &node.children {
                    debug!("    Проверка ребенка с типом: {}", child.node_type);
                    debug!("      Атрибуты ребенка: {:#?}", child.attributes);

                    match child.node_type.as_str() {
                        "Struct" => {
                            is_struct_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                struct_type = Some(name.to_string());
                                debug!("      НАЙДЕНА СТРУКТУРА как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        "Union" => {
                            is_union_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                union_type = Some(name.to_string());
                                debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        "Enum" => {
                            is_enum_var = true;
                            if let Some(name) =
                                child.attributes.get("name").and_then(|v| v.as_str())
                            {
                                enum_type = Some(name.to_string());
                                debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ как прямой ребенок: {}", name);
                                break;
                            }
                        }
                        _ => {}
                    }

                    // Проверяем атрибут type ребенка
                    if let Some(type_attr) = child.attributes.get("type") {
                        debug!("      Атрибут type ребенка: {:#?}", type_attr);
                        if let Some(type_obj) = type_attr.as_object() {
                            match type_obj.get("__node__").and_then(|v| v.as_str()) {
                                Some("Struct") => {
                                    is_struct_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        struct_type = Some(name.to_string());
                                        debug!("      НАЙДЕНА СТРУКТУРА в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                Some("Union") => {
                                    is_union_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        union_type = Some(name.to_string());
                                        debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                Some("Enum") => {
                                    is_enum_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        enum_type = Some(name.to_string());
                                        debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ в атрибуте type ребенка {}: {}", 
                                       child.node_type, name);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Проверяем детей ребенка
                    for grandchild in &child.children {
                        match grandchild.node_type.as_str() {
                            "Struct" => {
                                is_struct_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    struct_type = Some(name.to_string());
                                    debug!("      НАЙДЕНА СТРУКТУРА как внук: {}", name);
                                    break;
                                }
                            }
                            "Union" => {
                                is_union_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    union_type = Some(name.to_string());
                                    debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ как внук: {}", name);
                                    break;
                                }
                            }
                            "Enum" => {
                                is_enum_var = true;
                                if let Some(name) =
                                    grandchild.attributes.get("name").and_then(|v| v.as_str())
                                {
                                    enum_type = Some(name.to_string());
                                    debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ как внук: {}", name);
                                    break;
                                }
                            }
                            _ => {}
                        }

                        if let Some(type_attr) = grandchild.attributes.get("type") {
                            if let Some(type_obj) = type_attr.as_object() {
                                match type_obj.get("__node__").and_then(|v| v.as_str()) {
                                    Some("Struct") => {
                                        is_struct_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            struct_type = Some(name.to_string());
                                            debug!("      НАЙДЕНА СТРУКТУРА в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    Some("Union") => {
                                        is_union_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            union_type = Some(name.to_string());
                                            debug!("      НАЙДЕНО ОБЪЕДИНЕНИЕ в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    Some("Enum") => {
                                        is_enum_var = true;
                                        if let Some(name) =
                                            type_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            enum_type = Some(name.to_string());
                                            debug!("      НАЙДЕНО ПЕРЕЧИСЛЕНИЕ в атрибуте type внука {}: {}", 
                                           grandchild.node_type, name);
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    if is_struct_var || is_union_var || is_enum_var {
                        break;
                    }
                }
            }

            debug!(
            "  Результат анализа - is_struct_var: {}, struct_type: {:?}, is_union_var: {}, union_type: {:?}, is_enum_var: {}, enum_type: {:?}",
            is_struct_var, struct_type, is_union_var, union_type, is_enum_var, enum_type
        );

            // Ищем инициализатор среди детей
            let mut init_node = None;
            for child in &node.children {
                match child.node_type.as_str() {
                    "InitList" | "Constant" | "FuncCall" | "TernaryOp" | "ID" | "BinaryOp"
                    | "UnaryOp" | "NamedInitializer" | "StructRef" => {
                        init_node = Some(child);
                        debug!(
                            "  Найден инициализатор типа {} для переменной {}",
                            child.node_type, name
                        );
                        break;
                    }
                    _ => {}
                }
            }

            // В методе generate_declaration, при обнаружении указателя:
            if is_pointer {
                self.pointer_vars.insert(name.to_string());

                // Определяем уровень указателя
                let mut ptr_level = 1;
                let mut current = node;
                while let Some(child) = current.children.first() {
                    if child.node_type == "PtrDecl" {
                        ptr_level += 1;
                        current = child;
                    } else {
                        break;
                    }
                }

                // Проверяем, не является ли это указателем на массив
                if let Some(init) = init_node {
                    if init.node_type == "ID" {
                        if let Some(init_name) =
                            init.attributes.get("name").and_then(|v| v.as_str())
                        {
                            if self.array_vars.contains(init_name) {
                                self.array_vars.insert(name.to_string());
                            }
                        }
                    }
                }

                debug!(
                    "  Переменная {} помечена как указатель уровня {}",
                    name, ptr_level
                );
            }
            // Обрабатываем объявления в зависимости от типа
            if is_union_var {
                self.handle_union_declaration(name, union_type, init_node, &mut output)?;
            } else if is_struct_var {
                self.handle_struct_declaration(name, struct_type, init_node, &mut output)?;
            } else if is_enum_var {
                self.handle_enum_declaration(name, enum_type, init_node, &mut output)?;
            } else {
                self.handle_regular_declaration(name, init_node, &mut output)?;
            }

            Ok(output)
        } else {
            // Если нет имени, проверяем, может это определение enum без переменной
            for child in &node.children {
                if child.node_type == "Enum" {
                    debug!("Найдено определение ENUM без имени переменной");
                    return self.generate_enum(child);
                }
            }
            Ok(String::new())
        }
    }

    fn extract_type_name(&self, node: &ASTNode) -> Option<String> {
        match node.node_type.as_str() {
            "IdentifierType" => node
                .attributes
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from),
            "TypeDecl" => {
                for child in &node.children {
                    if let Some(name) = self.extract_type_name(child) {
                        return Some(name);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Обработка объявления переменной типа enum
    fn handle_enum_declaration(
        &mut self,
        name: &str,
        enum_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "handle_enum_declaration: name={}, enum_type={:?}, has_init={}",
            name,
            enum_type,
            init_node.is_some()
        );

        if let Some(init) = init_node {
            // Есть инициализатор
            let init_code = self.generate_expression(init)?;

            // Проверяем, является ли инициализатор простым идентификатором (например, RED)
            // и нужно ли его преобразовать в enum_type.RED
            if init.node_type == "ID" {
                if let Some(enum_name) = &enum_type {
                    // Проверяем, не является ли это уже полным именем с точкой
                    if !init_code.contains('.') {
                        // Преобразуем RED в Color.RED
                        debug!(
                            "Преобразуем enum значение {} в {}.{}",
                            init_code, enum_name, init_code
                        );
                        output.push_str(
                            &self.line(&format!("{} = {}.{}", name, enum_name, init_code)),
                        );
                        return Ok(());
                    }
                }
            }

            output.push_str(&self.line(&format!("{} = {}", name, init_code)));
        } else {
            // Нет инициализатора
            if let Some(enum_name) = enum_type {
                // Для enum переменных без инициализатора используем первое значение по умолчанию
                // Но мы не знаем первое значение, поэтому оставляем комментарий
                output.push_str(&self.line(&format!(
                    "{} = None  # TODO: Укажите значение enum {}.VALUE",
                    name, enum_name
                )));
            } else {
                output.push_str(&self.line(&format!("{} = None", name)));
            }
        }

        Ok(())
    }

    /// Обработка объявления структурной переменной
    fn handle_struct_declaration(
        &mut self,
        name: &str,
        struct_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "handle_struct_declaration: name={}, struct_type={:?}, has_init={}",
            name,
            struct_type,
            init_node.is_some()
        );

        if let Some(init) = init_node {
            // Есть инициализатор
            if init.node_type == "InitList" {
                // Проверяем, есть ли среди детей NamedInitializer (C99 designated initializers)
                let mut has_named = false;
                let mut named_values = Vec::new();

                for child in &init.children {
                    if child.node_type == "NamedInitializer" {
                        has_named = true;
                        // Сохраняем информацию об именованных инициализаторах
                        if let Some(struct_name) = &struct_type {
                            if let Some(_fields) = self.struct_info.get(struct_name) {
                                // Пытаемся извлечь имя поля и значение
                                if let Some(name_attr) = child.attributes.get("name[0]") {
                                    if let Some(name_obj) = name_attr.as_object() {
                                        if let Some(field_name) =
                                            name_obj.get("name").and_then(|v| v.as_str())
                                        {
                                            if let Some(expr_attr) = child.attributes.get("expr") {
                                                named_values.push((
                                                    field_name.to_string(),
                                                    expr_attr.clone(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if has_named {
                    // Для designated initializers создаем объект с параметрами по умолчанию
                    if let Some(struct_name) = struct_type {
                        if let Some(fields) = self.struct_info.get(&struct_name) {
                            let default_params = vec!["None".to_string(); fields.len()].join(", ");
                            output.push_str(
                                &self.line(&format!(
                                    "{} = {}({})",
                                    name, struct_name, default_params
                                )),
                            );

                            // Затем устанавливаем именованные поля
                            for (field_name, expr_attr) in named_values {
                                // Создаем временный узел для выражения
                                let temp_node = if let Some(expr_obj) = expr_attr.as_object() {
                                    ASTNode::new(
                                        expr_obj
                                            .get("__node__")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Value"),
                                    )
                                    .with_attr("value", expr_attr.clone())
                                } else {
                                    ASTNode::new("Value").with_attr("value", expr_attr)
                                };

                                let value = self.generate_expression(&temp_node)?;
                                output.push_str(
                                    &self.line(&format!("{}.{} = {}", name, field_name, value)),
                                );
                            }
                        } else {
                            output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));
                        }
                    }
                } else {
                    // Обычная инициализация списком
                    self.current_struct_type = struct_type.clone();
                    debug!(
                        "Установлен контекст структуры {:?} для InitList переменной {}",
                        struct_type, name
                    );
                    let init_code = self.generate_expression(init)?;
                    self.current_struct_type = None;

                    output.push_str(&self.line(&format!("{} = {}", name, init_code)));
                }
            } else {
                // Инициализация другим выражением
                let init_code = self.generate_expression(init)?;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            }
        } else {
            // Нет инициализатора
            if let Some(struct_name) = struct_type {
                if let Some(fields) = self.struct_info.get(&struct_name) {
                    if !fields.is_empty() {
                        let default_params = vec!["None".to_string(); fields.len()].join(", ");
                        output.push_str(
                            &self.line(&format!("{} = {}({})", name, struct_name, default_params)),
                        );
                    } else {
                        output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));
                    }
                } else {
                    output.push_str(&self.line(&format!("{} = {}(None)", name, struct_name)));
                }
            } else {
                warn!("Неизвестный тип структуры для переменной {}", name);
                output.push_str(&self.line(&format!("{} = None", name)));
            }
        }

        Ok(())
    }
    /// Обработка объявления переменной объединения
    fn handle_union_declaration(
        &mut self,
        name: &str,
        union_type: Option<String>,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "Обработка переменной объединения: {} типа {:?}",
            name, union_type
        );

        if let Some(init) = init_node {
            // Для объединений с инициализатором
            if init.node_type == "InitList" {
                self.current_struct_type = union_type.clone();
                debug!(
                    "Установлен контекст объединения {:?} для InitList переменной {}",
                    union_type, name
                );
                let init_code = self.generate_expression(init)?;
                self.current_struct_type = None;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            } else {
                let init_code = self.generate_expression(init)?;
                output.push_str(&self.line(&format!("{} = {}", name, init_code)));
            }
        } else {
            // Для объединений без инициализатора создаем экземпляр класса
            if let Some(union_name) = union_type {
                output.push_str(&self.line(&format!("{} = {}()", name, union_name)));
                debug!("Создан экземпляр объединения: {} = {}()", name, union_name);
            } else {
                warn!("Неизвестный тип объединения для переменной {}", name);
                output.push_str(&self.line(&format!("{} = None", name)));
            }
        }

        Ok(())
    }

    /// Обработка объявления обычной переменной
    fn handle_regular_declaration(
        &mut self,
        name: &str,
        init_node: Option<&ASTNode>,
        output: &mut String,
    ) -> Result<()> {
        debug!(
            "Обычная переменная (не структура и не объединение): {}",
            name
        );

        if let Some(init) = init_node {
            let init_code = self.generate_expression(init)?;
            output.push_str(&self.line(&format!("{} = {}", name, init_code)));
        } else {
            output.push_str(&self.line(&format!("{} = None", name)));
        }

        Ok(())
    }

    /// Генерирует присваивание
    fn generate_assignment(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация присваивания");
        debug!("  Детей у присваивания: {}", node.children.len());

        if node.children.len() >= 2 {
            let left_node = &node.children[0];
            let right_node = &node.children[1];

            // Проверяем, является ли левая часть разыменованием указателя
            if left_node.node_type == "UnaryOp" {
                if let Some(op) = left_node.attributes.get("op").and_then(|v| v.as_str()) {
                    if op == "*" && left_node.children.len() == 1 {
                        let inner = &left_node.children[0];

                        // Проверяем, является ли внутреннее выражение арифметикой указателей
                        if inner.node_type == "BinaryOp" {
                            // Это *(ptr + i) = value
                            if let Some(inner_op) =
                                inner.attributes.get("op").and_then(|v| v.as_str())
                            {
                                if inner_op == "+" && inner.children.len() == 2 {
                                    let ptr = &inner.children[0];
                                    let index = &inner.children[1];

                                    if ptr.node_type == "ID" {
                                        if let Some(ptr_name) =
                                            ptr.attributes.get("name").and_then(|v| v.as_str())
                                        {
                                            let index_expr = self.generate_expression(index)?;
                                            let right_expr =
                                                self.generate_expression(right_node)?;

                                            // Для указателя на массив: ptr[index] = value
                                            return Ok(self.line(&format!(
                                                "{}[{}] = {}",
                                                ptr_name, index_expr, right_expr
                                            )));
                                        }
                                    }
                                }
                            }
                        } else if inner.node_type == "ID" {
                            // Это *ptr = value
                            if let Some(ptr_name) =
                                inner.attributes.get("name").and_then(|v| v.as_str())
                            {
                                let right_expr = self.generate_expression(right_node)?;

                                debug!(
                                    "  Присваивание через указатель: *{} = {}",
                                    ptr_name, right_expr
                                );

                                // Проверяем, является ли ptr_name указателем на массив
                                if self.pointer_vars.contains(ptr_name)
                                    && self.array_vars.contains(ptr_name)
                                {
                                    // Для указателя на массив: ptr[0] = value
                                    return Ok(
                                        self.line(&format!("{}[0] = {}", ptr_name, right_expr))
                                    );
                                } else {
                                    // Для обычного указателя: ptr.value = value
                                    return Ok(
                                        self.line(&format!("{}.value = {}", ptr_name, right_expr))
                                    );
                                }
                            }
                        }
                    }
                }
            }
            // Проверяем, является ли левая часть обращением к элементу массива
            if left_node.node_type == "ArrayRef" {
                let left = self.generate_expression(left_node)?;
                let right = self.generate_expression(right_node)?;

                // Проверяем, не нужно ли добавить .value к правой части
                if right_node.node_type == "UnaryOp" {
                    if let Some(op) = right_node.attributes.get("op").and_then(|v| v.as_str()) {
                        if op == "*" {
                            // Это ptr = *something - разыменование в правой части
                            // Обрабатывается в generate_expression
                        }
                    }
                }

                return Ok(self.line(&format!("{} = {}", left, right)));
            }

            // Проверяем, является ли левая часть идентификатором, который может быть указателем
            if left_node.node_type == "ID" {
                if let Some(var_name) = left_node.attributes.get("name").and_then(|v| v.as_str()) {
                    let right_expr = self.generate_expression(right_node)?;

                    // Если это присваивание указателю (ptr = something)
                    if self.pointer_vars.contains(var_name) {
                        // Для указателей на массивы оставляем как есть
                        if self.array_vars.contains(var_name) {
                            return Ok(self.line(&format!("{} = {}", var_name, right_expr)));
                        }
                    }
                }
            }

            // Обычное присваивание
            let left = self.generate_expression(left_node)?;
            let right = self.generate_expression(right_node)?;

            // В методе generate_assignment, убедитесь, что правильно обрабатывается op
            let op = node
                .attributes
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("=");

            let py_op = match op {
                "=" => "=",
                "+=" => "+=",
                "-=" => "-=",
                "*=" => "*=",
                "/=" => "/=",
                "%=" => "%=",
                "&=" => "&=",
                "|=" => "|=",
                "^=" => "^=",
                "<<=" => "<<=",
                ">>=" => ">>=",
                _ => "=",
            };

            // При генерации кода используйте py_op
            if left_node.node_type == "ArrayRef" {
                let left = self.generate_expression(left_node)?;
                let right = self.generate_expression(right_node)?;
                return Ok(self.line(&format!("{} {} {}", left, py_op, right)));
            }

            debug!("  {} {} {}", left, py_op, right);
            Ok(self.line(&format!("{} {} {}", left, py_op, right)))
        } else {
            debug!("  Недостаточно детей для присваивания");
            Ok(String::new())
        }
    }
    /// Генерирует объявление массива как список Python
    fn generate_array_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        // Получаем имя массива
        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_array");

        // ВАЖНО: Добавляем переменную в множество массивов
        self.array_vars.insert(name.to_string());
        debug!("Переменная {} помечена как массив", name);

        debug!("Генерация объявления массива: {}", name);
        debug!("Детей у узла массива: {}", node.children.len());

        // Выводим всех детей для отладки
        for (i, child) in node.children.iter().enumerate() {
            debug!(
                "  Ребенок {}: тип={}, атрибуты={:?}",
                i, child.node_type, child.attributes
            );
        }

        // Определяем тип элементов массива
        let mut element_type = None;
        for child in &node.children {
            if child.node_type == "TypeDecl" {
                if let Some(type_attr) = child.attributes.get("type") {
                    if let Some(type_obj) = type_attr.as_object() {
                        if type_obj.get("__node__").and_then(|v| v.as_str())
                            == Some("IdentifierType")
                        {
                            if let Some(names) = type_obj.get("names") {
                                if let Some(names_array) = names.as_array() {
                                    if let Some(first) =
                                        names_array.first().and_then(|v| v.as_str())
                                    {
                                        element_type = Some(first.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Проверяем, является ли это строкой (char массив)
        let is_char_array = element_type.as_deref() == Some("char");
        debug!(
            "Тип элементов массива: {:?}, is_char_array: {}",
            element_type, is_char_array
        );

        // Ищем инициализатор в детях
        let mut init_values = Vec::new();
        let mut found_init_list = false;
        let mut is_string_initializer = false;
        let mut string_chars = Vec::new();

        // Сначала ищем InitList - только из него берем значения
        for child in &node.children {
            if child.node_type == "InitList" {
                debug!(
                    "Найден InitList (id: {:?}) с {} детьми",
                    child.coord,
                    child.children.len()
                );

                // Если это char массив, пытаемся собрать строку
                if is_char_array {
                    let mut all_chars = true;
                    let mut chars = Vec::new();

                    for val_child in &child.children {
                        if val_child.node_type == "Constant" {
                            if let Some(val) = val_child.attributes.get("value") {
                                if let Some(s) = val.as_str() {
                                    // Проверяем, является ли это символом в кавычках
                                    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                                        let c = &s[1..s.len() - 1];
                                        if c == "\\0" {
                                            break; // Конец строки
                                        }
                                        chars.push(c.to_string());
                                    } else {
                                        all_chars = false;
                                        break;
                                    }
                                } else {
                                    all_chars = false;
                                    break;
                                }
                            } else {
                                all_chars = false;
                                break;
                            }
                        } else {
                            all_chars = false;
                            break;
                        }
                    }

                    if all_chars && !chars.is_empty() {
                        is_string_initializer = true;
                        string_chars = chars;
                        found_init_list = true;
                        break;
                    }
                }

                // Если не строка или не удалось собрать строку, собираем как обычные значения
                for (j, val_child) in child.children.iter().enumerate() {
                    debug!(
                        "  InitList[{}]: тип={}, атрибуты={:?}",
                        j, val_child.node_type, val_child.attributes
                    );

                    let value = self.generate_expression(val_child)?;
                    debug!("    Значение: {}", value);
                    init_values.push(value);
                }
                found_init_list = true;
                break; // Берем только первый InitList
            }
        }

        // Если это строковая инициализация, создаем строку Python
        if is_string_initializer && !string_chars.is_empty() {
            let string_value = string_chars.join("");
            debug!("Преобразуем char массив в строку: {}", string_value);
            output.push_str(&self.line(&format!("{} = \"{}\"", name, string_value)));
            return Ok(output);
        }

        // Если нет InitList, тогда ищем прямые значения
        if !found_init_list {
            for child in &node.children {
                // Пропускаем узлы, которые являются размером массива
                if child.node_type == "Constant" {
                    if let Some(type_attr) = child.attributes.get("type") {
                        if let Some(type_str) = type_attr.as_str() {
                            if type_str == "int" && init_values.is_empty() {
                                debug!(
                                    "Пропускаем возможный размер массива: {:?}",
                                    child.attributes
                                );
                                continue;
                            }
                        }
                    }
                }

                if child.node_type == "Constant"
                    || child.node_type == "ID"
                    || child.node_type == "InitList"
                {
                    debug!("Прямое значение в массиве: тип={}", child.node_type);

                    // Если это char массив и это константа, проверяем на символ
                    if is_char_array && child.node_type == "Constant" {
                        if let Some(val) = child.attributes.get("value") {
                            if let Some(s) = val.as_str() {
                                if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
                                    let c = &s[1..s.len() - 1];
                                    if c == "\\0" {
                                        break;
                                    }
                                    // Если это одиночный символ, создаем строку
                                    if init_values.is_empty() {
                                        output
                                            .push_str(&self.line(&format!("{} = \"{}\"", name, c)));
                                        return Ok(output);
                                    }
                                }
                            }
                        }
                    }

                    let value = self.generate_expression(child)?;
                    init_values.push(value);
                }
            }
        }

        if !init_values.is_empty() {
            debug!("Инициализация массива значениями: {:?}", init_values);

            // Обрабатываем многомерные массивы
            if init_values
                .iter()
                .any(|v| v.starts_with('[') && v.ends_with(']'))
            {
                // Это уже вложенные списки, оставляем как есть
                output.push_str(&self.line(&format!("{} = [{}]", name, init_values.join(", "))));
            } else {
                // Обычный одномерный массив
                output.push_str(&self.line(&format!("{} = [{}]", name, init_values.join(", "))));
            }
        } else {
            debug!("Нет инициализатора, создаем пустой список");
            output.push_str(&self.line(&format!("{} = []", name)));
        }

        Ok(output)
    }
    /// Генерирует обращение к элементу массива
    fn generate_array_ref(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация обращения к элементу массива");

        if node.children.len() >= 2 {
            let array_name = self.generate_expression(&node.children[0])?;
            let index = self.generate_expression(&node.children[1])?;
            Ok(format!("{}[{}]", array_name, index))
        } else {
            Ok("[]".to_string())
        }
    }

    /// Генерирует for цикл
    fn generate_for(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        debug!("Генерация FOR цикла");
        debug!("Детей у for: {}", node.children.len());

        if node.children.len() >= 4 {
            output.push_str(&self.line("# Преобразование for цикла из C в while"));

            // Инициализация (индекс 0)
            if let Some(init) = node.children.get(0) {
                debug!("Инициализация for: тип={}", init.node_type);

                // DeclList может содержать несколько объявлений
                if init.node_type == "DeclList" {
                    for decl in &init.children {
                        output.push_str(&self.generate_statement(decl)?);
                    }
                } else {
                    output.push_str(&self.generate_statement(init)?);
                }
            }

            // Условие (индекс 1)
            if let Some(cond) = node.children.get(1) {
                let cond_code = self.generate_expression(cond)?;
                debug!("Условие for: {}", cond_code);
                output.push_str(&self.line(&format!("while {}:", cond_code)));

                self.indent_level += 1;

                // Тело цикла (индекс 3 - stmt)
                if let Some(body) = node.children.get(3) {
                    debug!("Тело for: тип={}", body.node_type);
                    if body.node_type == "Compound" {
                        for stmt in &body.children {
                            output.push_str(&self.generate_statement(stmt)?);
                        }
                    } else {
                        output.push_str(&self.generate_statement(body)?);
                    }
                }

                // Инкремент (индекс 2 - next) - добавляем в конец тела цикла
                if let Some(next) = node.children.get(2) {
                    debug!("Инкремент for: тип={}", next.node_type);

                    // Для унарных операций (i++) преобразуем в оператор присваивания
                    if next.node_type == "UnaryOp" {
                        if let Some(op) = next.attributes.get("op").and_then(|v| v.as_str()) {
                            if let Some(expr) = next.children.first() {
                                if expr.node_type == "ID" {
                                    if let Some(var_name) =
                                        expr.attributes.get("name").and_then(|v| v.as_str())
                                    {
                                        match op {
                                            "p++" | "post++" | "++" => {
                                                output.push_str(
                                                    &self.line(&format!("{} += 1", var_name)),
                                                );
                                            }
                                            "p--" | "post--" | "--" => {
                                                output.push_str(
                                                    &self.line(&format!("{} -= 1", var_name)),
                                                );
                                            }
                                            _ => {
                                                output.push_str(&self.generate_statement(next)?);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        output.push_str(&self.generate_statement(next)?);
                    }
                }

                self.indent_level -= 1;
            }
        } else {
            debug!("For имеет недостаточно детей: {}", node.children.len());
        }

        Ok(output)
    }

    /// Генерирует класс Python из структуры C
    fn generate_struct(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownStruct");

        debug!("Генерация класса из структуры: {}", name);
        debug!("Детей у структуры: {}", node.children.len());

        // Собираем имена полей структуры
        let mut field_names = Vec::new();
        for child in &node.children {
            if child.node_type == "Decl" {
                if let Some(field_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    field_names.push(field_name.to_string());
                }
            }
        }

        // Сохраняем информацию о структуре
        self.struct_info
            .insert(name.to_string(), field_names.clone());

        // Генерируем определение класса
        output.push_str(&self.line(&format!("class {}:", name)));
        self.indent_level += 1;

        // Генерируем метод __init__ с параметрами
        if field_names.is_empty() {
            output.push_str(&self.line("def __init__(self):"));
        } else {
            let params = field_names.join(", ");
            output.push_str(&self.line(&format!("def __init__(self, {}):", params)));
        }

        self.indent_level += 1;

        // Инициализируем поля в __init__
        for field_name in &field_names {
            output.push_str(&self.line(&format!("self.{} = {}", field_name, field_name)));
        }

        self.indent_level -= 1; // Выходим из __init__
        self.indent_level -= 1; // Выходим из класса
        output.push_str(&self.line(""));

        Ok(output)
    }

    /// Генерирует объявление переменной типа структуры
    fn generate_struct_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let var_name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация объявления переменной структуры: {}", var_name);

        // Ищем тип структуры среди детей
        let mut struct_type = None;
        for child in &node.children {
            if child.node_type == "StructRef" || child.node_type == "Struct" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    struct_type = Some(name.to_string());
                }
            }
        }

        let struct_type = struct_type.unwrap_or_else(|| "UnknownStruct".to_string());

        // Проверяем наличие инициализатора
        let mut init_code = None;
        for child in &node.children {
            if child.node_type != "StructRef" && child.node_type != "Struct" {
                init_code = Some(self.generate_expression(child)?);
                break;
            }
        }

        if let Some(code) = init_code {
            output.push_str(&self.line(&format!("{} = {}", var_name, code)));
        } else {
            // Создаем экземпляр с параметрами по умолчанию, если есть информация о структуре
            if let Some(fields) = self.struct_info.get(&struct_type) {
                if !fields.is_empty() {
                    let default_params = vec!["None".to_string(); fields.len()].join(", ");
                    output.push_str(&self.line(&format!(
                        "{} = {}({})",
                        var_name, struct_type, default_params
                    )));
                } else {
                    output.push_str(&self.line(&format!("{} = {}()", var_name, struct_type)));
                }
            } else {
                output.push_str(&self.line(&format!("{} = {}()", var_name, struct_type)));
            }
        }

        Ok(output)
    }

    // В методе generate_struct_ref, улучшите обработку указателей на объединения
    fn generate_struct_ref(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация доступа к полю структуры/объединения");
        debug!("Детей у StructRef: {}", node.children.len());

        if node.children.len() >= 2 {
            let struct_expr = self.generate_expression(&node.children[0])?;
            let field_expr = self.generate_expression(&node.children[1])?;

            // Проверяем, является ли struct_expr указателем
            let base_name = struct_expr.split('.').next().unwrap_or(&struct_expr);

            if self.pointer_vars.contains(base_name) {
                // Это доступ через указатель: ptr->field
                if struct_expr.contains(".value") {
                    // Уже есть .value, добавляем поле
                    Ok(format!("{}.{}", struct_expr, field_expr))
                } else {
                    // Добавляем .value перед полем
                    Ok(format!("{}.value.{}", struct_expr, field_expr))
                }
            } else {
                // Обычный доступ к полю
                Ok(format!("{}.{}", struct_expr, field_expr))
            }
        } else {
            Ok("None".to_string())
        }
    }
    // В методе generate_union, улучшите обработку вложенных структур
    fn generate_union(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownUnion");

        debug!("Генерация класса из объединения: {}", name);

        // Собираем информацию о полях и их типах
        let mut fields = Vec::new();
        let mut field_types = std::collections::HashMap::new(); // поле -> тип

        for child in &node.children {
            if child.node_type == "Decl" {
                if let Some(field_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    fields.push(field_name.to_string());

                    // Определяем тип поля
                    if let Some(type_attr) = child.attributes.get("type") {
                        field_types.insert(field_name.to_string(), type_attr.clone());
                    }
                }
            }
        }

        // Генерируем определение класса
        output.push_str(&self.line(&format!("class {}:", name)));
        self.indent_level += 1;

        // Генерируем метод __init__
        output.push_str(&self.line("def __init__(self):"));
        self.indent_level += 1;

        // Инициализируем поля объединения с учетом их типов
        for field_name in &fields {
            if let Some(type_value) = field_types.get(field_name) {
                if let Some(type_obj) = type_value.as_object() {
                    if let Some(node_type) = type_obj.get("__node__").and_then(|v| v.as_str()) {
                        if node_type == "Struct" {
                            if let Some(struct_name) = type_obj.get("name").and_then(|v| v.as_str())
                            {
                                // Это поле - вложенная структура
                                output.push_str(
                                    &self.line(&format!("self.{} = {}()", field_name, struct_name)),
                                );
                                continue;
                            }
                        }
                    }
                }
            }
            // Обычное поле
            output.push_str(&self.line(&format!("self.{} = None", field_name)));
        }

        // Если полей нет, добавляем pass
        if fields.is_empty() {
            output.push_str(&self.line("pass"));
        }

        self.indent_level -= 2; // Выходим из __init__ и класса
        output.push_str(&self.line(""));

        Ok(output)
    }
    /// Генерирует объявление переменной типа объединения
    fn generate_union_decl(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();

        let var_name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Генерация объявления переменной объединения: {}", var_name);

        // Ищем тип объединения среди детей
        let mut union_type = None;
        for child in &node.children {
            if child.node_type == "Union" {
                if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    union_type = Some(name.to_string());
                }
            }
        }

        let union_type = union_type.unwrap_or_else(|| "UnknownUnion".to_string());

        // Проверяем наличие инициализатора
        let mut init_code = None;
        for child in &node.children {
            if child.node_type != "Union" {
                init_code = Some(self.generate_expression(child)?);
                break;
            }
        }

        if let Some(code) = init_code {
            output.push_str(&self.line(&format!("{} = {}", var_name, code)));
        } else {
            output.push_str(&self.line(&format!("{} = {}()", var_name, union_type)));
        }

        Ok(output)
    }

    fn generate_enum(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        let mut enum_values = Vec::new();

        let name = node
            .attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("UnknownEnum");

        debug!("Генерация класса Enum из перечисления: {}", name);
        debug!("Детей у enum: {}", node.children.len());

        output.push_str(&self.line(&format!("class {}(enum.Enum):", name)));
        self.indent_level += 1;

        let mut has_explicit_values = false;
        let mut enum_items_found = false;

        for child in &node.children {
            debug!("  Ребенок enum: тип={}", child.node_type);

            if child.node_type == "EnumeratorList" {
                debug!("  Найден EnumeratorList с {} детьми", child.children.len());

                for enumerator in &child.children {
                    if enumerator.node_type == "Enumerator" {
                        if let Some(item_name) =
                            enumerator.attributes.get("name").and_then(|v| v.as_str())
                        {
                            enum_items_found = true;
                            enum_values.push(item_name.to_string());

                            if let Some(value) = enumerator.attributes.get("value") {
                                has_explicit_values = true;

                                // Пытаемся извлечь числовое значение
                                if let Some(value_obj) = value.as_object() {
                                    if let Some(value_str) =
                                        value_obj.get("value").and_then(|v| v.as_str())
                                    {
                                        if value_str.chars().all(|c| c.is_ascii_digit()) {
                                            output.push_str(
                                                &self.line(&format!(
                                                    "{} = {}",
                                                    item_name, value_str
                                                )),
                                            );
                                        } else {
                                            output.push_str(
                                                &self.line(&format!("{} = auto()", item_name)),
                                            );
                                        }
                                    } else {
                                        output.push_str(
                                            &self.line(&format!("{} = auto()", item_name)),
                                        );
                                    }
                                } else if let Some(value_str) = value.as_str() {
                                    if value_str.chars().all(|c| c.is_ascii_digit()) {
                                        output.push_str(
                                            &self.line(&format!("{} = {}", item_name, value_str)),
                                        );
                                    } else {
                                        output.push_str(
                                            &self.line(&format!("{} = auto()", item_name)),
                                        );
                                    }
                                } else if let Some(value_num) = value.as_i64() {
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, value_num)),
                                    );
                                } else {
                                    output.push_str(&self.line(&format!("{} = auto()", item_name)));
                                }
                            } else {
                                output.push_str(&self.line(&format!("{} = auto()", item_name)));
                            }
                        }
                    }
                }
            } else if child.node_type == "Enumerator" {
                if let Some(item_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    enum_items_found = true;
                    enum_values.push(item_name.to_string());

                    if let Some(value) = child.attributes.get("value") {
                        has_explicit_values = true;

                        if let Some(value_obj) = value.as_object() {
                            if let Some(value_str) = value_obj.get("value").and_then(|v| v.as_str())
                            {
                                if value_str.chars().all(|c| c.is_ascii_digit()) {
                                    output.push_str(
                                        &self.line(&format!("{} = {}", item_name, value_str)),
                                    );
                                } else {
                                    output.push_str(&self.line(&format!("{} = auto()", item_name)));
                                }
                            } else {
                                output.push_str(&self.line(&format!("{} = auto()", item_name)));
                            }
                        } else if let Some(value_str) = value.as_str() {
                            if value_str.chars().all(|c| c.is_ascii_digit()) {
                                output.push_str(
                                    &self.line(&format!("{} = {}", item_name, value_str)),
                                );
                            } else {
                                output.push_str(&self.line(&format!("{} = auto()", item_name)));
                            }
                        } else if let Some(value_num) = value.as_i64() {
                            output.push_str(&self.line(&format!("{} = {}", item_name, value_num)));
                        } else {
                            output.push_str(&self.line(&format!("{} = auto()", item_name)));
                        }
                    } else {
                        output.push_str(&self.line(&format!("{} = auto()", item_name)));
                    }
                }
            }
        }

        self.enum_info.insert(name.to_string(), enum_values);

        if !enum_items_found {
            debug!("  ВНИМАНИЕ: Не найдены элементы enum!");
            output.push_str(&self.line("# TODO: Добавьте элементы enum"));
        }

        if has_explicit_values {
            output.push_str(&self.line(""));
            output.push_str(&self.line("# Примечание: некоторые значения заданы явно, как в C"));
        }

        self.indent_level -= 1;
        output.push_str(&self.line(""));

        Ok(output)
    }

    /// Проверяет, является ли идентификатор значением enum
    fn is_enum_value(&self, name: &str) -> Option<String> {
        for (enum_name, values) in &self.enum_info {
            if values.contains(&name.to_string()) {
                return Some(enum_name.clone());
            }
        }
        None
    }

    fn generate_reference_class(&self) -> String {
        let mut output = String::new();

        output.push_str("\n# Класс для имитации ссылок и указателей C\n");
        output.push_str("class Reference:\n");
        output.push_str("    def __init__(self, value):\n");
        output.push_str("        self.value = value\n");
        output.push_str("    \n");
        output.push_str("    def __repr__(self):\n");
        output.push_str("        return f\"Reference({self.value})\"\n");
        output.push_str("    \n");
        output.push_str("    # Поддержка арифметики указателей\n");
        output.push_str("    def __add__(self, other):\n");
        output.push_str("        return Reference(self.value + other)\n");
        output.push_str("    \n");
        output.push_str("    def __sub__(self, other):\n");
        output.push_str("        return Reference(self.value - other)\n");
        output.push_str("    \n");
        output.push_str("    # Поддержка индексации (для pointer[index])\n");
        output.push_str("    def __getitem__(self, index):\n");
        output.push_str("        return self.value[index] if hasattr(self.value, '__getitem__') else self.value + index\n");
        output.push_str("    \n");
        output.push_str("    def __setitem__(self, index, value):\n");
        output.push_str("        if hasattr(self.value, '__setitem__'):\n");
        output.push_str("            self.value[index] = value\n");
        output.push_str("        else:\n");
        output.push_str("            # Для арифметики указателей\n");
        output.push_str("            self.value = value - index\n");
        output.push_str("\n");

        output
    }
}

// В методе generate, после импорта sys и os, добавьте:
impl Generator for PythonGenerator {
    type Output = String;

    fn generate(ast: &ASTNode) -> Result<String> {
        let mut generator = PythonGenerator::new();
        let mut output = String::new();

        output.push_str("# Generated by C to Python transpiler\n");
        output.push_str("# This is an approximate conversion\n\n");
        output.push_str("import sys\n");
        output.push_str("import os\n");
        output.push_str("import enum\n");
        output.push_str("from enum import auto\n");

        // Добавляем класс Reference для поддержки указателей
        output.push_str(&generator.generate_reference_class());

        // Обрабатываем определения структур, объединений и перечислений
        for node in &ast.children {
            if node.node_type == "Decl" {
                for child in &node.children {
                    match child.node_type.as_str() {
                        "Struct" => output.push_str(&generator.generate_struct(child)?),
                        "Union" => output.push_str(&generator.generate_union(child)?),
                        "Enum" => output.push_str(&generator.generate_enum(child)?),
                        _ => {}
                    }
                }
            } else {
                match node.node_type.as_str() {
                    "Enum" => output.push_str(&generator.generate_enum(node)?),
                    _ => {}
                }
            }
        }

        // Обрабатываем функции
        let mut has_main = false;
        for node in &ast.children {
            match node.node_type.as_str() {
                "FuncDef" | "FuncDecl" => {
                    output.push_str(&generator.generate_function(node)?);

                    // Проверяем, является ли эта функция main
                    if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                        if name == "main" {
                            has_main = true;
                        }
                    }
                }
                _ => {}
            }
        }

        // Добавляем конструкцию if __name__ == "__main__" для вызова main()
        if has_main {
            output.push_str("\nif __name__ == \"__main__\":\n");
            output.push_str("    main()\n");
        }

        Ok(output)
    }

    fn language_name() -> &'static str {
        "Python"
    }
}
