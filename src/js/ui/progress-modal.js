// Модуль модального окна прогресса установки

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let progressModalOverlay;
let progressModal;
let isRefreshing = false;

// Элементы управления
let progressMessage;
let progressPercent;
let progressBarFill;
let progressLog;
let closeProgressBtn;

export function initProgressModal() {
    progressModalOverlay = document.getElementById('progressModalOverlay');
    progressModal = progressModalOverlay?.querySelector('.progress-modal');

    if (!progressModalOverlay || !progressModal) {
        console.warn('Progress modal elements not found');
        return;
    }

    // Получаем элементы
    progressMessage = document.getElementById('progressMessage');
    progressPercent = document.getElementById('progressPercent');
    progressBarFill = document.getElementById('progressBarFill');
    progressLog = document.getElementById('progressLog');
    closeProgressBtn = document.getElementById('closeProgressModal');

    // Кнопка закрытия
    if (closeProgressBtn) {
        closeProgressBtn.addEventListener('click', closeProgressModal);
    }

    // Закрытие по клику на overlay
    progressModalOverlay.addEventListener('click', (e) => {
        if (e.target === progressModalOverlay && !isRefreshing) {
            closeProgressModal();
        }
    });

    // Закрытие по Escape
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && progressModalOverlay.classList.contains('active') && !isRefreshing) {
            closeProgressModal();
        }
    });

    // Слушаем события от Rust
    setupEventListeners();

    console.log('Progress modal initialized');
}

function setupEventListeners() {
    // Слушаем события прогресса
    listen('refresh-progress', (event) => {
        const data = event.payload;
        updateProgress(data.message, data.progress, data.is_error);
    });

    // Слушаем логи
    listen('refresh-log', (event) => {
        const logLine = event.payload;
        addLogLine(logLine);
    });
}

function updateProgress(message, progress, isError) {
    if (progressMessage) {
        progressMessage.textContent = message;
        if (isError) {
            progressMessage.style.color = 'var(--error)';
        } else {
            progressMessage.style.color = 'var(--text-primary)';
        }
    }

    if (progressPercent) {
        const percent = Math.round(progress * 100);
        progressPercent.textContent = `${percent}%`;
    }

    if (progressBarFill) {
        progressBarFill.style.width = `${progress * 100}%`;
    }

    // Если процесс завершен
    if (progress >= 1.0) {
        isRefreshing = false;
        if (closeProgressBtn) {
            closeProgressBtn.disabled = false;
        }
        
        if (isError) {
            addLogLine('Процесс завершен с ошибкой', 'error');
        } else {
            addLogLine('Процесс завершен успешно!', 'success');
        }
        
        // Автоматически закрываем через 3 секунды после успеха
        if (!isError) {
            setTimeout(() => {
                closeProgressModal();
                // Обновляем статусы сервисов после переустановки
                import('./settings-modal.js').then(({ refreshDockerServicesStatus }) => {
                    if (refreshDockerServicesStatus) {
                        refreshDockerServicesStatus();
                    }
                });
            }, 3000);
        }
    }
}

function addLogLine(text, type = '') {
    if (!progressLog) return;

    const logLine = document.createElement('div');
    logLine.className = `log-line ${type}`;
    logLine.textContent = text;
    progressLog.appendChild(logLine);
    
    // Прокрутка вниз
    progressLog.scrollTop = progressLog.scrollHeight;
}

export function openProgressModal() {
    if (!progressModalOverlay) return;
    
    isRefreshing = true;
    progressModalOverlay.classList.add('active');
    document.body.style.overflow = 'hidden';
    
    // Сбрасываем состояние
    if (closeProgressBtn) {
        closeProgressBtn.disabled = true;
    }
    
    // Очищаем лог
    if (progressLog) {
        progressLog.innerHTML = '<div class="log-line">Запуск процесса...</div>';
    }
    
    updateProgress('Инициализация...', 0, false);
}

export function closeProgressModal() {
    if (!progressModalOverlay || isRefreshing) return;
    
    progressModalOverlay.classList.remove('active');
    document.body.style.overflow = '';
}

export async function startRefreshProcess() {
    if (isRefreshing) {
        console.warn('Процесс уже запущен');
        return;
    }

    try {
        openProgressModal();
        
        // Запускаем скрипт refresh.sh
        const result = await invoke('run_refresh_script');
        
        console.log('Refresh script result:', result);
        
    } catch (error) {
        console.error('Ошибка при выполнении refresh скрипта:', error);
        updateProgress(`Ошибка: ${error}`, 1.0, true);
        addLogLine(`Ошибка: ${error}`, 'error');
        isRefreshing = false;
        if (closeProgressBtn) {
            closeProgressBtn.disabled = false;
        }
    }
}
