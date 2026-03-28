#!/bin/bash

set -e

echo "Сборка Docker образов для парсеров..."

# Сборка C парсера
echo "Сборка c-parser..."
cd c-parser
docker build -t c-parser:latest .
cd ..

# Сборка f2c сервиса
echo "Сборка f2c-service..."
cd f2c-service
docker build -t f2c-service:latest .
cd ..

# Сборка Python сервиса
echo "Сборка python-service..."
cd python-service
docker build -t python-service:latest .
cd ..

echo "Готово! Образы собраны:"
docker images | grep -E "c-parser|f2c-service|python-service"