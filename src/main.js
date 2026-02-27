const { invoke } = window.__TAURI__.core;

// Загружаем Highlight.js из глобальной переменной
const hljs = window.hljs;

const resizer = document.getElementById('resizer');
const leftColumn = document.querySelector('.left-column');
const rightColumn = document.querySelector('.right-column');
const gridContainer = document.querySelector('.grid-container');
//кнопка очистки ввода
const clearInput = document.getElementById('clearInput');
//кнопка перевода
const translateBtn = document.getElementById('translateBtn');
//textarea для ввода кода
const inputCode = document.getElementById('inputCode');
const outputCode = document.getElementById('outputCode');
const inputCodePreview = document.getElementById('inputCodePreview');
const outputCodePreview = document.getElementById('outputCodePreview');
const openFile = document.getElementById('openFile');
const exportFile = document.getElementById('exportFile');
//получаем селекты с языками
const inputLangSelect = document.getElementById('input-lang-select');
const outputLangSelect = document.getElementById('output-lang-select');

let isResizing = false;
let startX = 0;
let startLeftWidth = 0;
let scrollSyncEnabled = true; // Флаг для предотвращения циклической синхронизации

// Маппинг языков для Highlight.js
const languageMap = {
    // Входные языки
    'c': 'c',
    'fortran': 'fortran',
    'php': 'php',
    'cobol': 'cobol',
    // Выходные языки
    'python': 'python',
    'java': 'java',
    'go': 'go'
};

// Функция для обновления подсветки синтаксиса
function updateSyntaxHighlighting() {
    // Проверяем, загружен ли hljs
    if (!hljs) {
        console.warn('Highlight.js не загружен');
        return;
    }
    
    // Сохраняем позиции скролла
    const inputScrollTop = inputCode.scrollTop;
    const inputScrollLeft = inputCode.scrollLeft;
    const outputScrollTop = outputCode.scrollTop;
    const outputScrollLeft = outputCode.scrollLeft;
    
    // Обновляем подсветку для входного кода
    const inputLang = languageMap[inputLangSelect.value] || 'plaintext';
    const inputText = inputCode.value;
    
    if (inputText.trim()) {
        try {
            // Проверяем, поддерживается ли язык
            if (hljs.getLanguage(inputLang)) {
                inputCodePreview.innerHTML = hljs.highlight(inputText, { language: inputLang }).value;
            } else {
                console.warn(`Язык ${inputLang} не поддерживается Highlight.js`);
                inputCodePreview.innerHTML = escapeHtml(inputText);
            }
        } catch (e) {
            console.warn('Ошибка подсветки для входного кода:', e);
            inputCodePreview.innerHTML = escapeHtml(inputText);
        }
    } else {
        inputCodePreview.innerHTML = '';
    }
    
    // Обновляем подсветку для выходного кода
    const outputLang = languageMap[outputLangSelect.value] || 'plaintext';
    const outputText = outputCode.value;
    
    if (outputText.trim()) {
        try {
            // Проверяем, поддерживается ли язык
            if (hljs.getLanguage(outputLang)) {
                outputCodePreview.innerHTML = hljs.highlight(outputText, { language: outputLang }).value;
            } else {
                console.warn(`Язык ${outputLang} не поддерживается Highlight.js`);
                outputCodePreview.innerHTML = escapeHtml(outputText);
            }
        } catch (e) {
            console.warn('Ошибка подсветки для выходного кода:', e);
            outputCodePreview.innerHTML = escapeHtml(outputText);
        }
    } else {
        outputCodePreview.innerHTML = '';
    }
    
    // Восстанавливаем позиции скролла
    scrollSyncEnabled = false;
    inputCode.scrollTop = inputScrollTop;
    inputCode.scrollLeft = inputScrollLeft;
    outputCode.scrollTop = outputScrollTop;
    outputCode.scrollLeft = outputScrollLeft;
    
    // Синхронизируем preview
    syncScroll();
    scrollSyncEnabled = true;
}

// Функция для экранирования HTML (на случай ошибок подсветки)
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Функция для синхронизации скролла между textarea и preview
function syncScroll() {
    if (!scrollSyncEnabled) return;
    
    // Синхронизация для левой колонки
    const leftContainer = inputCode.closest('.code-container');
    const leftPreview = leftContainer.querySelector('.code-preview');
    if (leftPreview) {
        leftPreview.scrollTop = inputCode.scrollTop;
        leftPreview.scrollLeft = inputCode.scrollLeft;
    }
    
    // Синхронизация для правой колонки
    const rightContainer = outputCode.closest('.code-container');
    const rightPreview = rightContainer.querySelector('.code-preview');
    if (rightPreview) {
        rightPreview.scrollTop = outputCode.scrollTop;
        rightPreview.scrollLeft = outputCode.scrollLeft;
    }
}

// Устанавливаем начальные пропорции (50/50)
function setInitialWidths() {
    const containerWidth = gridContainer.clientWidth;
    // Вычитаем ширину резизера (5px)
    const columnsWidth = containerWidth - resizer.offsetWidth;
    
    // Устанавливаем равную ширину для обеих колонок
    leftColumn.style.width = `${columnsWidth / 2}px`;
    rightColumn.style.width = `${columnsWidth / 2}px`;
    
    // Убираем flex-basis, чтобы работали установленные ширины
    leftColumn.style.flex = 'none';
    rightColumn.style.flex = 'none';
}

// Вызываем при загрузке и при изменении размера окна
window.addEventListener('load', setInitialWidths);
window.addEventListener('resize', setInitialWidths);

// Обработчик события начала перемещения границы
resizer.addEventListener('mousedown', function(e) {
    isResizing = true;
    startX = e.clientX;
    startLeftWidth = leftColumn.offsetWidth;
    
    // Добавляем классы для предотвращения выделения текста
    document.body.classList.add('resizing');
});

// Перемещение границы
window.addEventListener('mousemove', function(e) {
    if (!isResizing) return;
    
    const deltaX = e.clientX - startX;
    const containerWidth = gridContainer.clientWidth;
    const resizerWidth = resizer.offsetWidth;
    
    // Минимальная ширина колонок (200px)
    const minWidth = 200;
    
    // Новая ширина левой колонки
    let newLeftWidth = startLeftWidth + deltaX;
    
    // Проверяем ограничения
    if (newLeftWidth < minWidth) {
        newLeftWidth = minWidth;
    }
    
    const maxLeftWidth = containerWidth - resizerWidth - minWidth;
    if (newLeftWidth > maxLeftWidth) {
        newLeftWidth = maxLeftWidth;
    }
    
    // Устанавливаем новые ширины
    leftColumn.style.width = `${newLeftWidth}px`;
    rightColumn.style.width = `${containerWidth - resizerWidth - newLeftWidth}px`;
    
    // Предотвращаем выделение текста во время ресайза
    e.preventDefault();
});

// Завершение изменения размера
window.addEventListener('mouseup', () => {
    if (isResizing) {
        isResizing = false;
        document.body.classList.remove('resizing');
    }
});

// Предотвращаем стандартное поведение браузера при перетаскивании
resizer.addEventListener('dragstart', (e) => {
    e.preventDefault();
});

// Дополнительная защита от случайного выделения
document.body.addEventListener('selectstart', (e) => {
    if (isResizing) {
        e.preventDefault();
    }
});

clearInput.addEventListener('click', function(e){
    e.preventDefault();
    inputCode.value='';
    updateSyntaxHighlighting();
});

openFile.addEventListener('click', async function(e){
    e.preventDefault();
    
    try {
        // Получаем выбранный входной язык
        const selectedLang = inputLangSelect.value;
        
        // Вызываем Rust-команду, которая:
        // 1. Открывает диалог с фильтрацией
        // 2. Читает файл
        // 3. Возвращает содержимое
        const fileContent = await invoke('open_file_with_filter', { lang: selectedLang });
        
        if (fileContent) {
            inputCode.value = fileContent;
            updateSyntaxHighlighting();
            console.log('Файл успешно загружен');
        }
    } catch (error) {
        console.error('Ошибка при открытии файла:', error);
        // Не показываем alert если пользователь просто отменил выбор
        if (!error.includes("Файл не выбран")) {
            alert('Ошибка при открытии файла: ' + error);
        }
    }
});

exportFile.addEventListener('click', async function(e){
    e.preventDefault();
    
    // Получаем содержимое для экспорта (сначала проверяем output, потом input)
    const contentToExport = outputCode.value || inputCode.value;
    
    if (!contentToExport.trim()) {
        alert('Нет содержимого для экспорта');
        return;
    }
    
    try {
        // Получаем выбранный выходной язык
        const selectedLang = outputLangSelect.value;
        
        // Вызываем Rust-команду для сохранения файла
        const savedPath = await invoke('save_file_with_filter', { 
            content: contentToExport,
            lang: selectedLang 
        });
        
        if (savedPath) {
            console.log('Файл успешно сохранен:', savedPath);
            alert(`Файл успешно сохранен: ${savedPath}`);
        }
    } catch (error) {
        console.error('Ошибка при сохранении файла:', error);
        // Не показываем alert если пользователь просто отменил сохранение
        if (!error.includes("Сохранение отменено")) {
            alert('Ошибка при сохранении файла: ' + error);
        }
    }
});

// Функция перевода кода
async function translateCode() {
    const inputLang = inputLangSelect.value;
    const outputLang = outputLangSelect.value;
    const code = inputCode.value.trim();
    
    if (!code) {
        alert('Введите код для перевода');
        return;
    }
    
    // Проверяем, поддерживается ли выбранная пара языков
    if (inputLang !== 'c' || outputLang !== 'python') {
        const errorMsg = `// Транспиляция из ${inputLang} в ${outputLang} пока не поддерживается\n// Доступно: C -> Python`;
        outputCode.value = errorMsg;
        updateSyntaxHighlighting();
        alert(`Пара языков ${inputLang} -> ${outputLang} пока не поддерживается. Доступно: C -> Python`);
        return;
    }
    
    // Блокируем кнопку на время перевода
    const originalText = translateBtn.textContent;
    translateBtn.textContent = 'Перевод...';
    translateBtn.style.opacity = '0.7';
    translateBtn.style.pointerEvents = 'none';
    
    try {
        console.log('Запуск транспиляции C -> Python');
        
        // Вызываем Rust-команду для транспиляции
        const result = await invoke('transpile_c_to_python', { code });
        
        if (result.success) {
            outputCode.value = result.output;
            updateSyntaxHighlighting();
            console.log('Транспиляция успешна');
            
            // Если есть AST для отладки, можно вывести в консоль
            if (result.ast_json) {
                console.log('AST получен');
            }
        } else {
            outputCode.value = '';
            updateSyntaxHighlighting();
            alert('Ошибка транспиляции: ' + result.error);
        }
    } catch (error) {
        console.error('Ошибка при переводе:', error);
        outputCode.value = '';
        updateSyntaxHighlighting();
        alert('Ошибка при переводе: ' + error);
    } finally {
        // Возвращаем кнопку в исходное состояние
        translateBtn.textContent = originalText;
        translateBtn.style.opacity = '1';
        translateBtn.style.pointerEvents = 'auto';
    }
}

// Добавляем обработчик на кнопку "Перевести"
if (translateBtn) {
    translateBtn.addEventListener('click', function(e) {
        e.preventDefault();
        translateCode();
    });
}

// Добавляем горячие клавиши (Ctrl+Enter для перевода)
inputCode.addEventListener('keydown', function(e) {
    if (e.ctrlKey && e.key === 'Enter') {
        e.preventDefault();
        translateCode();
    }
});

// Обновление подсветки при вводе текста
inputCode.addEventListener('input', updateSyntaxHighlighting);
outputCode.addEventListener('input', updateSyntaxHighlighting);

// Обновление подсветки при смене языка
inputLangSelect.addEventListener('change', updateSyntaxHighlighting);
outputLangSelect.addEventListener('change', updateSyntaxHighlighting);

// Синхронизация скролла с защитой от цикличности
inputCode.addEventListener('scroll', () => {
    if (scrollSyncEnabled) {
        requestAnimationFrame(syncScroll);
    }
});

outputCode.addEventListener('scroll', () => {
    if (scrollSyncEnabled) {
        requestAnimationFrame(syncScroll);
    }
});

// Добавляем обработку вставки текста
inputCode.addEventListener('paste', function(e) {
    // Даем время на вставку текста
    setTimeout(updateSyntaxHighlighting, 0);
});

outputCode.addEventListener('paste', function(e) {
    setTimeout(updateSyntaxHighlighting, 0);
});

// Добавляем обработку вырезания текста
inputCode.addEventListener('cut', function(e) {
    setTimeout(updateSyntaxHighlighting, 0);
});

outputCode.addEventListener('cut', function(e) {
    setTimeout(updateSyntaxHighlighting, 0);
});

// Функция для проверки статуса Docker (опционально)
async function checkDockerStatus() {
    try {
        const status = await invoke('check_parser_status');
        console.log('Статус Docker:', status.docker_available ? 'Доступен' : 'Не доступен');
        
        // Можно добавить индикатор в консоль или UI
        if (!status.docker_available) {
            console.warn('Docker не запущен. Транспиляция может не работать.');
        }
    } catch (error) {
        console.error('Ошибка при проверке Docker:', error);
    }
}

// Проверяем статус Docker при загрузке
window.addEventListener('load', function() {
    setInitialWidths();
    checkDockerStatus();
    
    // Даем время на загрузку Highlight.js
    setTimeout(updateSyntaxHighlighting, 100);
});

// Добавляем обработчик на загрузку Highlight.js (на случай асинхронной загрузки)
document.addEventListener('DOMContentLoaded', function() {
    // Если hljs загрузился после DOM, обновляем подсветку
    if (window.hljs) {
        updateSyntaxHighlighting();
    }
});