use anyhow::Result;
use iroh::{protocol::Router, Endpoint, RelayMode};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol};
use std::path::PathBuf;
use tokio::fs;

pub struct GinsengCore {
    pub endpoint: Endpoint,
    pub store: MemStore,
    pub blobs: BlobsProtocol,
    pub _router: Router,
}

impl GinsengCore {
    pub async fn new() -> Result<Self> {
        let endpoint = Endpoint::builder()
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .discovery_n0()
            .bind()
            .await?;

        let store = MemStore::new();

        let blobs = BlobsProtocol::new(&store, None);

        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(iroh_blobs::protocol::ALPN, blobs.clone())
            .spawn();

        Ok(Self {
            endpoint,
            store,
            blobs,
            _router: router,
        })
    }

    pub async fn share_file(&self, path: PathBuf) -> Result<String> {
        let canonical_path = fs::canonicalize(&path).await?;

        let tag = self.blobs.store().add_path(canonical_path).await?;

        let node_addr = self.endpoint.node_addr();

        let ticket = BlobTicket::new(node_addr, tag.hash, tag.format);

        Ok(ticket.to_string())
    }

    pub async fn download_file(&self, ticket_str: String, target: PathBuf) -> Result<()> {
        let ticket: BlobTicket = ticket_str.parse()?;

        let _connection = self
            .endpoint
            .connect(ticket.node_addr().clone(), iroh_blobs::protocol::ALPN)
            .await?;

        let downloader = self.store.downloader(&self.endpoint);

        downloader
            .download(ticket.hash(), Some(ticket.node_addr().node_id))
            .await?;

        if let Some(parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        let canonical_target = if let Some(parent) = target.parent() {
            let canonical_parent = fs::canonicalize(parent).await?;
            canonical_parent.join(
                target
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("downloaded_file")),
            )
        } else {
            let current_dir = std::env::current_dir()?;
            current_dir.join(
                target
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("downloaded_file")),
            )
        };

        self.blobs.export(ticket.hash(), canonical_target).await?;

        Ok(())
    }

    pub async fn node_info(&self) -> Result<String> {
        let node_id = self.endpoint.node_id();

        let endpoint_addr = self.endpoint.node_addr();

        Ok(format!(
            "Node ID: {}\nDirect addresses: {:?}\nRelay URL: {:?}",
            node_id,
            endpoint_addr.direct_addresses().collect::<Vec<_>>(),
            endpoint_addr.relay_url()
        ))
    }
}
