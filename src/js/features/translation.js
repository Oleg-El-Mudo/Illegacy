import { elements } from '../core/dom.js';
import { updateSyntaxHighlighting } from '../editor/syntax-highlight.js';

const { invoke } = window.__TAURI__.core;

/**
 * Удаляет комментарии из C кода
 * @param {string} code - исходный C код
 * @returns {string} - код без комментариев
 */
function removeCComments(code) {
    if (!code) return code;
    
    let result = '';
    let i = 0;
    const len = code.length;
    
    while (i < len) {
        const char = code[i];
        const nextChar = code[i + 1];
        
        // Проверяем начало многострочного комментария /*
        if (char === '/' && nextChar === '*') {
            i += 2; // Пропускаем /*
            
            // Ищем закрывающий */
            while (i < len - 1) {
                if (code[i] === '*' && code[i + 1] === '/') {
                    i += 2; // Пропускаем */
                    break;
                }
                i++;
            }
        }
        // Проверяем начало однострочного комментария //
        else if (char === '/' && nextChar === '/') {
            i += 2;
            
            // Пропускаем все до конца строки
            while (i < len && code[i] !== '\n' && code[i] !== '\r') {
                i++;
            }
            
            // Добавляем символ новой строки, если он был
            if (i < len && (code[i] === '\n' || code[i] === '\r')) {
                result += code[i];
                i++;
                
                // Проверяем \r\n
                if (i < len && code[i-1] === '\r' && code[i] === '\n') {
                    result += code[i];
                    i++;
                }
            }
        }
        else {
            // Обычный символ - добавляем в результат
            result += char;
            i++;
        }
    }
    
    return result;
}

function removeIncludeLines(code) {
    if (!code) return code;
    
    return code.split('\n')
        .filter(line => !line.trim().startsWith('#include'))
        .join('\n');
}

/**
 * Удаляет комментарии из кода в зависимости от языка
 * @param {string} code - исходный код
 * @param {string} language - язык программирования
 * @returns {string} - код без комментариев
 */
function removeCommentsByLanguage(code, language) {
    if (!code) return code;
    
    switch (language) {
        case 'c':
        case 'cpp':
        case 'c++':
        case 'java':
        case 'javascript':
        case 'typescript':
            return removeCComments(code);
        case 'python':
            return code;
        default:
            return code;
    }
}

/**
 * Очищает код от комментариев с сохранением отладочной информации
 * @param {string} code - исходный код
 * @param {string} language - язык
 * @returns {object} - объект с очищенным кодом и статистикой
 */
function cleanCode(code, language) {
    const originalLength = code.length;
    const originalLines = code.split('\n').length;
    
    // Сохраняем оригинал для отладки
    const original = code;
    
    let cleaned = removeCommentsByLanguage(code, language);
    
    // Для C языка также удаляем строки с #include
    if (language === 'c') {
        const includeCount = (code.match(/^#include/gm) || []).length;
        cleaned = removeIncludeLines(cleaned);
        
        if (includeCount > 0) {
            console.log(`Удалено строк с #include: ${includeCount}`);
        }
    }
    
    const cleanedLength = cleaned.length;
    const cleanedLines = cleaned.split('\n').filter(line => line.trim().length > 0).length;
    
    console.log(`Очистка комментариев и #include [${language}]:`);
    console.log(`  Оригинал: ${originalLength} символов, ${originalLines} строк`);
    console.log(`  После очистки: ${cleanedLength} символов, ${cleanedLines} непустых строк`);
    console.log(`  Удалено: ${originalLength - cleanedLength} символов`);
    
    return {
        original,
        cleaned,
        stats: {
            originalLength,
            cleanedLength,
            originalLines,
            cleanedLines
        }
    };
}
export async function translateCode() {
    const inputLang = elements.inputLangSelect.value;
    const outputLang = elements.outputLangSelect.value;
    let code = elements.inputCode.value;

    if (!code || !code.trim()) {
        alert('Введите код для перевода');
        return;
    }

    // Сохраняем оригинал для отладки
    const originalCode = code;

    // Проверяем поддерживаемую пару языков
    if (inputLang === 'c' && outputLang === 'python') {
        await translateCToPython(code);
    } else if (inputLang === 'fortran' && outputLang === 'python') {
        await translateFortranToPython(code);
    } else {
        const errorMsg = `// Транспиляция из ${inputLang} в ${outputLang} пока не поддерживается\n// Доступно: C -> Python, Fortran -> Python`;
        elements.outputCode.value = errorMsg;
        updateSyntaxHighlighting();
        alert(`Пара языков ${inputLang} -> ${outputLang} пока не поддерживается. Доступно: C -> Python, Fortran -> Python`);
        return;
    }
}


async function translateCToPython(code) {
    // Удаляем комментарии ТОЛЬКО для C кода
    console.log('=== Обработка C кода - удаление комментариев ===');
    console.log('Исходный код (первые 300 символов):');
    console.log(code.substring(0, 300));
    console.log(`Длина исходного кода: ${code.length} символов`);
    
    const cleaned = cleanCode(code, 'c');
    code = cleaned.cleaned;
    
    console.log('Очищенный код (первые 300 символов):');
    console.log(code.substring(0, 300));
    console.log(`Длина очищенного кода: ${code.length} символов`);

    // Если после удаления комментариев код стал пустым
    if (!code || !code.trim()) {
        elements.outputCode.value = '# Пустой код после удаления комментариев';
        updateSyntaxHighlighting();
        alert('Код содержит только комментарии. Нечего транслировать.');
        return;
    }

    await performTranslation(code, 'C', 'Python');
}

async function translateFortranToPython(code) {
    console.log('Запуск транспиляции Fortran -> Python (через C)');
    console.log('Исходный код Fortran:');
    console.log(code);

    // Для Fortran не удаляем комментарии - f2c сервис сам обработает
    await performTranslation(code, 'Fortran', 'Python');
}

async function performTranslation(code, fromLang, toLang) {
    const originalText = elements.translateBtn.textContent;
    elements.translateBtn.textContent = 'Перевод...';
    elements.translateBtn.style.opacity = '0.7';
    elements.translateBtn.style.pointerEvents = 'none';

    try {
        console.log(`=== Запуск транспиляции ${fromLang} -> ${toLang} ===`);
        console.log(`Длина кода перед отправкой в Rust: ${code.length} символов`);
        console.log('Код для отправки (первые 300 символов):');
        console.log(code.substring(0, 300));

        // Выбираем команду в зависимости от языка
        const command = fromLang === 'C' ? 'transpile_c_to_python' : 'transpile_fortran_to_python';
        console.log(`Вызов команды: ${command}`);
        const result = await invoke(command, { code });
        
        console.log('Получен результат от Rust:', result.success ? 'УСПЕХ' : 'ОШИБКА');

        if (result.success) {
            elements.outputCode.value = result.output;
            updateSyntaxHighlighting();

            console.log('Транспиляция успешна');
            console.log(`Сгенерировано ${result.output.split('\n').length} строк кода`);

            if (result.ast_json) {
                console.log('AST получен, размер:', JSON.stringify(result.ast_json).length);
            }

            // Показываем статистику
            console.log('Статистика транспиляции:');
            console.log(`  Исходный код: ${code.length} символов`);
            console.log(`  Сгенерировано: ${result.output.length} символов`);
        } else {
            elements.outputCode.value = '';
            updateSyntaxHighlighting();
            console.error('Ошибка транспиляции:', result.error);
            alert('Ошибка транспиляции: ' + result.error);
        }
    } catch (error) {
        console.error('Ошибка при переводе:', error);
        elements.outputCode.value = '';
        updateSyntaxHighlighting();
        alert('Ошибка при переводе: ' + error);
    } finally {
        elements.translateBtn.textContent = originalText;
        elements.translateBtn.style.opacity = '1';
        elements.translateBtn.style.pointerEvents = 'auto';
    }
}

export function initTranslation() {
    if (elements.translateBtn) {
        elements.translateBtn.addEventListener('click', function(e) {
            e.preventDefault();
            translateCode();
        });
    }
    
    elements.inputCode.addEventListener('keydown', function(e) {
        if (e.ctrlKey && e.key === 'Enter') {
            e.preventDefault();
            translateCode();
        }
    });
    
    // Добавляем кнопку для проверки очистки комментариев (опционально)
    addDebugButton();
}

function addDebugButton() {
    // Проверяем, не добавлена ли уже кнопка
    if (document.getElementById('debug-comments-btn')) return;
    
    const container = document.querySelector('.button-container');
    if (!container) return;
    
    const debugBtn = document.createElement('button');
    debugBtn.id = 'debug-comments-btn';
    debugBtn.textContent = 'Проверить комментарии';
    debugBtn.style.marginLeft = '10px';
    debugBtn.style.backgroundColor = '#6c757d';
    
    debugBtn.addEventListener('click', function() {
        const code = elements.inputCode.value;
        const cleaned = cleanCode(code, 'c');
        
        console.log('РЕЗУЛЬТАТ ОЧИСТКИ КОММЕНТАРИЕВ:');
        console.log('Оригинал:');
        console.log(code);
        console.log('После очистки:');
        console.log(cleaned.cleaned);
        
        alert(`Комментарии удалены. Смотрите консоль (F12) для деталей.`);
    });
    
    container.appendChild(debugBtn);
}

// проверка статуса Docker и f2c сервисов
export async function checkDockerStatus() {
    try {
        // Проверяем основной парсер C
        const cStatus = await invoke('check_parser_status');
        console.log('Статус C парсера:', cStatus.docker_available ? 'Доступен' : 'Не доступен');

        // Проверяем f2c сервис для Fortran
        const f2cStatus = await invoke('check_f2c_status');
        console.log('Статус f2c сервиса:', f2cStatus.available ? 'Доступен' : 'Не доступен');

        if (!cStatus.docker_available) {
            console.warn('Docker не запущен. Транспиляция C может не работать.');
        }

        if (!f2cStatus.available || !f2cStatus.f2c_available) {
            console.warn('f2c сервис не запущен. Транспиляция Fortran может не работать.');
            console.warn('Для запуска выполните: cd src-tauri/docker && docker run -d -p 5001:5001 f2c-service');
        }

        // Обновляем статус-бар
        updateDockerStatus(cStatus.docker_available, f2cStatus.available && f2cStatus.f2c_available);

        return {
            c_parser: cStatus.docker_available,
            f2c_service: f2cStatus.available && f2cStatus.f2c_available
        };
    } catch (error) {
        console.error('Ошибка при проверке сервисов:', error);
        
        // Обновляем статус-бар с ошибкой
        updateDockerStatus(false, false);
        
        return {
            c_parser: false,
            f2c_service: false,
            error: error.toString()
        };
    }
}

function updateDockerStatus(cParserReady, f2cReady) {
    if (!elements.statusDocker) return;
    
    if (cParserReady && f2cReady) {
        elements.statusDocker.innerHTML = 'Docker: <span style="color: var(--success)">Готов</span>';
    } else if (cParserReady || f2cReady) {
        elements.statusDocker.innerHTML = 'Docker: <span style="color: var(--warning)">Частично</span>';
    } else {
        elements.statusDocker.innerHTML = 'Docker: <span style="color: var(--error)">Не готов</span>';
    }
}