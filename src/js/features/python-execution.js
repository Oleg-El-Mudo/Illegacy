// Модуль выполнения Python кода

const { invoke } = window.__TAURI__.core;

let isPythonRunning = false;
let pythonExecution = null;

export function initPythonExecution() {
    const runOutputBtn = document.getElementById('runOutput');
    const stopOutputBtn = document.getElementById('stopOutput');
    const outputLangSelect = document.getElementById('output-lang-select');

    if (!runOutputBtn || !stopOutputBtn || !outputLangSelect) {
        console.warn('Python execution elements not found');
        return;
    }

    // Обработчик кнопки запуска
    runOutputBtn.addEventListener('click', async () => {
        await runPythonCode();
    });

    // Обработчик кнопки остановки
    stopOutputBtn.addEventListener('click', async () => {
        await stopPythonExecution();
    });

    // Проверка доступности Python при изменении языка
    outputLangSelect.addEventListener('change', () => {
        updatePythonButtonsState();
    });

    // Начальная проверка
    updatePythonButtonsState();

    console.log('Python execution initialized');
}

async function checkPythonServiceAvailable() {
    try {
        const status = await invoke('check_python_status');
        return status.available && status.python_available;
    } catch (error) {
        console.error('Ошибка проверки Python сервиса:', error);
        return false;
    }
}

function updatePythonButtonsState() {
    const runOutputBtn = document.getElementById('runOutput');
    const stopOutputBtn = document.getElementById('stopOutput');
    const outputLangSelect = document.getElementById('output-lang-select');

    if (!runOutputBtn || !stopOutputBtn || !outputLangSelect) return;

    const isPythonSelected = outputLangSelect.value === 'python';
    
    // Кнопки активны только если выбран Python и сервис доступен
    if (isPythonSelected) {
        // Проверяем доступность сервиса
        checkPythonServiceAvailable().then(available => {
            isPythonRunning = available;
            
            if (available) {
                runOutputBtn.disabled = false;
                runOutputBtn.title = 'Запустить Python код';
                stopOutputBtn.disabled = false;
                stopOutputBtn.title = 'Остановить выполнение';
            } else {
                runOutputBtn.disabled = true;
                runOutputBtn.title = 'Python сервис недоступен (требуется установка в настройках)';
                stopOutputBtn.disabled = true;
                stopOutputBtn.title = 'Python сервис недоступен';
            }
        });
    } else {
        // Для других языков кнопки пока отключены
        runOutputBtn.disabled = true;
        runOutputBtn.title = 'Запуск кода доступен только для Python';
        stopOutputBtn.disabled = true;
        stopOutputBtn.title = 'Остановка доступна только для Python';
    }
}

async function runPythonCode() {
    const outputCode = document.getElementById('outputCode');
    const runOutputBtn = document.getElementById('runOutput');
    const stopOutputBtn = document.getElementById('stopOutput');

    if (!outputCode || !runOutputBtn) return;

    const code = outputCode.value;

    if (!code || !code.trim()) {
        alert('Введите Python код для запуска');
        return;
    }

    // Блокируем кнопку запуска на время выполнения
    runOutputBtn.disabled = true;
    runOutputBtn.innerHTML = `
        <svg class="spinner" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
        </svg>
    `;

    try {
        // Импортируем функцию для записи в терминал
        const { writeToOutputTerminal } = await import('../ui/terminal-panel.js');
        
        writeToOutputTerminal('Запуск выполнения Python кода...', 'info');

        const result = await invoke('execute_python_code', { code });

        if (result.success) {
            // Выводим stdout в терминал
            if (result.stdout) {
                writeToOutputTerminal(result.stdout, 'success');
            }
            
            // Выводим stderr в терминал (если есть)
            if (result.stderr) {
                writeToOutputTerminal(result.stderr, 'error');
            }

            writeToOutputTerminal(`Выполнение завершено (код возврата: ${result.return_code || 0})`, 'info');
        } else {
            writeToOutputTerminal(`Ошибка выполнения: ${result.stderr || 'Неизвестная ошибка'}`, 'error');
        }
    } catch (error) {
        const { writeToOutputTerminal } = await import('../ui/terminal-panel.js');
        writeToOutputTerminal(`Критическая ошибка: ${error}`, 'error');
        console.error('Ошибка выполнения Python кода:', error);
        alert('Ошибка при выполнении Python кода: ' + error);
    } finally {
        // Восстанавливаем кнопку запуска
        runOutputBtn.disabled = false;
        runOutputBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="none">
                <polygon points="5 3 19 12 5 21 5 3"></polygon>
            </svg>
        `;
    }
}

async function stopPythonExecution() {
    const runOutputBtn = document.getElementById('runOutput');
    const stopOutputBtn = document.getElementById('stopOutput');

    try {
        const { writeToOutputTerminal } = await import('../ui/terminal-panel.js');
        
        writeToOutputTerminal('Остановка выполнения...', 'warning');

        const message = await invoke('stop_python_execution');
        
        writeToOutputTerminal(message, 'info');
    } catch (error) {
        const { writeToOutputTerminal } = await import('../ui/terminal-panel.js');
        writeToOutputTerminal(`Ошибка остановки: ${error}`, 'error');
        console.error('Ошибка остановки Python:', error);
    }
}

// Публичная функция для проверки доступности Python
export function isPythonAvailable() {
    return isPythonRunning;
}
