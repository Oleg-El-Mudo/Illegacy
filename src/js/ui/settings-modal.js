// Модуль модального окна настроек

const THEME_KEY = 'illegacy-theme';
const COMPACT_MODE_KEY = 'illegacy-compact-mode';
const FONT_SIZE_KEY = 'illegacy-font-size';
const TAB_SIZE_KEY = 'illegacy-tab-size';

let modalOverlay;
let settingsModal;

function openSettings() {
    if (!modalOverlay) return;
    modalOverlay.classList.add('active');
    document.body.style.overflow = 'hidden';
    
    // Проверяем статус Docker при открытии
    checkDockerStatusInModal();
}

function closeSettings() {
    if (!modalOverlay) return;
    modalOverlay.classList.remove('active');
    document.body.style.overflow = '';
}

export function initSettingsModal() {
    // Получаем элементы модального окна
    modalOverlay = document.getElementById('settingsModalOverlay');
    settingsModal = modalOverlay?.querySelector('.settings-modal');

    if (!modalOverlay || !settingsModal) {
        console.warn('Settings modal elements not found');
        return;
    }

    // Кнопки управления
    const openSettingsBtn = document.getElementById('openSettings');
    const closeSettingsBtn = document.getElementById('closeSettings');
    const cancelSettingsBtn = document.getElementById('cancelSettings');
    const saveSettingsBtn = document.getElementById('saveSettings');
    const resetSettingsBtn = document.getElementById('resetSettings');

    // Открытие модального окна
    if (openSettingsBtn) {
        openSettingsBtn.addEventListener('click', openSettings);
    }

    // Закрытие модального окна
    if (closeSettingsBtn) {
        closeSettingsBtn.addEventListener('click', closeSettings);
    }

    if (cancelSettingsBtn) {
        cancelSettingsBtn.addEventListener('click', closeSettings);
    }
    
    // Закрытие по клику на overlay
    modalOverlay.addEventListener('click', (e) => {
        if (e.target === modalOverlay) {
            closeSettings();
        }
    });
    
    // Закрытие по Escape
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && modalOverlay.classList.contains('active')) {
            closeSettings();
        }
    });
    
    // Сохранение настроек
    if (saveSettingsBtn) {
        saveSettingsBtn.addEventListener('click', saveSettings);
    }
    
    // Сброс настроек
    if (resetSettingsBtn) {
        resetSettingsBtn.addEventListener('click', resetSettings);
    }
    
    // Инициализация настроек
    initSettingsValues();
    
    // Навигация через табы
    setupTabs();
    
    console.log('Settings modal initialized');
}

function saveSettings(e) {
    if (e) e.preventDefault();

    // Сохраняем тему в localStorage (тема уже применена в реальном времени)
    const themeSelect = document.getElementById('theme-select');
    if (themeSelect) {
        localStorage.setItem(THEME_KEY, themeSelect.value);
    }

    // Сохраняем компактный режим в localStorage (уже применён)
    const compactModeToggle = document.getElementById('compact-mode');
    if (compactModeToggle) {
        localStorage.setItem(COMPACT_MODE_KEY, compactModeToggle.checked.toString());
    }

    // Сохраняем размер шрифта
    const fontSizeSelect = document.getElementById('font-size-select');
    if (fontSizeSelect) {
        localStorage.setItem(FONT_SIZE_KEY, fontSizeSelect.value);
        applyFontSize(parseInt(fontSizeSelect.value, 10));
    }

    // Сохраняем размер табуляции
    const tabSizeSelect = document.getElementById('tab-size-select');
    if (tabSizeSelect) {
        localStorage.setItem(TAB_SIZE_KEY, tabSizeSelect.value);
        applyTabSize(parseInt(tabSizeSelect.value, 10));
    }

    closeSettings();
}

function resetSettings(e) {
    if (e) e.preventDefault();
    
    if (confirm('Вы уверены, что хотите сбросить все настройки?')) {
        localStorage.removeItem(THEME_KEY);
        localStorage.removeItem(COMPACT_MODE_KEY);
        localStorage.removeItem(FONT_SIZE_KEY);
        localStorage.removeItem(TAB_SIZE_KEY);
        
        // Перезагружаем страницу для применения настроек по умолчанию
        window.location.reload();
    }
}

function initSettingsValues() {
    // Тема
    const themeSelect = document.getElementById('theme-select');
    if (themeSelect) {
        const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
        themeSelect.value = savedTheme;
        // Применяем тему сразу при инициализации
        applyTheme(savedTheme);
        // Добавляем live-применение темы при изменении
        themeSelect.addEventListener('change', function(e) {
            applyTheme(e.target.value);
        });
    }

    // Компактный режим
    const compactModeToggle = document.getElementById('compact-mode');
    if (compactModeToggle) {
        const compactMode = localStorage.getItem(COMPACT_MODE_KEY) === 'true';
        compactModeToggle.checked = compactMode;
        // Применяем компактный режим сразу
        applyCompactMode(compactMode);
        // Добавляем live-применение при изменении
        compactModeToggle.addEventListener('change', function(e) {
            applyCompactMode(e.target.checked);
        });
    }

    // Размер шрифта
    const fontSizeSelect = document.getElementById('font-size-select');
    if (fontSizeSelect) {
        const savedFontSize = localStorage.getItem(FONT_SIZE_KEY) || '14';
        fontSizeSelect.value = savedFontSize;
        // Добавляем live-применение при изменении
        fontSizeSelect.addEventListener('change', function(e) {
            applyFontSize(parseInt(e.target.value, 10));
        });
    }

    // Размер табуляции
    const tabSizeSelect = document.getElementById('tab-size-select');
    if (tabSizeSelect) {
        const savedTabSize = localStorage.getItem(TAB_SIZE_KEY) || '4';
        tabSizeSelect.value = savedTabSize;
    }
}

function applyTheme(theme) {
    const html = document.documentElement;
    
    if (theme === 'auto') {
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        html.setAttribute('data-bs-theme', prefersDark ? 'dark' : 'light');
    } else {
        html.setAttribute('data-bs-theme', theme);
    }
}

function applyCompactMode(enabled) {
    document.documentElement.setAttribute('data-compact-mode', enabled.toString());
}

function applyFontSize(size) {
    // Импортируем функцию applyFontSize из модуля zoom
    import('../editor/zoom.js').then(({ applyFontSize: applyZoomFontSize }) => {
        applyZoomFontSize(size);
    });
}

function applyTabSize(size) {
    const event = new CustomEvent('settings-change', {
        detail: { type: 'tab-size', value: size }
    });
    document.dispatchEvent(event);
}

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

async function checkDockerStatusInModal() {
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
