use crate::progress::ProgressEvent;
use crate::state::{AppState, DownloadResult};
use crate::utils::validate_and_canonicalize_paths;
use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum DownloadEvent<'a> {
    Started { detail: &'a str },
    Progress { detail: &'a str },
    Completed { detail: &'a str },
    Failed { detail: &'a str },
}

/// Share multiple files and return a ticket for downloading
///
/// # Arguments
/// * `channel` - Channel to send download events
/// * `state` - The Tauri application state
/// * `paths` - Vector of file paths to share
///
/// # Returns
/// A ticket string that can be used to download the files
///
/// # Errors
/// Returns an error if core is not initialized, paths are invalid, or sharing fails
#[tauri::command]
pub async fn share_files(
    channel: Channel<DownloadEvent<'_>>,
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
) -> Result<String, String> {
    channel
        .send(DownloadEvent::Started {
            detail: "Preparing to share files",
        })
        .unwrap();

    let core = state.get_core()?;

    let validated_paths = validate_and_canonicalize_paths(paths)?;

    core.share_files(&channel, validated_paths)
        .await
        .map_err(|error| error.to_string())
}

/// Download files using a ticket
///
/// # Arguments
/// * `state` - The Tauri application state
/// * `ticket` - The ticket string for the files to download
///
/// # Returns
/// DownloadResult containing metadata and download path
///
/// # Errors
/// Returns an error if core is not initialized or download fails
#[tauri::command]
pub async fn download_files(
    state: tauri::State<'_, AppState>,
    ticket: String,
) -> Result<DownloadResult, String> {
    let core = state.get_core()?;

    let (metadata, target_dir) = core
        .download_files(ticket)
        .await
        .map_err(|error| error.to_string())?;

    Ok(DownloadResult {
        metadata,
        download_path: target_dir.to_string_lossy().to_string(),
    })
}

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

/// Share a single file (convenience wrapper around share_files)
///
/// # Arguments
/// * `state` - The Tauri application state
/// * `path` - Path to the file to share
///
/// # Returns
/// A ticket string that can be used to download the file
///
/// # Errors
/// Returns an error if core is not initialized, path is invalid, or sharing fails
#[tauri::command]
pub async fn share_file(
    channel: Channel<DownloadEvent<'_>>,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    share_files(channel, state, vec![path]).await
}

/// Download a file using a ticket (convenience wrapper around download_files)
///
/// # Arguments
/// * `state` - The Tauri application state
/// * `ticket` - The ticket string for the file to download
/// * `_target` - Target path (currently unused, kept for API compatibility)
///
/// # Returns
/// Ok(()) if download succeeds
///
/// # Errors
/// Returns an error if core is not initialized or download fails
#[tauri::command]
pub async fn download_file(
    state: tauri::State<'_, AppState>,
    ticket: String,
    _target: String,
) -> Result<(), String> {
    let _result = download_files(state, ticket).await?;
    Ok(())
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
