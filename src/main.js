import { getDOMElements, setElements, elements } from './modules/dom-elements.js';
import { initResizer, setInitialWidths } from './modules/resizer.js';
import { initSyntaxHighlighting, updateSyntaxHighlighting } from './modules/syntax-highlight.js';
import { initFileHandlers } from './modules/file-handlers.js';
import { initTranslation, checkDockerStatus } from './modules/translation.js';
import { initZoom } from './modules/zoom.js';
import { initIndentation } from './modules/indentation.js';
import { initHistory } from './modules/history.js';
import { initKeyboardShortcuts } from './modules/keyboard-shortcuts.js';

// Применение сохранённой темы при загрузке
const THEME_KEY = 'illegacy-theme';
function applySavedTheme() {
    const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
    if (savedTheme === 'auto') {
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        document.documentElement.setAttribute('data-bs-theme', prefersDark ? 'dark' : 'light');
    } else {
        document.documentElement.setAttribute('data-bs-theme', savedTheme);
    }
}
applySavedTheme();

// Инициализация DOM-элементов
const domElements = getDOMElements();
setElements(domElements);

// Инициализация всех модулей
function initApp() {
    initResizer();
    initSyntaxHighlighting();
    initFileHandlers();
    initTranslation();
    initZoom();
    initIndentation();
    initHistory(); // Инициализируем историю
    initKeyboardShortcuts(); // Инициализируем горячие клавиши
    
    // Проверяем статус Docker при загрузке
    checkDockerStatus();
    
    // Даем время на загрузку Highlight.js
    setTimeout(updateSyntaxHighlighting, 100);
}

// Запускаем приложение после загрузки DOM
document.addEventListener('DOMContentLoaded', initApp);

// Дополнительная проверка загрузки Highlight.js
document.addEventListener('DOMContentLoaded', function() {
    if (window.hljs) {
        setTimeout(updateSyntaxHighlighting, 100);
    }
});