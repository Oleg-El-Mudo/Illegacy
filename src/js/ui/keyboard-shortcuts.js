import { elements } from '../core/dom.js';
import { undo, redo, handleUndoAfterFileLoad } from '../editor/history.js';

// Состояние клавиш-модификаторов
let ctrlPressed = false;
let shiftPressed = false;

// Инициализация горячих клавиш
export function initKeyboardShortcuts() {
    // Отслеживаем состояние клавиш
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('keyup', handleKeyUp);
    
    // Предотвращаем стандартное поведение браузера для наших комбинаций
    document.addEventListener('keydown', preventDefaultForShortcuts);
    
    console.log('Keyboard shortcuts initialized');
}

// Обработка нажатия клавиш
function handleKeyDown(e) {
    // Обновляем состояние модификаторов
    ctrlPressed = e.ctrlKey || e.metaKey;
    shiftPressed = e.shiftKey;
    
    // Не обрабатываем комбинации, если пользователь печатает в поле
    const isTyping = e.target.matches('textarea, input, [contenteditable="true"]');
    
    // Глобальные комбинации
    if (ctrlPressed) {
        switch (e.key.toLowerCase()) {
            case 's':
                // Ctrl+S - экспорт файла
                e.preventDefault();
                if (elements.exportFile) {
                    elements.exportFile.click(); // Просто кликаем по кнопке
                }
                break;
                
            case 'o':
                // Ctrl+O - открыть файл
                e.preventDefault();
                if (elements.openFile) {
                    elements.openFile.click();
                }
                break;
        }
    }
    
    // Комбинации, которые работают только когда фокус на поле ввода
    if (isTyping && e.target === elements.inputCode) {
        handleInputShortcuts(e);
    }
}

// Обработка комбинаций для поля ввода
function handleInputShortcuts(e) {
    const ctrl = e.ctrlKey || e.metaKey;
    const shift = e.shiftKey;
    
    if (ctrl) {
        switch (e.key.toLowerCase()) {
            case 'z':
                e.preventDefault();
                if (shift) {
                    // Ctrl+Shift+Z - повтор
                    redo();
                } else {
                    // Ctrl+Z - отмена
                    // Проверяем специальный случай с загрузкой файла
                    if (!handleUndoAfterFileLoad()) {
                        undo();
                    }
                }
                break;
                
            case 'y':
                // Ctrl+Y - повтор (альтернатива Ctrl+Shift+Z)
                e.preventDefault();
                redo();
                break;
                
            // Для остальных комбинаций оставляем стандартное поведение
            case 'a':
            case 'x':
            case 'c':
            case 'v':
                // Не переопределяем - пусть работают стандартно
                break;
        }
    }
}

// Предотвращение стандартного поведения браузера
function preventDefaultForShortcuts(e) {
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    
    // Наши комбинации, которые должны переопределять стандартные
    if (ctrl) {
        switch (key) {
            case 's': // Сохраняем Ctrl+S для экспорта
            case 'o': // Сохраняем Ctrl+O для открытия
            case 'z': // Сохраняем Ctrl+Z для отмены
            case 'y': // Сохраняем Ctrl+Y для повтора
                e.preventDefault();
                break;
        }
    }
}

function handleKeyUp(e) {
    // Обновляем состояние модификаторов
    ctrlPressed = e.ctrlKey || e.metaKey;
    shiftPressed = e.shiftKey;
}