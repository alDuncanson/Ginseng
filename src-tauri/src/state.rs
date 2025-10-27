use crate::core::{GinsengCore, ShareMetadata};
use tokio::sync::OnceCell;

/// Application state that holds the Ginseng core instance
#[derive(Default)]
pub struct AppState {
    pub(crate) core: OnceCell<GinsengCore>,
}

/// Result structure for download operations
#[derive(serde::Serialize)]
pub struct DownloadResult {
    pub metadata: ShareMetadata,
    pub download_path: String,
}

impl AppState {
    /// Get a reference to the initialized Ginseng core
    ///
    /// # Returns
    /// A reference to the GinsengCore instance
    ///
    /// # Errors
    /// Returns an error if the core has not been initialized yet
    pub fn get_core(&self) -> Result<&GinsengCore, String> {
        self.core
            .get()
            .ok_or_else(|| "Ginseng core not initialized yet".to_string())
    }
}

/// Initialize the Ginseng core and store it in the application state
///
/// # Arguments
/// * `state` - The Tauri application state
///
/// # Returns
/// Ok(()) if initialization succeeds
///
/// # Errors
/// Returns an error if core creation fails or if already initialized
pub async fn setup_ginseng(state: tauri::State<'_, AppState>) -> Result<(), anyhow::Error> {
    let core = GinsengCore::new().await?;

    state
        .core
        .set(core)
        .map_err(|_| anyhow::anyhow!("Ginseng core already initialized"))?;

    Ok(())
}
