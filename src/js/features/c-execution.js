// Модуль выполнения C кода

const { invoke } = window.__TAURI__.core;

let isCRunning = false;
let cExecution = null;

export function initCExecution() {
    const runInputBtn = document.getElementById('runInput');
    const stopInputBtn = document.getElementById('stopInput');
    const inputLangSelect = document.getElementById('input-lang-select');

    if (!runInputBtn || !stopInputBtn || !inputLangSelect) {
        console.warn('C execution elements not found');
        return;
    }

    // Обработчик кнопки запуска
    runInputBtn.addEventListener('click', async () => {
        await runCCode();
    });

    // Обработчик кнопки остановки
    stopInputBtn.addEventListener('click', async () => {
        await stopCExecution();
    });

    // Проверка доступности C при изменении языка
    inputLangSelect.addEventListener('change', () => {
        updateCButtonsState();
    });

    // Начальная проверка
    updateCButtonsState();

    console.log('C execution initialized');
}

async function checkCServiceAvailable() {
    try {
        const status = await invoke('check_c_status');
        return status.available && status.gcc_available;
    } catch (error) {
        console.error('Ошибка проверки C сервиса:', error);
        return false;
    }
}

function updateCButtonsState() {
    const runInputBtn = document.getElementById('runInput');
    const stopInputBtn = document.getElementById('stopInput');
    const inputLangSelect = document.getElementById('input-lang-select');

    if (!runInputBtn || !stopInputBtn || !inputLangSelect) return;

    const isCSelected = inputLangSelect.value === 'c';

    // Кнопки активны только если выбран C и сервис доступен
    if (isCSelected) {
        // Проверяем доступность сервиса
        checkCServiceAvailable().then(available => {
            isCRunning = available;

            if (available) {
                runInputBtn.disabled = false;
                runInputBtn.title = 'Запустить C код';
                stopInputBtn.disabled = false;
                stopInputBtn.title = 'Остановить выполнение';
            } else {
                runInputBtn.disabled = true;
                runInputBtn.title = 'C сервис недоступен (требуется Docker)';
                stopInputBtn.disabled = true;
                stopInputBtn.title = 'C сервис недоступен';
            }
        });
    } else {
        // Для других языков кнопки пока отключены
        runInputBtn.disabled = true;
        runInputBtn.title = 'Запуск кода доступен только для C';
        stopInputBtn.disabled = true;
        stopInputBtn.title = 'Остановка доступна только для C';
    }
}

async function runCCode() {
    const inputCode = document.getElementById('inputCode');
    const runInputBtn = document.getElementById('runInput');
    const stopInputBtn = document.getElementById('stopInput');

    if (!inputCode || !runInputBtn) return;

    const code = inputCode.value;

    if (!code || !code.trim()) {
        alert('Введите C код для запуска');
        return;
    }

    // Блокируем кнопку запуска на время выполнения
    runInputBtn.disabled = true;
    runInputBtn.innerHTML = `
        <svg class="spinner" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
        </svg>
    `;

    try {
        // Импортируем функцию для записи в терминал
        const { writeToInputTerminal } = await import('../ui/terminal-panel.js');

        writeToInputTerminal('Запуск выполнения C кода...', 'info');

        const result = await invoke('execute_c_code', { code });

        if (result.success) {
            // Выводим stdout в терминал
            if (result.stdout) {
                writeToInputTerminal(result.stdout, 'success');
            }

            // Выводим stderr в терминал (если есть)
            if (result.stderr) {
                writeToInputTerminal(result.stderr, 'error');
            }

            writeToInputTerminal(`Выполнение завершено (код возврата: ${result.return_code || 0})`, 'info');
        } else {
            writeToInputTerminal(`Ошибка выполнения: ${result.stderr || 'Неизвестная ошибка'}`, 'error');
        }
    } catch (error) {
        const { writeToInputTerminal } = await import('../ui/terminal-panel.js');
        writeToInputTerminal(`Критическая ошибка: ${error}`, 'error');
        console.error('Ошибка выполнения C кода:', error);
        alert('Ошибка при выполнении C кода: ' + error);
    } finally {
        // Восстанавливаем кнопку запуска
        runInputBtn.disabled = false;
        runInputBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
        `;
    }
}

async function stopCExecution() {
    const runInputBtn = document.getElementById('runInput');
    const stopInputBtn = document.getElementById('stopInput');

    try {
        const { writeToInputTerminal } = await import('../ui/terminal-panel.js');

        writeToInputTerminal('Остановка выполнения...', 'warning');

        const message = await invoke('stop_c_execution');

        writeToInputTerminal(message, 'info');
    } catch (error) {
        const { writeToInputTerminal } = await import('../ui/terminal-panel.js');
        writeToInputTerminal(`Ошибка остановки: ${error}`, 'error');
        console.error('Ошибка остановки C:', error);
    }
}

// Публичная функция для проверки доступности C
export function isCAvailable() {
    return isCRunning;
}
