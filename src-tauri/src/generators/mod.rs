pub mod python_gen;

use anyhow::Result;
use crate::ast::ASTNode;

/// Общий трейт для всех генераторов
pub trait Generator {
    type Output;
    
    /// Генерирует код из AST
    fn generate(ast: &ASTNode) -> Result<Self::Output>;
    
    /// Название языка
    fn language_name() -> &'static str;
}