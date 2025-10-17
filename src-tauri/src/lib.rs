// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(serde::Serialize)]
struct FileInfo {
    path: String,
    exists: bool,
    size: Option<u64>,
}

#[derive(serde::Serialize)]
struct ProcessFilesResponse {
    total: usize,
    processed: usize,
    files: Vec<FileInfo>,
}

/// Stub command: receives file paths from the frontend and returns basic metadata.
/// Extend this to perform your desired processing.
#[tauri::command]
fn process_files(paths: Vec<String>) -> Result<ProcessFilesResponse, String> {
    let mut files = Vec::with_capacity(paths.len());
    let mut processed = 0usize;

    for p in paths.into_iter() {
        match std::fs::metadata(&p) {
            Ok(meta) => {
                processed += 1;
                files.push(FileInfo {
                    path: p,
                    exists: true,
                    size: Some(meta.len()),
                });
            }
            Err(_) => {
                files.push(FileInfo {
                    path: p,
                    exists: false,
                    size: None,
                });
            }
        }
    }

    Ok(ProcessFilesResponse {
        total: files.len(),
        processed,
        files,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, process_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
