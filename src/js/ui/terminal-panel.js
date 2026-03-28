import { elements } from '../core/dom.js';

const TERMINAL_HEIGHT_KEY = 'illegacy-terminal-height';
const TERMINAL_HIDDEN_KEY = 'illegacy-terminal-hidden';
const TERMINAL_SPLIT_KEY = 'illegacy-terminal-split';
const DEFAULT_HEIGHT = 200;
const MIN_HEIGHT = 100;
const HIDDEN_TRIGGER = 50;
const DEFAULT_SPLIT = 50; // Процент для левой панели
const MIN_SPLIT = 20;
const MAX_SPLIT = 80;

let isResizing = false;
let isSplitResizing = false;
let startY = 0;
let startHeight = 0;
let startX = 0;
let startSplitPercent = 0;
let terminalPanel = null;
let terminalResizer = null;
let terminalSplitResizer = null;
let inputTerminalSection = null;
let outputTerminalSection = null;

export function initTerminalPanel() {
    terminalPanel = document.getElementById('terminalPanel');
    terminalResizer = document.getElementById('terminal-resizer');
    terminalSplitResizer = document.getElementById('terminal-split-resizer');
    inputTerminalSection = document.getElementById('inputTerminalSection');
    outputTerminalSection = document.getElementById('outputTerminalSection');

    if (!terminalPanel || !terminalResizer) {
        console.warn('Terminal panel elements not found');
        return;
    }

    // Восстанавливаем сохраненную высоту и состояние
    restoreTerminalState();

    // Инициализация изменения размера
    initResizer();
    initSplitResizer();

    // Кнопки управления
    initControls();

    // Горячая клавиша Ctrl+` для показа/скрытия терминала
    initKeyboardShortcut();

    console.log('Terminal panel initialized');
}

function initKeyboardShortcut() {
    document.addEventListener('keydown', function(e) {
        // Ctrl+` (backquote) или Ctrl+~ для показа/скрытия терминала
        if ((e.ctrlKey || e.metaKey) && e.key === '`') {
            e.preventDefault();
            toggleTerminal();
        }
    });
}

function restoreTerminalState() {
    const savedHeight = localStorage.getItem(TERMINAL_HEIGHT_KEY);
    const isHidden = localStorage.getItem(TERMINAL_HIDDEN_KEY) === 'true';
    const savedSplit = localStorage.getItem(TERMINAL_SPLIT_KEY);

    if (isHidden) {
        terminalPanel.classList.add('hidden');
        terminalResizer.style.display = 'none';
    } else if (savedHeight) {
        const height = parseInt(savedHeight, 10);
        if (height >= MIN_HEIGHT) {
            terminalPanel.style.height = `${height}px`;
        }
    } else {
        terminalPanel.style.height = `${DEFAULT_HEIGHT}px`;
    }

    // Восстанавливаем разделение панелей
    if (savedSplit && terminalSplitResizer) {
        const splitPercent = parseFloat(savedSplit);
        if (splitPercent >= MIN_SPLIT && splitPercent <= MAX_SPLIT) {
            inputTerminalSection.style.flex = `${splitPercent}`;
            outputTerminalSection.style.flex = `${100 - splitPercent}`;
        }
    }
}

function saveTerminalState() {
    const height = terminalPanel.offsetHeight;
    localStorage.setItem(TERMINAL_HEIGHT_KEY, height.toString());
    localStorage.setItem(TERMINAL_HIDDEN_KEY, terminalPanel.classList.contains('hidden').toString());

    // Сохраняем процент разделения
    if (inputTerminalSection && outputTerminalSection) {
        const totalFlex = parseFloat(inputTerminalSection.style.flex) || 1;
        const percent = (totalFlex / (totalFlex + (parseFloat(outputTerminalSection.style.flex) || 1))) * 100;
        localStorage.setItem(TERMINAL_SPLIT_KEY, percent.toFixed(1));
    }
}

function initResizer() {
    terminalResizer.addEventListener('mousedown', startResize);
    document.addEventListener('mousemove', resize);
    document.addEventListener('mouseup', stopResize);

    // Показ терминала при наведении на скрытый разделитель
    terminalResizer.addEventListener('mouseenter', showOnHover);
}

function initSplitResizer() {
    if (!terminalSplitResizer) return;

    terminalSplitResizer.addEventListener('mousedown', startSplitResize);
}

function startResize(e) {
    if (terminalPanel.classList.contains('hidden')) return;

    isResizing = true;
    startY = e.clientY;
    startHeight = terminalPanel.offsetHeight;

    terminalResizer.classList.add('resizing');
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';

    e.preventDefault();
}

function resize(e) {
    if (!isResizing) return;

    const deltaY = startY - e.clientY;
    const newHeight = startHeight + deltaY;

    // Ограничиваем минимальной и максимальной высотой
    const maxHeight = window.innerHeight - 300; // Минимум 300px для редактора
    const clampedHeight = Math.max(MIN_HEIGHT, Math.min(newHeight, maxHeight));

    terminalPanel.style.height = `${clampedHeight}px`;
}

function stopResize() {
    if (!isResizing) return;

    isResizing = false;
    terminalResizer.classList.remove('resizing');
    document.body.style.cursor = '';
    document.body.style.userSelect = '';

    saveTerminalState();
}

function startSplitResize(e) {
    isSplitResizing = true;
    startX = e.clientX;

    // Получаем текущую ширину панелей
    const inputRect = inputTerminalSection.getBoundingClientRect();
    const totalWidth = inputRect.width + outputTerminalSection.getBoundingClientRect().width;

    startSplitPercent = (inputRect.width / totalWidth) * 100;

    terminalSplitResizer.classList.add('resizing');
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    e.preventDefault();
}

function splitResize(e) {
    if (!isSplitResizing) return;

    const deltaX = e.clientX - startX;
    const terminalRect = terminalPanel.getBoundingClientRect();
    const deltaPercent = (deltaX / terminalRect.width) * 100;

    let newPercent = startSplitPercent + deltaPercent;
    newPercent = Math.max(MIN_SPLIT, Math.min(newPercent, MAX_SPLIT));

    inputTerminalSection.style.flex = `${newPercent}`;
    outputTerminalSection.style.flex = `${100 - newPercent}`;
}

function stopSplitResize() {
    if (!isSplitResizing) return;

    isSplitResizing = false;
    terminalSplitResizer.classList.remove('resizing');
    document.body.style.cursor = '';
    document.body.style.userSelect = '';

    saveTerminalState();
}

function showOnHover() {
    if (terminalPanel.classList.contains('hidden')) {
        // Показываем терминал при наведении на разделитель
        terminalPanel.classList.remove('hidden');
        terminalResizer.style.display = 'block';
        terminalPanel.style.height = `${HIDDEN_TRIGGER * 2}px`;
        saveTerminalState();
    }
}

function initControls() {
    // Кнопка скрытия/показа
    const toggleBtn = document.getElementById('toggleTerminal');
    if (toggleBtn) {
        toggleBtn.addEventListener('click', toggleTerminal);
    }

    // Кнопка очистки
    const clearBtn = document.getElementById('clearTerminal');
    if (clearBtn) {
        clearBtn.addEventListener('click', clearTerminal);
    }

    // Клик по заголовку терминала для раскрытия
    const terminalHeader = document.querySelector('.terminal-header');
    if (terminalHeader) {
        terminalHeader.addEventListener('dblclick', function() {
            if (terminalPanel.classList.contains('hidden')) {
                showTerminal();
            }
        });
    }

    // Наведение на скрытый терминал для подсказки
    terminalPanel.addEventListener('mouseenter', function() {
        if (terminalPanel.classList.contains('hidden')) {
            terminalPanel.style.cursor = 'ns-resize';
        }
    });

    terminalPanel.addEventListener('mouseleave', function() {
        terminalPanel.style.cursor = '';
    });

    // Клик по скрытому терминалу для раскрытия
    terminalPanel.addEventListener('click', function(e) {
        if (terminalPanel.classList.contains('hidden') && e.target.closest('.terminal-header')) {
            showTerminal();
        }
    });
}

function toggleTerminal() {
    const isHidden = terminalPanel.classList.toggle('hidden');

    if (isHidden) {
        // Скрываем терминал - оставляем только шапку
        terminalResizer.style.display = 'none';
    } else {
        // Показываем терминал
        showTerminal();
    }

    saveTerminalState();
}

function showTerminal() {
    terminalPanel.classList.remove('hidden');
    terminalResizer.style.display = 'block';
    
    // Восстанавливаем последнюю высоту
    const savedHeight = localStorage.getItem(TERMINAL_HEIGHT_KEY);
    if (savedHeight) {
        terminalPanel.style.height = savedHeight;
    } else {
        terminalPanel.style.height = `${DEFAULT_HEIGHT}px`;
    }
}

function clearTerminal() {
    const inputOutput = document.getElementById('inputTerminalOutput');
    const outputOutput = document.getElementById('outputTerminalOutput');

    if (inputOutput) {
        inputOutput.innerHTML = '<div class="terminal-line text-secondary">Терминал входного кода очищен</div>';
    }
    if (outputOutput) {
        outputOutput.innerHTML = '<div class="terminal-line text-secondary">Терминал выходного кода очищен</div>';
    }
}

// Публичные методы для записи в терминал
export function writeToTerminal(text, type = 'info', side = 'input') {
    const outputId = side === 'input' ? 'inputTerminalOutput' : 'outputTerminalOutput';
    const output = document.getElementById(outputId);
    if (!output) return;

    const line = document.createElement('div');
    line.className = `terminal-line terminal-${type}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${text}`;

    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

export function writeToInputTerminal(text, type = 'info') {
    writeToTerminal(text, type, 'input');
}

export function writeToOutputTerminal(text, type = 'info') {
    writeToTerminal(text, type, 'output');
}

export function clearTerminalOutput() {
    clearTerminal();
}

// Экспорт обработчика для глобального события
document.addEventListener('mousemove', splitResize);
document.addEventListener('mouseup', stopSplitResize);
