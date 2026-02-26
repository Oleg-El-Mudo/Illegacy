const resizer = document.getElementById('resizer');
const leftColumn = document.querySelector('.left-column');
const rightColumn = document.querySelector('.right-column');
const gridContainer = document.querySelector('.grid-container');
//кнопка очистки ввода
const clearInput = document.getElementById('clearInput');
//textarea для ввода кода
const inputCode = document.getElementById('inputCode');

let isResizing = false;
let startX = 0;
let startLeftWidth = 0;

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
})