@echo off
setlocal enabledelayedexpansion

echo Сборка Docker образов для парсеров...

echo Сборка c-parser...
cd c-parser
docker build -t c-parser:latest .
if errorlevel 1 (
    echo Ошибка сборки c-parser
    cd ..
    exit /b 1
)
cd ..

echo Сборка f2c-service...
cd f2c-service
docker build -t f2c-service:latest .
if errorlevel 1 (
    echo Ошибка сборки f2c-service
    cd ..
    exit /b 1
)
cd ..

echo Сборка python-service...
cd python-service
docker build -t python-service:latest .
if errorlevel 1 (
    echo Ошибка сборки python-service
    cd ..
    exit /b 1
)
cd ..

echo Сборка c-service...
cd c-service
docker build -t c-service:latest .
if errorlevel 1 (
    echo Ошибка сборки c-service
    cd ..
    exit /b 1
)
cd ..

echo Готово! Образы собраны:
docker images | findstr /R "c-parser f2c-service python-service c-service"
