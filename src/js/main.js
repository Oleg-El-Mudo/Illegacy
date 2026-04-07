import { getDOMElements, setElements, elements } from './core/dom.js';
import { initResizer, setInitialWidths } from './ui/resizer.js';
import { initSyntaxHighlighting, updateSyntaxHighlighting } from './editor/syntax-highlight.js';
import { initFileHandlers } from './features/file-handlers.js';
import { initTranslation, checkDockerStatus } from './features/translation.js';
import { initZoom } from './editor/zoom.js';
import { initIndentation } from './editor/indentation.js';
import { initHistory } from './editor/history.js';
import { initKeyboardShortcuts } from './ui/keyboard-shortcuts.js';
import { initSettingsModal } from './ui/settings-modal.js';
import { initTerminalPanel } from './ui/terminal-panel.js';
import { initAboutPage } from './ui/about.js';
import { initPythonExecution } from './features/python-execution.js';
import { initCExecution } from './features/c-execution.js';
import { initProgressModal } from './ui/progress-modal.js';
import { initLLMManager } from './features/llm-manager.js';
import { initAIOptimizer } from './features/ai-optimizer.js';

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

// Инициализация всех модулей
function initApp() {
    // Инициализация DOM-элементов после загрузки DOM
    const domElements = getDOMElements();
    setElements(domElements);

    initResizer();
    initSyntaxHighlighting();
    initFileHandlers();
    initTranslation();
    initZoom();
    initIndentation();
    initHistory(); // Инициализируем историю
    initKeyboardShortcuts(); // Инициализируем горячие клавиши
    initSettingsModal(); // Инициализируем модальное окно настроек
    initTerminalPanel(); // Инициализируем панель терминала
    initAboutPage(); // Инициализируем страницу "О программе"
    initPythonExecution(); // Инициализируем выполнение Python кода
    initCExecution(); // Инициализируем выполнение C кода
    initProgressModal(); // Инициализируем модальное окно прогресса
    initLLMManager(); // Инициализируем управление LLM моделями
    initAIOptimizer(); // Инициализируем ИИ-оптимизацию

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