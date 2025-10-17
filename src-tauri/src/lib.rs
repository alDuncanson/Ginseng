use std::path::PathBuf;
use std::sync::Arc;

use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol};
use tauri::Manager;
use tokio::sync::OnceCell;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(serde::Serialize)]
struct FileInfo {
    path: String,
    exists: bool,
    size: Option<u64>,
}

#[derive(serde::Serialize)]
struct ProcessFilesResponse {
    total: usize,
    processed: usize,
    files: Vec<FileInfo>,
}

/// Stub command: receives file paths from the frontend and returns basic metadata.
/// Extend this to perform your desired processing.
#[tauri::command]
fn process_files(paths: Vec<String>) -> Result<ProcessFilesResponse, String> {
    let mut files = Vec::with_capacity(paths.len());
    let mut processed = 0usize;

    for p in paths.into_iter() {
        match std::fs::metadata(&p) {
            Ok(meta) => {
                processed += 1;
                files.push(FileInfo {
                    path: p,
                    exists: true,
                    size: Some(meta.len()),
                });
            }
            Err(_) => {
                files.push(FileInfo {
                    path: p,
                    exists: false,
                    size: None,
                });
            }
        }
    }

    Ok(ProcessFilesResponse {
        total: files.len(),
        processed,
        files,
    })
}

// --- Iroh shared state ---
struct IrohInner {
    endpoint: Endpoint,
    store: MemStore,
    #[allow(dead_code)]
    router: Router,
}

#[derive(Default)]
struct AppState {
    iroh: OnceCell<Arc<IrohInner>>, // set during setup, read in commands
}

impl AppState {
    fn get(&self) -> Result<&Arc<IrohInner>, String> {
        self.iroh
            .get()
            .ok_or_else(|| "Iroh not initialized yet".to_string())
    }
}

#[tauri::command]
async fn iroh_send(state: tauri::State<'_, AppState>, path: String) -> Result<String, String> {
    let inner = state.get()?.clone();

    // Hash/import the file into the in-memory blob store
    let filename: PathBuf = PathBuf::from(path);
    let abs_path = std::path::absolute(&filename).map_err(|e| e.to_string())?;

    let tag = inner
        .store
        .blobs()
        .add_path(abs_path)
        .await
        .map_err(|e| e.to_string())?;

    // Create a ticket combining our node id and the blob hash/format
    let node_id = inner.endpoint.node_id();
    let ticket = BlobTicket::new(node_id.into(), tag.hash, tag.format);

    Ok(ticket.to_string())
}

#[tauri::command]
async fn iroh_download(
    state: tauri::State<'_, AppState>,
    ticket: String,
    dest_path: String,
) -> Result<(), String> {
    let inner = state.get()?.clone();

    let ticket: BlobTicket = ticket.parse::<BlobTicket>().map_err(|e| e.to_string())?;
    let dest: PathBuf = PathBuf::from(dest_path);
    let abs_path = std::path::absolute(dest).map_err(|e| e.to_string())?;

    // Create a downloader and fetch the blob
    let downloader = inner.store.downloader(&inner.endpoint);
    downloader
        .download(ticket.hash(), Some(ticket.node_addr().node_id))
        .await
        .map_err(|e| e.to_string())?;

    // Export to destination path
    inner
        .store
        .blobs()
        .export(ticket.hash(), abs_path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            // Initialize iroh endpoint, in-memory blobs store, and router synchronously at startup
            let state = app.state::<AppState>();
            tauri::async_runtime::block_on(async {
                // Endpoint for p2p networking
                let endpoint = Endpoint::builder()
                    .discovery_n0()
                    .bind()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;

                // In-memory blobs store and protocol
                let store = MemStore::new();
                let blobs = BlobsProtocol::new(&store, None);

                // Router to accept blobs connections
                let router = Router::builder(endpoint.clone())
                    .accept(iroh_blobs::ALPN, blobs)
                    .spawn();

                let inner = Arc::new(IrohInner {
                    endpoint,
                    store,
                    router,
                });
                // Set the shared state
                let _ = state.iroh.set(inner);

                Ok::<(), anyhow::Error>(())
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            process_files,
            iroh_send,
            iroh_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
