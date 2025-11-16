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

/// Share files with parallel progress tracking and real-time updates
///
/// Validates file paths, uploads files concurrently, and streams progress events
/// to the frontend via the provided channel.
///
/// # Arguments
///
/// * `channel` - Channel for sending progress events to the frontend
/// * `state` - The Tauri application state containing the initialized core
/// * `paths` - Vector of file or directory path strings to share
///
/// # Returns
///
/// A shareable ticket string that recipients can use to download the files
///
/// # Errors
///
/// Returns an error if core is not initialized, paths are invalid, or upload fails
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

/// Download files with parallel progress tracking and real-time updates
///
/// Parses the ticket, establishes a connection with the peer, downloads all files
/// concurrently, and streams progress events to the frontend.
///
/// # Arguments
///
/// * `channel` - Channel for sending progress events to the frontend
/// * `state` - The Tauri application state containing the initialized core
/// * `ticket` - The ticket string received from the sender
///
/// # Returns
///
/// Download result containing metadata and the path where files were saved
///
/// # Errors
///
/// Returns an error if core is not initialized, ticket is invalid, connection fails,
/// or download operation fails
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
