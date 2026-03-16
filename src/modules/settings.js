// Модуль настроек приложения

const THEME_KEY = 'illegacy-theme';

/**
 * Инициализация страницы настроек
 */
function initSettings() {
    const themeSelect = document.getElementById('theme-select');
    
    // Загружаем сохранённую тему
    const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
    themeSelect.value = savedTheme;
    
    // Применяем тему при загрузке
    applyTheme(savedTheme);
    
    // Обработчик изменения темы
    themeSelect.addEventListener('change', function(e) {
        const theme = e.target.value;
        localStorage.setItem(THEME_KEY, theme);
        applyTheme(theme);
    });
}

/**
 * Применяет выбранную тему
 * @param {string} theme - 'dark', 'light' или 'auto'
 */
function applyTheme(theme) {
    const html = document.documentElement;
    
    if (theme === 'auto') {
        // Системная тема
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        html.setAttribute('data-bs-theme', prefersDark ? 'dark' : 'light');
    } else {
        html.setAttribute('data-bs-theme', theme);
    }
}

// Инициализация после загрузки DOM
document.addEventListener('DOMContentLoaded', initSettings);
