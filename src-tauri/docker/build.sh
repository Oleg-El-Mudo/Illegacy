#!/bin/bash

set -e

echo "Сборка Docker образов для парсеров..."

# Сборка C парсера
echo "Сборка c-parser и f2c-service..."
cd c-parser
docker build -t c-pars
echo "Готово! Образы собраны:"
docker images | grep -E "c-parser"
docker images | grep -E "c-parser|f2c-service"