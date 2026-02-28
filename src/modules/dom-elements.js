// Получение ссылок на DOM-элементы
export function getDOMElements() {
    const elements = {
        resizer: document.getElementById('resizer'),
        leftColumn: document.querySelector('.left-column'),
        rightColumn: document.querySelector('.right-column'),
        gridContainer: document.querySelector('.grid-container'),
        clearInput: document.getElementById('clearInput'),
        translateBtn: document.getElementById('translateBtn'),
        inputCode: document.getElementById('inputCode'),
        outputCode: document.getElementById('outputCode'),
        inputCodePreview: document.getElementById('inputCodePreview'),
        outputCodePreview: document.getElementById('outputCodePreview'),
        openFile: document.getElementById('openFile'),
        exportFile: document.getElementById('exportFile'),
        inputLangSelect: document.getElementById('input-lang-select'),
        outputLangSelect: document.getElementById('output-lang-select')
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