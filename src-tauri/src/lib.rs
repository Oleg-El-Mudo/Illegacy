use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};
//чтение файла
use std::fs::File;
use std::io::Read;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_file_with_filter(app: AppHandle, lang: String) -> Result<String, String> {
    // Получаем фильтры для выбранного языка
    let filters = get_filters_for_language(&lang);
    
    // Создаем диалог
    let mut dialog = app.dialog().file();
    
    // Добавляем фильтры
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }
    
    // Открываем диалог и получаем путь
    let file_path = dialog.blocking_pick_file();
    
    // Проверяем, выбран ли файл
    match file_path {
        Some(path) => {
            // Преобразуем FilePath в строку пути
            let path_str = path.to_string();
            
            // Читаем содержимое файла
            match read_file_content(&path_str) {
                Ok(content) => Ok(content),
                Err(e) => Err(format!("Не удалось прочитать файл: {}", e))
            }
        },
        None => Err("Файл не выбран".to_string())
    }
}

fn get_filters_for_language(language: &str) -> Vec<FileFilter> {
    match language {
        "c" => vec![
            FileFilter { 
                name: "C/C++ Files".to_string(), 
                extensions: vec!["c".to_string(), "h".to_string()] 
            }
        ],
        "fortran" => vec![
            FileFilter { 
                name: "Fortran Files".to_string(), 
                extensions: vec![
                    "f".to_string(), 
                    "for".to_string(), 
                    "f90".to_string(), 
                    "f95".to_string()
                ] 
            }
        ],
        "php" => vec![
            FileFilter { 
                name: "PHP Files".to_string(), 
                extensions: vec!["php".to_string()] 
            }
        ],
        "cobol" => vec![
            FileFilter { 
                name: "COBOL Files".to_string(), 
                extensions: vec!["cob".to_string(), "cbl".to_string()] 
            }
        ],
        _ => vec![
            FileFilter { 
                name: "All Files".to_string(), 
                extensions: vec!["*".to_string()] 
            }
        ]
    }
}

fn read_file_content(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Ошибка открытия файла: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Ошибка чтения файла: {}", e))?;
    Ok(content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_file_with_filter])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}