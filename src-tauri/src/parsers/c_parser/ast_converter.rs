use anyhow::{anyhow, Result};
use log::debug;
use serde_json::Value;

use crate::ast::ASTNode;

use crate::parsers::c_parser::node_handlers::{
    handle_array_decl, handle_array_ref, handle_assignment, handle_binary_op, handle_case,
    handle_cast, handle_compound, handle_constant, handle_decl, handle_default, handle_dowhile,
    handle_enum, handle_enumerator, handle_expr_list, handle_for, handle_func_call, handle_func_def,
    handle_func_decl, handle_identifier_type, handle_id, handle_if, handle_init_list,
    handle_named_initializer, handle_param_decl, handle_param_list, handle_ptr_decl, handle_return,
    handle_struct, handle_struct_decl, handle_struct_ref, handle_switch, handle_ternary_op,
    handle_type_decl, handle_unary_op, handle_union, handle_union_decl, handle_while,
};

/// Конвертер JSON от парсера в ASTNode
pub struct AstConverter;

impl AstConverter {
    /// Преобразует JSON от парсера в наш ASTNode
    pub fn convert_json_to_ast(json_str: &str) -> Result<ASTNode> {
        debug!("Парсинг JSON ответа, длина: {} символов", json_str.len());

        let value: Value = serde_json::from_str(json_str)?;

        // Проверяем успешность
        if let Some(success) = value.get("success") {
            if !success.as_bool().unwrap_or(false) {
                let error = value
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                return Err(anyhow!("Ошибка парсера: {}", error));
            }
        }

        // Извлекаем AST
        let ast_value = value
            .get("ast")
            .ok_or_else(|| anyhow!("Нет поля ast в ответе: {:?}", value))?;

        Self::parse_ast_node(ast_value)
    }

    /// Рекурсивно парсит JSON в ASTNode
    pub fn parse_ast_node(value: &Value) -> Result<ASTNode> {
        if let Some(obj) = value.as_object() {
            // Проверяем, является ли это узлом
            if let Some(node_type) = obj.get("__node__").and_then(|v| v.as_str()) {
                let mut node = ASTNode::new(node_type);

                // Обрабатываем координаты
                if let Some(coord) = obj.get("coord").and_then(|c| c.as_str()) {
                    node.coord = Some(coord.to_string());
                }

                // Обработка различных типов узлов через отдельные функции
                match node_type {
                    "FuncDef" => handle_func_def(obj, &mut node)?,
                    "FuncDecl" => handle_func_decl(obj, &mut node)?,
                    "Decl" => handle_decl(obj, &mut node)?,
                    "IdentifierType" => handle_identifier_type(obj, &mut node)?,
                    "Constant" => handle_constant(obj, &mut node)?,
                    "ID" => handle_id(obj, &mut node)?,
                    "BinaryOp" => handle_binary_op(obj, &mut node)?,
                    "Assignment" => handle_assignment(obj, &mut node)?,
                    "UnaryOp" => handle_unary_op(obj, &mut node)?,
                    "Cast" => handle_cast(obj, &mut node)?,
                    "ParamList" => handle_param_list(obj, &mut node)?,
                    "FuncCall" => handle_func_call(obj, &mut node)?,
                    "ParamDecl" => handle_param_decl(obj, &mut node)?,
                    "ExprList" => handle_expr_list(obj, &mut node)?,
                    "If" => handle_if(obj, &mut node)?,
                    "Switch" => handle_switch(obj, &mut node)?,
                    "Case" => handle_case(obj, &mut node)?,
                    "Default" => handle_default(obj, &mut node)?,
                    "ArrayDecl" => handle_array_decl(obj, &mut node)?,
                    "PtrDecl" => handle_ptr_decl(obj, &mut node)?,
                    "InitList" => handle_init_list(obj, &mut node)?,
                    "NamedInitializer" => handle_named_initializer(obj, &mut node)?,
                    "ArrayRef" => handle_array_ref(obj, &mut node)?,
                    "TernaryOp" => handle_ternary_op(obj, &mut node)?,
                    "Return" => handle_return(obj, &mut node)?,
                    "Compound" => handle_compound(obj, &mut node)?,
                    "While" => handle_while(obj, &mut node)?,
                    "DoWhile" => handle_dowhile(obj, &mut node)?,
                    "For" => handle_for(obj, &mut node)?,
                    "Struct" => handle_struct(obj, &mut node)?,
                    "StructDecl" => handle_struct_decl(obj, &mut node)?,
                    "StructRef" => handle_struct_ref(obj, &mut node)?,
                    "Union" => handle_union(obj, &mut node)?,
                    "UnionDecl" => handle_union_decl(obj, &mut node)?,
                    "Enum" => handle_enum(obj, &mut node)?,
                    "Enumerator" => handle_enumerator(obj, &mut node)?,
                    "TypeDecl" => handle_type_decl(obj, &mut node)?,
                    _ => {
                        // Для остальных узлов просто копируем все атрибуты
                        debug!("Обработка узла типа: {}", node_type);
                    }
                }

                // Обрабатываем все остальные поля для детей
                Self::process_remaining_fields(obj, &mut node)?;

                Ok(node)
            } else {
                // Обычный объект (не узел)
                Ok(ASTNode::new("Object").with_attr("value", value.clone()))
            }
        } else if let Some(arr) = value.as_array() {
            // Массив узлов
            let mut children = Vec::new();
            for item in arr {
                if item.is_object() && item.get("__node__").is_some() {
                    children.push(Self::parse_ast_node(item)?);
                }
            }
            Ok(ASTNode::new("Array").with_attr("count", children.len()))
        } else {
            // Примитивное значение
            Ok(ASTNode::new("Value").with_attr("value", value.clone()))
        }
    }

    /// Обрабатывает все остальные поля JSON объекта для добавления дочерних узлов
    fn process_remaining_fields(obj: &serde_json::Map<String, Value>, node: &mut ASTNode) -> Result<()> {
        let skip_keys = [
            "__node__", "coord", "name", "type", "decl", "params", "init", "args",
            "cond", "iftrue", "iffalse", "stmt", "value", "stmts", "expr", "dim",
            "subscript", "to_type", "op", "quals", "declname", "struct_type",
            "union_type", "field_types", "_json", "param_names", "func_name",
        ];

        for (key, val) in obj {
            // Пропускаем уже обработанные ключи
            if skip_keys.contains(&key.as_str())
                || key.starts_with("exprs")
                || key.starts_with("block_items")
                || key.starts_with("params[")
                || key.starts_with("name[")
                || key.starts_with("stmts[")
            {
                continue;
            }

            // Если значение - объект с __node__, это дочерний узел
            if val.is_object() {
                if val.get("__node__").is_some() {
                    let new_node = Self::parse_ast_node(val)?;

                    // Проверяем, не добавлен ли уже такой узел
                    if !Self::node_already_exists(node, &new_node) {
                        node.children.push(new_node);
                    }
                }
            }
            // Если значение - массив, проверяем элементы
            else if val.is_array() {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        if item.is_object() && item.get("__node__").is_some() {
                            let new_node = Self::parse_ast_node(item)?;

                            if !Self::node_already_exists(node, &new_node) {
                                node.children.push(new_node);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Проверяет, существует ли уже узел с такими же характеристиками
    fn node_already_exists(parent: &ASTNode, new_node: &ASTNode) -> bool {
        for child in &parent.children {
            if child.node_type == new_node.node_type && child.coord == new_node.coord {
                debug!(
                    "Предотвращено дублирование узла типа {} с координатами {:?}",
                    child.node_type, child.coord
                );
                return true;
            }
        }
        false
    }

    /// Извлекает параметры из узла ParamList и добавляет их в узел функции
    pub fn extract_params_from_paramlist(
        paramlist_obj: &serde_json::Map<String, Value>,
        node: &mut ASTNode,
    ) -> Result<()> {
        let mut param_names = Vec::new();
        let mut param_nodes = Vec::new();

        // Собираем все параметры (они могут быть с ключами params[0], params[1] и т.д.)
        for (key, value) in paramlist_obj {
            if key.starts_with("params[") || key == "params" {
                debug!("Обработка параметра с ключом {}: {:#?}", key, value);

                if let Some(param_obj) = value.as_object() {
                    // Сохраняем узел параметра
                    if let Ok(param_node) = Self::parse_ast_node(value) {
                        // Извлекаем имя параметра перед добавлением в param_nodes
                        if let Some(param_name) = param_obj.get("name").and_then(|n| n.as_str()) {
                            param_names.push(Value::String(param_name.to_string()));
                            debug!("Найдено имя параметра: {}", param_name);
                        }
                        param_nodes.push(param_node);
                    }
                }
            }
        }

        // Сохраняем параметры как атрибуты
        if !param_names.is_empty() {
            node.attributes
                .insert("param_names".to_string(), Value::Array(param_names));
            debug!("Добавлен атрибут param_names");
        }

        if !param_nodes.is_empty() {
            let mut param_list_node = ASTNode::new("ParamList");
            param_list_node.children = param_nodes.clone();
            node.children.push(param_list_node);
            debug!("Добавлен ParamList с {} параметрами", param_nodes.len());
        }

        Ok(())
    }
}
