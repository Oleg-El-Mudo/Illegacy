import { languageMap } from '../core/constants.js';
import { escapeHtml } from '../core/utils.js';
import { elements } from '../core/dom.js';

// Загружаем Highlight.js из глобальной переменной
const hljs = window.hljs;
let scrollSyncEnabled = true;

export function setScrollSyncEnabled(value) {
    scrollSyncEnabled = value;
}

export function updateSyntaxHighlighting() {
    if (!hljs) {
        console.warn('Highlight.js не загружен');
        return;
    }
    
    // Сохраняем позиции скролла
    const inputScrollTop = elements.inputCode.scrollTop;
    const inputScrollLeft = elements.inputCode.scrollLeft;
    const outputScrollTop = elements.outputCode.scrollTop;
    const outputScrollLeft = elements.outputCode.scrollLeft;
    
    // Обновляем подсветку для входного кода
    const inputLang = languageMap[elements.inputLangSelect.value] || 'plaintext';
    const inputText = elements.inputCode.value;
    
    if (inputText.trim()) {
        try {
            if (hljs.getLanguage(inputLang)) {
                elements.inputCodePreview.innerHTML = hljs.highlight(inputText, { language: inputLang }).value;
            } else {
                console.warn(`Язык ${inputLang} не поддерживается Highlight.js`);
                elements.inputCodePreview.innerHTML = escapeHtml(inputText);
            }
        } catch (e) {
            console.warn('Ошибка подсветки для входного кода:', e);
            elements.inputCodePreview.innerHTML = escapeHtml(inputText);
        }
    } else {
        elements.inputCodePreview.innerHTML = '';
    }
    
    // Обновляем подсветку для выходного кода
    const outputLang = languageMap[elements.outputLangSelect.value] || 'plaintext';
    const outputText = elements.outputCode.value;
    
    if (outputText.trim()) {
        try {
            if (hljs.getLanguage(outputLang)) {
                elements.outputCodePreview.innerHTML = hljs.highlight(outputText, { language: outputLang }).value;
            } else {
                console.warn(`Язык ${outputLang} не поддерживается Highlight.js`);
                elements.outputCodePreview.innerHTML = escapeHtml(outputText);
            }
        } catch (e) {
            console.warn('Ошибка подсветки для выходного кода:', e);
            elements.outputCodePreview.innerHTML = escapeHtml(outputText);
        }
    } else {
        elements.outputCodePreview.innerHTML = '';
    }
    
    // Восстанавливаем позиции скролла
    scrollSyncEnabled = false;
    elements.inputCode.scrollTop = inputScrollTop;
    elements.inputCode.scrollLeft = inputScrollLeft;
    elements.outputCode.scrollTop = outputScrollTop;
    elements.outputCode.scrollLeft = outputScrollLeft;
    
    // Синхронизируем preview
    syncScroll();
    scrollSyncEnabled = true;
}

export function syncScroll() {
    if (!scrollSyncEnabled) return;
    
    // Синхронизация для левой колонки
    const leftContainer = elements.inputCode.closest('.code-container');
    const leftPreview = leftContainer.querySelector('.code-preview');
    if (leftPreview) {
        leftPreview.scrollTop = elements.inputCode.scrollTop;
        leftPreview.scrollLeft = elements.inputCode.scrollLeft;
    }
    
    // Синхронизация для правой колонки
    const rightContainer = elements.outputCode.closest('.code-container');
    const rightPreview = rightContainer.querySelector('.code-preview');
    if (rightPreview) {
        rightPreview.scrollTop = elements.outputCode.scrollTop;
        rightPreview.scrollLeft = elements.outputCode.scrollLeft;
    }
}

export function initSyntaxHighlighting() {
    if (!hljs) {
        console.warn('Highlight.js не загружен');
        return;
    }
    
    // Обновление подсветки при вводе текста
    elements.inputCode.addEventListener('input', updateSyntaxHighlighting);
    elements.outputCode.addEventListener('input', updateSyntaxHighlighting);
    
    // Обновление подсветки при смене языка
    elements.inputLangSelect.addEventListener('change', updateSyntaxHighlighting);
    elements.outputLangSelect.addEventListener('change', updateSyntaxHighlighting);
    
    // Синхронизация скролла
    elements.inputCode.addEventListener('scroll', () => {
        if (scrollSyncEnabled) {
            requestAnimationFrame(syncScroll);
        }
    });
    
    elements.outputCode.addEventListener('scroll', () => {
        if (scrollSyncEnabled) {
            requestAnimationFrame(syncScroll);
        }
    });
    
    // Обработка вставки/вырезания текста
    ['inputCode', 'outputCode'].forEach(id => {
        elements[id].addEventListener('paste', () => setTimeout(updateSyntaxHighlighting, 0));
        elements[id].addEventListener('cut', () => setTimeout(updateSyntaxHighlighting, 0));
    });
}