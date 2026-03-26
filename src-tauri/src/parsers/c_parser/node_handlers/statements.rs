use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::c_parser::ast_converter::AstConverter;

/// Обработка узла If
pub fn handle_if(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка IF узла");
    debug!("Содержимое IF узла: {:#?}", obj);

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(iftrue) = obj.get("iftrue") {
        if let Ok(body_node) = AstConverter::parse_ast_node(iftrue) {
            node.children.push(body_node);
        }
    }

    if let Some(iffalse) = obj.get("iffalse") {
        if let Ok(body_node) = AstConverter::parse_ast_node(iffalse) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла Switch
pub fn handle_switch(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка SWITCH узла");

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(stmt) = obj.get("stmt") {
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла Case
pub fn handle_case(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка CASE узла");

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(expr) = obj.get("expr") {
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    if let Some(stmts) = obj.get("stmts") {
        let mut compound_node = ASTNode::new("Compound");

        if let Some(stmts_array) = stmts.as_array() {
            for stmt in stmts_array {
                if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
                    compound_node.children.push(stmt_node);
                }
            }
        } else {
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

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(stmts) = obj.get("stmts") {
        let mut compound_node = ASTNode::new("Compound");

        if let Some(stmts_array) = stmts.as_array() {
            for stmt in stmts_array {
                if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
                    compound_node.children.push(stmt_node);
                }
            }
        } else {
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

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(expr) = obj.get("expr") {
        if let Ok(expr_node) = AstConverter::parse_ast_node(expr) {
            node.children.push(expr_node);
        }
    }

    Ok(())
}

/// Обработка узла Compound
pub fn handle_compound(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка Compound");

    if let Some(block_items) = obj.get("block_items") {
        if let Some(items_array) = block_items.as_array() {
            for item in items_array {
                if let Ok(stmt_node) = AstConverter::parse_ast_node(item) {
                    node.children.push(stmt_node);
                }
            }
        }
    } else {
        let mut i = 0;
        loop {
            let key = format!("block_items[{}]", i);
            if let Some(item) = obj.get(&key) {
                if let Ok(stmt_node) = AstConverter::parse_ast_node(item) {
                    node.children.push(stmt_node);
                }
                i += 1;
            } else {
                break;
            }
        }
    }

    Ok(())
}

/// Обработка узла While
pub fn handle_while(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка WHILE узла");

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(stmt) = obj.get("stmt") {
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    Ok(())
}

/// Обработка узла DoWhile
pub fn handle_dowhile(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка DO-WHILE узла");

    if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
        node.coord = Some(coord.to_string());
    }

    if let Some(stmt) = obj.get("stmt") {
        if let Ok(body_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(body_node);
        }
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    Ok(())
}

/// Обработка узла For
pub fn handle_for(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка FOR узла");

    if let Some(init) = obj.get("init") {
        if let Ok(init_node) = AstConverter::parse_ast_node(init) {
            node.children.push(init_node);
        }
    }

    if let Some(cond) = obj.get("cond") {
        if let Ok(cond_node) = AstConverter::parse_ast_node(cond) {
            node.children.push(cond_node);
        }
    }

    if let Some(next) = obj.get("next") {
        if let Ok(next_node) = AstConverter::parse_ast_node(next) {
            node.children.push(next_node);
        }
    }

    if let Some(stmt) = obj.get("stmt") {
        if let Ok(stmt_node) = AstConverter::parse_ast_node(stmt) {
            node.children.push(stmt_node);
        }
    }

    Ok(())
}
