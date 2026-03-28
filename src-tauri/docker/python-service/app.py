#!/usr/bin/env python3

from flask import Flask, request, jsonify
import subprocess
import tempfile
import os
import signal

app = Flask(__name__)

# Глобальная переменная для хранения текущего процесса
current_process = None

def execute_python_code(code):
    """
    Выполняет Python код и возвращает результат.

    Args:
        code: Python код для выполнения

    Returns:
        tuple: (stdout, stderr, return_code)
    """
    global current_process
    
    try:
        # Создаем временный файл для кода
        temp_dir = tempfile.mkdtemp()
        code_file = os.path.join(temp_dir, 'script.py')
        
        with open(code_file, 'w', newline='\n') as f:
            f.write(code)
        
        # Запускаем Python скрипт
        current_process = subprocess.Popen(
            ['python', code_file],
            cwd=temp_dir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            preexec_fn=os.setsid if os.name != 'nt' else None
        )
        
        stdout, stderr = current_process.communicate(timeout=30)
        return_code = current_process.returncode
        
        # Очищаем процесс
        current_process = None
        
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
        
        return stdout, stderr, return_code
        
    except subprocess.TimeoutExpired:
        if current_process:
            try:
                os.killpg(os.getpgid(current_process.pid), signal.SIGTERM)
            except:
                current_process.terminate()
        return "", "Execution timed out (30 seconds)", -1
    except Exception as e:
        return "", f"Execution error: {str(e)}", -1
    finally:
        current_process = None


@app.route('/health', methods=['GET'])
def health():
    """Проверка здоровья сервиса"""
    try:
        python_result = subprocess.run(
            ['python', '--version'],
            capture_output=True,
            timeout=2
        )
        python_available = python_result.returncode == 0
    except:
        python_available = False

    return jsonify({
        "status": "ok",
        "service": "python",
        "python_available": python_available
    })


@app.route('/execute', methods=['POST'])
def execute():
    """
    Выполняет Python код.

    Ожидает JSON: {"code": "print('Hello')"}

    Возвращает JSON: {
        "success": true,
        "stdout": "...",
        "stderr": "...",
        "return_code": 0
    }
    """
    try:
        data = request.get_json()
        if not data or 'code' not in data:
            return jsonify({"error": "No code provided"}), 400

        code = data['code']

        # Выполняем код
        stdout, stderr, return_code = execute_python_code(code)

        success = return_code == 0

        return jsonify({
            "success": success,
            "stdout": stdout,
            "stderr": stderr,
            "return_code": return_code
        })

    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e)
        }), 500


@app.route('/stop', methods=['POST'])
def stop():
    """
    Останавливает текущий выполняемый процесс.

    Возвращает JSON: {
        "success": true,
        "message": "Process stopped"
    }
    """
    global current_process
    
    if current_process:
        try:
            os.killpg(os.getpgid(current_process.pid), signal.SIGTERM)
            current_process = None
            return jsonify({
                "success": True,
                "message": "Process stopped"
            })
        except Exception as e:
            return jsonify({
                "success": False,
                "error": str(e)
            }), 500
    else:
        return jsonify({
            "success": True,
            "message": "No running process"
        })


@app.route('/info', methods=['GET'])
def info():
    """Информация о сервисе"""
    try:
        version_result = subprocess.run(
            ['python', '--version'],
            capture_output=True,
            text=True,
            timeout=2
        )
        version = version_result.stderr.strip() if version_result.stderr else version_result.stdout.strip()
    except:
        version = "Unknown"

    return jsonify({
        "description": "Python code executor",
        "version": version,
        "timeout": "30 seconds"
    })


if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5002, debug=False)
