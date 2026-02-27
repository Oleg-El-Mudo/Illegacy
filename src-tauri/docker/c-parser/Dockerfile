FROM python:3.11-slim

WORKDIR /app

# Копируем зависимости
COPY parser_service/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Копируем код сервиса
COPY parser_service/ .

# Создаем пользователя для безопасности
RUN useradd -m -u 1000 parser && chown -R parser:parser /app
USER parser

# Запускаем сервис
EXPOSE 5000
CMD ["python", "app.py"]