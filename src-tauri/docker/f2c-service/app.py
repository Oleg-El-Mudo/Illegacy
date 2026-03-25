#!/usr/bin/env python3

from flask import Flask, request, jsonify
import subprocess
import tempfile
import os
import re

app = Flask(__name__)

def convert_fortran_to_c(fortran_code):
    """
    Конвертирует Fortran код в C с помощью утилиты f2c.

    Args:
        fortran_code: Исходный код на Fortran

    Returns:
        tuple: (C код, статус успеха, сообщение об ошибке)
    """
    # Создаем временную директорию
    temp_dir = tempfile.mkdtemp()

    try:
        # Записываем Fortran код во временный файл
        # f2c ожидает файлы с расширением .f или .for
        fortran_file = os.path.join(temp_dir, 'input.f')
        with open(fortran_file, 'w', newline='\n') as f:
            # Добавляем завершающую новую строку если её нет
            code = fortran_code.rstrip() + '\n'
            f.write(code)

        # Отладка: читаем записанный файл
        with open(fortran_file, 'r') as f:
            written_code = f.read()
        print(f"Written to input.f ({len(written_code)} chars):")
        print(written_code[:500])

        # Запускаем f2c с дополнительными флагами
        # -E - подавляем #include
        result = subprocess.run(
            ['f2c', 'input.f'],
            cwd=temp_dir,
            capture_output=True,
            text=True,
            timeout=30
        )

        print(f"f2c return code: {result.returncode}")
        if result.stdout:
            print(f"f2c stdout: {result.stdout}")
        if result.stderr:
            print(f"f2c stderr: {result.stderr}")

        if result.returncode != 0:
            return None, False, f"f2c error: {result.stderr}"

        # Читаем сгенерированный C файл
        c_file = os.path.join(temp_dir, 'input.c')
        if not os.path.exists(c_file):
            # Проверяем какие файлы были созданы
            files = os.listdir(temp_dir)
            return None, False, f"f2c did not generate output file. Created files: {files}"

        with open(c_file, 'r') as f:
            c_code = f.read()

        print(f"Generated C code ({len(c_code)} chars):")
        print(c_code[:500])

        return c_code, True, None

    except subprocess.TimeoutExpired:
        return None, False, "f2c conversion timed out"
    except FileNotFoundError:
        return None, False, "f2c not found in PATH"
    except Exception as e:
        return None, False, f"Conversion error: {str(e)}"
    finally:
        # Очищаем временные файлы
        try:
            for filename in os.listdir(temp_dir):
                filepath = os.path.join(temp_dir, filename)
                try:
                    if os.path.isfile(filepath):
                        os.unlink(filepath)
                except:
                    pass
            os.rmdir(temp_dir)
        except:
            pass


def clean_c_code(c_code):
    """
    Очищает C код от заголовочных файлов и других элементов,
    которые не нужны для парсинга.
    
    Args:
        c_code: Исходный C код от f2c
        
    Returns:
        Очищенный C код
    """
    lines = c_code.split('\n')
    cleaned_lines = []
    
    # Флаги для пропуска определенных секций
    skip_until_endif = 0
    
    for line in lines:
        stripped = line.strip()
        
        # Пропускаем директивы препроцессора #include
        if stripped.startswith('#include'):
            continue
        
        # Пропускаем директивы #define (f2c генерирует много своих)
        if stripped.startswith('#define') and not stripped.startswith('#define _'):
            # Но сохраняем важные определения
            if 'ABS' in stripped or 'd_abs' in stripped or 'i_abs' in stripped:
                continue
            if 'POW' in stripped or 'pow' in stripped:
                continue
            if 'RETURN' in stripped:
                continue
        
        # Пропускаем #ifdef, #ifndef секции для f2c
        if stripped.startswith('#ifdef') or stripped.startswith('#ifndef'):
            skip_until_endif += 1
            continue
        
        if stripped == '#endif' and skip_until_endif > 0:
            skip_until_endif -= 1
            continue
        
        if skip_until_endif > 0:
            continue
        
        # Пропускаем #pragma
        if stripped.startswith('#pragma'):
            continue
        
        # Пропускаем пустые директивы
        if stripped == '#':
            continue
        
        # Сохраняем строку
        cleaned_lines.append(line)
    
    return '\n'.join(cleaned_lines)


@app.route('/health', methods=['GET'])
def health():
    """Проверка здоровья сервиса"""
    try:
        f2c_result = subprocess.run(
            ['f2c', '-v'],
            capture_output=True,
            timeout=2
        )
        f2c_available = f2c_result.returncode == 0 or 'f2c' in f2c_result.stderr.lower()
    except:
        f2c_available = False
    
    return jsonify({
        "status": "ok",
        "service": "f2c",
        "f2c_available": f2c_available
    })


@app.route('/convert', methods=['POST'])
def convert():
    """
    Конвертирует Fortran код в C.
    
    Ожидает JSON: {"code": "PROGRAM TEST\n...", "clean": true}
    
    Возвращает JSON: {
        "success": true,
        "c_code": "...",
        "original_length": 100,
        "converted_length": 150
    }
    """
    try:
        data = request.get_json()
        if not data or 'code' not in data:
            return jsonify({"error": "No code provided"}), 400
        
        fortran_code = data['code']
        should_clean = data.get('clean', True)
        
        # Конвертируем Fortran → C
        c_code, success, error = convert_fortran_to_c(fortran_code)
        
        if not success:
            return jsonify({
                "success": False,
                "error": error
            }), 400
        
        # Очищаем код если нужно
        if should_clean:
            c_code = clean_c_code(c_code)
        
        return jsonify({
            "success": True,
            "c_code": c_code,
            "original_length": len(fortran_code),
            "converted_length": len(c_code),
            "cleaned": should_clean
        })
        
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@app.route('/convert_file', methods=['POST'])
def convert_file():
    """
    Конвертирует Fortran файл в C.
    
    Ожидает multipart/form-data с файлом.
    
    Возвращает JSON с C кодом.
    """
    try:
        if 'file' not in request.files:
            return jsonify({"error": "No file provided"}), 400
        
        file = request.files['file']
        fortran_code = file.read().decode('utf-8')
        
        should_clean = request.form.get('clean', 'true').lower() == 'true'
        
        # Конвертируем Fortran → C
        c_code, success, error = convert_fortran_to_c(fortran_code)
        
        if not success:
            return jsonify({
                "success": False,
                "error": error
            }), 400
        
        # Очищаем код если нужно
        if should_clean:
            c_code = clean_c_code(c_code)
        
        return jsonify({
            "success": True,
            "c_code": c_code,
            "filename": file.filename,
            "cleaned": should_clean
        })
        
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@app.route('/info', methods=['GET'])
def info():
    """Информация о поддерживаемых возможностях"""
    return jsonify({
        "description": "Fortran to C converter using f2c",
        "supported_fortran": ["Fortran 77", "Fortran 90 (частично)"],
        "output": "C code (K&R style)",
        "cleaning": "Автоматическая очистка от f2c заголовков"
    })


if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5001, debug=False)
