use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::ast_converter::AstConverter;

/// Обработка узла Struct
pub fn handle_struct(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка STRUCT узла");
    debug!("ПОЛНОЕ содержимое STRUCT: {:#?}", obj);

    // Сохраняем имя структуры
    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
        debug!("Имя структуры: {}", name);
    }

    // Сохраняем всё JSON представление для дальнейшего использования
    node.attributes
        .insert("_json".to_string(), Value::Object(obj.clone()));

    // Обрабатываем поля структуры (decls)
    if let Some(decls) = obj.get("decls") {
        debug!("Поля структуры: {:#?}", decls);

        if let Some(decls_array) = decls.as_array() {
            for decl in decls_array {
                if let Ok(decl_node) = AstConverter::parse_ast_node(decl) {
                    node.children.push(decl_node);
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла StructDecl
pub fn handle_struct_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка STRUCT DECL (объявление переменной типа структуры)");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(type_obj) = obj.get("type") {
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    Ok(())
}

/// Обработка узла StructRef
pub fn handle_struct_ref(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка доступа к полю структуры");

    // Имя структуры (обычно ID)
    if let Some(name_obj) = obj.get("name") {
        if let Ok(name_node) = AstConverter::parse_ast_node(name_obj) {
            node.children.push(name_node);
        }
    }

    // Имя поля
    if let Some(field) = obj.get("field") {
        if let Ok(field_node) = AstConverter::parse_ast_node(field) {
            node.children.push(field_node);
        }
    }

    Ok(())
}

/// Обработка узла Union
pub fn handle_union(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка UNION узла");
    debug!("ПОЛНОЕ содержимое UNION: {:#?}", obj);

    // Сохраняем имя объединения
    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
        debug!("Имя объединения: {}", name);
    }

    // Сохраняем всё JSON представление для дальнейшего использования
    node.attributes
        .insert("_json".to_string(), Value::Object(obj.clone()));

    // Сохраняем информацию о полях для вложенных структур
    let mut field_types = serde_json::Map::new();

    // Обрабатываем поля объединения (decls)
    if let Some(decls) = obj.get("decls") {
        debug!("Поля объединения: {:#?}", decls);

        if let Some(decls_array) = decls.as_array() {
            for decl in decls_array {
                if let Ok(decl_node) = AstConverter::parse_ast_node(decl) {
                    // Сохраняем информацию о типе поля
                    if let Some(field_name) =
                        decl_node.attributes.get("name").and_then(|v: &Value| v.as_str())
                    {
                        if let Some(type_attr) = decl_node.attributes.get("type") {
                            field_types.insert(field_name.to_string(), type_attr.clone());
                        }
                    }
                    node.children.push(decl_node);
                }
            }
        }
    }

    if !field_types.is_empty() {
        node.attributes
            .insert("field_types".to_string(), Value::Object(field_types));
    }

    Ok(())
}

/// Обработка узла UnionDecl
pub fn handle_union_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка UNION DECL (объявление переменной типа объединения)");

    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
    }

    if let Some(type_obj) = obj.get("type") {
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    Ok(())
}

/// Обработка узла Enum
pub fn handle_enum(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ENUM узла");
    debug!("ПОЛНОЕ содержимое ENUM: {:#?}", obj);

    // Сохраняем имя перечисления
    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
        debug!("Имя перечисления: {}", name);
    }

    // Сохраняем всё JSON представление для дальнейшего использования
    node.attributes
        .insert("_json".to_string(), Value::Object(obj.clone()));

    // Обрабатываем значения перечисления (values)
    if let Some(values) = obj.get("values") {
        debug!("Значения enum: {:#?}", values);

        if let Some(values_array) = values.as_array() {
            for value in values_array {
                if let Ok(enumerator_node) = AstConverter::parse_ast_node(value) {
                    node.children.push(enumerator_node);
                }
            }
        }
    }

    Ok(())
}

/// Обработка узла Enumerator
pub fn handle_enumerator(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка ENUMERATOR узла");
    debug!("Содержимое ENUMERATOR: {:#?}", obj);

    // Сохраняем имя элемента перечисления
    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
        debug!("Имя элемента enum: {}", name);
    }

    // Сохраняем значение, если указано явно (например, RED=5)
    if let Some(value) = obj.get("value") {
        node.attributes.insert("value".to_string(), value.clone());
        debug!("Значение элемента enum: {:?}", value);
    }

    Ok(())
}

/// Обработка узла IdentifierType
pub fn handle_identifier_type(
    obj: &serde_json::Map<String, Value>,
    node: &mut ASTNode,
) -> Result<()> {
    // Для идентификаторов типов
    if let Some(names) = obj.get("names") {
        if let Some(names_array) = names.as_array() {
            if let Some(first_name) = names_array.first().and_then(|n| n.as_str()) {
                node.attributes
                    .insert("name".to_string(), Value::String(first_name.to_string()));
            }
        }
    }

    Ok(())
}

/// Обработка узла Constant
pub fn handle_constant(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    // Для констант
    if let Some(value) = obj.get("value") {
        // Если это строковое представление числа с суффиксом 'f', убираем суффикс
        if let Some(value_str) = value.as_str() {
            if value_str.ends_with('f') && value_str[..value_str.len() - 1].parse::<f64>().is_ok() {
                let clean_value = value_str[..value_str.len() - 1].to_string();
                node.attributes
                    .insert("value".to_string(), Value::String(clean_value));
            } else {
                node.attributes.insert("value".to_string(), value.clone());
            }
        } else {
            node.attributes.insert("value".to_string(), value.clone());
        }
    }
    if let Some(type_name) = obj.get("type") {
        node.attributes
            .insert("type".to_string(), type_name.clone());
    }

    Ok(())
}

/// Обработка узла ID
pub fn handle_id(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    // Для идентификаторов (имена переменных)
    if let Some(name) = obj.get("name") {
        node.attributes.insert("name".to_string(), name.clone());
    }

    Ok(())
}
