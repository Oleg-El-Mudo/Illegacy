import { elements } from './dom-elements.js';
import { MIN_COLUMN_WIDTH } from './constants.js';

let isResizing = false;
let startX = 0;
let startLeftWidth = 0;

export function setInitialWidths() {
    const containerWidth = elements.gridContainer.clientWidth;
    const columnsWidth = containerWidth - elements.resizer.offsetWidth;
    
    elements.leftColumn.style.width = `${columnsWidth / 2}px`;
    elements.rightColumn.style.width = `${columnsWidth / 2}px`;
    
    elements.leftColumn.style.flex = 'none';
    elements.rightColumn.style.flex = 'none';
}

export function initResizer() {
    // Обработчик события начала перемещения границы
    elements.resizer.addEventListener('mousedown', function(e) {
        isResizing = true;
        startX = e.clientX;
        startLeftWidth = elements.leftColumn.offsetWidth;
        
        document.body.classList.add('resizing');
    });

    // Перемещение границы
    window.addEventListener('mousemove', function(e) {
        if (!isResizing) return;
        
        const deltaX = e.clientX - startX;
        const containerWidth = elements.gridContainer.clientWidth;
        const resizerWidth = elements.resizer.offsetWidth;
        
        let newLeftWidth = startLeftWidth + deltaX;
        
        if (newLeftWidth < MIN_COLUMN_WIDTH) {
            newLeftWidth = MIN_COLUMN_WIDTH;
        }
        
        const maxLeftWidth = containerWidth - resizerWidth - MIN_COLUMN_WIDTH;
        if (newLeftWidth > maxLeftWidth) {
            newLeftWidth = maxLeftWidth;
        }
        
        elements.leftColumn.style.width = `${newLeftWidth}px`;
        elements.rightColumn.style.width = `${containerWidth - resizerWidth - newLeftWidth}px`;
        
        e.preventDefault();
    });

    // Завершение изменения размера
    window.addEventListener('mouseup', () => {
        if (isResizing) {
            isResizing = false;
            document.body.classList.remove('resizing');
        }
    });

    // Предотвращаем стандартное поведение браузера
    elements.resizer.addEventListener('dragstart', (e) => e.preventDefault());
    
    document.body.addEventListener('selectstart', (e) => {
        if (isResizing) e.preventDefault();
    });

    // Устанавливаем начальные пропорции
    window.addEventListener('load', setInitialWidths);
    window.addEventListener('resize', setInitialWidths);
}