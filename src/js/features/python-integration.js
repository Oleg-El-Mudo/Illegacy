// Модуль управления Python сервисом

const { invoke } = window.__TAURI__.core;

let pythonInstalled = false;
let pythonRunning = false;

export async function initPythonIntegration() {
    const downloadBtn = document.getElementById('python-download-btn');
    const removeBtn = document.getElementById('python-remove-btn');
    const statusBadge = document.getElementById('python-status-badge');
    const translatorItem = document.getElementById('python-translator-item');

    if (!downloadBtn || !removeBtn || !statusBadge) {
        console.warn('Python integration elements not found');
        return;
    }

    // Проверка статуса при загрузке
    await checkPythonStatus();

    // Обработчик кнопки скачивания
    downloadBtn.addEventListener('click', async () => {
        await downloadPythonService();
    });

    // Обработчик кнопки удаления
    removeBtn.addEventListener('click', async () => {
        await removePythonService();
    });

    console.log('Python integration initialized');
}

async function checkPythonStatus() {
    const downloadBtn = document.getElementById('python-download-btn');
    const removeBtn = document.getElementById('python-remove-btn');
    const statusBadge = document.getElementById('python-status-badge');
    const translatorItem = document.getElementById('python-translator-item');

    if (!downloadBtn || !removeBtn || !statusBadge) return;

    try {
        const status = await invoke('check_python_status');

        pythonInstalled = status.available && status.python_available;
        pythonRunning = pythonInstalled;

        if (pythonInstalled) {
            // Сервис доступен
            statusBadge.className = 'badge badge-success';
            statusBadge.textContent = 'Готов';
            translatorItem.classList.remove('disabled');
            
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
            translatorItem.classList.remove('disabled');
            
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
            translatorItem.classList.add('disabled');
            
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
        console.error('Ошибка проверки статуса Python:', error);
        statusBadge.className = 'badge badge-error';
        statusBadge.textContent = 'Ошибка';
        translatorItem.classList.add('disabled');
        downloadBtn.disabled = true;
        removeBtn.disabled = true;
    }
}

async function downloadPythonService() {
    const downloadBtn = document.getElementById('python-download-btn');
    const statusBadge = document.getElementById('python-status-badge');

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

        // Симуляция процесса установки (в реальности здесь будет логика Docker)
        // Для демонстрации просто ждем и затем показываем успех
        await new Promise(resolve => setTimeout(resolve, 2000));

        /* TODO

        1. Проверка наличия Docker
        2. Pull образа или build из Dockerfile
        3. Запуск контейнера
        
        Временное решение - просто показываем успех
        В будущем: invoke('install_python_service')
        */
        
        pythonInstalled = true;
        pythonRunning = true;
        
        await checkPythonStatus();
        
    } catch (error) {
        console.error('Ошибка установки Python:', error);
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

async function removePythonService() {
    if (!confirm('Вы уверены, что хотите удалить Python интерпретатор?')) {
        return;
    }

    const statusBadge = document.getElementById('python-status-badge');

    try {
        statusBadge.className = 'badge badge-warning';
        statusBadge.textContent = 'Удаление...';

        /* TODO
        
        В реальной реализации здесь будет:
        1. Остановка контейнера
        2. Удаление контейнера
        3. Удаление образа
        invoke('remove_python_service')
        
        */
        // Временное решение
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        pythonInstalled = false;
        pythonRunning = false;
        
        await checkPythonStatus();
        
    } catch (error) {
        console.error('Ошибка удаления Python:', error);
        statusBadge.className = 'badge badge-error';
        statusBadge.textContent = 'Ошибка';
    }
}

export function isPythonAvailable() {
    return pythonInstalled && pythonRunning;
}
