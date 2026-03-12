import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

// Константы
const MAX_HISTORY_SIZE = 100; // Максимальное количество шагов истории

// Состояние истории
let history = [];
let currentIndex = -1;
let isUndoRedoOperation = false;
let lastFileLoadValue = null; // Для отслеживания загрузки файла

// Типы действий
const ActionType = {
    EDIT: 'edit',
    FILE_LOAD: 'file_load',
    CLEAR: 'clear',
    TRANSLATE: 'translate'
};

// Сохранение состояния в историю
export function pushState(value, type = ActionType.EDIT, metadata = {}) {
    // Не сохраняем если это операция undo/redo
    if (isUndoRedoOperation) return;
    
    // Удаляем все состояния после текущего индекса
    if (currentIndex < history.length - 1) {
        history = history.slice(0, currentIndex + 1);
    }
    
    // Создаем новое состояние
    const state = {
        value,
        type,
        timestamp: Date.now(),
        metadata
    };
    
    // Добавляем в историю
    history.push(state);
    
    // Ограничиваем размер истории
    if (history.length > MAX_HISTORY_SIZE) {
        history.shift();
    } else {
        currentIndex = history.length - 1;
    }
    
    // Обновляем состояние кнопок отмены (если добавим)
    updateUndoButtons();
    
    console.log(`History: pushed ${type} state, index: ${currentIndex}, size: ${history.length}`);
}

// Отмена действия (Ctrl+Z)
export function undo() {
    if (currentIndex > 0) {
        isUndoRedoOperation = true;
        
        // Переходим к предыдущему состоянию
        currentIndex--;
        const state = history[currentIndex];
        
        // Применяем состояние
        applyState(state);
        
        isUndoRedoOperation = false;
        
        console.log(`History: undo to index ${currentIndex}`);
        return true;
    } else if (history.length > 0 && currentIndex === 0) {
        // Если мы на первом элементе, очищаем поле
        isUndoRedoOperation = true;
        
        // Временно отключаем сохранение в историю
        elements.inputCode.value = '';
        updateSyntaxHighlighting();
        
        isUndoRedoOperation = false;
        
        console.log('History: undo to empty');
        return true;
    }
    
    return false;
}

// Повтор действия (Ctrl+Shift+Z или Ctrl+Y)
export function redo() {
    if (currentIndex < history.length - 1) {
        isUndoRedoOperation = true;
        
        // Переходим к следующему состоянию
        currentIndex++;
        const state = history[currentIndex];
        
        // Применяем состояние
        applyState(state);
        
        isUndoRedoOperation = false;
        
        console.log(`History: redo to index ${currentIndex}`);
        return true;
    }
    
    return false;
}

// Применение состояния к полю ввода
function applyState(state) {
    elements.inputCode.value = state.value;
    
    // Если это была загрузка файла, запоминаем для особой обработки Ctrl+Z
    if (state.type === ActionType.FILE_LOAD) {
        lastFileLoadValue = state.value;
    }
    
    updateSyntaxHighlighting();
}

// Очистка истории 
export function clearHistory() {
    history = [];
    currentIndex = -1;
    lastFileLoadValue = null;
    console.log('History cleared');
}

// Получение текущего состояния
export function getCurrentState() {
    return history[currentIndex] || null;
}

// Обновление кнопок отмены 
function updateUndoButtons() {
    const canUndo = currentIndex > 0 || (history.length > 0 && currentIndex === 0);
    const canRedo = currentIndex < history.length - 1;
    
}

// Специальная обработка для Ctrl+Z после загрузки файла
export function handleUndoAfterFileLoad() {
    if (lastFileLoadValue !== null && elements.inputCode.value === lastFileLoadValue) {
        // Если текущее значение равно последнему загруженному файлу,
        // очищаем поле при первом Ctrl+Z
        elements.inputCode.value = '';
        updateSyntaxHighlighting();
        
        // Добавляем очистку в историю
        pushState('', ActionType.CLEAR);
        
        lastFileLoadValue = null;
        return true;
    }
    return false;
}

// Инициализация истории
export function initHistory() {
    // Сохраняем начальное состояние
    pushState(elements.inputCode.value, ActionType.CLEAR);
    
    // Отслеживаем изменения в поле ввода
    elements.inputCode.addEventListener('input', () => {
        pushState(elements.inputCode.value, ActionType.EDIT);
    });
    
    // Отслеживаем загрузку файлов (будет вызываться из file-handlers.js)
    document.addEventListener('file-loaded', (e) => {
        pushState(e.detail.content, ActionType.FILE_LOAD, { filename: e.detail.filename });
        lastFileLoadValue = e.detail.content;
    });
    
    // Отслеживаем очистку поля
    document.addEventListener('input-cleared', () => {
        pushState('', ActionType.CLEAR);
    });
    
    // Отслеживаем перевод
    document.addEventListener('translation-complete', (e) => {
        pushState(e.detail.output, ActionType.TRANSLATE, { from: e.detail.from, to: e.detail.to });
    });
    
    console.log('History module initialized');
}