use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::ast_converter::AstConverter;

/// Обработка узла FuncDef (определение функции)
pub fn handle_func_def(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
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
            if let Some(decl_name) = decl_obj.get("name").and_then(|n| n.as_str()) {
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
                            if args_obj
                                .get("__node__")
                                .and_then(|n| n.as_str())
                                .eq(&Some("ParamList"))
                            {
                                debug!("Найдены параметры функции в type.args");
                                AstConverter::extract_params_from_paramlist(args_obj, node)?;
                            }
                        }
                    }
                }
            }

            // 4. Также ищем параметры в прямом атрибуте args (если есть)
            if let Some(args) = decl_obj.get("args") {
                debug!("Найден прямой args в decl: {:#?}", args);
                if let Some(args_obj) = args.as_object() {
                    if args_obj
                        .get("__node__")
                        .and_then(|n| n.as_str())
                        .eq(&Some("ParamList"))
                    {
                        debug!("Найдены параметры функции в decl.args");
                        AstConverter::extract_params_from_paramlist(args_obj, node)?;
                    }
                }
            }
        }
    }

    // 5. Проверяем тип возврата
    if let Some(ret_type) = obj.get("type") {
        if let Some(ret_obj) = ret_type.as_object() {
            if let Some(type_name) = ret_obj.get("names").and_then(|n| n.as_array()) {
                if let Some(first_name) = type_name.first().and_then(|n| n.as_str()) {
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

    Ok(())
}

/// Обработка узла FuncDecl (объявление функции)
pub fn handle_func_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    // Обработка аналогична FuncDef, но без тела функции
    debug!("Обработка объявления функции (FuncDecl)");

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
            if let Some(decl_name) = decl_obj.get("name").and_then(|n| n.as_str()) {
                node.attributes.insert(
                    "name".to_string(),
                    Value::String(decl_name.to_string()),
                );
                debug!("Найдено имя функции (через decl): {}", decl_name);
            }

            // Ищем параметры функции в declarator
            if let Some(type_obj) = decl_obj.get("type") {
                debug!("Найден type в decl: {:#?}", type_obj);
                if let Some(type_obj) = type_obj.as_object() {
                    if let Some(args) = type_obj.get("args") {
                        debug!("Найден args в type: {:#?}", args);
                        if let Some(args_obj) = args.as_object() {
                            if args_obj
                                .get("__node__")
                                .and_then(|n| n.as_str())
                                .eq(&Some("ParamList"))
                            {
                                debug!("Найдены параметры функции в type.args");
                                AstConverter::extract_params_from_paramlist(args_obj, node)?;
                            }
                        }
                    }
                }
            }

            if let Some(args) = decl_obj.get("args") {
                debug!("Найден прямой args в decl: {:#?}", args);
                if let Some(args_obj) = args.as_object() {
                    if args_obj
                        .get("__node__")
                        .and_then(|n| n.as_str())
                        .eq(&Some("ParamList"))
                    {
                        debug!("Найдены параметры функции в decl.args");
                        AstConverter::extract_params_from_paramlist(args_obj, node)?;
                    }
                }
            }
        }
    }

    // Проверяем тип возврата
    if let Some(ret_type) = obj.get("type") {
        if let Some(ret_obj) = ret_type.as_object() {
            if let Some(type_name) = ret_obj.get("names").and_then(|n| n.as_array()) {
                if let Some(first_name) = type_name.first().and_then(|n| n.as_str()) {
                    node.attributes.insert(
                        "return_type".to_string(),
                        Value::String(first_name.to_string()),
                    );
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла ParamList
pub fn handle_param_list(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ParamList");

    // Сохраняем параметры как атрибут
    if let Some(params) = obj.get("params") {
        node.attributes.insert("params".to_string(), params.clone());

        // Добавляем параметры как детей
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
                    if let Some(first_name) = names_array.first().and_then(|n| n.as_str()) {
                        node.attributes.insert(
                            "type".to_string(),
                            Value::String(first_name.to_string()),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
