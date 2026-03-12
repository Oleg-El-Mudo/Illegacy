import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

// Константы для масштабирования
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 32;
const DEFAULT_FONT_SIZE = 14;
const ZOOM_STEP = 2;

// Ключ для сохранения в localStorage
const STORAGE_KEY = 'illegancy-font-size';

// Текущий размер шрифта
let currentFontSize = DEFAULT_FONT_SIZE;

// Загрузка сохраненного размера шрифта
export function loadFontSize() {
    try {
        const saved = localStorage.getItem(STORAGE_KEY);
        if (saved) {
            const size = parseInt(saved, 10);
            if (size >= MIN_FONT_SIZE && size <= MAX_FONT_SIZE) {
                currentFontSize = size;
            }
        }
    } catch (e) {
        console.warn('Failed to load font size from localStorage:', e);
    }
    return currentFontSize;
}

// Сохранение размера шрифта
export function saveFontSize(size) {
    try {
        localStorage.setItem(STORAGE_KEY, size.toString());
    } catch (e) {
        console.warn('Failed to save font size to localStorage:', e);
    }
}

// Применение размера шрифта ко всем элементам
export function applyFontSize(size) {
    // Ограничиваем размер
    size = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, size));
    
    if (size === currentFontSize) return;
    
    currentFontSize = size;
    
    // Сохраняем позиции скролла перед изменением
    const scrollPositions = {
        input: { top: elements.inputCode.scrollTop, left: elements.inputCode.scrollLeft },
        output: { top: elements.outputCode.scrollTop, left: elements.outputCode.scrollLeft }
    };
    
    // Применяем размер шрифта ко всем текстовым элементам
    const fontSizePx = `${size}px`;
    
    // Textarea для ввода
    elements.inputCode.style.fontSize = fontSizePx;
    elements.outputCode.style.fontSize = fontSizePx;
    
    // Preview элементы 
    const inputPreview = elements.inputCodePreview.closest('pre');
    const outputPreview = elements.outputCodePreview.closest('pre');
    
    if (inputPreview) {
        inputPreview.style.fontSize = fontSizePx;
    }
    if (outputPreview) {
        outputPreview.style.fontSize = fontSizePx;
    }
    
    // Также обновляем сам code элемент внутри preview
    elements.inputCodePreview.style.fontSize = fontSizePx;
    elements.outputCodePreview.style.fontSize = fontSizePx;
    
    // Сохраняем в localStorage
    saveFontSize(size);
    
    // Восстанавливаем позиции скролла
    requestAnimationFrame(() => {
        elements.inputCode.scrollTop = scrollPositions.input.top;
        elements.inputCode.scrollLeft = scrollPositions.input.left;
        elements.outputCode.scrollTop = scrollPositions.output.top;
        elements.outputCode.scrollLeft = scrollPositions.output.left;
        
        // Обновляем подсветку для корректного отображения
        updateSyntaxHighlighting();
    });
}

// Увеличение размера шрифта
export function zoomIn() {
    applyFontSize(currentFontSize + ZOOM_STEP);
}

// Уменьшение размера шрифта
export function zoomOut() {
    applyFontSize(currentFontSize - ZOOM_STEP);
}

// Сброс к размеру по умолчанию
export function zoomReset() {
    applyFontSize(DEFAULT_FONT_SIZE);
}

// Получение текущего размера шрифта
export function getCurrentFontSize() {
    return currentFontSize;
}

// Инициализация обработчиков масштабирования
export function initZoom() {
    // Загружаем сохраненный размер
    loadFontSize();
    
    // Применяем начальный размер
    applyFontSize(currentFontSize);
    
    // Обработчик для Ctrl + + / Ctrl + -
    document.addEventListener('keydown', (e) => {
        // Проверяем, не вводит ли пользователь текст в поле
        const isTyping = e.target.matches('textarea, input, [contenteditable="true"]');
        
        if (e.ctrlKey || e.metaKey) { // metaKey для Mac
            switch (e.key) {
                case '+':
                case '=': // = без Shift дает + на некоторых раскладках
                case 'Add': // Numpad +
                    e.preventDefault();
                    if (isTyping) {
                        // Если пользователь печатает, добавляем символ
                        return;
                    }
                    zoomIn();
                    break;
                    
                case '-':
                case 'Subtract': // Numpad -
                    e.preventDefault();
                    if (isTyping) {
                        return;
                    }
                    zoomOut();
                    break;
                    
                case '0':
                    e.preventDefault();
                    if (isTyping) {
                        return;
                    }
                    zoomReset();
                    break;
            }
        }
        
    });
    
    console.log('Zoom module initialized with font size:', currentFontSize);
}