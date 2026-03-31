// Модуль управления C сервисом

const { invoke } = window.__TAURI__.core;

let cInstalled = false;
let cRunning = false;

export async function initCIntegration() {
    const downloadBtn = document.getElementById('c-download-btn');
    const removeBtn = document.getElementById('c-remove-btn');
    const statusBadge = document.getElementById('c-status-badge');

    if (!downloadBtn || !removeBtn || !statusBadge) {
        console.warn('C integration elements not found');
        return;
    }

    // Проверка статуса при загрузке
    await checkCStatus();

    // Обработчик кнопки скачивания
    downloadBtn.addEventListener('click', async () => {
        await downloadCService();
    });

    // Обработчик кнопки удаления
    removeBtn.addEventListener('click', async () => {
        await removeCService();
    });

    console.log('C integration initialized');
}

async function checkCStatus() {
    const downloadBtn = document.getElementById('c-download-btn');
    const removeBtn = document.getElementById('c-remove-btn');
    const statusBadge = document.getElementById('c-status-badge');

    if (!downloadBtn || !removeBtn || !statusBadge) return;

    try {
        const status = await invoke('check_c_status');

        cInstalled = status.available && status.gcc_available;
        cRunning = cInstalled;

        if (cInstalled) {
            // Сервис доступен
            statusBadge.className = 'badge badge-success';
            statusBadge.textContent = 'Готов';
            downloadBtn.disabled = true;
            downloadBtn.classList.remove('jb-button-primary');
            downloadBtn.classList.add('jb-button-success');
            downloadBtn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
                <span>Установлен</span>
            `;

            removeBtn.disabled = false;
        } else if (status.available) {
            // Контейнер есть, но сервис не работает
            statusBadge.className = 'badge badge-warning';
            statusBadge.textContent = 'Остановлен';
            downloadBtn.disabled = false;
            downloadBtn.classList.add('jb-button-primary');
            downloadBtn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="23 4 23 10 17 10"></polyline>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                </svg>
                <span>Запустить</span>
            `;

            removeBtn.disabled = false;
        } else {
            // Сервис не установлен
            statusBadge.className = 'badge badge-secondary';
            statusBadge.textContent = 'Недоступно';
            downloadBtn.disabled = false;
            downloadBtn.classList.add('jb-button-primary');
            downloadBtn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
                <span>Скачать</span>
            `;

            removeBtn.disabled = true;
        }
    } catch (error) {
        console.error('Ошибка проверки статуса C:', error);
        statusBadge.className = 'badge badge-error';
        statusBadge.textContent = 'Ошибка';
        downloadBtn.disabled = true;
        removeBtn.disabled = true;
    }
}

async function downloadCService() {
    const downloadBtn = document.getElementById('c-download-btn');
    const statusBadge = document.getElementById('c-status-badge');

    if (!downloadBtn || !statusBadge) return;

    try {
        // Показываем индикатор загрузки
        const originalContent = downloadBtn.innerHTML;
        downloadBtn.disabled = true;
        downloadBtn.innerHTML = `
            <svg class="spinner" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
            </svg>
            <span>Загрузка...</span>
        `;
        statusBadge.className = 'badge badge-warning';
        statusBadge.textContent = 'Загрузка...';

        // В реальной реализации здесь будет:
        // 1. Проверка наличия Docker
        // 2. Pull образа или build из Dockerfile
        // 3. Запуск контейнера
        // invoke('install_c_service')

        // Симуляция процесса установки для демонстрации
        await new Promise(resolve => setTimeout(resolve, 2000));

        cInstalled = true;
        cRunning = true;

        await checkCStatus();

    } catch (error) {
        console.error('Ошибка установки C:', error);
        statusBadge.className = 'badge badge-error';
        statusBadge.textContent = 'Ошибка';
        downloadBtn.disabled = false;
        downloadBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="12" y1="5" x2="12" y2="19"></line>
                <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
            <span>Повторить</span>
        `;
    }
}

async function removeCService() {
    if (!confirm('Вы уверены, что хотите удалить C сервис?')) {
        return;
    }

    const statusBadge = document.getElementById('c-status-badge');

    try {
        statusBadge.className = 'badge badge-warning';
        statusBadge.textContent = 'Удаление...';

        // В реальной реализации здесь будет:
        // 1. Остановка контейнера
        // 2. Удаление контейнера
        // 3. Удаление образа
        // invoke('remove_c_service')

        // Временное решение
        await new Promise(resolve => setTimeout(resolve, 1000));

        cInstalled = false;
        cRunning = false;

        await checkCStatus();

    } catch (error) {
        console.error('Ошибка удаления C:', error);
        statusBadge.className = 'badge badge-error';
        statusBadge.textContent = 'Ошибка';
    }
}

export function isCAvailable() {
    return cInstalled && cRunning;
}
