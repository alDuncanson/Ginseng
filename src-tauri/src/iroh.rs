use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Debug)]
pub(crate) struct IrohInner {
    endpoint: Endpoint,
    store: MemStore,
    #[allow(dead_code)]
    router: Router,
}

#[derive(Default)]
pub struct AppState {
    pub(crate) iroh: OnceCell<Arc<IrohInner>>,
}

impl AppState {
    fn get(&self) -> Result<&Arc<IrohInner>, String> {
        self.iroh
            .get()
            .ok_or_else(|| "Iroh not initialized yet".to_string())
    }
}

pub async fn setup_iroh(state: tauri::State<'_, AppState>) -> Result<(), anyhow::Error> {
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;
    let store = MemStore::new();
    let blobs = BlobsProtocol::new(&store, None);
    let router = Router::builder(endpoint.clone())
        .accept(iroh_blobs::ALPN, blobs)
        .spawn();

    let inner = Arc::new(IrohInner {
        endpoint,
        store,
        router,
    });
    let _ = state.iroh.set(inner);
    Ok(())
}

#[tauri::command]
pub async fn iroh_send(state: tauri::State<'_, AppState>, path: String) -> Result<String, String> {
    let inner = state.get()?.clone();
    let path: PathBuf = PathBuf::from(path);
    let tag = inner
        .store
        .blobs()
        .add_path(path)
        .await
        .map_err(|e| e.to_string())?;
    let node_id = inner.endpoint.node_id();
    let ticket = BlobTicket::new(node_id.into(), tag.hash, tag.format);

    Ok(ticket.to_string())
}

#[tauri::command]
pub async fn iroh_download(
    state: tauri::State<'_, AppState>,
    ticket: String,
    target: String,
) -> Result<(), String> {
    let inner = state.get()?.clone();
    let ticket: BlobTicket = ticket.parse::<BlobTicket>().map_err(|e| e.to_string())?;
    let target: PathBuf = PathBuf::from(target);
    let downloader = inner.store.downloader(&inner.endpoint);
    downloader
        .download(ticket.hash(), Some(ticket.node_addr().node_id))
        .await
        .map_err(|e| e.to_string())?;

    inner
        .store
        .blobs()
        .export(ticket.hash(), target)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
