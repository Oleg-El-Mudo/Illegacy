use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::ast_converter::AstConverter;

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

    // Ищем операнд (expr)
    if let Some(expr) = obj.get("expr") {
        debug!("Операнд унарной операции: {:#?}", expr);
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}

/// Обработка узла TernaryOp
pub fn handle_ternary_op(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка TernaryOp узла");
    debug!("Содержимое TernaryOp: {:#?}", obj);

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Обрабатываем условие (cond)
    if let Some(cond) = obj.get("cond") {
        debug!("Условие TernaryOp: {:#?}", cond);
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    // Обрабатываем значение если true (iftrue)
    if let Some(iftrue) = obj.get("iftrue") {
        debug!("Значение если true: {:#?}", iftrue);
        if let Ok(true_node) = AstConverter::parse_ast_node(iftrue) {
            node.children.push(true_node);
        }
    }

    // Обрабатываем значение если false (iffalse)
    if let Some(iffalse) = obj.get("iffalse") {
        debug!("Значение если false: {:#?}", iffalse);
        if let Ok(false_node) = AstConverter::parse_ast_node(iffalse) {
            node.children.push(false_node);
        }
    }

    Ok(())
}

/// Обработка узла Cast
pub fn handle_cast(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка приведения типа (cast)");
    debug!("Содержимое Cast: {:#?}", obj);

    // Сохраняем тип, к которому приводим
    if let Some(to_type) = obj.get("to_type") {
        node.attributes
            .insert("to_type".to_string(), to_type.clone());
    }

    // Обрабатываем выражение, которое приводится
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
    debug!("Содержимое узла FuncCall: {:#?}", obj);

    // Ищем имя функции
    if let Some(name_obj) = obj.get("name") {
        if let Some(name_obj) = name_obj.as_object() {
            if let Some(name) = name_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes
                    .insert("name".to_string(), Value::String(name.to_string()));

                // СОХРАНЯЕМ ИМЯ ФУНКЦИИ В АТРИБУТЕ func_name ДЛЯ ЛЕГКОГО ДОСТУПА
                node.attributes
                    .insert("func_name".to_string(), Value::String(name.to_string()));

                debug!("Найдено имя функции в вызове: {}", name);
            }
        }
    }

    // Ищем аргументы
    if let Some(args) = obj.get("args") {
        debug!("Найдены аргументы функции: {:#?}", args);
        if let Some(args_obj) = args.as_object() {
            if args_obj
                .get("__node__")
                .and_then(|n| n.as_str())
                .eq(&Some("ExprList"))
            {
                debug!("Найдены аргументы функции (ExprList)");
                if let Ok(args_node) = AstConverter::parse_ast_node(args) {
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

    Ok(())
}

/// Обработка узла ArrayRef
pub fn handle_array_ref(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка обращения к элементу массива");
    debug!("Содержимое ArrayRef: {:#?}", obj);

    // Имя массива
    if let Some(name_obj) = obj.get("name") {
        if let Ok(name_node) = AstConverter::parse_ast_node(name_obj) {
            node.children.push(name_node);
        }
    }

    // Индекс
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
    debug!("Содержимое ExprList: {:#?}", obj);

    // В pycparser ExprList может иметь поля вида "exprs[0]", "exprs[1]" и т.д.
    let mut values = std::collections::HashMap::new();

    for (key, value) in obj {
        if key.starts_with("exprs[") && key.ends_with(']') {
            // Извлекаем индекс из ключа "exprs[0]" -> "0"
            if let Some(index_str) = key
                .strip_prefix("exprs[")
                .and_then(|s| s.strip_suffix(']'))
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
            if let Ok(expr_node) = AstConverter::parse_ast_node(value) {
                node.children.push(expr_node);
                debug!("Добавлен аргумент");
            }
        }
    }

    debug!(
        "Итоговое количество детей в ExprList: {}",
        node.children.len()
    );

    Ok(())
}

/// Обработка узла InitList
pub fn handle_init_list(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка InitList");
    debug!("Содержимое InitList: {:#?}", obj);

    // В pycparser InitList может иметь поля вида "exprs[0]", "exprs[1]" и т.д.
    // Собираем все значения в вектор, чтобы избежать дублирования
    let mut values = std::collections::HashMap::new();

    for (key, value) in obj {
        if key.starts_with("exprs[") && key.ends_with(']') {
            // Извлекаем индекс из ключа "exprs[0]" -> "0"
            if let Some(index_str) = key
                .strip_prefix("exprs[")
                .and_then(|s| s.strip_suffix(']'))
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
            if let Ok(expr_node) = AstConverter::parse_ast_node(value) {
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
                debug!("Количество выражений в InitList: {}", exprs_array.len());
                for (i, expr) in exprs_array.iter().enumerate() {
                    debug!("  InitList[{}]: {:#?}", i, expr);
                    if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
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

    Ok(())
}

/// Обработка узла NamedInitializer
pub fn handle_named_initializer(
    obj: &serde_json::Map<String, Value>,
    node: &mut ASTNode,
) -> Result<()> {
    debug!("Обработка NamedInitializer (именованного инициализатора)");
    debug!("Содержимое NamedInitializer: {:#?}", obj);

    // Обрабатываем имя поля (может быть в виде "name[0]" или просто "name")
    let mut name_indices = Vec::new();
    for (key, value) in obj {
        if key.starts_with("name[") && key.ends_with(']') {
            if let Some(index_str) = key
                .strip_prefix("name[")
                .and_then(|s| s.strip_suffix(']'))
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
        node.attributes.insert(format!("name[{}]", i), (*value).clone());
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
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}
