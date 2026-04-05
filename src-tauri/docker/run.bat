@echo off
setlocal enabledelayedexpansion

echo Запуск C парсера...
docker run -d --name c-parser-service -p 5000:5000 --restart unless-stopped c-parser:latest

echo Запуск f2c сервиса...
docker run -d --name f2c-service -p 5001:5001 --restart unless-stopped f2c-service:latest

echo Запуск Python сервиса...
docker run -d --name python-service -p 5002:5002 --restart unless-stopped python-service:latest

echo Запуск C service...
docker run -d --name c-service -p 5003:5003 --restart unless-stopped c-service:latest

echo Парсеры запущены:
docker ps | findstr /R "c-parser f2c-service python-service c-service"

echo.
echo Для остановки:
echo   docker stop c-parser-service f2c-service python-service c-service
echo Для удаления:
echo   docker rm c-parser-service f2c-service python-service c-service
