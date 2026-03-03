// main.js
import { getDOMElements, setElements, elements } from './modules/dom-elements.js';
import { initResizer, setInitialWidths } from './modules/resizer.js';
import { initSyntaxHighlighting, updateSyntaxHighlighting } from './modules/syntax-highlight.js';
import { initFileHandlers } from './modules/file-handlers.js';
import { initTranslation, checkDockerStatus } from './modules/translation.js';
import { initZoom } from './modules/zoom.js';
import { initIndentation } from './modules/indentation.js'; // Импортируем новый модуль

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
    initIndentation(); // Добавляем инициализацию выравнивания
    
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