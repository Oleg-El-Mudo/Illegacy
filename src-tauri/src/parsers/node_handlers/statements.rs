use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::ast_converter::AstConverter;

/// Обработка узла If
pub fn handle_if(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
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
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    // Обрабатываем тело if (then)
    if let Some(iftrue) = obj.get("iftrue") {
        debug!("Тело THEN (iftrue): {:#?}", iftrue);
        if let Some(coord) = iftrue.get("coord").and_then(|c| c.as_str()) {
            debug!("Тело THEN координаты: {}", coord);
        }
        if let Ok(body_node) = AstConverter::parse_ast_node(iftrue) {
            node.children.push(body_node);
        }
    }

    // Обрабатываем тело else (iffalse)
    if let Some(iffalse) = obj.get("iffalse") {
        debug!("Тело ELSE (iffalse): {:#?}", iffalse);
        if let Some(coord) = iffalse.get("coord").and_then(|c| c.as_str()) {
            debug!("Тело ELSE координаты: {}", coord);
        }
        if let Ok(body_node) = AstConverter::parse_ast_node(iffalse) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла Switch
pub fn handle_switch(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка SWITCH узла");
    debug!("Содержимое SWITCH: {:#?}", obj);

    // Сохраняем координаты
    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Обрабатываем выражение switch (условие)
    if let Some(cond) = obj.get("cond") {
        debug!("Условие SWITCH: {:#?}", cond);
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    // Обрабатываем тело switch (список case/default)
    if let Some(stmt) = obj.get("stmt") {
        debug!("Тело SWITCH: {:#?}", stmt);
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла Case
pub fn handle_case(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка CASE узла");
    debug!("Содержимое CASE: {:#?}", obj);

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Обрабатываем значение case (expr)
    if let Some(expr) = obj.get("expr") {
        debug!("Значение CASE: {:#?}", expr);
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    // Обрабатываем операторы в case (stmts)
    if let Some(stmts) = obj.get("stmts") {
        debug!("Операторы CASE: {:#?}", stmts);

        // Создаем Compound узел для операторов
        let mut compound_node = ASTNode::new("Compound");

        if let Some(stmts_array) = stmts.as_array() {
            for stmt in stmts_array {
                if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
                    compound_node.children.push(stmt_node);
                }
            }
        } else {
            // Может быть объект с полями stmts[0], stmts[1] и т.д.
            let mut i = 0;
            loop {
                let key = format!("stmts[{}]", i);
                if let Some(stmt) = obj.get(&key) {
                    if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
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

    Ok(())
}

/// Обработка узла Default
pub fn handle_default(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
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
                if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
                    compound_node.children.push(stmt_node);
                }
            }
        } else {
            // Может быть объект с полями stmts[0], stmts[1] и т.д.
            let mut i = 0;
            loop {
                let key = format!("stmts[{}]", i);
                if let Some(stmt) = obj.get(&key) {
                    if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
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

    Ok(())
}

/// Обработка узла Return
pub fn handle_return(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка RETURN узла");
    debug!("Содержимое Return: {:#?}", obj);

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Ищем выражение для возврата (может быть в поле "expr")
    if let Some(expr) = obj.get("expr") {
        debug!("Выражение return: {:#?}", expr);
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}

/// Обработка узла Compound (блок кода)
pub fn handle_compound(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка Compound");

    // В pycparser Compound может иметь поле "block_items" с массивом операторов
    if let Some(block_items) = obj.get("block_items") {
        debug!("Найдены block_items в Compound");
        if let Some(items_array) = block_items.as_array() {
            debug!("Количество операторов в Compound: {}", items_array.len());
            for (i, item) in items_array.iter().enumerate() {
                debug!("  Оператор {} в Compound: {:#?}", i, item);
                if let Ok(stmt_node) = AstConverter::parse_ast_node(item) {
                    node.children.push(stmt_node);
                }
            }
        }
    } else {
        debug!("Ищем отдельные поля в Compound");
        let mut i = 0;
        loop {
            let key = format!("block_items[{}]", i);
            if let Some(item) = obj.get(&key) {
                debug!("Найдено поле {} в Compound", key);
                if let Ok(stmt_node) = AstConverter::parse_ast_node(item) {
                    node.children.push(stmt_node);
                }
                i += 1;
            } else {
                break;
            }
        }
    }

    debug!("Compound обработан, добавлено {} детей", node.children.len());

    Ok(())
}

/// Обработка узла While
pub fn handle_while(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка WHILE узла");
    debug!("Содержимое WHILE: {:#?}", obj);

    // Сохраняем координаты
    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Обрабатываем условие (cond)
    if let Some(cond) = obj.get("cond") {
        debug!("Условие WHILE: {:#?}", cond);
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    // Обрабатываем тело (stmt)
    if let Some(stmt) = obj.get("stmt") {
        debug!("Тело WHILE: {:#?}", stmt);
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла DoWhile
pub fn handle_dowhile(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка DO-WHILE узла");
    debug!("Содержимое DO-WHILE: {:#?}", obj);

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    // Тело цикла (stmt)
    if let Some(stmt) = obj.get("stmt") {
        debug!("Тело DO-WHILE: {:#?}", stmt);
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    // Условие (cond)
    if let Some(cond) = obj.get("cond") {
        debug!("Условие DO-WHILE: {:#?}", cond);
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    Ok(())
}

/// Обработка узла For
pub fn handle_for(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка FOR узла");
    debug!("ПОЛНОЕ содержимое FOR узла: {:#?}", obj);

    // Обрабатываем все атрибуты для отладки
    for (key, value) in obj {
        debug!("  FOR атрибут {}: {:#?}", key, value);
    }

    // Обрабатываем все поля как детей
    if let Some(init) = obj.get("init") {
        debug!("FOR init: {:#?}", init);
        if let Ok(init_node) = AstConverter::parse_ast_node(init) {
            node.children.push(init_node);
        }
    }

    if let Some(cond) = obj.get("cond") {
        debug!("FOR cond: {:#?}", cond);
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(next) = obj.get("next") {
        debug!("FOR next: {:#?}", next);
        if let Ok(next_node) = AstConverter::parse_ast_node(next) {
            node.children.push(next_node);
        }
    }

    if let Some(stmt) = obj.get("stmt") {
        debug!("FOR stmt: {:#?}", stmt);
        if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(stmt_node);
        }
    }

    Ok(())
}
