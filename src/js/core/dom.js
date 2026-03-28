// Получение ссылок на DOM-элементы
export function getDOMElements() {
    const elements = {
        // Resizer
        resizer: document.getElementById('resizer'),

        // Editor panels
        leftPanel: document.querySelector('.editor-panel:first-child'),
        rightPanel: document.querySelector('.editor-panel:last-child'),
        mainContent: document.querySelector('.main-content'),

        // Buttons
        clearInput: document.getElementById('clearInput'),
        translateBtn: document.getElementById('translateBtn'),
        openFile: document.getElementById('openFile'),
        exportFile: document.getElementById('exportFile'),
        openSettings: document.getElementById('openSettings'),

        // Code areas
        inputCode: document.getElementById('inputCode'),
        outputCode: document.getElementById('outputCode'),
        inputCodePreview: document.getElementById('inputCodePreview'),
        outputCodePreview: document.getElementById('outputCodePreview'),

        // Language selects
        inputLangSelect: document.getElementById('input-lang-select'),
        outputLangSelect: document.getElementById('output-lang-select'),

        // Status bar
        statusFile: document.getElementById('status-file'),
        statusDocker: document.getElementById('status-docker'),
        statusZoom: document.getElementById('status-zoom'),

        // Zoom indicator
        zoomIndicator: document.getElementById('zoomIndicator'),

        // Terminal
        terminalPanel: document.getElementById('terminalPanel'),
        terminalOutput: document.getElementById('terminalOutput'),
        terminalResizer: document.getElementById('terminal-resizer'),
        inputTerminalOutput: document.getElementById('inputTerminalOutput'),
        outputTerminalOutput: document.getElementById('outputTerminalOutput'),
        terminalSplitResizer: document.getElementById('terminal-split-resizer')
    };

    // Проверка наличия всех элементов
    Object.entries(elements).forEach(([key, value]) => {
        if (!value) console.warn(`Element ${key} not found`);
    });

    return elements;
}

// Глобальные ссылки для использования в других модулях
export let elements = {};
export function setElements(el) {
    elements = el;
}