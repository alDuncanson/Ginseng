use anyhow::Result;
use iroh::{protocol::Router, Endpoint, RelayMode};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol, Hash};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareMetadata {
    pub files: Vec<FileInfo>,
    pub share_type: ShareType,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShareType {
    SingleFile,
    MultipleFiles,
    Directory { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareBundle {
    pub metadata: ShareMetadata,
    pub metadata_hash: String,
}

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

    pub async fn share_files(&self, paths: Vec<PathBuf>) -> Result<String> {
        if paths.is_empty() {
            return Err(anyhow::anyhow!("No files provided"));
        }

        let mut metadata = ShareMetadata {
            files: Vec::new(),
            share_type: ShareType::SingleFile,
            total_size: 0,
        };

        if paths.len() == 1 {
            let path = &paths[0];
            let canonical_path = fs::canonicalize(path).await?;

            if canonical_path.is_file() {
                // Single file
                metadata.share_type = ShareType::SingleFile;
                self.add_single_file(&mut metadata, &canonical_path).await?;
            } else if canonical_path.is_dir() {
                // Directory
                let dir_name = canonical_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder")
                    .to_string();

                metadata.share_type = ShareType::Directory { name: dir_name };
                self.add_directory(&mut metadata, &canonical_path).await?;
            } else {
                return Err(anyhow::anyhow!("Path is neither a file nor a directory"));
            }
        } else {
            // Multiple files
            metadata.share_type = ShareType::MultipleFiles;
            for path in &paths {
                let canonical_path = fs::canonicalize(path).await?;
                if canonical_path.is_file() {
                    self.add_single_file(&mut metadata, &canonical_path).await?;
                } else {
                    return Err(anyhow::anyhow!(
                        "All paths must be files when sharing multiple items"
                    ));
                }
            }
        }

        // Store metadata as a blob
        let metadata_json = serde_json::to_string(&metadata)?;
        let metadata_bytes = metadata_json.as_bytes();
        let metadata_tag = self
            .blobs
            .store()
            .add_bytes(metadata_bytes.to_vec())
            .await?;

        let bundle = ShareBundle {
            metadata,
            metadata_hash: metadata_tag.hash.to_string(),
        };

        let bundle_json = serde_json::to_string(&bundle)?;
        let bundle_bytes = bundle_json.as_bytes();
        let bundle_tag = self.blobs.store().add_bytes(bundle_bytes.to_vec()).await?;

        let node_addr = self.endpoint.node_addr();
        let ticket = BlobTicket::new(node_addr, bundle_tag.hash, bundle_tag.format);

        Ok(ticket.to_string())
    }

    async fn add_single_file(&self, metadata: &mut ShareMetadata, file_path: &Path) -> Result<()> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_size = fs::metadata(file_path).await?.len();
        let tag = self.blobs.store().add_path(file_path).await?;

        metadata.files.push(FileInfo {
            name: file_name.clone(),
            relative_path: file_name,
            size: file_size,
            hash: tag.hash.to_string(),
        });

        metadata.total_size += file_size;
        Ok(())
    }

    async fn add_directory(&self, metadata: &mut ShareMetadata, dir_path: &Path) -> Result<()> {
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path
                    .strip_prefix(dir_path)?
                    .to_str()
                    .unwrap_or("unknown")
                    .to_string();

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let file_size = fs::metadata(path).await?.len();
                let tag = self.blobs.store().add_path(path).await?;

                metadata.files.push(FileInfo {
                    name: file_name,
                    relative_path: relative_path.clone(),
                    size: file_size,
                    hash: tag.hash.to_string(),
                });

                metadata.total_size += file_size;
            }
        }
        Ok(())
    }

    pub async fn download_files(&self, ticket_str: String) -> Result<(ShareMetadata, PathBuf)> {
        let ticket: BlobTicket = ticket_str.parse()?;

        let _connection = self
            .endpoint
            .connect(ticket.node_addr().clone(), iroh_blobs::protocol::ALPN)
            .await?;

        let downloader = self.store.downloader(&self.endpoint);

        // Download the bundle
        downloader
            .download(ticket.hash(), Some(ticket.node_addr().node_id))
            .await?;

        // Get the bundle content using temporary file
        let temp_bundle_path =
            std::env::temp_dir().join(format!("ginseng_bundle_{}", ticket.hash()));
        self.blobs.export(ticket.hash(), &temp_bundle_path).await?;
        let bundle_json = fs::read_to_string(&temp_bundle_path).await?;
        let bundle: ShareBundle = serde_json::from_str(&bundle_json)?;

        // Clean up temp bundle file
        fs::remove_file(&temp_bundle_path).await?;

        // Download all individual files
        for file_info in &bundle.metadata.files {
            let file_hash: Hash = file_info.hash.parse()?;
            downloader
                .download(file_hash, Some(ticket.node_addr().node_id))
                .await?;
        }

        // Get downloads directory
        let downloads_dir = self.get_downloads_directory()?;

        // Create target directory based on share type
        let target_dir = match &bundle.metadata.share_type {
            ShareType::SingleFile => downloads_dir.clone(),
            ShareType::MultipleFiles => {
                let target =
                    downloads_dir.join(format!("ginseng_files_{}", chrono::Utc::now().timestamp()));
                fs::create_dir_all(&target).await?;
                target
            }
            ShareType::Directory { name } => {
                let target = downloads_dir.join(name);
                fs::create_dir_all(&target).await?;
                target
            }
        };

        // Export files to their final locations
        for file_info in &bundle.metadata.files {
            let file_hash: Hash = file_info.hash.parse()?;
            let target_path = target_dir.join(&file_info.relative_path);

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Export the file
            self.blobs.export(file_hash, &target_path).await?;
        }

        Ok((bundle.metadata, target_dir))
    }

    fn get_downloads_directory(&self) -> Result<PathBuf> {
        if let Some(downloads_dir) = dirs::download_dir() {
            Ok(downloads_dir)
        } else if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir.join("Downloads"))
        } else {
            Ok(std::env::current_dir()?.join("downloads"))
        }
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
