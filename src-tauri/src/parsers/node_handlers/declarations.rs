use anyhow::Result;
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;
use crate::parsers::ast_converter::AstConverter;

/// Обработка узла Decl (объявление переменной/параметра)
pub fn handle_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка Decl узла");
    debug!("Полное содержимое Decl: {:#?}", obj);

    // Для объявлений переменных
    let decl_name = obj.get("name").and_then(|n| n.as_str()).map(String::from);
    debug!("Имя в Decl: {:?}", decl_name);

    if let Some(ref name) = decl_name {
        node.attributes
            .insert("name".to_string(), Value::String(name.clone()));
    }

    // Сначала обрабатываем инициализатор, если есть
    let mut init_node = None;
    if let Some(init) = obj.get("init") {
        debug!("Инициализатор в Decl: {:#?}", init);
        if let Ok(init_node_val) = AstConverter::parse_ast_node(init) {
            debug!("Создан инициализатор типа: {}", init_node_val.node_type);
            init_node = Some(init_node_val);
        }
    }

    // Проверяем тип объявления
    if let Some(type_obj) = obj.get("type") {
        debug!("Тип в Decl: {:#?}", type_obj);

        if let Some(type_obj_map) = type_obj.as_object() {
            if let Some(type_node) = type_obj_map.get("__node__").and_then(|n| n.as_str()) {
                debug!("Тип узла: {}", type_node);

                if type_node == "ArrayDecl" {
                    debug!("Найдено объявление массива в Decl");

                    if let Ok(mut array_node) = AstConverter::parse_ast_node(type_obj) {
                        if let Some(ref name) = decl_name {
                            array_node.attributes.insert(
                                "name".to_string(),
                                Value::String(name.clone()),
                            );
                            debug!("Добавлено имя массива: {}", name);
                        }

                        // Добавляем инициализатор как ребенка массива, если он есть
                        if let Some(init) = init_node {
                            debug!("Добавляем инициализатор к узлу массива");
                            array_node.children.push(init);
                        }

                        node.children.push(array_node);
                    }
                } else {
                    // Обычное объявление - добавляем тип и инициализатор как детей Decl
                    if let Ok(type_decl_node) = AstConverter::parse_ast_node(type_obj) {
                        node.children.push(type_decl_node);
                    }
                    // Для обычных объявлений добавляем инициализатор отдельно
                    if let Some(init) = init_node {
                        node.children.push(init);
                    }
                }
            }
        }
    } else if let Some(init) = init_node {
        // Если нет типа, но есть инициализатор
        node.children.push(init);
    }

    Ok(())
}

/// Обработка узла ArrayDecl
pub fn handle_array_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления массива");
    debug!("Содержимое ArrayDecl: {:#?}", obj);

    // Сохраняем имя массива
    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(name.to_string()));
        debug!("Имя массива: {}", name);
    }

    // Обрабатываем тип элементов
    if let Some(type_obj) = obj.get("type") {
        debug!("Тип элементов массива: {:#?}", type_obj);
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    // Обрабатываем размер массива (если указан)
    if let Some(dim) = obj.get("dim") {
        debug!("Размер массива: {:#?}", dim);
        if let Ok(dim_node) = AstConverter::parse_ast_node(dim) {
            node.children.push(dim_node);
        }
    }

    Ok(())
}

/// Обработка узла PtrDecl (объявление указателя)
pub fn handle_ptr_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка объявления указателя");
    debug!("Содержимое PtrDecl: {:#?}", obj);

    // Сохраняем квалификаторы (const и т.д.)
    if let Some(quals) = obj.get("quals") {
        node.attributes.insert("quals".to_string(), quals.clone());
    }

    // Обрабатываем тип, на который указывает указатель
    if let Some(type_obj) = obj.get("type") {
        debug!("Тип, на который указывает указатель: {:#?}", type_obj);
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }
    }

    Ok(())
}

/// Обработка узла TypeDecl
pub fn handle_type_decl(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
    debug!("Обработка TypeDecl узла");
    debug!("Содержимое TypeDecl: {:#?}", obj);

    // Сохраняем имя объявления (имя переменной/параметра)
    if let Some(declname) = obj.get("declname").and_then(|v| v.as_str()) {
        node.attributes
            .insert("name".to_string(), Value::String(declname.to_string()));
        debug!("Имя в TypeDecl: {}", declname);
    }

    // ВАЖНО: Сохраняем информацию о типе из вложенного узла
    if let Some(type_obj) = obj.get("type") {
        debug!("Тип в TypeDecl: {:#?}", type_obj);

        // Парсим вложенный тип как отдельный узел и добавляем как ребенка
        if let Ok(type_node) = AstConverter::parse_ast_node(type_obj) {
            node.children.push(type_node);
        }

        // Также сохраняем JSON представление типа как атрибут
        // Это критически важно для определения структурных переменных!
        node.attributes.insert("type".to_string(), type_obj.clone());

        // Проверяем, является ли тип структурой
        if let Some(type_obj_map) = type_obj.as_object() {
            if let Some(node_type) = type_obj_map.get("__node__").and_then(|n| n.as_str()) {
                if node_type == "Struct" {
                    debug!("TypeDecl указывает на структуру!");
                    if let Some(struct_name) = type_obj_map.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert(
                            "struct_type".to_string(),
                            Value::String(struct_name.to_string()),
                        );
                        debug!("Имя структуры: {}", struct_name);
                    }
                } else if node_type == "Union" {
                    if let Some(union_name) = type_obj_map.get("name").and_then(|n| n.as_str()) {
                        node.attributes.insert(
                            "union_type".to_string(),
                            Value::String(union_name.to_string()),
                        );
                        debug!("Имя объединения: {}", union_name);
                    }
                }
            }
        }
    }

    // Сохраняем квалификаторы типа (const, volatile и т.д.)
    if let Some(quals) = obj.get("quals") {
        node.attributes.insert("quals".to_string(), quals.clone());
    }

    Ok(())
}
