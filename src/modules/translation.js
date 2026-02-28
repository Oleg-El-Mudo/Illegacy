import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

const { invoke } = window.__TAURI__.core;

export async function translateCode() {
    const inputLang = elements.inputLangSelect.value;
    const outputLang = elements.outputLangSelect.value;
    const code = elements.inputCode.value.trim();
    
    if (!code) {
        alert('Введите код для перевода');
        return;
    }
    
    if (inputLang !== 'c' || outputLang !== 'python') {
        const errorMsg = `// Транспиляция из ${inputLang} в ${outputLang} пока не поддерживается\n// Доступно: C -> Python`;
        elements.outputCode.value = errorMsg;
        updateSyntaxHighlighting();
        alert(`Пара языков ${inputLang} -> ${outputLang} пока не поддерживается. Доступно: C -> Python`);
        return;
    }
    
    const originalText = elements.translateBtn.textContent;
    elements.translateBtn.textContent = 'Перевод...';
    elements.translateBtn.style.opacity = '0.7';
    elements.translateBtn.style.pointerEvents = 'none';
    
    try {
        console.log('Запуск транспиляции C -> Python');
        
        const result = await invoke('transpile_c_to_python', { code });
        
        if (result.success) {
            elements.outputCode.value = result.output;
            updateSyntaxHighlighting();
            console.log('Транспиляция успешна');
            
            if (result.ast_json) {
                console.log('AST получен');
            }
        } else {
            elements.outputCode.value = '';
            updateSyntaxHighlighting();
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
}

// Опционально: проверка статуса Docker
export async function checkDockerStatus() {
    try {
        const status = await invoke('check_parser_status');
        console.log('Статус Docker:', status.docker_available ? 'Доступен' : 'Не доступен');
        
        if (!status.docker_available) {
            console.warn('Docker не запущен. Транспиляция может не работать.');
        }
    } catch (error) {
        console.error('Ошибка при проверке Docker:', error);
    }
}