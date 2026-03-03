use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::collections::HashSet;

use super::Generator;
use crate::ast::ASTNode;

/// Генератор Python кода из AST
pub struct PythonGenerator {
    /// Текущий уровень отступа
    indent_level: usize,
    /// Размер отступа (пробелы)
    indent_size: usize,
    /// Таблица символов для отслеживания объявленных переменных
    symbols: HashSet<String>,
    /// Режим генерации (выражение или оператор)
    in_expression: bool,
    /// Текущий тип структуры при инициализации
    current_struct_type: Option<String>,
}

impl PythonGenerator {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            indent_size: 4,
            symbols: HashSet::new(),
            in_expression: false,
            current_struct_type: None,
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
            "Switch" => self.generate_switch(node), // Добавить эту строку
            "FuncCall" => {
                let expr = self.generate_expression(node)?;
                Ok(self.line(&expr))
            }
            "UnaryOp" => self.generate_unary_stmt(node),
            // В методе generate_statement для "Compound"
            "Compound" => {
                let mut output = String::new();

                // Просто генерируем операторы в том порядке, в котором они идут в AST
                for stmt in &node.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }

                Ok(output)
            }
            "Break" => Ok(self.line("break")),
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
            "Constant" => {
                if let Some(value) = node.attributes.get("value") {
                    if let Some(s) = value.as_str() {
                        // Проверяем, является ли строка числом
                        if s.parse::<i32>().is_ok() || s.parse::<f64>().is_ok() {
                            Ok(s.to_string())
                        } else {
                            // Это строка, оставляем как есть (с кавычками)
                            Ok(s.to_string())
                        }
                    } else if let Some(n) = value.as_i64() {
                        Ok(n.to_string())
                    } else if let Some(n) = value.as_f64() {
                        Ok(n.to_string())
                    } else {
                        Ok(value.to_string())
                    }
                } else {
                    Ok("None".to_string())
                }
            }

            "ID" => {
                if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    Ok(name.to_string())
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

                // Проверяем, нужно ли обернуть левую и правую части в скобки
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

                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };

                debug!("  Операнд: '{}'", expr);

                match op {
                    "++" | "p++" | "post++" => {
                        // Для постфиксного инкремента как выражения
                        Ok(format!("({} + 1)", expr))
                    }
                    "--" | "p--" | "post--" => {
                        // Для постфиксного декремента как выражения
                        Ok(format!("({} - 1)", expr))
                    }
                    "-" => {
                        // Унарный минус
                        if expr.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            // Это число
                            Ok(format!("-{}", expr))
                        } else if expr.starts_with('-') {
                            // Уже отрицательное число
                            Ok(expr)
                        } else {
                            // Выражение в скобках
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
                    } else {
                        let value = self.generate_expression_internal(child)?;
                        debug!(
                            "  InitList {} ребенок {}: тип={}, значение={}",
                            node_id, i, child.node_type, value
                        );
                        values.push(value);
                    }
                }

                // Проверяем, не дублируются ли значения
                let mut unique_values = Vec::new();
                for value in &values {
                    if !unique_values.contains(value) {
                        unique_values.push(value.clone());
                    } else {
                        debug!("Найдено дублирующееся значение: {}", value);
                    }
                }

                if unique_values.len() != values.len() {
                    debug!(
                        "Обнаружено дублирование! Было {}, стало {}",
                        values.len(),
                        unique_values.len()
                    );
                    values = unique_values;
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
            // В методе generate_expression_internal, добавьте обработку "TernaryOp":
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
                // Для префиксных и постфиксных операций с переменными
                if child.node_type == "ID" {
                    if let Some(var_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                        match op {
                            "++" | "p++" | "post++" => {
                                return Ok(self.line(&format!("{} += 1", var_name)));
                            }
                            "--" | "p--" | "post--" => {
                                return Ok(self.line(&format!("{} -= 1", var_name)));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Если не удалось обработать как оператор с переменной, генерируем как выражение
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

    fn needs_parentheses(&self, op: &str, left: &str, right: &str) -> bool {
        // Проверяем, содержит ли левое или правое выражение операторы
        let left_has_ops = left.contains("+")
            || left.contains("-")
            || left.contains("*")
            || left.contains("/")
            || left.contains("and")
            || left.contains("or");
        let right_has_ops = right.contains("+")
            || right.contains("-")
            || right.contains("*")
            || right.contains("/")
            || right.contains("and")
            || right.contains("or");

        // Для умножения и деления - если внутри есть сложение/вычитание, нужны скобки
        if op == "*" || op == "/" {
            if left_has_ops || right_has_ops {
                return true;
            }
        }

        // Для сложения и вычитания - если внутри есть операторы с более высоким приоритетом
        if op == "+" || op == "-" {
            // Не нужны скобки, если внутри только умножение/деление (они имеют higher precedence)
            // Но если есть другие операторы сложения/вычитания, то скобки не нужны
            return false;
        }

        // Для логических операций
        if op == "&&" || op == "||" {
            if left_has_ops || right_has_ops {
                return true;
            }
        }

        false
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

            // Второй ребенок - операторы в case
            if let Some(stmts) = node.children.get(1) {
                debug!("Операторы case тип: {}", stmts.node_type);
                if stmts.node_type == "Compound" {
                    debug!("  Compound с {} детьми", stmts.children.len());
                    for stmt in &stmts.children {
                        let stmt_code = self.generate_statement(stmt)?;
                        output.push_str(&stmt_code);
                    }
                } else {
                    // Одиночный оператор без {}
                    debug!("  Одиночный оператор");
                    let stmt_code = self.generate_statement(stmts)?;
                    output.push_str(&stmt_code);
                }
            } else {
                debug!("Нет операторов в case!");
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

        // Операторы в default
        if let Some(stmts) = node.children.first() {
            debug!("Операторы default тип: {}", stmts.node_type);
            if stmts.node_type == "Compound" {
                debug!("  Compound с {} детьми", stmts.children.len());
                for stmt in &stmts.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }
            } else {
                debug!("  Одиночный оператор");
                output.push_str(&self.generate_statement(stmts)?);
            }
        } else {
            debug!("Нет операторов в default!");
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

            // Определяем, является ли это структурной переменной
            let mut is_struct_var = false;
            let mut struct_type = None;

            // 1. Проверяем атрибут type самого узла Decl
            if let Some(type_attr) = node.attributes.get("type") {
                debug!("  Атрибут type узла: {:#?}", type_attr);
                if let Some(type_obj) = type_attr.as_object() {
                    if type_obj.get("__node__").and_then(|v| v.as_str()) == Some("Struct") {
                        is_struct_var = true;
                        if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                            struct_type = Some(name.to_string());
                            debug!("  Найдена структура в атрибуте type узла: {}", name);
                        }
                    }
                }
            }

            // 2. Проверяем детей
            if !is_struct_var {
                for child in &node.children {
                    debug!("    Проверка ребенка с типом: {}", child.node_type);
                    debug!("      Атрибуты ребенка: {:#?}", child.attributes);

                    // Проверяем, не является ли сам ребенок структурой
                    if child.node_type == "Struct" {
                        is_struct_var = true;
                        if let Some(name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                            struct_type = Some(name.to_string());
                            debug!("      НАЙДЕНА СТРУКТУРА как прямой ребенок: {}", name);
                            break;
                        }
                    }

                    // Проверяем атрибут type ребенка (это самое важное!)
                    if let Some(type_attr) = child.attributes.get("type") {
                        debug!("      Атрибут type ребенка: {:#?}", type_attr);
                        if let Some(type_obj) = type_attr.as_object() {
                            if type_obj.get("__node__").and_then(|v| v.as_str()) == Some("Struct") {
                                is_struct_var = true;
                                if let Some(name) = type_obj.get("name").and_then(|v| v.as_str()) {
                                    struct_type = Some(name.to_string());
                                    debug!(
                                        "      НАЙДЕНА СТРУКТУРА в атрибуте type ребенка {}: {}",
                                        child.node_type, name
                                    );
                                    break;
                                }
                            }
                        }
                    }

                    // Также проверяем детей ребенка (на случай если структура еще глубже)
                    for grandchild in &child.children {
                        if grandchild.node_type == "Struct" {
                            is_struct_var = true;
                            if let Some(name) =
                                grandchild.attributes.get("name").and_then(|v| v.as_str())
                            {
                                struct_type = Some(name.to_string());
                                debug!("      НАЙДЕНА СТРУКТУРА как внук: {}", name);
                                break;
                            }
                        }

                        if let Some(type_attr) = grandchild.attributes.get("type") {
                            if let Some(type_obj) = type_attr.as_object() {
                                if type_obj.get("__node__").and_then(|v| v.as_str())
                                    == Some("Struct")
                                {
                                    is_struct_var = true;
                                    if let Some(name) =
                                        type_obj.get("name").and_then(|v| v.as_str())
                                    {
                                        struct_type = Some(name.to_string());
                                        debug!(
                                            "      НАЙДЕНА СТРУКТУРА в атрибуте type внука {}: {}",
                                            grandchild.node_type, name
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if is_struct_var {
                        break;
                    }
                }
            }

            debug!(
                "  Результат анализа - is_struct_var: {}, struct_type: {:?}",
                is_struct_var, struct_type
            );

            // Ищем инициализатор среди детей
            let mut init_node = None;
            for child in &node.children {
                match child.node_type.as_str() {
                    "InitList" | "Constant" | "FuncCall" | "TernaryOp" | "ID" | "BinaryOp"
                    | "UnaryOp" => {
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

            if is_struct_var {
                debug!(
                    "Обработка структурной переменной: {} типа {:?}",
                    name, struct_type
                );

                if let Some(init) = init_node {
                    // Для структур с инициализатором
                    if init.node_type == "InitList" {
                        // Устанавливаем контекст структуры для правильной обработки InitList
                        self.current_struct_type = struct_type.clone();
                        debug!(
                            "Установлен контекст структуры {:?} для InitList переменной {}",
                            struct_type, name
                        );
                        let init_code = self.generate_expression(init)?;
                        self.current_struct_type = None;
                        output.push_str(&self.line(&format!("{} = {}", name, init_code)));
                    } else {
                        let init_code = self.generate_expression(init)?;
                        output.push_str(&self.line(&format!("{} = {}", name, init_code)));
                    }
                } else {
                    // Для структур без инициализатора создаем экземпляр класса
                    if let Some(struct_name) = struct_type {
                        output.push_str(&self.line(&format!("{} = {}()", name, struct_name)));
                        debug!("Создан экземпляр структуры: {} = {}()", name, struct_name);
                    } else {
                        warn!("Неизвестный тип структуры для переменной {}", name);
                        output.push_str(&self.line(&format!("{} = None", name)));
                    }
                }
            } else {
                debug!("Обычная переменная (не структура): {}", name);
                // Обычное объявление переменной
                if let Some(init) = init_node {
                    let init_code = self.generate_expression(init)?;
                    output.push_str(&self.line(&format!("{} = {}", name, init_code)));
                } else {
                    output.push_str(&self.line(&format!("{} = None", name)));
                }
            }

            Ok(output)
        } else {
            Ok(String::new())
        }
    }

    /// Генерирует присваивание
    fn generate_assignment(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация присваивания");
        debug!("  Детей у присваивания: {}", node.children.len());

        if node.children.len() >= 2 {
            let left = self.generate_expression(&node.children[0])?;
            let right = self.generate_expression(&node.children[1])?;

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
                _ => "=",
            };

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

        debug!("Генерация объявления массива: {}", name);
        debug!("Детей у узла массива: {}", node.children.len());

        // Выводим всех детей для отладки
        for (i, child) in node.children.iter().enumerate() {
            debug!(
                "  Ребенок {}: тип={}, атрибуты={:?}",
                i, child.node_type, child.attributes
            );
        }

        // Проверяем, является ли это строкой (char массив с одним строковым литералом)
        let mut is_string = false;
        let mut string_value = None;

        // Ищем инициализатор в детях
        let mut init_values = Vec::new();
        let mut found_init_list = false;

        // Сначала ищем InitList - только из него берем значения
        for child in &node.children {
            if child.node_type == "InitList" {
                debug!(
                    "Найден InitList (id: {:?}) с {} детьми",
                    child.coord,
                    child.children.len()
                );

                // Выводим детей InitList
                for (j, val_child) in child.children.iter().enumerate() {
                    debug!(
                        "  InitList[{}]: тип={}, атрибуты={:?}",
                        j, val_child.node_type, val_child.attributes
                    );

                    // Проверяем, является ли это строкой
                    if val_child.node_type == "Constant" {
                        if let Some(type_attr) = val_child.attributes.get("type") {
                            if let Some(type_str) = type_attr.as_str() {
                                if type_str == "string" {
                                    is_string = true;
                                    if let Some(val) = val_child.attributes.get("value") {
                                        if let Some(s) = val.as_str() {
                                            // Убираем внешние кавычки если они есть
                                            let clean_str = s.trim_matches('"');
                                            string_value = Some(clean_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let value = self.generate_expression(val_child)?;
                    debug!("    Значение: {}", value);
                    init_values.push(value);
                }
                found_init_list = true;
                break; // Берем только первый InitList
            }
        }

        // Если это строковый массив с одним элементом, преобразуем в строку Python
        if is_string && init_values.len() == 1 {
            if let Some(s) = string_value {
                debug!("Преобразуем char массив в строку: {}", s);
                output.push_str(&self.line(&format!("{} = \"{}\"", name, s)));
                return Ok(output);
            }
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

                if child.node_type == "Constant" || child.node_type == "ID" {
                    debug!("Прямое значение в массиве: тип={}", child.node_type);

                    // Проверяем, является ли это строкой
                    if child.node_type == "Constant" {
                        if let Some(type_attr) = child.attributes.get("type") {
                            if let Some(type_str) = type_attr.as_str() {
                                if type_str == "string" {
                                    is_string = true;
                                    if let Some(val) = child.attributes.get("value") {
                                        if let Some(s) = val.as_str() {
                                            let clean_str = s.trim_matches('"');
                                            string_value = Some(clean_str.to_string());
                                        }
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

        // Если это строковый массив с одним элементом, преобразуем в строку Python
        if is_string && init_values.len() == 1 {
            if let Some(s) = string_value {
                debug!("Преобразуем char массив в строку: {}", s);
                output.push_str(&self.line(&format!("{} = \"{}\"", name, s)));
                return Ok(output);
            }
        }

        if !init_values.is_empty() {
            debug!("Инициализация массива значениями: {:?}", init_values);
            output.push_str(&self.line(&format!("{} = [{}]", name, init_values.join(", "))));
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

        // Генерируем определение класса
        output.push_str(&self.line(&format!("class {}:", name)));
        self.indent_level += 1;

        // Генерируем метод __init__
        output.push_str(&self.line("def __init__(self):"));
        self.indent_level += 1;

        // Собираем поля структуры
        for child in &node.children {
            if child.node_type == "Decl" {
                if let Some(field_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                    output.push_str(&self.line(&format!("self.{} = None", field_name)));
                }
            }
        }

        self.indent_level -= 1; // Выходим из __init__
        self.indent_level -= 1; // Выходим из класса
        output.push_str(&self.line(""));

        Ok(output)
    }
    fn extract_field_type(&self, node: &ASTNode) -> String {
        for child in &node.children {
            if child.node_type == "TypeDecl" {
                for grandchild in &child.children {
                    match grandchild.node_type.as_str() {
                        "IdentifierType" => {
                            if let Some(type_name) =
                                grandchild.attributes.get("name").and_then(|v| v.as_str())
                            {
                                return match type_name {
                                    "int" => "int".to_string(),
                                    "char" => "str".to_string(),
                                    "float" => "float".to_string(),
                                    "double" => "float".to_string(),
                                    _ => type_name.to_string(),
                                };
                            }
                        }
                        "Struct" => {
                            if let Some(struct_name) =
                                grandchild.attributes.get("name").and_then(|v| v.as_str())
                            {
                                return format!("struct {}", struct_name);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        "Any".to_string()
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
            output.push_str(&self.line(&format!("{} = {}()", var_name, struct_type)));
        }

        Ok(output)
    }

    /// Генерирует доступ к полю структуры
    fn generate_struct_ref(&mut self, node: &ASTNode) -> Result<String> {
        debug!("Генерация доступа к полю структуры");
        debug!("Детей у StructRef: {}", node.children.len());

        if node.children.len() >= 2 {
            let struct_expr = self.generate_expression(&node.children[0])?;
            let field_expr = self.generate_expression(&node.children[1])?;

            Ok(format!("{}.{}", struct_expr, field_expr))
        } else {
            Ok("None".to_string())
        }
    }
}

impl Generator for PythonGenerator {
    type Output = String;

    fn generate(ast: &ASTNode) -> Result<String> {
        let mut generator = PythonGenerator::new();
        let mut output = String::new();

        output.push_str("# Generated by C to Python transpiler\n");
        output.push_str("# This is an approximate conversion\n\n");
        output.push_str("import sys\n");
        output.push_str("import os\n\n");

        // ОТЛАДКА: выводим все корневые узлы
        debug!("Корневые узлы AST:");
        for (i, node) in ast.children.iter().enumerate() {
            debug!("  Корневой узел {}: тип={}", i, node.node_type);
        }

        // Сначала обрабатываем все определения структур (они могут быть в Decl узлах)
        for node in &ast.children {
            if node.node_type == "Decl" {
                // Проверяем, не является ли этот Decl определением структуры
                for child in &node.children {
                    if child.node_type == "Struct" {
                        debug!("Найден Struct внутри Decl на верхнем уровне");
                        output.push_str(&generator.generate_struct(child)?);
                        break;
                    }
                }
            }
        }

        // Затем обрабатываем все функции
        for node in &ast.children {
            match node.node_type.as_str() {
                "FuncDef" | "FuncDecl" => {
                    output.push_str(&generator.generate_function(node)?);
                }
                _ => {}
            }
        }

        Ok(output)
    }
    fn language_name() -> &'static str {
        "Python"
    }
}
