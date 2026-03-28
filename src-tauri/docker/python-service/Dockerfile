FROM python:3.11-slim

WORKDIR /app

# Копируем зависимости
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Копируем код сервиса
COPY . .

# Создаем пользователя для безопасности
RUN useradd -m -u 1000 parser && chown -R parser:parser /app
USER parser

# Запускаем сервис
EXPOSE 5002
CMD ["python", "app.py"]
