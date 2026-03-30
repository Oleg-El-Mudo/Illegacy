FROM python:3.11-slim

WORKDIR /app

# Копируем зависимости
COPY parser_service/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Копируем код сервиса
COPY parser_service/ .

# Создаем директорию для входных файлов и пользователя для безопасности
RUN mkdir -p /input && \
    useradd -m -u 1000 parser && \
    chown -R parser:parser /app && \
    chmod 755 /input

# Пользователь parser будет запускать приложение
USER parser

# Запускаем сервис
EXPOSE 5000
CMD ["python", "app.py"]