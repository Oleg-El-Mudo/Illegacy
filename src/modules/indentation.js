import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

// Константы
const TAB_SIZE = 4; // Размер табуляции в пробелах
const INDENT_REGEX = /^[\t ]+/; // Регулярка для поиска отступов в начале строки

// Преобразование табуляций в пробелы 
export function tabsToSpaces(text) {
    return text.replace(/\t/g, ' '.repeat(TAB_SIZE));
}

// Преобразование пробелов в табуляции 
export function spacesToTabs(text) {
    return text.replace(new RegExp(' '.repeat(TAB_SIZE), 'g'), '\t');
}

// Получение текущего отступа строки
export function getCurrentIndent(line) {
    const match = line.match(INDENT_REGEX);
    return match ? match[0] : '';
}

// Расчет уровня отступа в единицах табуляции
export function getIndentLevel(line) {
    const indent = getCurrentIndent(line);
    let level = 0;
    
    for (let i = 0; i < indent.length; i++) {
        if (indent[i] === '\t') {
            level += 1;
        } else {
            level += 1 / TAB_SIZE; // Пробелы считаем как часть табуляции
        }
    }
    
    return Math.floor(level); // Округляем вниз до целого числа табуляций
}

// Автоматическое выравнивание новой строки
export function autoIndent(textarea, insertText = '\n') {
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const value = textarea.value;
    
    // Находим текущую строку и её отступ
    const lines = value.split('\n');
    let currentLineIndex = 0;
    let charCount = 0;
    
    for (let i = 0; i < lines.length; i++) {
        const lineLength = lines[i].length + 1; // +1 для символа новой строки
        if (charCount + lineLength > start) {
            currentLineIndex = i;
            break;
        }
        charCount += lineLength;
    }
    
    const currentLine = lines[currentLineIndex] || '';
    const currentIndent = getCurrentIndent(currentLine);
    
    // Определяем, нужно ли увеличить отступ (если строка заканчивается на '{')
    let newIndent = currentIndent;
    if (currentLine.trimEnd().endsWith('{')) {
        newIndent += '\t';
    }
    
    // Вставляем новую строку с отступом
    const beforeCursor = value.substring(0, start);
    const afterCursor = value.substring(end);
    
    const newValue = beforeCursor + insertText + newIndent + afterCursor;
    textarea.value = newValue;
    
    // Устанавливаем курсор после вставленного текста и отступа
    const newCursorPos = start + insertText.length + newIndent.length;
    textarea.setSelectionRange(newCursorPos, newCursorPos);
    
    return newValue;
}

// Обработка нажатия Tab
export function handleTabKey(e) {
    e.preventDefault();
    
    const textarea = e.target;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const value = textarea.value;
    
    // Если есть выделенный текст
    if (start !== end) {
        handleTabOnSelection(textarea, start, end);
    } else {
        // Просто вставляем табуляцию
        const beforeCursor = value.substring(0, start);
        const afterCursor = value.substring(end);
        
        textarea.value = beforeCursor + '\t' + afterCursor;
        
        // Перемещаем курсор после вставленной табуляции
        const newCursorPos = start + 1;
        textarea.setSelectionRange(newCursorPos, newCursorPos);
    }
    
    // Обновляем подсветку синтаксиса
    updateSyntaxHighlighting();
}

// Обработка Tab на выделенном тексте
function handleTabOnSelection(textarea, start, end) {
    const value = textarea.value;
    
    // Находим начало и конец строк, затрагиваемых выделением
    const lines = value.split('\n');
    let startLineIndex = 0;
    let endLineIndex = 0;
    let charCount = 0;
    
    // Находим индекс начальной строки
    for (let i = 0; i < lines.length; i++) {
        const lineLength = lines[i].length + 1;
        if (charCount + lineLength > start) {
            startLineIndex = i;
            break;
        }
        charCount += lineLength;
    }
    
    charCount = 0;
    // Находим индекс конечной строки
    for (let i = 0; i < lines.length; i++) {
        const lineLength = lines[i].length + 1;
        if (charCount + lineLength > end) {
            endLineIndex = i;
            break;
        }
        charCount += lineLength;
    }
    
    // Добавляем табуляцию в начало каждой выбранной строки
    for (let i = startLineIndex; i <= endLineIndex; i++) {
        lines[i] = '\t' + lines[i];
    }
    
    const newValue = lines.join('\n');
    textarea.value = newValue;
    
    // Корректируем выделение (сдвигаем на количество добавленных табуляций)
    const linesAdded = endLineIndex - startLineIndex + 1;
    textarea.setSelectionRange(start + linesAdded, end + linesAdded);
}

// Обработка нажатия Shift+Tab
export function handleShiftTabKey(e) {
    e.preventDefault();
    
    const textarea = e.target;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const value = textarea.value;
    
    // Находим строки для обработки
    const lines = value.split('\n');
    let startLineIndex = 0;
    let endLineIndex = 0;
    let charCount = 0;
    
    // Находим индекс начальной строки
    for (let i = 0; i < lines.length; i++) {
        const lineLength = lines[i].length + 1;
        if (charCount + lineLength > start) {
            startLineIndex = i;
            break;
        }
        charCount += lineLength;
    }
    
    charCount = 0;
    // Находим индекс конечной строки
    for (let i = 0; i < lines.length; i++) {
        const lineLength = lines[i].length + 1;
        if (charCount + lineLength > end) {
            endLineIndex = i;
            break;
        }
        charCount += lineLength;
    }
    
    let removedTabs = 0;
    
    // Удаляем один таб или 4 пробела из начала каждой выбранной строки
    for (let i = startLineIndex; i <= endLineIndex; i++) {
        if (lines[i].startsWith('\t')) {
            lines[i] = lines[i].substring(1);
            removedTabs++;
        } else if (lines[i].startsWith(' '.repeat(TAB_SIZE))) {
            lines[i] = lines[i].substring(TAB_SIZE);
            removedTabs++;
        }
    }
    
    const newValue = lines.join('\n');
    textarea.value = newValue;
    
    // Корректируем выделение (сдвигаем назад)
    if (start !== end) {
        textarea.setSelectionRange(start - removedTabs, end - removedTabs);
    } else {
        textarea.setSelectionRange(start - 1, end - 1);
    }
    
    // Обновляем подсветку синтаксиса
    updateSyntaxHighlighting();
}

// Обработка нажатия Enter
export function handleEnterKey(e) {
    e.preventDefault();
    autoIndent(e.target);
    updateSyntaxHighlighting();
}

// Инициализация обработчиков для выравнивания
export function initIndentation() {
    const textarea = elements.inputCode;
    
    // Обработка клавиш
    textarea.addEventListener('keydown', (e) => {
        switch (e.key) {
            case 'Tab':
                if (e.shiftKey) {
                    handleShiftTabKey(e);
                } else {
                    handleTabKey(e);
                }
                break;
                
            case 'Enter':
                handleEnterKey(e);
                break;
        }
    });
    
    // Опционально: обработка вставки текста для нормализации табуляций
    textarea.addEventListener('paste', (e) => {
        setTimeout(() => {
            const value = textarea.value;
            // textarea.value = spacesToTabs(tabsToSpaces(value));
            updateSyntaxHighlighting();
        }, 0);
    });
    
    console.log('Indentation module initialized');
}