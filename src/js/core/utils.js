// Вспомогательные функции
export function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

export function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}