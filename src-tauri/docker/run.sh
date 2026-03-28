#!/bin/bash

# Запуск C парсера в фоне
echo "Запуск C парсера..."
docker run -d \
  --name c-parser-service \
  -p 5000:5000 \
  --restart unless-stopped \
  c-parser:latest

# Запуск f2c сервиса (Fortran -> C) в фоне
echo "Запуск f2c сервиса..."
docker run -d \
  --name f2c-service \
  -p 5001:5001 \
  --restart unless-stopped \
  f2c-service:latest

# Запуск Python сервиса в фоне
echo "Запуск Python сервиса..."
docker run -d \
  --name python-service \
  -p 5002:5002 \
  --restart unless-stopped \
  python-service:latest

echo "Парсеры запущены:"
docker ps | grep -E "c-parser|f2c-service|python-service"

echo "
Для остановки:
  docker stop c-parser-service f2c-service python-service
Для удаления:
  docker rm c-parser-service f2c-service python-service
"