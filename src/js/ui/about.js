// Модуль для отображения информации о приложении
import { elements } from '../core/dom.js';

const VERSION_KEY = 'illegacy-app-version';

export function initAboutPage() {
    loadAppVersion();
    console.log('About page initialized');
}

async function loadAppVersion() {
    try {
        // Пытаемся получить версию через Tauri API
        if (window.__TAURI__) {
            const { getVersion } = window.__TAURI__.app;
            if (getVersion) {
                const version = await getVersion();
                updateVersionDisplay(version);
                return;
            }
        }

        // Если Tauri API недоступно, пробуем загрузить из файла конфигурации
        const response = await fetch('../src-tauri/tauri.conf.json');
        if (response.ok) {
            const config = await response.json();
            updateVersionDisplay(config.version);
        }
    } catch (error) {
        console.warn('Не удалось загрузить версию приложения:', error);
        // Используем версию по умолчанию
        updateVersionDisplay('0.2.4');
    }
}

function updateVersionDisplay(version) {
    const versionElement = document.getElementById('app-version');
    if (versionElement) {
        versionElement.textContent = version;
    }
}
