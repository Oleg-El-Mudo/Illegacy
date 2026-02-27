#!/bin/bash

# Запуск C парсера в фоне
echo "Запуск C парсера..."
docker run -d \
  --name c-parser-service \
  -p 5000:5000 \
  --restart unless-stopped \
  c-parser:latest

echo "Парсеры запущены:"
docker ps | grep -E "c-parser"

echo "
Для остановки: docker stop c-parser-service
Для удаления: docker rm c-parser-service
"