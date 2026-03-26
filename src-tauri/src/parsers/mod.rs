pub mod ast_converter;
pub mod c_parser;
pub mod docker;
pub mod http_client;
pub mod node_handlers;

use anyhow::Result;
use crate::ast::ASTNode;

/// Общий трейт для всех парсеров
pub trait Parser {
    /// Парсит код и возвращает AST
    fn parse(code: &str) -> Result<ASTNode>;

    /// Проверяет доступность парсера
    fn is_available() -> bool;
}