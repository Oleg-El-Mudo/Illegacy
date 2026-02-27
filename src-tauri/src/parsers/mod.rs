pub mod c_parser;

use anyhow::Result;
use crate::ast::ASTNode;

/// Общий трейт для всех парсеров
pub trait Parser {
    /// Парсит код и возвращает AST
    fn parse(code: &str) -> Result<ASTNode>;
    
    /// Проверяет доступность парсера
    fn is_available() -> bool;
}