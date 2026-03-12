use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Общий узел AST для всех языков
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ASTNode {
    /// Тип узла (FunctionDecl, IfStmt, BinaryOp и т.д.)
    pub node_type: String,
    
    /// Дочерние узлы
    pub children: Vec<ASTNode>,
    
    /// Координаты в исходном коде
    pub coord: Option<String>,
    
    /// Дополнительные атрибуты (имена, типы, значения)
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl ASTNode {
    pub fn new(node_type: &str) -> Self {
        Self {
            node_type: node_type.to_string(),
            children: Vec::new(),
            coord: None,
            attributes: HashMap::new(),
        }
    }
    
    /// Добавить атрибут
    pub fn with_attr(mut self, key: &str, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.attributes.insert(key.to_string(), value);
        }
        self
    }
    
    /// Добавить ребенка
    pub fn with_child(mut self, child: ASTNode) -> Self {
        self.children.push(child);
        self
    }
}