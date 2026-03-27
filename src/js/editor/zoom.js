import { elements } from '../core/dom.js';
import { updateSyntaxHighlighting } from './syntax-highlight.js';

// Константы для масштабирования
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 32;
const DEFAULT_FONT_SIZE = 14; // Базовый размер шрифта = 100%
const ZOOM_STEP = 2;

// Ключ для сохранения в localStorage
const STORAGE_KEY = 'illegacy-font-size';

// Базовый размер шрифта (считается 100%)
let baseFontSize = DEFAULT_FONT_SIZE;

// Текущий масштаб в процентах относительно базового размера
let currentZoomPercent = 100;

// Загрузка сохраненного размера шрифта (базовый размер)
export function loadFontSize() {
    try {
        const saved = localStorage.getItem(STORAGE_KEY);
        if (saved) {
            const size = parseInt(saved, 10);
            if (size >= MIN_FONT_SIZE && size <= MAX_FONT_SIZE) {
                baseFontSize = size;
            }
        }
    } catch (e) {
        console.warn('Failed to load font size from localStorage:', e);
    }
    return baseFontSize;
}

// Сохранение базового размера шрифта
export function saveFontSize(size) {
    try {
        localStorage.setItem(STORAGE_KEY, size.toString());
    } catch (e) {
        console.warn('Failed to save font size to localStorage:', e);
    }
}

// Применение размера шрифта ко всем элементам
export function applyFontSize(size) {
    // Ограничиваем размер
    size = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, size));

    if (size === baseFontSize && currentZoomPercent === 100) return;

    baseFontSize = size;
    currentZoomPercent = 100; // Сбрасываем зум к 100% при смене базового размера

    // Сохраняем позиции скролла перед изменением
    const scrollPositions = {
        input: { top: elements.inputCode.scrollTop, left: elements.inputCode.scrollLeft },
        output: { top: elements.outputCode.scrollTop, left: elements.outputCode.scrollLeft }
    };

    // Обновляем CSS-переменную для применения ко всему интерфейсу
    document.documentElement.style.setProperty('--font-size-base', `${size}px`);

    // Применяем размер шрифта ко всем текстовым элементам
    const fontSizePx = `${size}px`;

    // Textarea для ввода
    elements.inputCode.style.fontSize = fontSizePx;
    elements.outputCode.style.fontSize = fontSizePx;

    // Preview элементы
    const inputPreview = elements.inputCodePreview.closest('pre');
    const outputPreview = elements.outputCodePreview.closest('pre');

    if (inputPreview) {
        inputPreview.style.fontSize = fontSizePx;
    }
    if (outputPreview) {
        outputPreview.style.fontSize = fontSizePx;
    }

    // Также обновляем сам code элемент внутри preview
    elements.inputCodePreview.style.fontSize = fontSizePx;
    elements.outputCodePreview.style.fontSize = fontSizePx;

    // Сохраняем в localStorage
    saveFontSize(size);

    // Обновляем индикатор масштаба
    updateZoomIndicator();

    // Восстанавливаем позиции скролла
    requestAnimationFrame(() => {
        elements.inputCode.scrollTop = scrollPositions.input.top;
        elements.inputCode.scrollLeft = scrollPositions.input.left;
        elements.outputCode.scrollTop = scrollPositions.output.top;
        elements.outputCode.scrollLeft = scrollPositions.output.left;

        // Обновляем подсветку для корректного отображения
        updateSyntaxHighlighting();
    });
}

// Увеличение размера шрифта (увеличиваем масштаб)
export function zoomIn() {
    const newZoom = Math.min(currentZoomPercent + 10, Math.round((MAX_FONT_SIZE / baseFontSize) * 100));
    applyZoomPercent(newZoom);
}

// Уменьшение размера шрифта (уменьшаем масштаб)
export function zoomOut() {
    const newZoom = Math.max(currentZoomPercent - 10, Math.round((MIN_FONT_SIZE / baseFontSize) * 100));
    applyZoomPercent(newZoom);
}

// Сброс к 100% масштабу
export function zoomReset() {
    applyZoomPercent(100);
}

// Применение масштаба в процентах
function applyZoomPercent(percent) {
    if (percent === currentZoomPercent) return;
    
    currentZoomPercent = percent;
    const newSize = Math.round(baseFontSize * (percent / 100));
    const clampedSize = Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, newSize));

    // Сохраняем позиции скролла перед изменением
    const scrollPositions = {
        input: { top: elements.inputCode.scrollTop, left: elements.inputCode.scrollLeft },
        output: { top: elements.outputCode.scrollTop, left: elements.outputCode.scrollLeft }
    };

    // Обновляем CSS-переменную для применения ко всему интерфейсу
    document.documentElement.style.setProperty('--font-size-base', `${clampedSize}px`);

    // Применяем размер шрифта ко всем текстовым элементам
    const fontSizePx = `${clampedSize}px`;

    // Textarea для ввода
    elements.inputCode.style.fontSize = fontSizePx;
    elements.outputCode.style.fontSize = fontSizePx;

    // Preview элементы
    const inputPreview = elements.inputCodePreview.closest('pre');
    const outputPreview = elements.outputCodePreview.closest('pre');

    if (inputPreview) {
        inputPreview.style.fontSize = fontSizePx;
    }
    if (outputPreview) {
        outputPreview.style.fontSize = fontSizePx;
    }

    // Также обновляем сам code элемент внутри preview
    elements.inputCodePreview.style.fontSize = fontSizePx;
    elements.outputCodePreview.style.fontSize = fontSizePx;

    // Обновляем индикатор масштаба
    updateZoomIndicator();

    // Восстанавливаем позиции скролла
    requestAnimationFrame(() => {
        elements.inputCode.scrollTop = scrollPositions.input.top;
        elements.inputCode.scrollLeft = scrollPositions.input.left;
        elements.outputCode.scrollTop = scrollPositions.output.top;
        elements.outputCode.scrollLeft = scrollPositions.output.left;

        // Обновляем подсветку для корректного отображения
        updateSyntaxHighlighting();
    });
}

// Получение текущего размера шрифта
export function getCurrentFontSize() {
    return Math.round(baseFontSize * (currentZoomPercent / 100));
}

// Обновление индикатора масштаба
export function updateZoomIndicator() {
    // Обновляем индикатор в статус-баре
    if (elements.statusZoom) {
        elements.statusZoom.textContent = `${currentZoomPercent}%`;
    }

    // Обновляем всплывающий индикатор
    if (elements.zoomIndicator) {
        elements.zoomIndicator.textContent = `${currentZoomPercent}%`;
        elements.zoomIndicator.classList.add('show');

        // Скрываем через 1.5 секунды
        setTimeout(() => {
            elements.zoomIndicator.classList.remove('show');
        }, 1500);
    }
}

// Инициализация обработчиков масштабирования
export function initZoom() {
    // Загружаем сохраненный базовый размер шрифта
    loadFontSize();

    // Применяем CSS-переменную для базового размера
    document.documentElement.style.setProperty('--font-size-base', `${baseFontSize}px`);

    // Применяем базовый размер (100% масштаб)
    applyFontSize(baseFontSize);

    // Обработчик для Ctrl + + / Ctrl + - (клавиатура)
    document.addEventListener('keydown', (e) => {
        if (e.ctrlKey || e.metaKey) {
            const key = e.key;

            // Ctrl + 0 - сброс масштаба к 100%
            if (key === '0') {
                e.preventDefault();
                zoomReset();
                return;
            }

            // Ctrl + + / Ctrl + = / Ctrl + NumpadAdd - увеличить
            if (key === '+' || key === '=' || key === 'Add') {
                e.preventDefault();
                zoomIn();
                return;
            }

            // Ctrl + - / Ctrl + NumpadSubtract - уменьшить
            if (key === '-' || key === 'Subtract') {
                e.preventDefault();
                zoomOut();
                return;
            }
        }
    });

    // Обработчик для Ctrl + колесико мыши
    document.addEventListener('wheel', (e) => {
        if (e.ctrlKey) {
            e.preventDefault();

            if (e.deltaY < 0) {
                // Колесико вверх - увеличение
                zoomIn();
            } else {
                // Колесико вниз - уменьшение
                zoomOut();
            }
        }
    }, { passive: false });

    console.log('Zoom module initialized with base font size:', baseFontSize);
}