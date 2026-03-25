import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

const { invoke } = window.__TAURI__.core;

export function initFileHandlers() {
    elements.clearInput.addEventListener('click', function(e){
        e.preventDefault();
        elements.inputCode.value = '';
        updateSyntaxHighlighting();
        updateStatusBar('Готов к работе');
    });

    elements.openFile.addEventListener('click', async function(e){
        e.preventDefault();

        try {
            const selectedLang = elements.inputLangSelect.value;
            const fileContent = await invoke('open_file_with_filter', { lang: selectedLang });

            if (fileContent) {
                elements.inputCode.value = fileContent;
                updateSyntaxHighlighting();
                console.log('Файл успешно загружен');
                updateStatusBar('Файл загружен');
            }
        } catch (error) {
            console.error('Ошибка при открытии файла:', error);
            if (!error.includes("Файл не выбран")) {
                alert('Ошибка при открытии файла: ' + error);
            }
            updateStatusBar('Ошибка при открытии файла');
        }
    });

    elements.exportFile.addEventListener('click', async function(e){
        e.preventDefault();

        const contentToExport = elements.outputCode.value || elements.inputCode.value;

        if (!contentToExport.trim()) {
            alert('Нет содержимого для экспорта');
            return;
        }

        try {
            const selectedLang = elements.outputLangSelect.value;
            const savedPath = await invoke('save_file_with_filter', {
                content: contentToExport,
                lang: selectedLang
            });

            if (savedPath) {
                console.log('Файл успешно сохранен:', savedPath);
                alert(`Файл успешно сохранен: ${savedPath}`);
                updateStatusBar(`Файл сохранен: ${savedPath.split('/').pop() || savedPath.split('\\').pop()}`);
            }
        } catch (error) {
            console.error('Ошибка при сохранении файла:', error);
            if (!error.includes("Сохранение отменено")) {
                alert('Ошибка при сохранении файла: ' + error);
            }
            updateStatusBar('Ошибка при сохранении файла');
        }
    });
}

/**
 * Обновляет статус-бар
 */
function updateStatusBar(message) {
    if (elements.statusFile) {
        elements.statusFile.textContent = message;
    }
}

// При загрузке файла:
export async function openFile() {
    try {
        const result = await window.__TAURI__.dialog.open({
            multiple: false,
            filters: [{
                name: 'Source Code',
                extensions: ['c', 'h', 'php', 'f90', 'f', 'cob', 'cbl']
            }]
        });
        
        if (result) {
            const content = await window.__TAURI__.fs.readTextFile(result);
            elements.inputCode.value = content;
            updateSyntaxHighlighting();
            
            // Генерируем событие о загрузке файла
            const event = new CustomEvent('file-loaded', {
                detail: {
                    content: content,
                    filename: result.split('/').pop() || result.split('\\').pop()
                }
            });
            document.dispatchEvent(event);
        }
    } catch (error) {
        console.error('Error opening file:', error);
    }
}

// При очистке ввода:
export function clearInput() {
    elements.inputCode.value = '';
    updateSyntaxHighlighting();
    
    // Генерируем событие об очистке
    const event = new CustomEvent('input-cleared');
    document.dispatchEvent(event);
}

// При экспорте файла:
export async function exportFile() {
    const content = elements.outputCode.value;
    if (!content.trim()) {
        alert('Нет данных для экспорта');
        return;
    }
    
    try {
        const result = await window.__TAURI__.dialog.save({
            filters: [{
                name: 'Python Code',
                extensions: ['py']
            }]
        });
        
        if (result) {
            await window.__TAURI__.fs.writeTextFile(result, content);
        }
    } catch (error) {
        console.error('Error exporting file:', error);
    }
}