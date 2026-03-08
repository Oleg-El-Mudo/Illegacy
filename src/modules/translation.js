import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

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
            i += 2; // Пропускаем //
            
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
            // Для Python можно добавить удаление #
            // Но пока не нужно
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
    
    // Удаляем комментарии
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
    
    if (inputLang !== 'c' || outputLang !== 'python') {
        const errorMsg = `// Транспиляция из ${inputLang} в ${outputLang} пока не поддерживается\n// Доступно: C -> Python`;
        elements.outputCode.value = errorMsg;
        updateSyntaxHighlighting();
        alert(`Пара языков ${inputLang} -> ${outputLang} пока не поддерживается. Доступно: C -> Python`);
        return;
    }
    
    // Удаляем комментарии ТОЛЬКО для C кода
    console.log('Обработка C кода - удаление комментариев');
    const cleaned = cleanCode(code, 'c');
    code = cleaned.cleaned;
    
    // Если после удаления комментариев код стал пустым
    if (!code || !code.trim()) {
        elements.outputCode.value = '# Пустой код после удаления комментариев';
        updateSyntaxHighlighting();
        alert('Код содержит только комментарии. Нечего транслировать.');
        return;
    }
    
    const originalText = elements.translateBtn.textContent;
    elements.translateBtn.textContent = 'Перевод...';
    elements.translateBtn.style.opacity = '0.7';
    elements.translateBtn.style.pointerEvents = 'none';
    
    try {
        console.log('Запуск транспиляции C -> Python');
        console.log('Код для отправки (первые 200 символов):');
        console.log(code.substring(0, 200));
        
        const result = await invoke('transpile_c_to_python', { code });
        
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
            console.log(`  Исходный код: ${originalCode.length} символов`);
            console.log(`  После удаления комментариев: ${code.length} символов`);
            console.log(`  Сгенерировано: ${result.output.length} символов`);
        } else {
            elements.outputCode.value = '';
            updateSyntaxHighlighting();
            console.error('Ошибка транспиляции:', result.error);
            
            // Пытаемся определить, связана ли ошибка с комментариями
            if (result.error && result.error.includes('comment')) {
                alert('Ошибка связана с комментариями. Попробуйте удалить комментарии вручную.');
            } else {
                alert('Ошибка транспиляции: ' + result.error);
            }
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

/**
 * Добавляет отладочную кнопку для проверки удаления комментариев
 */
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

// Опционально: проверка статуса Docker
export async function checkDockerStatus() {
    try {
        const status = await invoke('check_parser_status');
        console.log('Статус Docker:', status.docker_available ? 'Доступен' : 'Не доступен');
        
        if (!status.docker_available) {
            console.warn('Docker не запущен. Транспиляция может не работать.');
        }
        
        return status;
    } catch (error) {
        console.error('Ошибка при проверке Docker:', error);
        return { docker_available: false, error: error.toString() };
    }
}