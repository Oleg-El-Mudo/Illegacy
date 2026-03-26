use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::c_parser::ast_converter::AstConverter;

/// Обработка узла Decl (объявление переменной/параметра)
pub fn handle_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка Decl узла");

    let decl_name = obj.get("name").and_then(|n| n.as_str()).map(String::from);

    if let Some(ref name) = decl_name {
        node.attributes.insert("name".to_string(), Value::String(name.clone()));
    }

    let mut init_node = None;
    if let Some(init) = obj.get("init") {
        if let Ok(init_node_val) = AstConverter::parse_ast_node(init) {
            init_node = Some(init_node_val);
        }
    }

    if let Some(type_obj) = obj.get("type") {
        if let Some(type_obj_map) = type_obj.as_object() {
            if let Some(type_node) = type_obj_map.get("__node__").and_then(|n| n.as_str()) {
                if type_node == "ArrayDecl" {
                    if let Ok(mut array_node) = AstConverter::parse_ast_node(type_obj) {
                        if let Some(ref name) = decl_name {
                            array_node.attributes.insert("name".to_string(), Value::String(name.clone()));
                        }
                        if let Some(init) = init_node {
                            array_node.children.push(init);
                        }
                        node.children.push(array_node);
                    }
                } else {
                    if let Ok(type_decl_node) = AstConverter::parse_ast_node(type_obj) {
                        node.children.push(type_decl_node);
                    }
                    if let Some(init) = init_node {
                        node.children.push(init);
                    }
                }
            }
        }
    } else if let Some(init) = init_node {
        node.children.push(init);
    }

    Ok(())
}

/// Обработка узла ArrayDecl
pub fn handle_array_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления массива");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(type_obj) = obj.get("type") {
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    if let Some(dim) = obj.get("dim") {
        if let Ok(dim_node) = AstConverter::parse_ast_node(dim) {
            node.children.push(dim_node);
        }
    }

    Ok(())
}

/// Обработка узла PtrDecl (объявление указателя)
pub fn handle_ptr_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления указателя");

    if let Some(quals) = obj.get("quals") {
        node.attributes.insert("quals".to_string(), quals.clone());
    }

    if let Some(type_obj) = obj.get("type") {
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    Ok(())
}

/// Обработка узла TypeDecl
pub fn handle_type_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка TypeDecl узла");

    if let Some(declname) = obj.get("declname").and_then(|v| v.as_str()) {
        node.attributes.insert("name".to_string(), Value::String(declname.to_string()));
    }

    if let Some(type_obj) = obj.get("type") {
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }

        node.attributes.insert("type".to_string(), type_obj.clone());

        if let Some(type_obj_map) = type_obj.as_object() {
            if let Some(node_type) = type_obj_map.get("__node__").and_then(|n| n.as_str()) {
                if node_type == "Struct" {
                    if let Some(struct_name) = type_obj_map.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert("struct_type".to_string(), Value::String(struct_name.to_string()));
                    }
                } else if node_type == "Union" {
                    if let Some(union_name) = type_obj_map.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert("union_type".to_string(), Value::String(union_name.to_string()));
                    }
                }
            }
        }
    }

    if let Some(quals) = obj.get("quals") {
        node.attributes.insert("quals".to_string(), quals.clone());
    }

    Ok(())
}
