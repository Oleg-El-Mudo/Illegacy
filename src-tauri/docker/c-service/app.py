#!/usr/bin/env python3

from flask import Flask, request, jsonify
import subprocess
import tempfile
import os
import signal

app = Flask(__name__)

# Глобальная переменная для хранения текущего процесса
current_process = None

def execute_c_code(code):
    """
    Выполняет C код (компилирует и запускает) и возвращает результат.

    Args:
        code: C код для выполнения

    Returns:
        tuple: (stdout, stderr, return_code)
    """
    global current_process

    try:
        # Создаем временную директорию
        temp_dir = tempfile.mkdtemp()
        c_file = os.path.join(temp_dir, 'program.c')
        executable = os.path.join(temp_dir, 'program')

        # Записываем C код во временный файл
        with open(c_file, 'w', newline='\n') as f:
            f.write(code)

        # Шаг 1: Компилируем код с помощью gcc
        # -lm подключает математическую библиотеку (sqrt, pow, sin, cos, etc.)
        compile_result = subprocess.run(
            ['gcc', '-o', executable, c_file, '-std=c99', '-lm'],
            capture_output=True,
            text=True,
            timeout=30
        )

        if compile_result.returncode != 0:
            # Ошибка компиляции
            try:
                for filename in os.listdir(temp_dir):
                    filepath = os.path.join(temp_dir, filename)
                    if os.path.isfile(filepath):
                        os.unlink(filepath)
                os.rmdir(temp_dir)
            except:
                pass
            return "", f"Ошибка компиляции:\n{compile_result.stderr}", -1

        # Шаг 2: Запускаем скомпилированную программу
        current_process = subprocess.Popen(
            [executable],
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
        return "", "Выполнение превысило лимит времени (30 секунд)", -1
    except FileNotFoundError:
        return "", "gcc не найден в PATH", -1
    except Exception as e:
        return "", f"Ошибка выполнения: {str(e)}", -1
    finally:
        current_process = None


@app.route('/health', methods=['GET'])
def health():
    """Проверка здоровья сервиса"""
    try:
        gcc_result = subprocess.run(
            ['gcc', '--version'],
            capture_output=True,
            timeout=2
        )
        gcc_available = gcc_result.returncode == 0
    except:
        gcc_available = False

    return jsonify({
        "status": "ok",
        "service": "c-service",
        "gcc_available": gcc_available
    })


@app.route('/execute', methods=['POST'])
def execute():
    """
    Выполняет C код (компилирует и запускает).

    Ожидает JSON: {"code": "int main() { printf(\"Hello\"); return 0; }"}

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
        stdout, stderr, return_code = execute_c_code(code)

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
            ['gcc', '--version'],
            capture_output=True,
            text=True,
            timeout=2
        )
        version = version_result.stdout.strip().split('\n')[0] if version_result.stdout else "Unknown"
    except:
        version = "Unknown"

    return jsonify({
        "description": "C code executor using GCC",
        "version": version,
        "timeout": "30 seconds"
    })


if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5003, debug=False)
