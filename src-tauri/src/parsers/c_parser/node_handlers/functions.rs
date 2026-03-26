use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::c_parser::ast_converter::AstConverter;

/// Обработка узла FuncDef (определение функции)
pub fn handle_func_def(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления функции");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(decl) = obj.get("decl") {
        if let Some(decl_obj) = decl.as_object() {
            if let Some(decl_name) = decl_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes.insert("name".to_string(), Value::String(decl_name.to_string()));
            }

            if let Some(type_obj) = decl_obj.get("type") {
                if let Some(type_obj) = type_obj.as_object() {
                    if let Some(args) = type_obj.get("args") {
                        if let Some(args_obj) = args.as_object() {
                            if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ParamList") {
                                AstConverter::extract_params_from_paramlist(args_obj, node)?;
                            }
                        }
                    }
                }
            }

            if let Some(args) = decl_obj.get("args") {
                if let Some(args_obj) = args.as_object() {
                    if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ParamList") {
                        AstConverter::extract_params_from_paramlist(args_obj, node)?;
                    }
                }
            }
        }
    }

    if let Some(ret_type) = obj.get("type") {
        if let Some(ret_obj) = ret_type.as_object() {
            if let Some(type_name) = ret_obj.get("names").and_then(|n| n.as_array()) {
                if let Some(first_name) = type_name.first().and_then(|n| n.as_str()) {
                    node.attributes.insert("return_type".to_string(), Value::String(first_name.to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла FuncDecl (объявление функции)
pub fn handle_func_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления функции (FuncDecl)");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(decl) = obj.get("decl") {
        if let Some(decl_obj) = decl.as_object() {
            if let Some(decl_name) = decl_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes.insert("name".to_string(), Value::String(decl_name.to_string()));
            }

            if let Some(type_obj) = decl_obj.get("type") {
                if let Some(type_obj) = type_obj.as_object() {
                    if let Some(args) = type_obj.get("args") {
                        if let Some(args_obj) = args.as_object() {
                            if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ParamList") {
                                AstConverter::extract_params_from_paramlist(args_obj, node)?;
                            }
                        }
                    }
                }
            }

            if let Some(args) = decl_obj.get("args") {
                if let Some(args_obj) = args.as_object() {
                    if args_obj.get("__node__").and_then(|n| n.as_str()) == Some("ParamList") {
                        AstConverter::extract_params_from_paramlist(args_obj, node)?;
                    }
                }
            }
        }
    }

    if let Some(ret_type) = obj.get("type") {
        if let Some(ret_obj) = ret_type.as_object() {
            if let Some(type_name) = ret_obj.get("names").and_then(|n| n.as_array()) {
                if let Some(first_name) = type_name.first().and_then(|n| n.as_str()) {
                    node.attributes.insert("return_type".to_string(), Value::String(first_name.to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла ParamList
pub fn handle_param_list(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ParamList");

    if let Some(params) = obj.get("params") {
        node.attributes.insert("params".to_string(), params.clone());

        if let Some(params_array) = params.as_array() {
            for param in params_array {
                if let Ok(param_node) = AstConverter::parse_ast_node(param) {
                    node.children.push(param_node);
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла ParamDecl
pub fn handle_param_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ParamDecl");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes.insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(type_obj) = obj.get("type") {
        if let Some(type_obj) = type_obj.as_object() {
            if let Some(names) = type_obj.get("names") {
                if let Some(names_array) = names.as_array() {
                    if let Some(first_name) = names_array.first().and_then(|n| n.as_str()) {
                        node.attributes.insert("type".to_string(), Value::String(first_name.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}
