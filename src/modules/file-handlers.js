import { elements } from './dom-elements.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

const { invoke } = window.__TAURI__.core;

export function initFileHandlers() {
    elements.clearInput.addEventListener('click', function(e){
        e.preventDefault();
        elements.inputCode.value = '';
        updateSyntaxHighlighting();
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
            }
        } catch (error) {
            console.error('Ошибка при открытии файла:', error);
            if (!error.includes("Файл не выбран")) {
                alert('Ошибка при открытии файла: ' + error);
            }
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
            }
        } catch (error) {
            console.error('Ошибка при сохранении файла:', error);
            if (!error.includes("Сохранение отменено")) {
                alert('Ошибка при сохранении файла: ' + error);
            }
        }
    });
}