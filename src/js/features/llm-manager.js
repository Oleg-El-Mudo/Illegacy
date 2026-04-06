// Модуль управления локальными LLM моделями через Ollama

const { invoke } = window.__TAURI__.core;

// Состояние модуля
let ollamaStatus = {
    available: false,
    models: [],
    activeModel: null
};

let availableModels = [];

// Состояние скачивания моделей
let pullProgress = {}; // { modelName: { progress, status, message } }

// DOM элементы
let elements = {};

// Инициализация DOM элементов
function cacheElements() {
    elements = {
        llmToggle: document.getElementById('llm-toggle'),
        llmStatus: document.getElementById('llm-status'),
        llmModelsSection: document.getElementById('llm-models-section'),
        llmInstalledModelsSection: document.getElementById('llm-installed-models-section'),
        llmAvailableModelsSection: document.getElementById('llm-available-models-section'),
        llmActiveModel: document.getElementById('llm-active-model'),
        llmRefreshModelsBtn: document.getElementById('llm-refresh-models-btn'),
        llmInstalledModelsList: document.getElementById('llm-installed-models-list'),
        llmAvailableModelsList: document.getElementById('llm-available-models-list')
    };
}

// Инициализация модуля
export function initLLMManager() {
    console.log('Инициализация модуля управления LLM');
    
    cacheElements();
    setupEventListeners();
    restoreSavedSettings();
    checkOllamaStatus();
    
    // НЕ опрашиваем периодически - только при открытии настроек
}

// Настройка обработчиков событий
function setupEventListeners() {
    // Переключатель LLM
    if (elements.llmToggle) {
        elements.llmToggle.addEventListener('change', handleLLMToggle);
    }

    // Кнопка обновления моделей
    if (elements.llmRefreshModelsBtn) {
        elements.llmRefreshModelsBtn.addEventListener('click', refreshModels);
    }

    // Изменение активной модели
    if (elements.llmActiveModel) {
        elements.llmActiveModel.addEventListener('change', handleActiveModelChange);
    }

    // Подписка на события прогресса скачивания из Tauri
    setupPullProgressListener();
}

// Подписка на события прогресса скачивания
function setupPullProgressListener() {
    const { listen } = window.__TAURI__.event;
    
    listen('ollama-pull-progress', (event) => {
        const progress = event.payload;
        console.log('Прогресс скачивания:', progress);
        
        // Сохраняем прогресс
        pullProgress[progress.model] = progress;
        
        // Обновляем UI
        updatePullProgressUI(progress);
        
        // Если скачивание завершено или ошибка, обновляем список моделей
        if (progress.status === 'success' || progress.status === 'error') {
            setTimeout(() => {
                delete pullProgress[progress.model];
                loadModels();
            }, 2000);
        }
    });
}

// Обработка переключателя LLM
async function handleLLMToggle(event) {
    const enabled = event.target.checked;
    
    if (enabled) {
        // Проверяем статус Ollama
        const status = await checkOllamaStatus();
        
        if (!status.available) {
            event.target.checked = false;
            alert('Ollama сервис не доступен. Убедитесь, что Docker запущен и Ollama контейнер работает.');
            return;
        }
        
        // Показываем секции управления моделями
        showModelsSections();
        await loadModels();
    } else {
        // Скрываем секции управления моделями
        hideModelsSections();
    }
    
    // Сохраняем состояние
    localStorage.setItem('llm-enabled', enabled.toString());
}

// Показать секции управления моделями
function showModelsSections() {
    if (elements.llmModelsSection) {
        elements.llmModelsSection.style.display = 'block';
    }
    if (elements.llmInstalledModelsSection) {
        elements.llmInstalledModelsSection.style.display = 'block';
    }
    if (elements.llmAvailableModelsSection) {
        elements.llmAvailableModelsSection.style.display = 'block';
    }
}

// Скрыть секции управления моделями
function hideModelsSections() {
    if (elements.llmModelsSection) {
        elements.llmModelsSection.style.display = 'none';
    }
    if (elements.llmInstalledModelsSection) {
        elements.llmInstalledModelsSection.style.display = 'none';
    }
    if (elements.llmAvailableModelsSection) {
        elements.llmAvailableModelsSection.style.display = 'none';
    }
}

// Проверка статуса Ollama
export async function checkOllamaStatus() {
    try {
        const status = await invoke('check_ollama_status');
        
        ollamaStatus = {
            available: status.available,
            models: status.models || [],
            activeModel: ollamaStatus.activeModel
        };
        
        updateStatusUI();
        
        return ollamaStatus;
    } catch (error) {
        console.error('Ошибка проверки статуса Ollama:', error);
        
        ollamaStatus = {
            available: false,
            models: [],
            activeModel: null
        };
        
        updateStatusUI();
        
        return ollamaStatus;
    }
}

// Обновление UI статуса
function updateStatusUI() {
    if (!elements.llmStatus) return;
    
    // Обновляем статус в модальном окне настроек
    if (ollamaStatus.available) {
        elements.llmStatus.className = 'badge badge-success ml-2';
        elements.llmStatus.textContent = 'Готов';
        
        // Если переключатель включен, показываем модели
        if (elements.llmToggle && elements.llmToggle.checked) {
            showModelsSections();
        }
    } else {
        elements.llmStatus.className = 'badge badge-error ml-2';
        elements.llmStatus.textContent = 'Не доступен';
        
        // Отключаем переключатель если Ollama не доступен
        if (elements.llmToggle) {
            elements.llmToggle.checked = false;
            elements.llmToggle.disabled = true;
        }
        
        hideModelsSections();
    }
    
    // Обновляем статус в статус-баре
    updateStatusBarStatus();
}

// Обновление статуса в статус-баре
function updateStatusBarStatus() {
    const statusOllama = document.getElementById('status-ollama');
    if (!statusOllama) return;
    
    if (ollamaStatus.available) {
        const modelCount = ollamaStatus.models.length;
        const modelText = modelCount === 0 ? 'нет моделей' : 
                         modelCount === 1 ? '1 модель' : 
                         modelCount < 5 ? `${modelCount} модели` : `${modelCount} моделей`;
        
        statusOllama.innerHTML = `Ollama: <span style="color: var(--success)">Готов</span> (${modelText})`;
        statusOllama.className = 'ready';
    } else {
        statusOllama.innerHTML = 'Ollama: <span style="color: var(--error)">Не доступен</span>';
        statusOllama.className = 'error';
    }
}

// Загрузка моделей
export async function loadModels() {
    await Promise.all([
        loadInstalledModels(),
        loadAvailableModels()
    ]);
}

// Обновление моделей (по кнопке)
async function refreshModels() {
    if (elements.llmRefreshModelsBtn) {
        elements.llmRefreshModelsBtn.disabled = true;
        elements.llmRefreshModelsBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="spinning">
                <polyline points="23 4 23 10 17 10"></polyline>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
            </svg>
            <span>Обновление...</span>
        `;
    }
    
    try {
        await loadModels();
    } finally {
        if (elements.llmRefreshModelsBtn) {
            elements.llmRefreshModelsBtn.disabled = false;
            elements.llmRefreshModelsBtn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="23 4 23 10 17 10"></polyline>
                    <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
                </svg>
                <span>Обновить</span>
            `;
        }
    }
}

// Загрузка установленных моделей
async function loadInstalledModels() {
    try {
        const status = await invoke('check_ollama_status');
        ollamaStatus.models = status.models || [];
        
        renderInstalledModels();
        updateActiveModelSelect();
    } catch (error) {
        console.error('Ошибка загрузки установленных моделей:', error);
        showInstalledModelsError('Ошибка загрузки моделей');
    }
}

// Загрузка доступных моделей
async function loadAvailableModels() {
    try {
        availableModels = await invoke('get_available_models');
        renderAvailableModels();
    } catch (error) {
        console.error('Ошибка загрузки доступных моделей:', error);
        showAvailableModelsError('Ошибка загрузки доступных моделей');
    }
}

// Отрисовка установленных моделей
function renderInstalledModels() {
    if (!elements.llmInstalledModelsList) return;
    
    if (ollamaStatus.models.length === 0) {
        elements.llmInstalledModelsList.innerHTML = `
            <div class="llm-empty-state">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="7 10 12 15 17 10"></polyline>
                    <line x1="12" y1="15" x2="12" y2="3"></line>
                </svg>
                <div>Нет установленных моделей</div>
                <div style="font-size: 12px; margin-top: 8px;">Выберите модель из списка доступных ниже</div>
            </div>
        `;
        return;
    }
    
    elements.llmInstalledModelsList.innerHTML = ollamaStatus.models.map(model => {
        const sizeGB = (model.size / (1024 * 1024 * 1024)).toFixed(2);
        const isActive = model.name === ollamaStatus.activeModel;
        
        return `
            <div class="llm-model-item ${isActive ? 'active' : ''}">
                <div class="llm-model-info">
                    <span class="llm-model-name">${model.name}</span>
                    <span class="llm-model-size">${sizeGB} GB • ${new Date(model.modified_at).toLocaleDateString('ru-RU')}</span>
                </div>
                <div class="llm-model-actions">
                    ${!isActive ? `
                        <button class="jb-button jb-button-tertiary jb-button-sm" onclick="window.llmManager.setActiveModel('${model.name}')">
                            Выбрать
                        </button>
                    ` : `
                        <span class="badge badge-success">Активна</span>
                    `}
                    <button class="jb-button jb-button-tertiary jb-button-sm jb-button-danger" onclick="window.llmManager.deleteModel('${model.name}')" title="Удалить модель">
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="3 6 5 6 21 6"></polyline>
                            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                        </svg>
                    </button>
                </div>
            </div>
        `;
    }).join('');
}

// Обновление селектора активной модели
function updateActiveModelSelect() {
    if (!elements.llmActiveModel) return;
    
    elements.llmActiveModel.innerHTML = `
        <option value="">Не выбрано</option>
        ${ollamaStatus.models.map(model => `
            <option value="${model.name}" ${model.name === ollamaStatus.activeModel ? 'selected' : ''}>
                ${model.name}
            </option>
        `).join('')}
    `;
}

// Отрисовка доступных моделей
function renderAvailableModels() {
    if (!elements.llmAvailableModelsList) return;
    
    if (availableModels.length === 0) {
        elements.llmAvailableModelsList.innerHTML = `
            <div class="llm-empty-state">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                </svg>
                <div>Нет доступных моделей</div>
            </div>
        `;
        return;
    }
    
    elements.llmAvailableModelsList.innerHTML = availableModels.map(model => {
        const isInstalled = ollamaStatus.models.some(m => m.name === model.name);
        const isPulling = pullProgress[model.name] && pullProgress[model.name].status !== 'success' && pullProgress[model.name].status !== 'error';
        const pullProg = pullProgress[model.name];
        
        return `
            <div class="llm-available-model-item" id="model-item-${model.name.replace(/[^a-z0-9]/gi, '-')}">
                <div class="llm-available-model-info">
                    <span class="llm-available-model-name">${model.name}</span>
                    <span class="llm-available-model-description">${model.description}</span>
                    <div class="llm-available-model-meta">
                        <span class="llm-available-model-category">${model.category}</span>
                        <span class="llm-available-model-size">${model.size}</span>
                    </div>
                    ${isPulling && pullProg ? `
                        <div class="llm-pull-progress">
                            <div class="llm-pull-progress-header">
                                <span class="llm-pull-status ${pullProg.status}">${getPullStatusText(pullProg.status)}</span>
                                <span class="llm-pull-percent">${(pullProg.progress * 100).toFixed(1)}%</span>
                            </div>
                            <div class="llm-pull-progress-bar">
                                <div class="llm-pull-progress-fill" style="width: ${pullProg.progress * 100}%"></div>
                            </div>
                            <div class="llm-pull-message">${pullProg.message}</div>
                        </div>
                    ` : ''}
                </div>
                <div class="llm-available-model-actions">
                    ${isInstalled ? `
                        <span class="badge badge-success">Установлена</span>
                    ` : isPulling ? `
                        <button class="jb-button jb-button-tertiary jb-button-sm" disabled>
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="spinning">
                                <path d="M21 12a9 9 0 11-6.219-8.56"></path>
                            </svg>
                            <span>Скачивание...</span>
                        </button>
                    ` : `
                        <button class="jb-button jb-button-primary jb-button-sm" onclick="window.llmManager.pullModel('${model.name}')">
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                                <polyline points="7 10 12 15 17 10"></polyline>
                                <line x1="12" y1="15" x2="12" y2="3"></line>
                            </svg>
                            <span>Скачать</span>
                        </button>
                    `}
                </div>
            </div>
        `;
    }).join('');
}

// Получение текста статуса скачивания
function getPullStatusText(status) {
    switch (status) {
        case 'starting': return 'Подготовка...';
        case 'downloading': return 'Скачивание...';
        case 'verifying': return 'Проверка...';
        case 'success': return 'Завершено';
        case 'error': return 'Ошибка';
        default: return status;
    }
}

// Обновление UI прогресса скачивания
function updatePullProgressUI(progress) {
    const modelName = progress.model;
    const modelItemId = `model-item-${modelName.replace(/[^a-z0-9]/gi, '-')}`;
    const modelItem = document.getElementById(modelItemId);
    
    if (!modelItem) {
        // Если элемент не найден, перерисовываем весь список
        renderAvailableModels();
        return;
    }
    
    // Находим контейнер прогресса или создаём новый
    let progressContainer = modelItem.querySelector('.llm-pull-progress');
    
    if (!progressContainer) {
        // Создаём контейнер прогресса
        const info = modelItem.querySelector('.llm-available-model-info');
        const meta = info.querySelector('.llm-available-model-meta');
        progressContainer = document.createElement('div');
        progressContainer.className = 'llm-pull-progress';
        meta.after(progressContainer);
    }
    
    // Обновляем содержимое
    progressContainer.innerHTML = `
        <div class="llm-pull-progress-header">
            <span class="llm-pull-status ${progress.status}">${getPullStatusText(progress.status)}</span>
            <span class="llm-pull-percent">${(progress.progress * 100).toFixed(1)}%</span>
        </div>
        <div class="llm-pull-progress-bar">
            <div class="llm-pull-progress-fill" style="width: ${progress.progress * 100}%"></div>
        </div>
        <div class="llm-pull-message">${progress.message}</div>
    `;
    
    // Обновляем кнопку
    const actions = modelItem.querySelector('.llm-available-model-actions');
    if (actions && progress.status !== 'success' && progress.status !== 'error') {
        actions.innerHTML = `
            <button class="jb-button jb-button-tertiary jb-button-sm" disabled>
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="spinning">
                    <path d="M21 12a9 9 0 11-6.219-8.56"></path>
                </svg>
                <span>Скачивание...</span>
            </button>
        `;
    } else if (actions && progress.status === 'success') {
        actions.innerHTML = `
            <span class="badge badge-success">Установлена</span>
        `;
    } else if (actions && progress.status === 'error') {
        actions.innerHTML = `
            <button class="jb-button jb-button-primary jb-button-sm" onclick="window.llmManager.pullModel('${modelName}')">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                    <polyline points="7 10 12 15 17 10"></polyline>
                    <line x1="12" y1="15" x2="12" y2="3"></line>
                </svg>
                <span>Повторить</span>
            </button>
        `;
    }
}

// Обработка изменения активной модели
async function handleActiveModelChange(event) {
    const modelName = event.target.value;
    
    if (modelName) {
        await setActiveModel(modelName);
    }
}

// Установка активной модели
export async function setActiveModel(modelName) {
    try {
        const result = await invoke('set_active_ollama_model', { modelName });
        
        if (result.success) {
            ollamaStatus.activeModel = modelName;
            
            // Обновляем UI
            if (elements.llmActiveModel) {
                elements.llmActiveModel.value = modelName;
            }
            
            renderInstalledModels();
            
            // Сохраняем выбор
            localStorage.setItem('llm-active-model', modelName);
        } else {
            alert('Ошибка выбора модели: ' + (result.error || 'Неизвестная ошибка'));
        }
    } catch (error) {
        console.error('Ошибка установки активной модели:', error);
        alert('Ошибка установки активной модели: ' + error);
    }
}

// Скачивание модели
export async function pullModel(modelName) {
    try {
        // Инициализируем прогресс
        pullProgress[modelName] = {
            model: modelName,
            status: 'starting',
            progress: 0.0,
            message: 'Подготовка к скачиванию...'
        };
        
        // Обновляем UI сразу
        updatePullProgressUI(pullProgress[modelName]);
        
        const result = await invoke('pull_ollama_model', { modelName });
        
        if (result.success) {
            // Успех уже обработан через event
            console.log(result.message);
        } else {
            alert('Ошибка скачивания модели: ' + (result.error || 'Неизвестная ошибка'));
            delete pullProgress[modelName];
            updatePullProgressUI({ model: modelName, status: 'error', progress: 0, message: result.error });
        }
    } catch (error) {
        console.error('Ошибка скачивания модели:', error);
        delete pullProgress[modelName];
        updatePullProgressUI({ model: modelName, status: 'error', progress: 0, message: error });
    }
}

// Удаление модели
export async function deleteModel(modelName) {
    if (!confirm(`Удалить модель ${modelName}? Это действие нельзя отменить.`)) {
        return;
    }
    
    try {
        const result = await invoke('delete_ollama_model', { modelName });
        
        if (result.success) {
            // Если удалили активную модель, сбрасываем выбор
            if (modelName === ollamaStatus.activeModel) {
                ollamaStatus.activeModel = null;
                localStorage.removeItem('llm-active-model');
            }
            
            // Обновляем список моделей
            await loadModels();
        } else {
            alert('Ошибка удаления модели: ' + (result.error || 'Неизвестная ошибка'));
        }
    } catch (error) {
        console.error('Ошибка удаления модели:', error);
        alert('Ошибка удаления модели: ' + error);
    }
}

// Показать прогресс скачивания
function showPullProgress(modelName) {
    // Можно использовать существующий модальный окно прогресса
    console.log(`Скачивание модели: ${modelName}`);
}

// Скрыть прогресс скачивания
function hidePullProgress() {
    console.log('Скачивание завершено');
}

// Показать ошибку установленных моделей
function showInstalledModelsError(message) {
    if (elements.llmInstalledModelsList) {
        elements.llmInstalledModelsList.innerHTML = `
            <div class="llm-empty-state">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                </svg>
                <div>${message}</div>
            </div>
        `;
    }
}

// Показать ошибку доступных моделей
function showAvailableModelsError(message) {
    if (elements.llmAvailableModelsList) {
        elements.llmAvailableModelsList.innerHTML = `
            <div class="llm-empty-state">
                <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                </svg>
                <div>${message}</div>
            </div>
        `;
    }
}

// Восстановление сохраненных настроек
function restoreSavedSettings() {
    const llmEnabled = localStorage.getItem('llm-enabled') === 'true';
    const activeModel = localStorage.getItem('llm-active-model');
    
    if (llmEnabled && elements.llmToggle) {
        elements.llmToggle.checked = true;
        showModelsSections();
    }
    
    if (activeModel) {
        ollamaStatus.activeModel = activeModel;
    }
}

// Экспортируем публичные функции
export {
    ollamaStatus,
    availableModels
};

// Делаем функции доступными глобально для onclick обработчиков
if (typeof window !== 'undefined') {
    window.llmManager = {
        setActiveModel,
        pullModel,
        deleteModel,
        checkOllamaStatus,
        loadModels,
        refreshModels
    };
}
