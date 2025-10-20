mod iroh;
use tauri::Manager;

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
            iroh::iroh_send,
            iroh::iroh_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
