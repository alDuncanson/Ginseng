pub mod core;
mod iroh;
mod utils;

pub use crate::core::{FileInfo, ProcessFilesResponse};
use tauri::Manager;

/// Tauri command wrapper kept as `process_files` to match frontend.
#[tauri::command]
fn process_files(paths: Vec<String>) -> Result<ProcessFilesResponse, String> {
    Ok(core::process_paths(paths))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(iroh::AppState::default())
        .setup(|app| {
            let state = app.state::<iroh::AppState>();
            tauri::async_runtime::block_on(iroh::setup_iroh(state))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            process_files,
            iroh::iroh_send,
            iroh::iroh_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_files_command_uses_core() {
        let resp = process_files(vec!["/definitely/not/here".into()]).unwrap();
        assert_eq!(resp.total, 1);
    }
}
