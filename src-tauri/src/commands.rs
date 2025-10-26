use crate::core::GinsengCore;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Default)]
pub struct AppState {
    pub(crate) core: OnceCell<Arc<GinsengCore>>,
}

impl AppState {
    fn get(&self) -> Result<&Arc<GinsengCore>, String> {
        self.core
            .get()
            .ok_or_else(|| "Ginseng core not initialized yet".to_string())
    }
}

pub async fn setup_ginseng(state: tauri::State<'_, AppState>) -> Result<(), anyhow::Error> {
    let core = GinsengCore::new().await?;

    state
        .core
        .set(Arc::new(core))
        .map_err(|_| anyhow::anyhow!("Ginseng core already initialized"))?;

    Ok(())
}

#[tauri::command]
pub async fn share_file(state: tauri::State<'_, AppState>, path: String) -> Result<String, String> {
    let core = state.get()?;
    let path_buf = std::fs::canonicalize(&path).map_err(|e| format!("Invalid file path: {}", e))?;
    core.share_file(path_buf).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_file(
    state: tauri::State<'_, AppState>,
    ticket: String,
    target: String,
) -> Result<(), String> {
    let core = state.get()?;
    let target_path = std::path::PathBuf::from(&target);

    core.download_file(ticket, target_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn node_info(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let core = state.get()?;

    core.node_info().await.map_err(|e| e.to_string())
}
