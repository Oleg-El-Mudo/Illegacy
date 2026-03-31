use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::c_parser::ast_converter::AstConverter;

/// Обработка узла BinaryOp
pub fn handle_binary_op(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    if let Some(op) = obj.get("op") {
        node.attributes.insert("op".to_string(), op.clone());
    }
    Ok(())
}

/// Обработка узла Assignment
pub fn handle_assignment(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    if let Some(op) = obj.get("op") {
        node.attributes.insert("op".to_string(), op.clone());
    }
    Ok(())
}

/// Обработка узла UnaryOp
pub fn handle_unary_op(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    if let Some(op) = obj.get("op") {
        node.attributes.insert("op".to_string(), op.clone());
    }

    if let Some(expr) = obj.get("expr") {
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}

/// Обработка узла TernaryOp
pub fn handle_ternary_op(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка TernaryOp узла");

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(iftrue) = obj.get("iftrue") {
        if let Ok(true_node) = AstConverter::parse_ast_node(iftrue) {
            node.children.push(true_node);
        }
    }

    if let Some(iffalse) = obj.get("iffalse") {
        if let Ok(false_node) = AstConverter::parse_ast_node(iffalse) {
            node.children.push(false_node);
        }
    }

    Ok(())
}

/// Обработка узла Cast
pub fn handle_cast(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка приведения типа (cast)");

    if let Some(to_type) = obj.get("to_type") {
        node.attributes.insert("to_type".to_string(), to_type.clone());
    }

    if let Some(expr) = obj.get("expr") {
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}

/// Обработка узла FuncCall
pub fn handle_func_call(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка вызова функции");

    if let Some(name_obj) = obj.get("name") {
        if let Some(name_obj) = name_obj.as_object() {
            if let Some(name) = name_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes.insert("name".to_string(), Value::String(name.to_string()));
                node.attributes.insert("func_name".to_string(), Value::String(name.to_string()));
                debug!("Найдено имя функции в вызове: {}", name);
            }
        }
    }

    // Обработка аргументов функции
    if let Some(args) = obj.get("args") {
        debug!("Найден args для функции: {:#?}", args);
        
        // Проверяем, является ли args непосредственно ExprList
        if let Some(args_obj) = args.as_object() {
            if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ExprList") {
                debug!("args является ExprList, парсим как узел");
                if let Ok(args_node) = AstConverter::parse_ast_node(args) {
                    node.children.push(args_node);
                }
            } else {
                // Возможно args - это массив выражений напрямую
                if let Some(exprs) = args_obj.get("exprs") {
                    if let Some(exprs_array) = exprs.as_array() {
                        debug!("Найден массив exprs с {} элементами", exprs_array.len());
                        for expr in exprs_array {
                            if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
                                node.children.push(expr_node);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла ArrayRef
pub fn handle_array_ref(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка обращения к элементу массива");

    if let Some(name_obj) = obj.get("name") {
        if let Ok(name_node) = AstConverter::parse_ast_node(name_obj) {
            node.children.push(name_node);
        }
    }

    if let Some(subscript) = obj.get("subscript") {
        if let Ok(index_node) = AstConverter::parse_ast_node(subscript) {
            node.children.push(index_node);
        }
    }

    Ok(())
}

/// Обработка узла ExprList
pub fn handle_expr_list(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ExprList");

    let mut values = std::collections::HashMap::new();

    // Сначала пробуем найти ключи exprs[0], exprs[1] и т.д.
    for (key, value) in obj {
        if key.starts_with("exprs[") && key.ends_with(']') {
            if let Some(index_str) = key.strip_prefix("exprs[").and_then(|s| s.strip_suffix(']')) {
                if let Ok(index) = index_str.parse::<usize>() {
                    values.insert(index, value);
                }
            }
        }
    }

    // Если не найдены ключи exprs[0], exprs[1], пробуем найти массив exprs
    if values.is_empty() {
        if let Some(exprs) = obj.get("exprs") {
            if let Some(exprs_array) = exprs.as_array() {
                debug!("Найден массив exprs с {} элементами", exprs_array.len());
                for (index, expr) in exprs_array.iter().enumerate() {
                    values.insert(index, expr);
                }
            }
        }
    }

    let mut indices: Vec<_> = values.keys().collect();
    indices.sort();

    for index in indices {
        if let Some(value) = values.get(index) {
            debug!("  Добавляем аргумент {}: тип={}", index, 
                value.as_object().and_then(|o| o.get("__node__")).and_then(|n| n.as_str()).unwrap_or("unknown"));
            if let Ok(expr_node) = AstConverter::parse_ast_node(value) {
                node.children.push(expr_node);
            }
        }
    }

    debug!("  Всего добавлено {} детей", node.children.len());

    Ok(())
}

/// Обработка узла InitList
pub fn handle_init_list(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка InitList");

    let mut values = std::collections::HashMap::new();

    // Сначала пробуем найти ключи exprs[0], exprs[1] и т.д.
    for (key, value) in obj {
        if key.starts_with("exprs[") && key.ends_with(']') {
            if let Some(index_str) = key.strip_prefix("exprs[").and_then(|s| s.strip_suffix(']')) {
                if let Ok(index) = index_str.parse::<usize>() {
                    values.insert(index, value);
                }
            }
        }
    }

    // Если не найдены ключи exprs[0], exprs[1], пробуем найти массив exprs
    if values.is_empty() {
        if let Some(exprs) = obj.get("exprs") {
            if let Some(exprs_array) = exprs.as_array() {
                debug!("Найден массив exprs в InitList с {} элементами", exprs_array.len());
                for (index, expr) in exprs_array.iter().enumerate() {
                    values.insert(index, expr);
                }
            }
        }
    }

    let mut indices: Vec<_> = values.keys().collect();
    indices.sort();

    for index in indices {
        if let Some(value) = values.get(index) {
            if let Ok(expr_node) = AstConverter::parse_ast_node(value) {
                node.children.push(expr_node);
            }
        }
    }

    debug!("  Всего добавлено {} детей в InitList", node.children.len());

    Ok(())
}

/// Обработка узла NamedInitializer
pub fn handle_named_initializer(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка NamedInitializer");

    let mut name_indices = Vec::new();
    for (key, value) in obj {
        if key.starts_with("name[") && key.ends_with(']') {
            if let Some(index_str) = key.strip_prefix("name[").and_then(|s| s.strip_suffix(']')) {
                if let Ok(index) = index_str.parse::<usize>() {
                    name_indices.push((index, value));
                }
            }
        }
    }

    name_indices.sort_by_key(|(i, _)| *i);
    for (i, (_, value)) in name_indices.iter().enumerate() {
        node.attributes.insert(format!("name[{}]", i), (*value).clone());
    }

    if name_indices.is_empty() {
        if let Some(name) = obj.get("name") {
            node.attributes.insert("name".to_string(), name.clone());
        }
    }

    if let Some(expr) = obj.get("expr") {
        node.attributes.insert("expr".to_string(), expr.clone());
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}
