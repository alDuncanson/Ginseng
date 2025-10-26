mod commands;
mod core;
use tauri::Manager;

pub use core::GinsengCore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .setup(|app| {
            let state = app.state::<commands::AppState>();
            tauri::async_runtime::block_on(commands::setup_ginseng(state))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::share_file,
            commands::share_files,
            commands::download_file,
            commands::download_files,
            commands::node_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
