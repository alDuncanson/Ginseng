use crate::progress::{
    FileProgress, FileStatus, ProgressEvent, ProgressTracker, RateLimiter, TransferStage,
    TransferType,
};
use crate::utils::{
    calculate_relative_path, calculate_total_size, extract_directory_name, extract_file_name,
    get_downloads_directory, validate_paths_not_empty,
};
use anyhow::Result;

use futures::stream::{self, StreamExt};
use iroh::{endpoint::Connection, protocol::Router, Endpoint, RelayMode};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol, Hash};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::ipc::Channel;
use tokio::fs;
use walkdir::WalkDir;

/// Information about a file being shared or downloaded.
///
/// Contains metadata needed to reconstruct the file on the receiving end,
/// including its content hash for verification and relative path for proper placement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    /// The file name (e.g., "document.pdf")
    pub name: String,
    /// The relative path from the share root (e.g., "folder/document.pdf")
    pub relative_path: String,
    /// File size in bytes
    pub size: u64,
    /// Content-addressed hash for retrieving the file from the blob store
    pub hash: String,
}

/// The type of content being shared, which affects how files are organized on download.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShareType {
    /// A single file is being shared
    SingleFile,
    /// Multiple individual files are being shared (will be placed in a timestamped folder)
    MultipleFiles,
    /// A directory is being shared (will preserve the directory name)
    Directory {
        /// The name of the directory being shared
        name: String,
    },
}

/// Metadata describing what is being shared.
///
/// This contains all the information needed to download and reconstruct
/// the shared content on the receiving end.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShareMetadata {
    /// List of all files included in this share
    pub files: Vec<FileInfo>,
    /// The type of share (single file, multiple files, or directory)
    pub share_type: ShareType,
    /// Total size of all files in bytes
    pub total_size: u64,
}

/// A complete share bundle containing metadata and its verification hash.
///
/// This is the top-level structure that gets stored as a blob and referenced
/// by the share ticket. It enables integrity verification of the metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareBundle {
    /// The share metadata containing file information
    pub metadata: ShareMetadata,
    /// Hash of the metadata for integrity verification
    pub metadata_hash: String,
}

/// Core functionality for peer-to-peer file sharing using Iroh.
///
/// This struct encapsulates all the networking and storage components needed
/// for sharing and downloading files in a decentralized manner. It handles
/// the entire lifecycle from file ingestion to ticket generation for sharing,
/// and from ticket parsing to file reconstruction for downloading.
pub struct GinsengCore {
    /// Iroh endpoint for P2P networking
    pub endpoint: Endpoint,
    /// In-memory blob store for content-addressed storage
    pub store: MemStore,
    /// Protocol handler for blob operations (upload/download)
    pub blobs: BlobsProtocol,
    /// Router for handling incoming connections and protocol routing
    pub router: Router,
}

impl GinsengCore {
    /// Creates a new GinsengCore instance with default configuration.
    ///
    /// Sets up the Iroh endpoint with relay discovery, creates an in-memory blob store,
    /// and initializes the protocol router for handling P2P connections.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint cannot be created or bound to a port.
    pub async fn new() -> Result<Self> {
        let endpoint = create_endpoint().await?;
        let store = MemStore::new();
        let blobs = BlobsProtocol::new(&store, None);
        let router = create_router(&endpoint, &blobs);

        Ok(Self {
            endpoint,
            store,
            blobs,
            router,
        })
    }

    /// Returns information about this node's network configuration.
    ///
    /// Provides details about the node ID, direct addresses, and relay URL
    /// for debugging and network diagnostics.
    pub async fn node_info(&self) -> Result<String> {
        format_node_info(&self.endpoint)
    }

    /// Shares files with parallel processing and real-time progress updates
    ///
    /// Processes multiple files concurrently using tokio, providing streaming
    /// progress updates through the channel for each file and overall transfer.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel for sending progress events to the frontend
    /// * `paths` - Vector of file or directory paths to share
    ///
    /// # Returns
    ///
    /// A ticket string that can be shared to download the files
    ///
    /// # Errors
    ///
    /// Returns an error if paths are invalid, files cannot be read, or blob storage fails
    pub async fn share_files_parallel(
        &self,
        channel: Channel<ProgressEvent>,
        paths: Vec<PathBuf>,
    ) -> Result<String> {
        validate_paths_not_empty(&paths)?;

        let tracker = ProgressTracker::new(uuid::Uuid::new_v4().to_string(), TransferType::Upload);
        let rate_limiter = RateLimiter::new(Duration::from_millis(100));

        // Send initial event
        channel
            .send(ProgressEvent::TransferStarted {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        tracker.set_stage(TransferStage::Initializing).await;

        // Collect file paths to process
        let file_paths = collect_file_paths(&paths).await?;

        // Initialize file progress entries
        for (file_path, base_path) in &file_paths {
            let name = extract_file_name(file_path);
            let relative_path = calculate_relative_path(file_path, base_path)?;
            let size = get_file_size(file_path).await?;
            tracker
                .add_file(FileProgress::new(name, relative_path, size))
                .await;
        }

        channel
            .send(ProgressEvent::TransferProgress {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        tracker.set_stage(TransferStage::Transferring).await;

        // Process files in parallel with streaming progress
        let upload_concurrency = std::cmp::min(8, num_cpus::get());
        let (file_info_tx, mut file_info_rx) = tokio::sync::mpsc::unbounded_channel::<FileInfo>();
        let channel = Arc::new(channel);

        // Create file entries with IDs
        let snapshot = tracker.get_snapshot().await;
        let file_entries: Vec<_> = file_paths
            .iter()
            .enumerate()
            .map(|(idx, (path, base))| {
                (
                    path.clone(),
                    base.clone(),
                    snapshot.files[idx].file_id.clone(),
                )
            })
            .collect();

        // Process files concurrently
        let blobs = self.blobs.clone();
        let tracker_clone = tracker.clone();
        let limiter_clone = rate_limiter.clone();
        let channel_clone = channel.clone();
        let tx_clone = file_info_tx.clone();

        stream::iter(file_entries)
            .for_each_concurrent(upload_concurrency, move |(path, base, file_id)| {
                let blobs = blobs.clone();
                let tracker = tracker_clone.clone();
                let channel = channel_clone.clone();
                let limiter = limiter_clone.clone();
                let tx = tx_clone.clone();

                async move {
                    if let Err(e) =
                        upload_one_file(path, base, file_id, blobs, tracker, channel, limiter, tx)
                            .await
                    {
                        eprintln!("Upload failed: {}", e);
                    }
                }
            })
            .await;

        // Close sender and collect results
        drop(file_info_tx);
        let mut file_infos = Vec::new();
        while let Some(info) = file_info_rx.recv().await {
            file_infos.push(info);
        }

        let total_size = calculate_total_size(file_infos.iter().map(|f| f.size));
        let share_type = determine_share_type(&paths, &file_infos);

        let metadata = ShareMetadata {
            files: file_infos,
            share_type,
            total_size,
        };

        tracker.set_stage(TransferStage::Finalizing).await;

        let metadata_hash = store_metadata_as_blob(&self.blobs, &metadata).await?;
        let bundle = ShareBundle {
            metadata,
            metadata_hash,
        };
        let (bundle_hash, bundle_format) = store_bundle_as_blob(&self.blobs, &bundle).await?;
        let ticket = create_share_ticket(&self.endpoint, &bundle_hash, &bundle_format)?;

        tracker.complete().await;
        channel
            .send(ProgressEvent::TransferCompleted {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        Ok(ticket)
    }

    /// Downloads files with parallel processing and real-time progress updates
    ///
    /// Parses the ticket, connects to the peer, downloads all files, and provides
    /// streaming progress updates for each file and the overall transfer.
    ///
    /// # Arguments
    ///
    /// * `channel` - Channel for sending progress events to the frontend
    /// * `ticket_str` - The ticket string received from the sender
    ///
    /// # Returns
    ///
    /// A tuple containing the share metadata and the path where files were saved
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid, connection fails, or downloads fail
    pub async fn download_files_parallel(
        &self,
        channel: Channel<ProgressEvent>,
        ticket_str: String,
    ) -> Result<(ShareMetadata, PathBuf)> {
        let tracker =
            ProgressTracker::new(uuid::Uuid::new_v4().to_string(), TransferType::Download);
        let rate_limiter = RateLimiter::new(Duration::from_millis(100));

        channel
            .send(ProgressEvent::TransferStarted {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        tracker.set_stage(TransferStage::Connecting).await;

        let ticket = parse_ticket(&ticket_str)?;
        let bundle =
            download_and_parse_bundle(&self.endpoint, &self.blobs, &self.store, &ticket).await?;

        let target_directory = determine_target_directory(&bundle.metadata)?;

        // Initialize file progress
        for file_info in &bundle.metadata.files {
            tracker
                .add_file(FileProgress::new(
                    file_info.name.clone(),
                    file_info.relative_path.clone(),
                    file_info.size,
                ))
                .await;
        }

        tracker.set_stage(TransferStage::Transferring).await;
        channel
            .send(ProgressEvent::TransferProgress {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        // Download files in parallel with streaming progress
        let download_concurrency = 6;
        let channel = Arc::new(channel);

        // Create file entries with IDs
        let snapshot = tracker.get_snapshot().await;
        let file_entries: Vec<_> = bundle
            .metadata
            .files
            .iter()
            .enumerate()
            .map(|(idx, file)| (file.clone(), snapshot.files[idx].file_id.clone()))
            .collect();

        // Process downloads concurrently
        let endpoint = self.endpoint.clone();
        let blobs = self.blobs.clone();
        let tracker_clone = tracker.clone();
        let limiter_clone = rate_limiter.clone();
        let channel_clone = channel.clone();
        let peer_id = ticket.addr().id;
        let target_dir = target_directory.clone();

        stream::iter(file_entries)
            .for_each_concurrent(download_concurrency, move |(file_info, file_id)| {
                let endpoint = endpoint.clone();
                let blobs = blobs.clone();
                let tracker = tracker_clone.clone();
                let channel = channel_clone.clone();
                let limiter = limiter_clone.clone();
                let target_dir = target_dir.clone();

                async move {
                    if let Err(e) = download_one_file(
                        file_info, file_id, endpoint, blobs, peer_id, target_dir, tracker, channel,
                        limiter,
                    )
                    .await
                    {
                        eprintln!("Download failed: {}", e);
                    }
                }
            })
            .await;

        tracker.complete().await;
        channel
            .send(ProgressEvent::TransferCompleted {
                transfer: tracker.get_snapshot().await,
            })
            .ok();

        Ok((bundle.metadata, target_directory))
    }

    /// CLI version - share files without progress tracking
    pub async fn share_files_cli(&self, paths: Vec<PathBuf>) -> Result<String> {
        validate_paths_not_empty(&paths)?;
        let metadata = create_share_metadata(&self.blobs, &paths).await?;
        let metadata_hash = store_metadata_as_blob(&self.blobs, &metadata).await?;
        let bundle = ShareBundle {
            metadata,
            metadata_hash,
        };
        let (bundle_hash, bundle_format) = store_bundle_as_blob(&self.blobs, &bundle).await?;
        create_share_ticket(&self.endpoint, &bundle_hash, &bundle_format)
    }

    /// CLI version - download files without progress tracking
    pub async fn download_files_cli(&self, ticket_str: String) -> Result<(ShareMetadata, PathBuf)> {
        let ticket = parse_ticket(&ticket_str)?;
        let bundle =
            download_and_parse_bundle(&self.endpoint, &self.blobs, &self.store, &ticket).await?;
        let target_directory = determine_target_directory(&bundle.metadata)?;
        download_all_files(
            &self.endpoint,
            &self.blobs,
            &bundle.metadata,
            &target_directory,
            &ticket,
        )
        .await?;
        Ok((bundle.metadata, target_directory))
    }

    /// Gracefully shuts down the router and endpoint.
    ///
    /// This should be called before ending the process to ensure proper cleanup
    /// of network resources and connections. Following Iroh's Router documentation
    /// recommendations for graceful shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if the router shutdown fails.
    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }
}

/// Creates and configures an Iroh endpoint for P2P networking.
///
/// Sets up the endpoint with blob protocol support, default relay mode,
/// and n0 discovery for finding peers on the network.
async fn create_endpoint() -> Result<Endpoint> {
    Endpoint::builder()
        .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
        .relay_mode(RelayMode::Default)
        .bind()
        .await
        .map_err(|error| anyhow::anyhow!("Failed to create endpoint: {}", error))
}

/// Creates a protocol router that handles incoming blob protocol connections.
///
/// The router accepts connections using the blob protocol ALPN and routes
/// them to the appropriate blob protocol handler.
fn create_router(endpoint: &Endpoint, blobs: &BlobsProtocol) -> Router {
    iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_blobs::protocol::ALPN, blobs.clone())
        .spawn()
}

/// Creates share metadata based on the number and type of paths provided.
///
/// Uses different strategies:
/// - Single path: Detects if it's a file or directory and handles accordingly
/// - Multiple paths: Validates all are files and creates a multiple files share
async fn create_share_metadata(blobs: &BlobsProtocol, paths: &[PathBuf]) -> Result<ShareMetadata> {
    if paths.len() == 1 {
        create_single_path_metadata(blobs, &paths[0]).await
    } else {
        create_multiple_files_metadata(blobs, paths).await
    }
}

/// Creates metadata for a single file or directory path.
///
/// Canonicalizes the path and determines whether it's a file or directory,
/// then delegates to the appropriate metadata creation function.
async fn create_single_path_metadata(blobs: &BlobsProtocol, path: &Path) -> Result<ShareMetadata> {
    let canonical_path = fs::canonicalize(path).await?;

    match (canonical_path.is_file(), canonical_path.is_dir()) {
        (true, false) => create_single_file_metadata(blobs, &canonical_path).await,
        (false, true) => create_directory_metadata(blobs, &canonical_path).await,
        _ => anyhow::bail!("Path is neither a file nor a directory"),
    }
}

/// Creates metadata for sharing a single file.
///
/// Stores the file as a blob and creates a ShareMetadata with SingleFile type.
async fn create_single_file_metadata(
    blobs: &BlobsProtocol,
    file_path: &Path,
) -> Result<ShareMetadata> {
    let file_info = create_file_info(blobs, file_path, file_path).await?;

    Ok(ShareMetadata {
        files: vec![file_info.clone()],
        share_type: ShareType::SingleFile,
        total_size: file_info.size,
    })
}

/// Creates metadata for sharing an entire directory.
///
/// Recursively walks the directory, stores all files as blobs,
/// and creates metadata preserving the directory structure.
async fn create_directory_metadata(
    blobs: &BlobsProtocol,
    dir_path: &Path,
) -> Result<ShareMetadata> {
    let directory_name = extract_directory_name(dir_path);
    let file_infos = collect_directory_files(blobs, dir_path).await?;
    let total_size = calculate_total_size(file_infos.iter().map(|f| f.size));

    Ok(ShareMetadata {
        files: file_infos,
        share_type: ShareType::Directory {
            name: directory_name,
        },
        total_size,
    })
}

/// Creates metadata for sharing multiple individual files.
///
/// Validates that all paths are files (no directories allowed in multi-file shares),
/// stores each file as a blob, and creates metadata with MultipleFiles type.
async fn create_multiple_files_metadata(
    blobs: &BlobsProtocol,
    paths: &[PathBuf],
) -> Result<ShareMetadata> {
    validate_all_paths_are_files(paths).await?;

    let mut file_infos = Vec::new();
    for path in paths {
        let canonical_path = fs::canonicalize(path).await?;
        let file_info = create_file_info(blobs, &canonical_path, &canonical_path).await?;
        file_infos.push(file_info);
    }

    let total_size = calculate_total_size(file_infos.iter().map(|f| f.size));

    Ok(ShareMetadata {
        files: file_infos,
        share_type: ShareType::MultipleFiles,
        total_size,
    })
}

/// Validates that all provided paths are files, not directories.
///
/// Used for multiple file sharing to ensure consistent behavior.
///
/// # Errors
///
/// Returns an error if any path is not a file.
async fn validate_all_paths_are_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let canonical_path = fs::canonicalize(path).await?;
        if !canonical_path.is_file() {
            anyhow::bail!("All paths must be files when sharing multiple items");
        }
    }
    Ok(())
}

/// Creates FileInfo metadata for a single file.
///
/// Extracts the file name, calculates the relative path from the base path,
/// gets the file size, and stores the file content as a blob.
///
/// # Arguments
///
/// * `file_path` - The absolute path to the file
/// * `base_path` - The base path for calculating relative paths
async fn create_file_info(
    blobs: &BlobsProtocol,
    file_path: &Path,
    base_path: &Path,
) -> Result<FileInfo> {
    let file_name = extract_file_name(file_path);
    let relative_path = calculate_relative_path(file_path, base_path)?;
    let file_size = get_file_size(file_path).await?;
    let file_hash = store_file_as_blob(blobs, file_path).await?;

    Ok(FileInfo {
        name: file_name,
        relative_path,
        size: file_size,
        hash: file_hash,
    })
}

/// Gets the size of a file in bytes.
async fn get_file_size(file_path: &Path) -> Result<u64> {
    fs::metadata(file_path)
        .await
        .map(|metadata| metadata.len())
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to get file size for '{}': {}",
                file_path.display(),
                error
            )
        })
}

/// Downloads a single file with streaming progress updates
///
/// Processes download progress events and updates the tracker in real-time
async fn download_one_file(
    file_info: FileInfo,
    file_id: String,
    endpoint: Endpoint,
    blobs: BlobsProtocol,
    peer_id: iroh::EndpointId,
    target_dir: PathBuf,
    tracker: ProgressTracker,
    channel: Arc<Channel<ProgressEvent>>,
    limiter: RateLimiter,
) -> Result<()> {
    use iroh_blobs::api::downloader::DownloadProgressItem as DP;

    tracker
        .update_file(&file_id, |f| {
            f.status = FileStatus::Transferring;
        })
        .await;

    let file_hash: Hash = file_info
        .hash
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid hash: {}", e))?;

    // Start streaming download
    let downloader = blobs.store().downloader(&endpoint);
    let download = downloader.download(file_hash, Some(peer_id));
    let mut stream = download.stream().await?;

    while let Some(evt) = stream.next().await {
        match evt {
            DP::Progress(total_bytes) => {
                let transferred = total_bytes.min(file_info.size);
                tracker
                    .update_file(&file_id, |f| {
                        f.transferred_bytes = transferred;
                    })
                    .await;
            }
            DP::Error(e) => {
                return Err(anyhow::anyhow!("Download error: {}", e));
            }
            DP::DownloadError => {
                return Err(anyhow::anyhow!(
                    "Download failed for file '{}'",
                    file_info.name
                ));
            }
            _ => {
                // Handle other events like TryProvider, ProviderFailed, PartComplete
            }
        }

        // Rate-limited progress updates
        if limiter.should_emit().await {
            channel
                .send(ProgressEvent::TransferProgress {
                    transfer: tracker.get_snapshot().await,
                })
                .ok();
        }
    }

    // Stream complete - export to filesystem
    let target_path = target_dir.join(&file_info.relative_path);
    ensure_parent_directory_exists(&target_path).await?;
    blobs
        .export(file_hash, &target_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to export file '{}': {}", file_info.name, e))?;

    tracker
        .update_file(&file_id, |f| {
            f.transferred_bytes = f.total_bytes;
            f.status = FileStatus::Completed;
        })
        .await;

    // Force emit on completion
    limiter.force_emit().await;
    channel
        .send(ProgressEvent::TransferProgress {
            transfer: tracker.get_snapshot().await,
        })
        .ok();

    Ok(())
}

/// Uploads a single file with streaming progress updates
///
/// Processes add_path progress events and updates the tracker in real-time
async fn upload_one_file(
    path: PathBuf,
    base: PathBuf,
    file_id: String,
    blobs: BlobsProtocol,
    tracker: ProgressTracker,
    channel: Arc<Channel<ProgressEvent>>,
    limiter: RateLimiter,
    fileinfo_tx: tokio::sync::mpsc::UnboundedSender<FileInfo>,
) -> Result<()> {
    use iroh_blobs::api::blobs::AddProgressItem;

    tracker
        .update_file(&file_id, |f| {
            f.status = FileStatus::Transferring;
        })
        .await;

    let name = extract_file_name(&path);
    let relative_path = calculate_relative_path(&path, &base)?;

    // Start streaming add
    let add_progress = blobs.store().add_path(path.clone());
    let mut stream = add_progress.stream().await;

    let mut copy_bytes = 0u64;
    let mut outboard_bytes = 0u64;
    let mut total = None;

    while let Some(evt) = stream.next().await {
        match evt {
            AddProgressItem::Size(sz) => {
                total = Some(sz);
                tracker
                    .update_file(&file_id, |f| {
                        if f.total_bytes == 0 {
                            f.total_bytes = sz;
                        }
                    })
                    .await;
            }
            AddProgressItem::CopyProgress(off) => {
                copy_bytes = off;
                if let Some(t) = total {
                    let transferred = copy_bytes.max(outboard_bytes).min(t);
                    tracker
                        .update_file(&file_id, |f| {
                            f.transferred_bytes = transferred;
                        })
                        .await;
                }
            }
            AddProgressItem::OutboardProgress(off) => {
                outboard_bytes = off;
                if let Some(t) = total {
                    let transferred = copy_bytes.max(outboard_bytes).min(t);
                    tracker
                        .update_file(&file_id, |f| {
                            f.transferred_bytes = transferred;
                        })
                        .await;
                }
            }
            AddProgressItem::CopyDone => {
                // Optional: could emit status change here
            }
            AddProgressItem::Done(tag) => {
                let hash = tag.hash().to_string();
                let size = total.unwrap_or(0);

                tracker
                    .update_file(&file_id, |f| {
                        f.transferred_bytes = f.total_bytes;
                        f.status = FileStatus::Completed;
                    })
                    .await;

                // Force emit on completion
                limiter.force_emit().await;
                channel
                    .send(ProgressEvent::TransferProgress {
                        transfer: tracker.get_snapshot().await,
                    })
                    .ok();

                let info = FileInfo {
                    name: name.clone(),
                    relative_path: relative_path.clone(),
                    size,
                    hash,
                };
                fileinfo_tx.send(info).ok();
            }
            AddProgressItem::Error(e) => {
                return Err(anyhow::anyhow!("Add progress error: {}", e));
            }
        }

        // Rate-limited progress updates
        if limiter.should_emit().await {
            channel
                .send(ProgressEvent::TransferProgress {
                    transfer: tracker.get_snapshot().await,
                })
                .ok();
        }
    }

    Ok(())
}

/// Stores a file as a content-addressed blob and returns its hash.
///
/// The file is read and stored in the blob store, returning a hash
/// that can be used to retrieve the content later.
async fn store_file_as_blob(blobs: &BlobsProtocol, file_path: &Path) -> Result<String> {
    blobs
        .store()
        .add_path(file_path)
        .await
        .map(|tag| tag.hash.to_string())
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to store file '{}' as blob: {}",
                file_path.display(),
                error
            )
        })
}

/// Recursively collects all files in a directory and creates FileInfo for each.
///
/// Uses WalkDir to traverse the directory tree and processes only regular files,
/// creating FileInfo structures with paths relative to the directory root.
async fn collect_directory_files(blobs: &BlobsProtocol, dir_path: &Path) -> Result<Vec<FileInfo>> {
    let mut file_infos = Vec::new();

    for entry in WalkDir::new(dir_path).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            let file_info = create_file_info(blobs, path, dir_path).await?;
            file_infos.push(file_info);
        }
    }

    Ok(file_infos)
}

/// Collects all file paths from the given paths (files and directories)
async fn collect_file_paths(paths: &[PathBuf]) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut file_paths = Vec::new();

    for path in paths {
        let canonical = fs::canonicalize(path).await?;
        if canonical.is_file() {
            file_paths.push((canonical.clone(), canonical.clone()));
        } else if canonical.is_dir() {
            for entry in WalkDir::new(&canonical).into_iter().filter_map(Result::ok) {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    file_paths.push((entry_path.to_path_buf(), canonical.clone()));
                }
            }
        }
    }

    Ok(file_paths)
}

/// Determines share type from paths and file infos
fn determine_share_type(paths: &[PathBuf], file_infos: &[FileInfo]) -> ShareType {
    if paths.len() == 1 {
        let path = &paths[0];
        if path.is_file() {
            ShareType::SingleFile
        } else {
            ShareType::Directory {
                name: extract_directory_name(path),
            }
        }
    } else if file_infos.len() == 1 {
        ShareType::SingleFile
    } else {
        ShareType::MultipleFiles
    }
}

/// Serializes share metadata to JSON and stores it as a blob.
async fn store_metadata_as_blob(blobs: &BlobsProtocol, metadata: &ShareMetadata) -> Result<String> {
    let metadata_json = serde_json::to_string(metadata)?;
    store_json_as_blob(blobs, &metadata_json).await
}

/// Serializes a share bundle to JSON and stores it as a blob.
///
/// Returns both the hash and format information needed to create a ticket.
async fn store_bundle_as_blob(
    blobs: &BlobsProtocol,
    bundle: &ShareBundle,
) -> Result<(Hash, iroh_blobs::BlobFormat)> {
    let bundle_json = serde_json::to_string(bundle)?;
    let add_progress = blobs.store().add_bytes(bundle_json.into_bytes());
    let tag = add_progress
        .await
        .map_err(|error| anyhow::anyhow!("Failed to store bundle as blob: {}", error))?;
    Ok((tag.hash, tag.format))
}

/// Stores a JSON string as a blob and returns its hash.
async fn store_json_as_blob(blobs: &BlobsProtocol, json: &str) -> Result<String> {
    let add_progress = blobs.store().add_bytes(json.as_bytes().to_vec());
    let tag = add_progress
        .await
        .map_err(|error| anyhow::anyhow!("Failed to store JSON as blob: {}", error))?;
    Ok(tag.hash.to_string())
}

/// Creates a shareable ticket string from a bundle hash and format.
///
/// The ticket contains the node address and blob information needed
/// for others to download the shared content.
fn create_share_ticket(
    endpoint: &Endpoint,
    bundle_hash: &Hash,
    bundle_format: &iroh_blobs::BlobFormat,
) -> Result<String> {
    let endpoint_addr = endpoint.addr();
    let ticket = BlobTicket::new(endpoint_addr, *bundle_hash, *bundle_format);
    Ok(ticket.to_string())
}

/// Parses a ticket string into a BlobTicket structure.
fn parse_ticket(ticket_str: &str) -> Result<BlobTicket> {
    ticket_str
        .parse::<BlobTicket>()
        .map_err(|error| anyhow::anyhow!("Failed to parse ticket: {}", error))
}

/// Downloads a bundle from a peer and parses it into a ShareBundle.
///
/// Establishes a connection to the peer, downloads the bundle blob,
/// exports it to a temporary file, parses the JSON, and cleans up.
async fn download_and_parse_bundle(
    endpoint: &Endpoint,
    blobs: &BlobsProtocol,
    store: &MemStore,
    ticket: &BlobTicket,
) -> Result<ShareBundle> {
    let _connection = establish_connection(endpoint, ticket).await?;
    download_blob(endpoint, store, ticket).await?;
    parse_bundle_from_blob(blobs, ticket).await
}

/// Establishes a P2P connection to the node specified in the ticket.
async fn establish_connection(endpoint: &Endpoint, ticket: &BlobTicket) -> Result<Connection> {
    endpoint
        .connect(ticket.addr().clone(), iroh_blobs::protocol::ALPN)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to establish connection: {}", error))
}

/// Downloads a blob from a peer into the local store.
async fn download_blob(endpoint: &Endpoint, store: &MemStore, ticket: &BlobTicket) -> Result<()> {
    let downloader = store.downloader(endpoint);
    downloader
        .download(ticket.hash(), Some(ticket.addr().id))
        .await
        .map_err(|error| anyhow::anyhow!("Failed to download blob: {}", error))
}

/// Exports a blob to a temporary file, parses it as JSON, and cleans up.
async fn parse_bundle_from_blob(blobs: &BlobsProtocol, ticket: &BlobTicket) -> Result<ShareBundle> {
    let temp_bundle_path = create_temp_bundle_path(ticket);
    blobs.export(ticket.hash(), &temp_bundle_path).await?;

    let bundle_json = fs::read_to_string(&temp_bundle_path).await?;
    let bundle = serde_json::from_str(&bundle_json)?;

    fs::remove_file(&temp_bundle_path).await?;
    Ok(bundle)
}

/// Creates a temporary file path for bundle extraction using the ticket hash.
fn create_temp_bundle_path(ticket: &BlobTicket) -> PathBuf {
    std::env::temp_dir().join(format!("ginseng_bundle_{}", ticket.hash()))
}

/// Determines where to save downloaded files based on the share type.
///
/// - Single file: Downloads directory
/// - Multiple files: Timestamped subdirectory in Downloads
/// - Directory: Named subdirectory in Downloads
fn determine_target_directory(metadata: &ShareMetadata) -> Result<PathBuf> {
    let downloads_dir = get_downloads_directory()?;

    let target_dir = match &metadata.share_type {
        ShareType::SingleFile => downloads_dir,
        ShareType::MultipleFiles => {
            let timestamp = chrono::Utc::now().timestamp();
            downloads_dir.join(format!("ginseng_files_{}", timestamp))
        }
        ShareType::Directory { name } => downloads_dir.join(name),
    };

    Ok(target_dir)
}

/// Downloads all files referenced in the metadata to the target directory.
///
/// Uses a two-phase approach:
/// 1. Download all file blobs to ensure they're available
/// 2. Export all files to their target locations with proper directory structure
async fn download_all_files(
    endpoint: &Endpoint,
    blobs: &BlobsProtocol,
    metadata: &ShareMetadata,
    target_dir: &Path,
    ticket: &BlobTicket,
) -> Result<()> {
    let downloader = blobs.store().downloader(endpoint);

    for file_info in &metadata.files {
        let file_hash: Hash = file_info.hash.parse::<Hash>().map_err(|error| {
            anyhow::anyhow!("Invalid hash for file '{}': {}", file_info.name, error)
        })?;

        downloader
            .download(file_hash, Some(ticket.addr().id))
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "Failed to download file '{}' ({}): {}",
                    file_info.name,
                    file_hash,
                    error
                )
            })?;
    }

    for file_info in &metadata.files {
        export_individual_file(blobs, file_info, target_dir)
            .await
            .map_err(|error| {
                anyhow::anyhow!("Failed to export file '{}': {}", file_info.name, error)
            })?;
    }

    Ok(())
}

/// Exports a single file from the blob store to its target location.
///
/// Creates necessary parent directories and exports the file using
/// its relative path to maintain directory structure.
async fn export_individual_file(
    blobs: &BlobsProtocol,
    file_info: &FileInfo,
    target_dir: &Path,
) -> Result<()> {
    let file_hash: Hash = file_info.hash.parse::<Hash>().map_err(|error| {
        anyhow::anyhow!("Invalid hash for file '{}': {}", file_info.name, error)
    })?;
    let target_file_path = target_dir.join(&file_info.relative_path);

    ensure_parent_directory_exists(&target_file_path)
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to create directory for '{}': {}",
                file_info.relative_path,
                error
            )
        })?;

    blobs
        .export(file_hash, &target_file_path)
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "Failed to export '{}' to '{}': {}",
                file_info.name,
                target_file_path.display(),
                error
            )
        })?;

    Ok(())
}

/// Ensures that the parent directory of a file path exists.
///
/// Creates all necessary parent directories if they don't exist.
async fn ensure_parent_directory_exists(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(())
}

/// Formats node information for display, including ID, addresses, and relay.
fn format_node_info(endpoint: &Endpoint) -> Result<String> {
    let endpoint_id = endpoint.id();
    let endpoint_addr = endpoint.addr();

    Ok(format!(
        "Endpoint ID: {}\nDirect addresses: {:?}\nRelay URL: {:?}",
        endpoint_id,
        endpoint_addr.ip_addrs().collect::<Vec<_>>(),
        endpoint_addr.relay_urls().next()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_determine_target_directory_single_file() {
        let metadata = ShareMetadata {
            files: vec![],
            share_type: ShareType::SingleFile,
            total_size: 0,
        };

        let result = determine_target_directory(&metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_determine_target_directory_multiple_files() {
        let metadata = ShareMetadata {
            files: vec![],
            share_type: ShareType::MultipleFiles,
            total_size: 0,
        };

        let result = determine_target_directory(&metadata);
        assert!(result.is_ok());
        assert!(result.unwrap().to_string_lossy().contains("ginseng_files_"));
    }

    #[test]
    fn test_determine_target_directory_directory() {
        let metadata = ShareMetadata {
            files: vec![],
            share_type: ShareType::Directory {
                name: "test_folder".to_string(),
            },
            total_size: 0,
        };

        let result = determine_target_directory(&metadata);
        assert!(result.is_ok());
        assert!(result.unwrap().to_string_lossy().ends_with("test_folder"));
    }

    #[test]
    fn test_create_temp_bundle_path() {
        let ticket_str = "blobafkfrvhakfhakfhakfhakfhakfhakfhfkafkafkafka";
        let ticket: BlobTicket = ticket_str.parse::<BlobTicket>().unwrap_or_else(|_| {
            let temp_dir = TempDir::new().unwrap();
            let temp_file = temp_dir.path().join("temp_ticket");
            std::fs::write(&temp_file, "dummy").unwrap();

            let dummy_hash = iroh_blobs::Hash::new([0u8; 32]);
            let dummy_endpoint_id = iroh::EndpointId::from_bytes(&[1u8; 32]).unwrap();
            let dummy_addr = iroh::EndpointAddr::new(dummy_endpoint_id);
            BlobTicket::new(dummy_addr, dummy_hash, iroh_blobs::BlobFormat::Raw)
        });

        let path = create_temp_bundle_path(&ticket);
        assert!(path.to_string_lossy().contains("ginseng_bundle_"));
    }

    #[test]
    fn test_parse_ticket_invalid() {
        let result = parse_ticket("invalid_ticket");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_json_as_blob() {
        let core = GinsengCore::new().await.unwrap();
        let json = r#"{"test": "data"}"#;

        let result = store_json_as_blob(&core.blobs, json).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_single_file_metadata_with_temp_file() {
        let core = GinsengCore::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&temp_file, "test content").await.unwrap();

        let result = create_single_file_metadata(&core.blobs, &temp_file).await;
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.share_type, ShareType::SingleFile);
        assert_eq!(metadata.files.len(), 1);
        assert_eq!(metadata.files[0].name, "test.txt");
        assert_eq!(metadata.total_size, 12);
    }

    #[tokio::test]
    async fn test_create_directory_metadata_with_temp_dir() {
        let core = GinsengCore::new().await.unwrap();
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        tokio::fs::create_dir(&sub_dir).await.unwrap();

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = sub_dir.join("file2.txt");
        tokio::fs::write(&file1, "content1").await.unwrap();
        tokio::fs::write(&file2, "content2").await.unwrap();

        let result = create_directory_metadata(&core.blobs, temp_dir.path()).await;
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(matches!(metadata.share_type, ShareType::Directory { .. }));
        assert_eq!(metadata.files.len(), 2);
        assert_eq!(metadata.total_size, 16);
    }
}
