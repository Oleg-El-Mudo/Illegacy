@echo off
setlocal enabledelayedexpansion

echo === Перезагрузка всех Docker сервисов ===
echo.

echo === Шаг 1: Остановка и удаление контейнеров ===
call :cleanup_container "c-parser-service"
call :cleanup_container "f2c-service"
call :cleanup_container "python-service"
call :cleanup_container "c-service"
echo.

echo === Шаг 2: Удаление старых образов ===
call :remove_image "c-parser:latest"
call :remove_image "f2c-service:latest"
call :remove_image "python-service:latest"
call :remove_image "c-service:latest"
echo.

echo === Шаг 3: Сборка новых образов ===
call build.bat
if errorlevel 1 (
    echo Ошибка сборки образов
    exit /b 1
)
echo.

echo === Шаг 4: Запуск сервисов ===
call run.bat
echo.

echo === Перезагрузка завершена! ===
echo.
echo Проверка статуса сервисов:
docker ps --filter "name=c-parser-service" --filter "name=f2c-service" --filter "name=python-service" --filter "name=c-service" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

exit /b 0

:cleanup_container
set "container_name=%~1"
echo Остановка контейнера: %container_name%

docker ps -q -f name=^/%container_name%$ | findstr . >nul 2>&1
if not errorlevel 1 (
    docker stop "%container_name%" >nul 2>&1
    echo [OK] Контейнер остановлен
) else (
    echo Контейнер не запущен
)

docker ps -aq -f name=^/%container_name%$ | findstr . >nul 2>&1
if not errorlevel 1 (
    docker rm "%container_name%" >nul 2>&1
    echo [OK] Контейнер удалён
) else (
    echo Контейнер не найден
)
exit /b 0

:remove_image
set "image_name=%~1"
echo Удаление образа: %image_name%

docker images -q "%image_name%" | findstr . >nul 2>&1
if not errorlevel 1 (
    docker rmi "%image_name%" >nul 2>&1
    echo [OK] Образ удалён
) else (
    echo Образ не найден
)
exit /b 0
