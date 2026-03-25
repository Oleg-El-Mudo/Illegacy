// Модуль настроек приложения

const THEME_KEY = 'illegacy-theme';
const COMPACT_MODE_KEY = 'illegacy-compact-mode';
const FONT_SIZE_KEY = 'illegacy-font-size';
const TAB_SIZE_KEY = 'illegacy-tab-size';

function initSettings() {
    // Тема
    const themeSelect = document.getElementById('theme-select');
    if (themeSelect) {
        const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
        themeSelect.value = savedTheme;
        applyTheme(savedTheme);

        themeSelect.addEventListener('change', function(e) {
            const theme = e.target.value;
            localStorage.setItem(THEME_KEY, theme);
            applyTheme(theme);
        });
    }

    // Компактный режим
    const compactModeToggle = document.getElementById('compact-mode');
    if (compactModeToggle) {
        const compactMode = localStorage.getItem(COMPACT_MODE_KEY) === 'true';
        compactModeToggle.checked = compactMode;
        applyCompactMode(compactMode);

        compactModeToggle.addEventListener('change', function(e) {
            const enabled = e.target.checked;
            localStorage.setItem(COMPACT_MODE_KEY, enabled.toString());
            applyCompactMode(enabled);
        });
    }

    // Размер шрифта
    const fontSizeSelect = document.getElementById('font-size-select');
    if (fontSizeSelect) {
        const savedFontSize = localStorage.getItem(FONT_SIZE_KEY) || '14';
        fontSizeSelect.value = savedFontSize;
        
        fontSizeSelect.addEventListener('change', function(e) {
            const size = e.target.value;
            localStorage.setItem(FONT_SIZE_KEY, size.toString());
            applyFontSize(parseInt(size, 10));
        });
    }

    // Размер табуляции
    const tabSizeSelect = document.getElementById('tab-size-select');
    if (tabSizeSelect) {
        const savedTabSize = localStorage.getItem(TAB_SIZE_KEY) || '4';
        tabSizeSelect.value = savedTabSize;
        
        tabSizeSelect.addEventListener('change', function(e) {
            const size = e.target.value;
            localStorage.setItem(TAB_SIZE_KEY, size.toString());
            applyTabSize(parseInt(size, 10));
        });
    }

    // Кнопка сохранения
    const saveButton = document.getElementById('saveSettings');
    if (saveButton) {
        saveButton.addEventListener('click', function(e) {
            e.preventDefault();
            window.location.href = 'index.html';
        });
    }

    // Кнопка сброса настроек
    const resetButton = document.getElementById('resetSettings');
    if (resetButton) {
        resetButton.addEventListener('click', function(e) {
            e.preventDefault();
            if (confirm('Вы уверены, что хотите сбросить все настройки?')) {
                localStorage.removeItem(THEME_KEY);
                localStorage.removeItem(COMPACT_MODE_KEY);
                localStorage.removeItem(FONT_SIZE_KEY);
                localStorage.removeItem(TAB_SIZE_KEY);
                window.location.reload();
            }
        });
    }

    // Навигация через табы
    setupTabs();

    // Проверка статуса Docker
    checkDockerStatus();
}

/**
 * Применяет выбранную тему
 * @param {string} theme - 'dark', 'light' или 'auto'
 */
function applyTheme(theme) {
    const html = document.documentElement;

    if (theme === 'auto') {
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        html.setAttribute('data-bs-theme', prefersDark ? 'dark' : 'light');
    } else {
        html.setAttribute('data-bs-theme', theme);
    }
}

/**
 * Применяет компактный режим
 * @param {boolean} enabled
 */
function applyCompactMode(enabled) {
    document.documentElement.setAttribute('data-compact-mode', enabled.toString());
}

/**
 * Применяет размер шрифта
 * @param {number} size
 */
function applyFontSize(size) {
    const event = new CustomEvent('settings-change', {
        detail: { type: 'font-size', value: size }
    });
    document.dispatchEvent(event);
}

/**
 * Применяет размер табуляции
 * @param {number} size
 */
function applyTabSize(size) {
    const event = new CustomEvent('settings-change', {
        detail: { type: 'tab-size', value: size }
    });
    document.dispatchEvent(event);
}

/**
 * Настройка табов
 */
function setupTabs() {
    const tabs = document.querySelectorAll('.settings-tab');
    const sections = document.querySelectorAll('.settings-section');

    tabs.forEach(tab => {
        tab.addEventListener('click', function() {
            const targetTab = this.dataset.tab;

            // Удаляем активный класс у всех табов и секций
            tabs.forEach(t => t.classList.remove('active'));
            sections.forEach(s => s.classList.remove('active'));

            // Добавляем активный класс текущему табу и секции
            this.classList.add('active');
            const targetSection = document.getElementById(targetTab);
            if (targetSection) {
                targetSection.classList.add('active');
            }
        });
    });
}

async function checkDockerStatus() {
    const dockerStatusEl = document.getElementById('docker-status');
    if (!dockerStatusEl) return;

    try {
        const { invoke } = window.__TAURI__.core;
        
        const cStatus = await invoke('check_parser_status');
        const f2cStatus = await invoke('check_f2c_status');

        const cReady = cStatus.docker_available;
        const f2cReady = f2cStatus.available && f2cStatus.f2c_available;

        if (cReady && f2cReady) {
            dockerStatusEl.className = 'badge badge-success';
            dockerStatusEl.textContent = 'Готов';
        } else if (cReady || f2cReady) {
            dockerStatusEl.className = 'badge badge-warning';
            dockerStatusEl.textContent = 'Частично';
        } else {
            dockerStatusEl.className = 'badge badge-error';
            dockerStatusEl.textContent = 'Не готов';
        }
    } catch (error) {
        console.error('Ошибка при проверке сервисов:', error);
        dockerStatusEl.className = 'badge badge-error';
        dockerStatusEl.textContent = 'Ошибка';
    }
}

// Инициализация после загрузки DOM
document.addEventListener('DOMContentLoaded', initSettings);
