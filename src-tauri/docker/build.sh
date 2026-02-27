#!/bin/bash

set -e

echo "Сборка Docker образов для парсеров..."

# Сборка C парсера
echo "Сборка c-parser..."
cd c-parser
docker build -t c-parser:latest .
cd ..

echo "Готово! Образы собраны:"
docker images | grep -E "c-parser"