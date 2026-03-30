#!/bin/bash

set -e

echo "=== Перезагрузка всех Docker сервисов ==="
echo ""

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функция для остановки и удаления контейнера
cleanup_container() {
    local container_name=$1
    echo -e "${YELLOW}Остановка контейнера: ${container_name}${NC}"
    
    # Останавливаем контейнер если он запущен
    if docker ps -q -f name=^/${container_name}$ | grep -q .; then
        docker stop "${container_name}" 2>/dev/null || true
        echo -e "${GREEN}✓ Контейнер остановлен${NC}"
    else
        echo "Контейнер не запущен"
    fi
    
    # Удаляем контейнер если он существует
    if docker ps -aq -f name=^/${container_name}$ | grep -q .; then
        docker rm "${container_name}" 2>/dev/null || true
        echo -e "${GREEN}✓ Контейнер удалён${NC}"
    else
        echo "Контейнер не найден"
    fi
}

# Функция для удаления образа
remove_image() {
    local image_name=$1
    echo -e "${YELLOW}Удаление образа: ${image_name}${NC}"
    
    # Удаляем образ если он существует
    if docker images -q "${image_name}" | grep -q .; then
        docker rmi "${image_name}" 2>/dev/null || true
        echo -e "${GREEN}✓ Образ удалён${NC}"
    else
        echo "Образ не найден"
    fi
}

echo "=== Шаг 1: Остановка и удаление контейнеров ==="
cleanup_container "c-parser-service"
cleanup_container "f2c-service"
cleanup_container "python-service"
echo ""

echo "=== Шаг 2: Удаление старых образов ==="
remove_image "c-parser:latest"
remove_image "f2c-service:latest"
remove_image "python-service:latest"
echo ""

echo "=== Шаг 3: Сборка новых образов ==="
./build.sh
echo ""

echo "=== Шаг 4: Запуск сервисов ==="
./run.sh
echo ""

echo -e "${GREEN}=== Перезагрузка завершена! ===${NC}"
echo ""
echo "Проверка статуса сервисов:"
docker ps --filter "name=c-parser-service" --filter "name=f2c-service" --filter "name=python-service" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
