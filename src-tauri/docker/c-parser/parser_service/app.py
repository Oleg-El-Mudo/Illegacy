#!/usr/bin/env python3
"""
Микросервис для парсинга C кода с использованием pycparser.
Поддерживает стандартные библиотеки C.
"""
from flask import Flask, request, jsonify
from pycparser import c_parser, c_ast
import json
import sys
import traceback
import tempfile
import os
import subprocess

app = Flask(__name__)

# Фейковые определения для стандартных библиотек C
FAKE_LIBC_DEFINITIONS = """
// Базовые типы
typedef unsigned int size_t;
typedef int FILE;
typedef long time_t;
typedef long clock_t;

// Константы
#define NULL ((void*)0)
#define EOF (-1)
#define M_PI 3.14159265358979323846
#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
#define RAND_MAX 32767
#define CLOCKS_PER_SEC 1000000

// Функции ввода-вывода (stdio.h)
int printf(const char *format, ...);
int scanf(const char *format, ...);
int sprintf(char *str, const char *format, ...);
int snprintf(char *str, size_t size, const char *format, ...);
int puts(const char *s);
int putchar(int c);
int getchar(void);
char *gets(char *s);
FILE *fopen(const char *filename, const char *mode);
int fclose(FILE *stream);
int fprintf(FILE *stream, const char *format, ...);
int fscanf(FILE *stream, const char *format, ...);
char *fgets(char *s, int size, FILE *stream);
int fputs(const char *s, FILE *stream);
int fgetc(FILE *stream);
int fputc(int c, FILE *stream);
int feof(FILE *stream);
int ferror(FILE *stream);
void rewind(FILE *stream);
int fseek(FILE *stream, long offset, int whence);
long ftell(FILE *stream);
void clearerr(FILE *stream);
int remove(const char *filename);
int rename(const char *oldname, const char *newname);
FILE *tmpfile(void);
char *tmpnam(char *s);

// Строковые функции (string.h)
size_t strlen(const char *s);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
char *strcat(char *dest, const char *src);
char *strncat(char *dest, const char *src, size_t n);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
char *strstr(const char *haystack, const char *needle);
char *strdup(const char *s);
char *strtok(char *str, const char *delim);
void *memcpy(void *dest, const void *src, size_t n);
void *memmove(void *dest, const void *src, size_t n);
void *memset(void *s, int c, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);

// Математические функции (math.h)
double sin(double x);
double cos(double x);
double tan(double x);
double asin(double x);
double acos(double x);
double atan(double x);
double atan2(double y, double x);
double sinh(double x);
double cosh(double x);
double tanh(double x);
double exp(double x);
double log(double x);
double log10(double x);
double sqrt(double x);
double pow(double x, double y);
double fabs(double x);
double ceil(double x);
double floor(double x);
double fmod(double x, double y);

// Функции стандартной библиотеки (stdlib.h)
int atoi(const char *nptr);
long atol(const char *nptr);
double atof(const char *nptr);
int abs(int j);
long labs(long j);
int rand(void);
void srand(unsigned int seed);
void *malloc(size_t size);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void exit(int status);
int system(const char *command);
char *getenv(const char *name);
int setenv(const char *name, const char *value, int overwrite);
void abort(void);
void qsort(void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
void *bsearch(const void *key, const void *base, size_t nmemb, size_t size, int (*compar)(const void *, const void *));
int atexit(void (*func)(void));
char *getenv(const char *name);

// Функции времени (time.h)
time_t time(time_t *t);
double difftime(time_t time1, time_t time0);
clock_t clock(void);
char *ctime(const time_t *timep);
char *asctime(const struct tm *tm);
struct tm *gmtime(const time_t *timep);
struct tm *localtime(const time_t *timep);
time_t mktime(struct tm *tm);
int strftime(char *s, size_t max, const char *format, const struct tm *tm);
void tzset(void);

// Функции для работы с символами (ctype.h)
int isalnum(int c);
int isalpha(int c);
int isdigit(int c);
int islower(int c);
int isupper(int c);
int isspace(int c);
int isxdigit(int c);
int iscntrl(int c);
int isgraph(int c);
int isprint(int c);
int ispunct(int c);
int tolower(int c);
int toupper(int c);

// Функции для работы с сигналами (signal.h)
typedef void (*sighandler_t)(int);
sighandler_t signal(int signum, sighandler_t handler);
int raise(int sig);

// Функции для работы с ошибками (errno.h)
extern int errno;

// Функции для работы с утверждениями (assert.h)
void assert(int expression);

// Функции для работы с множествами (setjmp.h)
typedef int jmp_buf[64];
int setjmp(jmp_buf env);
void longjmp(jmp_buf env, int val);

// Функции для работы с переменным числом аргументов (stdarg.h)
typedef void *va_list;
#define va_start(ap, param) ((ap) = (void *)(&(param) + 1))
#define va_arg(ap, type) (*(type *)((ap) = (void *)((char *)(ap) + sizeof(type)) - sizeof(type)))
#define va_end(ap)

// Функции для работы с локалями (locale.h)
struct lconv;
char *setlocale(int category, const char *locale);
struct lconv *localeconv(void);
"""

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
                value = getattr(obj, attr)
                # Преобразуем значение в JSON-совместимый формат
                if isinstance(value, c_ast.Node):
                    result[attr] = self.default(value)
                elif isinstance(value, list):
                    result[attr] = [self.default(item) if isinstance(item, c_ast.Node) else item for item in value]
                else:
                    result[attr] = value
            
            return result
        elif isinstance(obj, list):
            return [self.default(item) for item in obj]
        elif isinstance(obj, dict):
            return {key: self.default(value) for key, value in obj.items()}
        return super().default(obj)

def preprocess_with_gcc(code, add_fake_libc=True):
    """
    Выполняет препроцессинг C кода с помощью GCC.
    Если add_fake_libc=True, добавляет фейковые определения стандартных функций.
    """
    with tempfile.NamedTemporaryFile(mode='w', suffix='.c', delete=False) as f:
        # Добавляем фейковые определения если нужно
        if add_fake_libc:
            f.write(FAKE_LIBC_DEFINITIONS)
        
        f.write(code)
        f.write("\n")
        f.flush()
        
        try:
            # Выполняем препроцессинг с gcc
            result = subprocess.run(
                ['gcc', '-E', '-P', '-nostdinc', '-undef', f.name],
                capture_output=True,
                text=True,
                timeout=5
            )
            
            if result.returncode == 0:
                preprocessed = result.stdout
            else:
                print(f"GCC preprocessing failed: {result.stderr}", file=sys.stderr)
                preprocessed = code
                
        except FileNotFoundError:
            print("GCC not found, using raw code", file=sys.stderr)
            preprocessed = code
        except subprocess.TimeoutExpired:
            print("GCC preprocessing timed out", file=sys.stderr)
            preprocessed = code
        except Exception as e:
            print(f"Preprocessing error: {e}", file=sys.stderr)
            preprocessed = code
    
    # Удаляем временный файл
    try:
        os.unlink(f.name)
    except:
        pass
    
    return preprocessed

@app.route('/health', methods=['GET'])
def health():
    """Проверка здоровья сервиса"""
    try:
        gcc_available = subprocess.run(
            ['gcc', '--version'], 
            capture_output=True, 
            timeout=1
        ).returncode == 0
    except:
        gcc_available = False
    
    return jsonify({
        "status": "ok",
        "service": "c-parser",
        "gcc_available": gcc_available,
        "pycparser_version": c_parser.__version__ if hasattr(c_parser, '__version__') else "unknown"
    })

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
        
        # Параметры обработки
        preprocess = data.get('preprocess', True)
        add_fake_libc = data.get('add_fake_libc', True)
        
        # Выполняем препроцессинг если нужно
        if preprocess:
            original_code = code
            code = preprocess_with_gcc(code, add_fake_libc)
            print(f"Original code length: {len(original_code)}", file=sys.stderr)
            print(f"Preprocessed code length: {len(code)}", file=sys.stderr)
        
        # Создаем парсер
        parser = c_parser.CParser()
        
        # Парсим код
        ast = parser.parse(code, filename='<stdin>')
        
        # Конвертируем в JSON
        ast_dict = json.loads(json.dumps(ast, cls=ASTEncoder))
        
        return jsonify({
            "success": True,
            "ast": ast_dict,
            "preprocessed": preprocess,
            "fake_libc_added": add_fake_libc if preprocess else False
        })
        
    except c_parser.ParseError as e:
        return jsonify({
            "success": False,
            "error": f"Parse error: {str(e)}",
            "traceback": traceback.format_exc()
        }), 400
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
        
        # Получаем параметры из формы
        preprocess = request.form.get('preprocess', 'true').lower() == 'true'
        add_fake_libc = request.form.get('add_fake_libc', 'true').lower() == 'true'
        
        # Выполняем препроцессинг если нужно
        if preprocess:
            code = preprocess_with_gcc(code, add_fake_libc)
        
        # Создаем парсер
        parser = c_parser.CParser()
        
        # Парсим код
        ast = parser.parse(code, filename=file.filename)
        
        # Конвертируем в JSON
        ast_dict = json.loads(json.dumps(ast, cls=ASTEncoder))
        
        return jsonify({
            "success": True,
            "ast": ast_dict,
            "preprocessed": preprocess,
            "fake_libc_added": add_fake_libc if preprocess else False,
            "filename": file.filename
        })
        
    except c_parser.ParseError as e:
        return jsonify({
            "success": False,
            "error": f"Parse error: {str(e)}",
            "traceback": traceback.format_exc()
        }), 400
    except Exception as e:
        return jsonify({
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }), 400

@app.route('/info', methods=['GET'])
def info():
    """Информация о поддерживаемых функциях"""
    return jsonify({
        "supported_libraries": [
            "stdio.h",
            "stdlib.h", 
            "string.h",
            "math.h",
            "time.h",
            "ctype.h",
            "assert.h",
            "signal.h",
            "setjmp.h",
            "locale.h",
            "errno.h"
        ],
        "gcc_preprocessing": True,
        "fake_libc_definitions": True
    })

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000, debug=False)