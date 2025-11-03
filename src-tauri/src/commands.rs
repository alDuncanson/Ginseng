use crate::progress::ProgressEvent;
use crate::state::{AppState, DownloadResult};
use crate::utils::validate_and_canonicalize_paths;
use tauri::ipc::Channel;







/// Get information about the current node
///
/// # Arguments
/// * `state` - The Tauri application state
///
/// # Returns
/// Node information as a string
///
/// # Errors
/// Returns an error if core is not initialized or node info retrieval fails
#[tauri::command]
pub async fn node_info(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let core = state.get_core()?;

    core.node_info().await.map_err(|error| error.to_string())
}





/// Share files with parallel progress tracking
#[tauri::command]
pub async fn share_files_parallel(
    channel: Channel<ProgressEvent>,
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<String, String> {
    let core = state.get_core()?;
    let validated_paths = validate_and_canonicalize_paths(paths)?;

    core.share_files_parallel(channel, validated_paths)
        .await
        .map_err(|error| error.to_string())
}

/// Download files with parallel progress tracking
#[tauri::command]
pub async fn download_files_parallel(
    channel: Channel<ProgressEvent>,
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<DownloadResult, String> {
    let core = state.get_core()?;

    let (metadata, target_dir) = core
        .download_files_parallel(channel, ticket)
        .await
        .map_err(|error| error.to_string())?;

    Ok(DownloadResult {
        metadata,
        download_path: target_dir.to_string_lossy().to_string(),
    })
}
