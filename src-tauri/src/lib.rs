mod commands;
pub mod core;
pub mod progress;
mod state;
mod utils;
use tauri::Manager;

pub use core::{GinsengCore, ShareType};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .setup(|app| {
            let state = app.state::<state::AppState>();
            tauri::async_runtime::block_on(state::setup_ginseng(state))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::share_files_parallel,
            commands::download_files_parallel,
            commands::node_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
