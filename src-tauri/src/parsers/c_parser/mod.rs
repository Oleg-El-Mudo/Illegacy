//! C Parser module
//!
//! Модуль для парсинга C кода с использованием pycparser в Docker контейнере.
//!
//! Структура модуля:
//! - `docker` - работа с Docker контейнером
//! - `http_client` - HTTP клиент для взаимодействия с парсером
//! - `ast_converter` - конвертация JSON от парсера в ASTNode
//! - `node_handlers` - обработчики различных типов узлов AST

pub mod ast_converter;
pub mod docker;
pub mod http_client;
pub mod node_handlers;

use anyhow::{anyhow, Result};
use log::info;

use self::ast_converter::AstConverter;
use self::docker::DockerService;
use self::http_client::ParserHttpClient;

use crate::ast::ASTNode;
use crate::parsers::Parser as BaseParser;

/// Клиент для взаимодействия с Docker контейнером парсера C
pub struct CParser {
    docker_service: DockerService,
    http_client: ParserHttpClient,
}

impl CParser {
    /// Создает новый экземпляр парсера
    pub fn new() -> Self {
        Self {
            docker_service: DockerService::new(),
            http_client: ParserHttpClient::default(),
        }
    }

    /// Асинхронный метод парсинга
    pub async fn parse_async(code: &str) -> Result<ASTNode> {
        let parser = Self::new();

        if !parser.docker_service.is_available() {
            return Err(anyhow!(
                "Docker не доступен. Запустите Docker и перезапустите приложение."
            ));
        }

        // Пробуем сначала HTTP API (если контейнер уже запущен)
        match parser
            .http_client
            .parse("http://localhost:5000", code)
            .await
        {
            Ok(response) => {
                info!("Использован HTTP API парсера");
                AstConverter::convert_json_to_ast(&response)
            }
            Err(http_err) => {
                // Если HTTP не работает, запускаем разовый контейнер
                info!(
                    "HTTP API не доступен ({}), запускаем разовый контейнер",
                    http_err
                );

                match parser
                    .docker_service
                    .run_container_clean(code, "c-parser:latest")
                    .await
                {
                    Ok(response) => {
                        info!("Docker контейнер успешно выполнен");
                        AstConverter::convert_json_to_ast(&response)
                    }
                    Err(docker_err) => {
                        Err(docker_err)
                    }
                }
            }
        }
    }
}

impl Default for CParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseParser for CParser {
    fn parse(_code: &str) -> Result<ASTNode> {
        Err(anyhow!(
            "Используйте parse_async в асинхронном контексте"
        ))
    }

    fn is_available() -> bool {
        DockerService::new().is_available()
    }
}
