//! Модуль содержит парсеры для различных языков программирования.
//! Каждый парсер расположен в отдельной подпапке.
//!
//! Структура:
//! - `c_parser/` - парсер C кода (pycparser + Docker)
//! - `mod.rs` - общий трейт Parser для всех парсеров

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
