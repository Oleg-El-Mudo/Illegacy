// Модуль ИИ-оптимизации кода через Ollama

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Состояние модуля
let optimizerState = {
    isOptimizing: false,
    originalCode: null, // Код от генератора (для отмены)
    optimizedCode: null, // Оптимизированный код
    abortController: null // Для отмены запроса
};

// DOM элементы
let elements = {};

// Инициализация DOM элементов
function cacheElements() {
    elements = {
        aiOptimizeBtn: document.getElementById('aiOptimizeBtn'),
        outputCode: document.getElementById('outputCode'),
        outputLangSelect: document.getElementById('output-lang-select'),
        translateBtn: document.getElementById('translateBtn'),
        statusAIOptimizer: document.getElementById('status-ai-optimizer'),
        statusAIOptimizerText: document.getElementById('status-ai-optimizer-text')
    };
}

// Инициализация модуля
export function initAIOptimizer() {
    console.log('Инициализация модуля ИИ-оптимизации');

    cacheElements();
    setupEventListeners();
    setupOllamaStatusListener();
    
    // Проверяем начальное состояние
    updateOptimizeButtonState();
}

// Настройка обработчиков событий
function setupEventListeners() {
    // Кнопка оптимизации
    if (elements.aiOptimizeBtn) {
        elements.aiOptimizeBtn.addEventListener('click', handleOptimizeClick);
    }

    // Обновляем состояние кнопки при изменении кода или языка
    if (elements.outputCode) {
        elements.outputCode.addEventListener('input', handleOutputCodeChange);
    }

    if (elements.outputLangSelect) {
        elements.outputLangSelect.addEventListener('change', handleOutputLangChange);
    }
}

// Подписка на события статуса Ollama
function setupOllamaStatusListener() {
    const { listen } = window.__TAURI__.event;

    // Слушаем события изменения статуса Ollama из llm-manager
    // Используем кастомное событие для обновления состояния
    window.addEventListener('ollama-status-changed', (event) => {
        const { available, activeModel } = event.detail;
        console.log('Статус Ollama изменён:', { available, activeModel });
        updateOptimizeButtonState();
    });
}

// Обработка изменения выходного кода
function handleOutputCodeChange() {
    updateOptimizeButtonState();
}

// Обработка изменения выходного языка
function handleOutputLangChange() {
    updateOptimizeButtonState();
}

// Обновление состояния кнопки оптимизации
function updateOptimizeButtonState() {
    if (!elements.aiOptimizeBtn) return;

    const hasOutputCode = elements.outputCode && elements.outputCode.value.trim().length > 0;
    const hasOllama = window.llmManager && 
                      window.llmManager.ollamaStatus && 
                      window.llmManager.ollamaStatus.available &&
                      window.llmManager.ollamaStatus.activeModel;

    // Показываем статус-бар ИИ-оптимизатора если Ollama доступен
    if (elements.statusAIOptimizer) {
        if (hasOllama) {
            elements.statusAIOptimizer.style.display = 'flex';
        } else {
            elements.statusAIOptimizer.style.display = 'none';
        }
    }

    // Кнопка активна только если:
    // 1. Есть переведенный код
    // 2. Ollama доступен и выбрана модель
    // 3. Не идет процесс оптимизации
    const shouldEnable = hasOutputCode && hasOllama && !optimizerState.isOptimizing;

    elements.aiOptimizeBtn.disabled = !shouldEnable;

    // Обновляем текст кнопки в зависимости от состояния
    if (optimizerState.isOptimizing) {
        elements.aiOptimizeBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="spinning">
                <path d="M21 12a9 9 0 11-6.219-8.56"></path>
            </svg>
            <span>Оптимизация...</span>
        `;
        
        // Обновляем статус-бар
        if (elements.statusAIOptimizerText) {
            elements.statusAIOptimizerText.textContent = 'ИИ: Оптимизация...';
        }
    } else if (optimizerState.optimizedCode) {
        // Если уже оптимизировано, показываем возможность отмены
        elements.aiOptimizeBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="1 4 1 10 7 10"></polyline>
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
            </svg>
            <span>Отменить оптимизацию</span>
        `;
        
        // Обновляем статус-бар
        if (elements.statusAIOptimizerText) {
            elements.statusAIOptimizerText.textContent = 'ИИ: Оптимизировано';
        }
    } else {
        elements.aiOptimizeBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2a10 10 0 0 1 10 10 10 10 0 0 1-10 10A10 10 0 0 1 2 12 10 10 0 0 1 12 2z"></path>
                <path d="M12 8v4l3 3"></path>
            </svg>
            <span>ИИ-оптимизация</span>
        `;
        
        // Обновляем статус-бар
        if (elements.statusAIOptimizerText && hasOllama) {
            elements.statusAIOptimizerText.textContent = 'ИИ: Готов';
        }
    }
}

// Обработка клика по кнопке оптимизации
async function handleOptimizeClick() {
    // Если уже идет оптимизация - игнорируем
    if (optimizerState.isOptimizing) return;

    // Если есть оптимизированный код - отменяем оптимизацию
    if (optimizerState.optimizedCode) {
        handleCancelOptimization();
        return;
    }

    // Запускаем оптимизацию
    await startOptimization();
}

// Запуск оптимизации
async function startOptimization() {
    if (!elements.outputCode || !elements.outputLangSelect) {
        alert('Ошибка: не найден код или язык');
        return;
    }

    const outputCode = elements.outputCode.value;
    const outputLang = elements.outputLangSelect.value;

    if (!outputCode.trim()) {
        alert('Нет кода для оптимизации');
        return;
    }

    // Сохраняем оригинальный код для возможности отмены
    optimizerState.originalCode = outputCode;
    optimizerState.optimizedCode = null;
    optimizerState.isOptimizing = true;

    // Обновляем UI
    updateOptimizeButtonState();
    showOptimizationIndicator();

    try {
        console.log('=== Запуск ИИ-оптимизации ===');
        console.log(`Язык: ${outputLang}`);
        console.log(`Длина кода: ${outputCode.length} символов`);

        // Вызываем Rust команду для оптимизации через Ollama
        const result = await invoke('optimize_code_with_ollama', {
            code: outputCode,
            targetLang: outputLang
        });

        if (result.success && result.optimized_code) {
            console.log('Оптимизация успешна');
            
            // Сохраняем оптимизированный код
            optimizerState.optimizedCode = result.optimized_code;
            
            // Обновляем textarea оптимизированным кодом
            elements.outputCode.value = result.optimized_code;
            
            // Обновляем подсветку синтаксиса
            import('../editor/syntax-highlight.js').then(({ updateSyntaxHighlighting }) => {
                updateSyntaxHighlighting();
            });

            // Показываем уведомление об успехе
            showOptimizationSuccess(result);
        } else {
            console.error('Ошибка оптимизации:', result.error);
            alert('Ошибка оптимизации: ' + (result.error || 'Неизвестная ошибка'));
            
            // Возвращаем оригинальный код при ошибке
            if (optimizerState.originalCode) {
                elements.outputCode.value = optimizerState.originalCode;
            }
        }
    } catch (error) {
        console.error('Ошибка при оптимизации:', error);
        alert('Ошибка при оптимизации: ' + error);
        
        // Возвращаем оригинальный код при ошибке
        if (optimizerState.originalCode) {
            elements.outputCode.value = optimizerState.originalCode;
        }
    } finally {
        // Сбрасываем состояние
        optimizerState.isOptimizing = false;
        
        // Обновляем UI
        updateOptimizeButtonState();
        hideOptimizationIndicator();
    }
}

// Отмена оптимизации
function handleCancelOptimization() {
    if (!optimizerState.originalCode) {
        console.warn('Нет оригинального кода для возврата');
        return;
    }

    console.log('Отмена оптимизации - возврат к оригинальному переводу');
    
    // Возвращаем оригинальный код
    elements.outputCode.value = optimizerState.originalCode;
    
    // Очищаем состояние
    optimizerState.originalCode = null;
    optimizerState.optimizedCode = null;

    // Обновляем подсветку синтаксиса
    import('../editor/syntax-highlight.js').then(({ updateSyntaxHighlighting }) => {
        updateSyntaxHighlighting();
    });

    // Обновляем статус-бар
    if (elements.statusAIOptimizerText) {
        elements.statusAIOptimizerText.textContent = 'ИИ: Отменено';
        
        // Возвращаем статус "Готов" через 2 секунды
        setTimeout(() => {
            if (elements.statusAIOptimizerText) {
                elements.statusAIOptimizerText.textContent = 'ИИ: Готов';
            }
        }, 2000);
    }

    // Обновляем UI
    updateOptimizeButtonState();

    console.log('Оптимизация отменена, восстановлен оригинальный перевод');
}

// Показать индикатор оптимизации
function showOptimizationIndicator() {
    // Добавляем визуальный индикатор в статус-бар
    const statusOllama = document.getElementById('status-ollama');
    if (statusOllama) {
        const originalHTML = statusOllama.innerHTML;
        statusOllama.innerHTML = 'ИИ-оптимизация: <span style="color: var(--warning)">В процессе...</span>';
        statusOllama.dataset.original = originalHTML;
    }

    // Добавляем класс загрузки кнопке
    if (elements.aiOptimizeBtn) {
        elements.aiOptimizeBtn.classList.add('loading');
    }
}

// Скрыть индикатор оптимизации
function hideOptimizationIndicator() {
    // Восстанавливаем статус-бар
    const statusOllama = document.getElementById('status-ollama');
    if (statusOllama && statusOllama.dataset.original) {
        statusOllama.innerHTML = statusOllama.dataset.original;
        delete statusOllama.dataset.original;
    }

    // Убираем класс загрузки
    if (elements.aiOptimizeBtn) {
        elements.aiOptimizeBtn.classList.remove('loading');
    }
}

// Показать уведомление об успехе
function showOptimizationSuccess(result) {
    console.log('ИИ-оптимизация завершена успешно');
    console.log(`Удалено конструкций: ${result.removed_patterns || 0}`);
    console.log(`Оптимизировано строк: ${result.optimized_lines || 0}`);

    // Обновляем статус-бар
    if (elements.statusAIOptimizerText) {
        elements.statusAIOptimizerText.textContent = `ИИ: Оптимизировано (-${result.removed_patterns || 0} строк)`;
        
        // Возвращаем статус "Готов" через 5 секунд
        setTimeout(() => {
            if (elements.statusAIOptimizerText) {
                elements.statusAIOptimizerText.textContent = 'ИИ: Готов';
            }
        }, 5000);
    }
}

// Получение текущего состояния
export function getOptimizerState() {
    return { ...optimizerState };
}

// Проверка, активна ли оптимизация
export function isOptimizing() {
    return optimizerState.isOptimizing;
}

// Проверка, есть ли оптимизированный код
export function hasOptimizedCode() {
    return optimizerState.optimizedCode !== null;
}

// Экспорт для глобального доступа
if (typeof window !== 'undefined') {
    window.aiOptimizer = {
        initAIOptimizer,
        getOptimizerState,
        isOptimizing,
        hasOptimizedCode,
        handleCancelOptimization,
        updateOptimizeButtonState
    };
}
