use crate::core::{GinsengCore, ShareMetadata};
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
pub async fn share_files(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<String, String> {
    let core = state.get()?;

    let path_bufs: Result<Vec<std::path::PathBuf>, String> = paths
        .iter()
        .map(|path| {
            std::fs::canonicalize(path).map_err(|e| format!("Invalid file path '{}': {}", path, e))
        })
        .collect();

    let path_bufs = path_bufs?;

    core.share_files(path_bufs).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<DownloadResult, String> {
    let core = state.get()?;

    let (metadata, target_dir) = core
        .download_files(ticket)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DownloadResult {
        metadata,
        download_path: target_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn node_info(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let core = state.get()?;

    core.node_info().await.map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct DownloadResult {
    pub metadata: ShareMetadata,
    pub download_path: String,
}

// Legacy commands for backward compatibility
#[tauri::command]
pub async fn share_file(state: tauri::State<'_, AppState>, path: String) -> Result<String, String> {
    share_files(state, vec![path]).await
}

#[tauri::command]
pub async fn download_file(
    state: tauri::State<'_, AppState>,
    ticket: String,
    _target: String, // Ignored - we now auto-download to Downloads folder
) -> Result<(), String> {
    let _result = download_files(state, ticket).await?;
    Ok(())
}
