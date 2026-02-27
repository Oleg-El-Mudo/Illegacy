#!/usr/bin/env python3
"""
Микросервис для парсинга C кода с использованием pycparser.
Возвращает AST в формате JSON.
"""
from flask import Flask, request, jsonify
from pycparser import c_parser, c_ast
import json
import sys
import traceback

app = Flask(__name__)

class ASTEncoder(json.JSONEncoder):
    """Кастомный JSON энкодер для pycparser AST"""
    def default(self, obj):
        if isinstance(obj, c_ast.Node):
            result = {
                '__node__': obj.__class__.__name__,
                'coord': str(obj.coord) if obj.coord else None
            }
            # Добавляем все атрибуты узла
            for attr in obj.attr_names:
                result[attr] = getattr(obj, attr)
            # Добавляем детей
            for child_name, child in obj.children():
                result[child_name] = child
            return result
        elif isinstance(obj, list):
            return [self.default(item) for item in obj]
        elif isinstance(obj, dict):
            return {key: self.default(value) for key, value in obj.items()}
        return super().default(obj)

@app.route('/health', methods=['GET'])
def health():
    """Проверка здоровья сервиса"""
    return jsonify({"status": "ok", "service": "c-parser"})

@app.route('/parse', methods=['POST'])
def parse():
    """
    Парсит C код и возвращает AST.
    Ожидает JSON: {"code": "int main() { return 0; }"}
    """
    try:
        data = request.get_json()
        if not data or 'code' not in data:
            return jsonify({"error": "No code provided"}), 400
        
        code = data['code']
        
        # Создаем парсер
        parser = c_parser.CParser()
        
        # Парсим код
        ast = parser.parse(code)
        
        # Конвертируем в JSON
        ast_json = json.dumps(ast, cls=ASTEncoder, indent=2)
        
        return jsonify({
            "success": True,
            "ast": json.loads(ast_json)
        })
        
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 400

@app.route('/parse_file', methods=['POST'])
def parse_file():
    """
    Парсит C файл и возвращает AST.
    Ожидает multipart/form-data с файлом.
    """
    try:
        if 'file' not in request.files:
            return jsonify({"error": "No file provided"}), 400
        
        file = request.files['file']
        code = file.read().decode('utf-8')
        
        # Создаем парсер
        parser = c_parser.CParser()
        
        # Парсим код
        ast = parser.parse(code, filename=file.filename)
        
        # Конвертируем в JSON
        ast_json = json.dumps(ast, cls=ASTEncoder, indent=2)
        
        return jsonify({
            "success": True,
            "ast": json.loads(ast_json)
        })
        
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 400

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000, debug=False)