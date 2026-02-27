use anyhow::{Result, anyhow};
use std::collections::HashSet;
use log::{debug, warn, info};

use crate::ast::ASTNode;
use super::Generator;

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
}

impl PythonGenerator {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            indent_size: 4,
            symbols: HashSet::new(),
            in_expression: false,
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
        let name = node.attributes.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        debug!("Генерация функции: {}", name);
        
        // Собираем параметры функции
        let mut params = Vec::new();
        
        // Ищем параметры в детях
for child in &node.children {
    if child.node_type == "ParamList" {
        for param in &child.children {
            if let Some(param_name) = self.extract_param_name(param) {
                // Клонируем для второго использования
                params.push(param_name.clone());
                self.symbols.insert(param_name);
            }
        }
    }
}
        
        // Если параметры не найдены, проверяем атрибуты
        if params.is_empty() {
            if let Some(params_attr) = node.attributes.get("params") {
                if let Some(params_array) = params_attr.as_array() {
                    for param in params_array {
                        if let Some(param_obj) = param.as_object() {
                            if let Some(param_name) = param_obj.get("name").and_then(|v| v.as_str()) {
                                params.push(param_name.to_string());
                                self.symbols.insert(param_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Генерируем определение функции
        output.push_str(&self.line(&format!("def {}({}):", name, params.join(", "))));
        
        self.indent_level += 1;
        
        // Генерируем тело функции
        let mut has_return = false;
        for child in &node.children {
            if child.node_type == "Compound" {
                for stmt in &child.children {
                    let stmt_code = self.generate_statement(stmt)?;
                    output.push_str(&stmt_code);
                    if stmt.node_type == "Return" {
                        has_return = true;
                    }
                }
            }
        }
        
        // Если нет return, добавляем pass
        if !has_return && output.trim().ends_with(':') {
            output.push_str(&self.line("    pass"));
        }
        
        self.indent_level -= 1;
        output.push_str(&self.line(""));
        
        Ok(output)
    }
    
    /// Извлекает имя параметра из узла
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
        match node.node_type.as_str() {
            "Return" => self.generate_return(node),
            "If" => self.generate_if(node),
            "While" => self.generate_while(node),
            "For" => self.generate_for(node),
            "Assignment" => self.generate_assignment(node),
            "Decl" => self.generate_declaration(node),
            "FuncCall" => {
                let expr = self.generate_expression(node)?;
                Ok(self.line(&expr))
            },
            "UnaryOp" => self.generate_unary_stmt(node),
            "Compound" => {
                let mut output = String::new();
                for stmt in &node.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }
                Ok(output)
            },
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
                        Ok(s.to_string())
                    } else if let Some(n) = value.as_number() {
                        Ok(n.to_string())
                    } else {
                        Ok(value.to_string())
                    }
                } else {
                    Ok("None".to_string())
                }
            },
            
            "ID" => {
                if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
                    Ok(name.to_string())
                } else {
                    Ok("unknown".to_string())
                }
            },
            
            "BinaryOp" => {
                let op = node.attributes.get("op")
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
                
                Ok(format!("{} {} {}", left, py_op, right))
            },
            
            "UnaryOp" => {
                let op = node.attributes.get("op")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                
                let expr = if let Some(child) = node.children.first() {
                    self.generate_expression_internal(child)?
                } else {
                    String::new()
                };
                
                match op {
                    "++" => Ok(format!("({} + 1)", expr)),
                    "--" => Ok(format!("({} - 1)", expr)),
                    "p++" | "post++" => Ok(format!("({} + 1)", expr)),
                    "p--" | "post--" => Ok(format!("({} - 1)", expr)),
                    "-" => Ok(format!("-{}", expr)),
                    "+" => Ok(format!("+{}", expr)),
                    "!" => Ok(format!("not {}", expr)),
                    _ => {
                        debug!("Неизвестный унарный оператор: {}", op);
                        Ok(expr)
                    }
                }
            },
            
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
                            if let Some(id_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                                if name.is_none() {
                                    name = Some(id_name.to_string());
                                } else {
                                    // Это может быть аргумент
                                    args_exprs.push(id_name.to_string());
                                }
                            }
                        },
                        "ExprList" => {
                            // Это список аргументов
                            for arg in &child.children {
                                let arg_expr = self.generate_expression_internal(arg)?;
                                if !arg_expr.is_empty() {
                                    args_exprs.push(arg_expr);
                                }
                            }
                        },
                        _ => {
                            // Другие типы как аргументы
                            let arg_expr = self.generate_expression_internal(child)?;
                            if !arg_expr.is_empty() {
                                args_exprs.push(arg_expr);
                            }
                        }
                    }
                }
                
                let name = name.unwrap_or_else(|| {
                    warn!("Не удалось определить имя функции");
                    "unknown".to_string()
                });
                
                debug!("Вызов функции: {} с аргументами: {:?}", name, args_exprs);
                Ok(format!("{}({})", name, args_exprs.join(", ")))
            },
            
            "ExprList" => {
                let mut exprs = Vec::new();
                for child in &node.children {
                    let expr = self.generate_expression_internal(child)?;
                    if !expr.is_empty() {
                        exprs.push(expr);
                    }
                }
                Ok(exprs.join(", "))
            },
            
            _ => {
                debug!("Неизвестное выражение: {}", node.node_type);
                Ok("None".to_string())
            }
        }
    }
    
    /// Генерирует унарный оператор как оператор
    fn generate_unary_stmt(&mut self, node: &ASTNode) -> Result<String> {
        if let Some(op) = node.attributes.get("op").and_then(|v| v.as_str()) {
            if let Some(child) = node.children.first() {
                if child.node_type == "ID" {
                    if let Some(var_name) = child.attributes.get("name").and_then(|v| v.as_str()) {
                        match op {
                            "++" | "p++" | "post++" => {
                                return Ok(self.line(&format!("{} += 1", var_name)));
                            },
                            "--" | "p--" | "post--" => {
                                return Ok(self.line(&format!("{} -= 1", var_name)));
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Если не удалось обработать как оператор, генерируем как выражение
        let expr = self.generate_expression(node)?;
        Ok(self.line(&expr))
    }
    
    /// Генерирует return
    fn generate_return(&mut self, node: &ASTNode) -> Result<String> {
        if let Some(expr) = node.children.first() {
            let expr_code = self.generate_expression(expr)?;
            Ok(self.line(&format!("return {}", expr_code)))
        } else {
            Ok(self.line("return"))
        }
    }
    
    /// Генерирует if
    fn generate_if(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        
        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            output.push_str(&self.line(&format!("if {}:", cond_code)));
            
            self.indent_level += 1;
            if let Some(body) = node.children.get(1) {
                for stmt in &body.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }
            }
            self.indent_level -= 1;
            
            if node.children.len() > 2 {
                output.push_str(&self.line("else:"));
                self.indent_level += 1;
                if let Some(else_body) = node.children.get(2) {
                    for stmt in &else_body.children {
                        output.push_str(&self.generate_statement(stmt)?);
                    }
                }
                self.indent_level -= 1;
            }
        }
        
        Ok(output)
    }
    
    /// Генерирует while
    fn generate_while(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        
        if let Some(cond) = node.children.first() {
            let cond_code = self.generate_expression(cond)?;
            output.push_str(&self.line(&format!("while {}:", cond_code)));
            
            self.indent_level += 1;
            if let Some(body) = node.children.get(1) {
                for stmt in &body.children {
                    output.push_str(&self.generate_statement(stmt)?);
                }
            }
            self.indent_level -= 1;
        }
        
        Ok(output)
    }
    
    /// Генерирует объявление переменной
    fn generate_declaration(&mut self, node: &ASTNode) -> Result<String> {
        if let Some(name) = node.attributes.get("name").and_then(|v| v.as_str()) {
            self.symbols.insert(name.to_string());
            
            if !node.children.is_empty() {
                let init_code = self.generate_expression(&node.children[0])?;
                Ok(self.line(&format!("{} = {}", name, init_code)))
            } else {
                Ok(self.line(&format!("{} = None", name)))
            }
        } else {
            Ok(String::new())
        }
    }
    
    /// Генерирует присваивание
    fn generate_assignment(&mut self, node: &ASTNode) -> Result<String> {
        if node.children.len() >= 2 {
            let left = self.generate_expression(&node.children[0])?;
            let right = self.generate_expression(&node.children[1])?;
            
            let op = node.attributes.get("op")
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
            
            Ok(self.line(&format!("{} {} {}", left, py_op, right)))
        } else {
            Ok(String::new())
        }
    }
    
    /// Генерирует for (упрощенно)
    fn generate_for(&mut self, node: &ASTNode) -> Result<String> {
        let mut output = String::new();
        
        if node.children.len() >= 3 {
            output.push_str(&self.line("# Примечание: for цикл из C конвертирован в while"));
            
            if let Some(init) = node.children.get(0) {
                output.push_str(&self.generate_statement(init)?);
            }
            
            if let Some(cond) = node.children.get(1) {
                let cond_code = self.generate_expression(cond)?;
                output.push_str(&self.line(&format!("while {}:", cond_code)));
                
                self.indent_level += 1;
                
                if let Some(body) = node.children.get(2) {
                    for stmt in &body.children {
                        output.push_str(&self.generate_statement(stmt)?);
                    }
                }
                
                if let Some(inc) = node.children.get(3) {
                    output.push_str(&self.generate_statement(inc)?);
                }
                
                self.indent_level -= 1;
            }
        }
        
        Ok(output)
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
        
        for node in &ast.children {
            match node.node_type.as_str() {
                "FuncDef" | "FuncDecl" => {
                    output.push_str(&generator.generate_function(node)?);
                }
                "Decl" => {
                    output.push_str(&generator.generate_declaration(node)?);
                }
                _ => {
                    debug!("Пропуск корневого узла: {}", node.node_type);
                }
            }
        }
        
        Ok(output)
    }
    
    fn language_name() -> &'static str {
        "Python"
    }
}