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
use tauri::ipc::{Channel, InvokeResponseBody};
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

/// Task definition for uploading a single file.
#[derive(Debug, Clone)]
struct UploadFileTask {
    absolute_path: PathBuf,
    share_root: PathBuf,
    file_id: String,
}

/// Task definition for downloading a single file.
#[derive(Debug, Clone)]
struct DownloadFileTask {
    file_info: FileInfo,
    file_id: String,
}

/// Core functionality for peer-to-peer file sharing using Iroh.
///
/// This struct encapsulates all the networking and storage components needed
/// for sharing and downloading files in a decentralized manner. It handles
/// the entire lifecycle from file ingestion to ticket generation for sharing,
/// and from ticket parsing to file reconstruction for downloading.
///
/// # Progress Tracking Architecture
///
/// File transfers use a two-component system for progress tracking:
///
/// 1. **ProgressTracker**: Thread-safe in-memory state (`Arc<RwLock<TransferProgress>>`)
///    - Aggregates progress from multiple parallel file operations
///    - Calculates overall statistics (totals, rates, ETAs)
///    - Multiple concurrent tasks can safely update it
///    - Think of it as a "database" of current transfer state
///
/// 2. **Progress Channel**: One-way Tauri IPC communication to frontend
///    - Sends progress event snapshots to the UI
///    - Fire-and-forget (doesn't store state)
///    - Think of it as a "notification bus"
///
/// The pattern: Parallel tasks update the tracker → periodically take a snapshot →
/// send snapshot through channel to UI. This allows aggregating progress from many
/// concurrent file operations into coherent overall transfer statistics.
pub struct GinsengCore {
    /// Iroh endpoint for P2P networking
    pub endpoint: Endpoint,
    /// In-memory blob store for content-addressed storage
    pub store: MemStore,
    /// Protocol handler for blob operations (upload/download)
    pub blob_protocol: BlobsProtocol,
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
        let blob_protocol = BlobsProtocol::new(&store, None);
        let router = create_router(&endpoint, &blob_protocol);

        Ok(Self {
            endpoint,
            store,
            blob_protocol,
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

        let progress_tracker = ProgressTracker::new(uuid::Uuid::new_v4().to_string(), TransferType::Upload);
        let progress_rate_limiter = RateLimiter::new(Duration::from_millis(16));

        channel
            .send(ProgressEvent::TransferStarted {
                transfer: progress_tracker.get_snapshot().await,
            })
            .ok();

        progress_tracker.set_stage(TransferStage::Initializing).await;

        let upload_tasks = initialize_upload_tasks(&paths, &progress_tracker).await?;

        channel
            .send(ProgressEvent::TransferProgress {
                transfer: progress_tracker.get_snapshot().await,
            })
            .ok();

        progress_tracker.set_stage(TransferStage::Transferring).await;

        let file_infos = upload_files_concurrently(
            upload_tasks,
            &self.blob_protocol,
            &progress_tracker,
            &Arc::new(channel.clone()),
            &progress_rate_limiter,
        )
        .await;

        let ticket = finalize_share_bundle(
            file_infos,
            &paths,
            &self.blob_protocol,
            &self.endpoint,
            &progress_tracker,
        )
        .await?;

        progress_tracker.complete().await;
        channel
            .send(ProgressEvent::TransferCompleted {
                transfer: progress_tracker.get_snapshot().await,
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
        let progress_tracker =
            ProgressTracker::new(uuid::Uuid::new_v4().to_string(), TransferType::Download);
        let progress_rate_limiter = RateLimiter::new(Duration::from_millis(100));

        channel
            .send(ProgressEvent::TransferStarted {
                transfer: progress_tracker.get_snapshot().await,
            })
            .ok();

        progress_tracker.set_stage(TransferStage::Connecting).await;

        let ticket = parse_ticket(&ticket_str)?;
        let bundle =
            download_and_parse_bundle(&self.endpoint, &self.blob_protocol, &self.store, &ticket).await?;

        let target_directory = determine_target_directory(&bundle.metadata)?;

        for file_info in &bundle.metadata.files {
            progress_tracker
                .add_file(FileProgress::new(
                    file_info.name.clone(),
                    file_info.relative_path.clone(),
                    file_info.size,
                ))
                .await;
        }

        progress_tracker.set_stage(TransferStage::Transferring).await;
        channel
            .send(ProgressEvent::TransferProgress {
                transfer: progress_tracker.get_snapshot().await,
            })
            .ok();

        let download_concurrency = 6;
        let progress_channel = Arc::new(channel);

        let snapshot = progress_tracker.get_snapshot().await;
        let download_tasks: Vec<DownloadFileTask> = bundle
            .metadata
            .files
            .iter()
            .enumerate()
            .map(|(file_index, file_info)| DownloadFileTask {
                file_info: file_info.clone(),
                file_id: snapshot.files[file_index].file_id.clone(),
            })
            .collect();

        let endpoint_clone = self.endpoint.clone();
        let blob_protocol_clone = self.blob_protocol.clone();
        let progress_tracker_clone = progress_tracker.clone();
        let progress_rate_limiter_clone = progress_rate_limiter.clone();
        let progress_channel_clone = progress_channel.clone();
        let peer_id = ticket.addr().id;
        let target_directory_clone = target_directory.clone();

        stream::iter(download_tasks)
            .for_each_concurrent(download_concurrency, move |download_task| {
                let endpoint = endpoint_clone.clone();
                let blob_protocol = blob_protocol_clone.clone();
                let progress_tracker = progress_tracker_clone.clone();
                let progress_channel = progress_channel_clone.clone();
                let progress_rate_limiter = progress_rate_limiter_clone.clone();
                let target_directory = target_directory_clone.clone();

                async move {
                    if let Err(error) = download_one_file(
                        download_task.file_info,
                        download_task.file_id,
                        endpoint,
                        blob_protocol,
                        peer_id,
                        target_directory,
                        progress_tracker,
                        progress_channel,
                        progress_rate_limiter,
                    )
                    .await
                    {
                        eprintln!("Download failed: {}", error);
                    }
                }
            })
            .await;

        progress_tracker.complete().await;
        progress_channel
            .send(ProgressEvent::TransferCompleted {
                transfer: progress_tracker.get_snapshot().await,
            })
            .ok();

        Ok((bundle.metadata, target_directory))
    }

    /// CLI version of share_files_parallel without progress updates
    ///
    /// Uses a no-op channel for CLI environments where progress events are not needed.
    ///
    /// # Arguments
    ///
    /// * `paths` - Vector of file or directory paths to share
    ///
    /// # Returns
    ///
    /// A shareable ticket string
    ///
    /// # Errors
    ///
    /// Returns an error if sharing fails
    pub async fn share_files_cli(&self, paths: Vec<PathBuf>) -> Result<String> {
        let channel = Channel::new(|_event: InvokeResponseBody| Ok(()));

        self.share_files_parallel(channel, paths).await
    }

    /// CLI version of download_files_parallel without progress updates
    ///
    /// Uses a no-op channel for CLI environments where progress events are not needed.
    ///
    /// # Arguments
    ///
    /// * `ticket_str` - The ticket string received from the sender
    ///
    /// # Returns
    ///
    /// Tuple containing the share metadata and download path
    ///
    /// # Errors
    ///
    /// Returns an error if download fails
    pub async fn download_files_cli(&self, ticket_str: String) -> Result<(ShareMetadata, PathBuf)> {
        let channel = Channel::new(|_event: InvokeResponseBody| Ok(()));

        self.download_files_parallel(channel, ticket_str).await
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

/// Creates and configures an Iroh endpoint for P2P networking
///
/// Sets up the endpoint with blob protocol support, default relay mode,
/// and peer discovery for finding nodes on the network.
///
/// # Errors
///
/// Returns an error if the endpoint cannot be created or bound to a port
async fn create_endpoint() -> Result<Endpoint> {
    Endpoint::builder()
        .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
        .relay_mode(RelayMode::Default)
        .bind()
        .await
        .map_err(|error| anyhow::anyhow!("Failed to create endpoint: {}", error))
}

/// Creates a protocol router that handles incoming blob protocol connections
///
/// Configures the router to accept connections using the blob protocol ALPN
/// and routes them to the appropriate blob protocol handler.
///
/// # Arguments
///
/// * `endpoint` - The Iroh endpoint to attach the router to
/// * `blob_protocol` - The blob protocol handler for processing connections
///
/// # Returns
///
/// Configured and spawned router ready to handle incoming connections
fn create_router(endpoint: &Endpoint, blob_protocol: &BlobsProtocol) -> Router {
    iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_blobs::protocol::ALPN, blob_protocol.clone())
        .spawn()
}

/// Gets the size of a file in bytes
///
/// # Arguments
///
/// * `file_path` - Path to the file to measure
///
/// # Returns
///
/// File size in bytes
///
/// # Errors
///
/// Returns an error if the file cannot be accessed or metadata cannot be read
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

/// Initializes upload tasks and file progress tracking
///
/// Collects all files from the provided paths, creates FileProgress entries
/// in the tracker, and returns a list of UploadFileTask structs ready for
/// concurrent processing.
///
/// # Arguments
///
/// * `paths` - Slice of file or directory paths to process
/// * `progress_tracker` - Shared progress tracker for file operations
///
/// # Returns
///
/// Vector of upload tasks ready for parallel execution
///
/// # Errors
///
/// Returns an error if file metadata cannot be read or paths are invalid
async fn initialize_upload_tasks(
    paths: &[PathBuf],
    progress_tracker: &ProgressTracker,
) -> Result<Vec<UploadFileTask>> {
    let file_paths = collect_file_paths(paths).await?;

    for (absolute_path, share_root) in &file_paths {
        let name = extract_file_name(absolute_path);
        let relative_path = calculate_relative_path(absolute_path, share_root)?;
        let size = get_file_size(absolute_path).await?;
        progress_tracker
            .add_file(FileProgress::new(name, relative_path, size))
            .await;
    }

    let snapshot = progress_tracker.get_snapshot().await;
    let upload_tasks: Vec<UploadFileTask> = file_paths
        .iter()
        .enumerate()
        .map(|(file_index, (absolute_path, share_root))| UploadFileTask {
            absolute_path: absolute_path.clone(),
            share_root: share_root.clone(),
            file_id: snapshot.files[file_index].file_id.clone(),
        })
        .collect();

    Ok(upload_tasks)
}

/// Uploads files concurrently using buffer_unordered
///
/// Processes upload tasks in parallel and collects the resulting FileInfo
/// structs. Uses buffer_unordered to limit concurrency while maintaining
/// high throughput. Failed uploads are logged and filtered out.
///
/// # Arguments
///
/// * `upload_tasks` - Vector of upload tasks to process
/// * `blob_protocol` - Protocol handler for blob storage operations
/// * `progress_tracker` - Shared progress tracker for updating file states
/// * `progress_channel` - Channel for sending progress events to frontend
/// * `progress_rate_limiter` - Rate limiter to prevent excessive progress updates
///
/// # Returns
///
/// Vector of successfully uploaded FileInfo structs
async fn upload_files_concurrently(
    upload_tasks: Vec<UploadFileTask>,
    blob_protocol: &BlobsProtocol,
    progress_tracker: &ProgressTracker,
    progress_channel: &Arc<Channel<ProgressEvent>>,
    progress_rate_limiter: &RateLimiter,
) -> Vec<FileInfo> {
    let upload_concurrency = std::cmp::min(8, num_cpus::get());

    stream::iter(upload_tasks)
        .map(|upload_task| {
            upload_one_file(
                upload_task.absolute_path,
                upload_task.share_root,
                upload_task.file_id,
                blob_protocol.clone(),
                progress_tracker.clone(),
                progress_channel.clone(),
                progress_rate_limiter.clone(),
            )
        })
        .buffer_unordered(upload_concurrency)
        .filter_map(|result| async move { result.ok() })
        .collect()
        .await
}

/// Finalizes the share bundle by creating metadata and generating a ticket
///
/// Takes the uploaded FileInfo structs, creates ShareMetadata, stores it
/// as a blob, and generates a shareable ticket string.
///
/// # Arguments
///
/// * `file_infos` - Vector of file information from successful uploads
/// * `paths` - Original paths that were shared
/// * `blob_protocol` - Protocol handler for storing metadata
/// * `endpoint` - Endpoint for generating the ticket address
/// * `progress_tracker` - Progress tracker to update with finalizing stage
///
/// # Returns
///
/// Shareable ticket string for downloading the files
///
/// # Errors
///
/// Returns an error if metadata storage or ticket generation fails
async fn finalize_share_bundle(
    file_infos: Vec<FileInfo>,
    paths: &[PathBuf],
    blob_protocol: &BlobsProtocol,
    endpoint: &Endpoint,
    progress_tracker: &ProgressTracker,
) -> Result<String> {
    let total_size = calculate_total_size(file_infos.iter().map(|file_info| file_info.size));
    let share_type = determine_share_type(paths, &file_infos);

    let metadata = ShareMetadata {
        files: file_infos,
        share_type,
        total_size,
    };

    progress_tracker.set_stage(TransferStage::Finalizing).await;

    let metadata_hash = store_metadata_as_blob(blob_protocol, &metadata).await?;
    let bundle = ShareBundle {
        metadata,
        metadata_hash,
    };
    let (bundle_hash, bundle_format) = store_bundle_as_blob(blob_protocol, &bundle).await?;
    let ticket = create_share_ticket(endpoint, &bundle_hash, &bundle_format)?;

    Ok(ticket)
}

/// Downloads a single file with streaming progress updates
///
/// Establishes a download stream, processes progress events, exports the file
/// to the target directory, and updates the progress tracker in real-time.
///
/// # Arguments
///
/// * `file_info` - Metadata about the file to download
/// * `file_id` - Unique identifier for this file in the progress tracker
/// * `endpoint` - Endpoint for connecting to the peer
/// * `blob_protocol` - Protocol handler for blob operations
/// * `peer_id` - ID of the peer to download from
/// * `target_directory` - Directory where the file will be saved
/// * `progress_tracker` - Shared progress tracker for updating transfer state
/// * `progress_channel` - Channel for sending progress events to frontend
/// * `progress_rate_limiter` - Rate limiter to prevent excessive progress updates
///
/// # Errors
///
/// Returns an error if the hash is invalid, download fails, or file export fails
async fn download_one_file(
    file_info: FileInfo,
    file_id: String,
    endpoint: Endpoint,
    blob_protocol: BlobsProtocol,
    peer_id: iroh::EndpointId,
    target_directory: PathBuf,
    progress_tracker: ProgressTracker,
    progress_channel: Arc<Channel<ProgressEvent>>,
    progress_rate_limiter: RateLimiter,
) -> Result<()> {
    use iroh_blobs::api::downloader::DownloadProgressItem as DP;

    progress_tracker
        .update_file(&file_id, |file_progress| {
            file_progress.status = FileStatus::Transferring;
        })
        .await;

    let file_hash: Hash = file_info
        .hash
        .parse()
        .map_err(|error| anyhow::anyhow!("Invalid hash: {}", error))?;

    let downloader = blob_protocol.store().downloader(&endpoint);
    let download = downloader.download(file_hash, Some(peer_id));
    let mut stream = download.stream().await?;

    while let Some(event) = stream.next().await {
        match event {
            DP::Progress(total_bytes) => {
                let transferred = total_bytes.min(file_info.size);
                progress_tracker
                    .update_file(&file_id, |file_progress| {
                        file_progress.transferred_bytes = transferred;
                    })
                    .await;
            }
            DP::Error(error) => {
                return Err(anyhow::anyhow!("Download error: {}", error));
            }
            DP::DownloadError => {
                return Err(anyhow::anyhow!(
                    "Download failed for file '{}'",
                    file_info.name
                ));
            }
            _ => {}
        }

        if progress_rate_limiter.should_emit().await {
            progress_channel
                .send(ProgressEvent::TransferProgress {
                    transfer: progress_tracker.get_snapshot().await,
                })
                .ok();
        }
    }

    let target_path = target_directory.join(&file_info.relative_path);
    ensure_parent_directory_exists(&target_path).await?;
    blob_protocol
        .export(file_hash, &target_path)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to export file '{}': {}", file_info.name, error))?;

    progress_tracker
        .update_file(&file_id, |file_progress| {
            file_progress.transferred_bytes = file_progress.total_bytes;
            file_progress.status = FileStatus::Completed;
        })
        .await;

    progress_rate_limiter.force_emit().await;
    progress_channel
        .send(ProgressEvent::TransferProgress {
            transfer: progress_tracker.get_snapshot().await,
        })
        .ok();

    Ok(())
}

/// Uploads a single file with streaming progress updates
///
/// Adds the file to blob storage, processes progress events, and updates the
/// tracker in real-time. Returns the FileInfo on success.
///
/// # Arguments
///
/// * `absolute_path` - Full path to the file to upload
/// * `share_root` - Root directory for calculating relative paths
/// * `file_id` - Unique identifier for this file in the progress tracker
/// * `blob_protocol` - Protocol handler for blob storage operations
/// * `progress_tracker` - Shared progress tracker for updating transfer state
/// * `progress_channel` - Channel for sending progress events to frontend
/// * `progress_rate_limiter` - Rate limiter to prevent excessive progress updates
///
/// # Returns
///
/// FileInfo containing metadata about the uploaded file
///
/// # Errors
///
/// Returns an error if the file cannot be read or upload fails
async fn upload_one_file(
    absolute_path: PathBuf,
    share_root: PathBuf,
    file_id: String,
    blob_protocol: BlobsProtocol,
    progress_tracker: ProgressTracker,
    progress_channel: Arc<Channel<ProgressEvent>>,
    progress_rate_limiter: RateLimiter,
) -> Result<FileInfo> {
    use iroh_blobs::api::blobs::AddProgressItem;

    progress_tracker
        .update_file(&file_id, |file_progress| {
            file_progress.status = FileStatus::Transferring;
        })
        .await;

    let name = extract_file_name(&absolute_path);
    let relative_path = calculate_relative_path(&absolute_path, &share_root)?;

    let add_progress = blob_protocol.store().add_path(absolute_path.clone());
    let mut stream = add_progress.stream().await;

    let mut copy_bytes = 0u64;
    let mut outboard_bytes = 0u64;
    let mut total_bytes = None;
    let mut result_file_info: Option<FileInfo> = None;

    while let Some(event) = stream.next().await {
        match event {
            AddProgressItem::Size(size) => {
                total_bytes = Some(size);
                progress_tracker
                    .update_file(&file_id, |file_progress| {
                        if file_progress.total_bytes == 0 {
                            file_progress.total_bytes = size;
                        }
                    })
                    .await;
            }
            AddProgressItem::CopyProgress(offset) => {
                copy_bytes = offset;
                if let Some(total) = total_bytes {
                    let transferred = copy_bytes.max(outboard_bytes).min(total);
                    progress_tracker
                        .update_file(&file_id, |file_progress| {
                            file_progress.transferred_bytes = transferred;
                        })
                        .await;
                }
            }
            AddProgressItem::OutboardProgress(offset) => {
                outboard_bytes = offset;
                if let Some(total) = total_bytes {
                    let transferred = copy_bytes.max(outboard_bytes).min(total);
                    progress_tracker
                        .update_file(&file_id, |file_progress| {
                            file_progress.transferred_bytes = transferred;
                        })
                        .await;
                }
            }
            AddProgressItem::CopyDone => {}
            AddProgressItem::Done(tag) => {
                let hash = tag.hash().to_string();
                let size = total_bytes.unwrap_or(0);

                progress_tracker
                    .update_file(&file_id, |file_progress| {
                        file_progress.transferred_bytes = file_progress.total_bytes;
                        file_progress.status = FileStatus::Completed;
                    })
                    .await;

                progress_rate_limiter.force_emit().await;
                progress_channel
                    .send(ProgressEvent::TransferProgress {
                        transfer: progress_tracker.get_snapshot().await,
                    })
                    .ok();

                result_file_info = Some(FileInfo {
                    name: name.clone(),
                    relative_path: relative_path.clone(),
                    size,
                    hash,
                });
            }
            AddProgressItem::Error(error) => {
                return Err(anyhow::anyhow!("Add progress error: {}", error));
            }
        }

        if progress_rate_limiter.should_emit().await {
            progress_channel
                .send(ProgressEvent::TransferProgress {
                    transfer: progress_tracker.get_snapshot().await,
                })
                .ok();
        }
    }

    result_file_info.ok_or_else(|| anyhow::anyhow!("Upload did not complete successfully"))
}

/// Collects all file paths from the given paths (files and directories)
///
/// Recursively walks directories to find all files, pairs each file with its
/// share root for relative path calculation.
///
/// # Arguments
///
/// * `paths` - Slice of file or directory paths to collect from
///
/// # Returns
///
/// Vector of tuples containing (file_path, share_root) pairs
///
/// # Errors
///
/// Returns an error if paths cannot be canonicalized
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
///
/// Analyzes the input paths to determine if this is a single file, multiple files,
/// or a directory share.
///
/// # Arguments
///
/// * `paths` - Original paths that were shared
/// * `file_infos` - Collected file information
///
/// # Returns
///
/// ShareType indicating the nature of the share
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

/// Serializes share metadata to JSON and stores it as a blob
///
/// # Arguments
///
/// * `blob_protocol` - Protocol handler for blob storage operations
/// * `metadata` - Share metadata to serialize and store
///
/// # Returns
///
/// Hash of the stored metadata blob
///
/// # Errors
///
/// Returns an error if serialization or storage fails
async fn store_metadata_as_blob(blob_protocol: &BlobsProtocol, metadata: &ShareMetadata) -> Result<String> {
    let metadata_json = serde_json::to_string(metadata)?;
    store_json_as_blob(blob_protocol, &metadata_json).await
}

/// Serializes a share bundle to JSON and stores it as a blob
///
/// # Arguments
///
/// * `blob_protocol` - Protocol handler for blob storage operations
/// * `bundle` - Share bundle to serialize and store
///
/// # Returns
///
/// Tuple containing the blob hash and format information needed for tickets
///
/// # Errors
///
/// Returns an error if serialization or storage fails
async fn store_bundle_as_blob(
    blob_protocol: &BlobsProtocol,
    bundle: &ShareBundle,
) -> Result<(Hash, iroh_blobs::BlobFormat)> {
    let bundle_json = serde_json::to_string(bundle)?;
    let add_progress = blob_protocol.store().add_bytes(bundle_json.into_bytes());
    let tag = add_progress
        .await
        .map_err(|error| anyhow::anyhow!("Failed to store bundle as blob: {}", error))?;
    Ok((tag.hash, tag.format))
}

/// Stores a JSON string as a blob and returns its hash
///
/// # Arguments
///
/// * `blob_protocol` - Protocol handler for blob storage operations
/// * `json` - JSON string to store as a blob
///
/// # Returns
///
/// Hash of the stored JSON blob
///
/// # Errors
///
/// Returns an error if storage fails
async fn store_json_as_blob(blob_protocol: &BlobsProtocol, json: &str) -> Result<String> {
    let add_progress = blob_protocol.store().add_bytes(json.as_bytes().to_vec());
    let tag = add_progress
        .await
        .map_err(|error| anyhow::anyhow!("Failed to store JSON as blob: {}", error))?;
    Ok(tag.hash.to_string())
}

/// Creates a shareable ticket string from a bundle hash and format
///
/// # Arguments
///
/// * `endpoint` - Endpoint containing the node address
/// * `bundle_hash` - Hash of the bundle blob
/// * `bundle_format` - Format information for the blob
///
/// # Returns
///
/// Base32-encoded ticket string containing the node address and blob information
///
/// # Errors
///
/// Returns an error if ticket generation fails
fn create_share_ticket(
    endpoint: &Endpoint,
    bundle_hash: &Hash,
    bundle_format: &iroh_blobs::BlobFormat,
) -> Result<String> {
    let endpoint_addr = endpoint.addr();
    let ticket = BlobTicket::new(endpoint_addr, *bundle_hash, *bundle_format);
    Ok(ticket.to_string())
}

/// Parses a ticket string into a BlobTicket structure
///
/// # Arguments
///
/// * `ticket_str` - Base32-encoded ticket string
///
/// # Returns
///
/// Parsed BlobTicket containing peer address and blob information
///
/// # Errors
///
/// Returns an error if the ticket is malformed or invalid
fn parse_ticket(ticket_str: &str) -> Result<BlobTicket> {
    ticket_str
        .parse::<BlobTicket>()
        .map_err(|error| anyhow::anyhow!("Failed to parse ticket: {}", error))
}

/// Downloads a bundle from a peer and parses it into a ShareBundle
///
/// Establishes a connection to the peer, downloads the bundle blob,
/// exports it to a temporary file, parses the JSON, and cleans up.
///
/// # Arguments
///
/// * `endpoint` - Endpoint for connecting to the peer
/// * `blob_protocol` - Protocol handler for blob operations
/// * `store` - Blob store for downloading data
/// * `ticket` - Ticket containing peer address and bundle information
///
/// # Returns
///
/// Parsed ShareBundle containing metadata and file information
///
/// # Errors
///
/// Returns an error if connection, download, or parsing fails
async fn download_and_parse_bundle(
    endpoint: &Endpoint,
    blob_protocol: &BlobsProtocol,
    store: &MemStore,
    ticket: &BlobTicket,
) -> Result<ShareBundle> {
    let _connection = establish_connection(endpoint, ticket).await?;
    download_blob(endpoint, store, ticket).await?;
    parse_bundle_from_blob(blob_protocol, ticket).await
}

/// Establishes a P2P connection to the node specified in the ticket
///
/// # Arguments
///
/// * `endpoint` - Local endpoint to connect from
/// * `ticket` - Ticket containing the peer's address information
///
/// # Returns
///
/// Active connection to the peer
///
/// # Errors
///
/// Returns an error if connection cannot be established
async fn establish_connection(endpoint: &Endpoint, ticket: &BlobTicket) -> Result<Connection> {
    endpoint
        .connect(ticket.addr().clone(), iroh_blobs::protocol::ALPN)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to establish connection: {}", error))
}

/// Downloads a blob from a peer into the local store
///
/// # Arguments
///
/// * `endpoint` - Endpoint for connecting to the peer
/// * `store` - Blob store for saving downloaded data
/// * `ticket` - Ticket containing peer address and blob hash
///
/// # Errors
///
/// Returns an error if the download fails
async fn download_blob(endpoint: &Endpoint, store: &MemStore, ticket: &BlobTicket) -> Result<()> {
    let downloader = store.downloader(endpoint);
    downloader
        .download(ticket.hash(), Some(ticket.addr().id))
        .await
        .map_err(|error| anyhow::anyhow!("Failed to download blob: {}", error))
}

/// Exports a blob to a temporary file, parses it as JSON, and cleans up
///
/// # Arguments
///
/// * `blob_protocol` - Protocol handler for blob export operations
/// * `ticket` - Ticket containing the blob hash to export
///
/// # Returns
///
/// Parsed ShareBundle from the blob contents
///
/// # Errors
///
/// Returns an error if export, parsing, or cleanup fails
async fn parse_bundle_from_blob(blob_protocol: &BlobsProtocol, ticket: &BlobTicket) -> Result<ShareBundle> {
    let temp_bundle_path = create_temp_bundle_path(ticket);
    blob_protocol.export(ticket.hash(), &temp_bundle_path).await?;

    let bundle_json = fs::read_to_string(&temp_bundle_path).await?;
    let bundle = serde_json::from_str(&bundle_json)?;

    fs::remove_file(&temp_bundle_path).await?;
    Ok(bundle)
}

/// Creates a temporary file path for bundle extraction using the ticket hash
///
/// # Arguments
///
/// * `ticket` - Ticket containing the hash to use in the filename
///
/// # Returns
///
/// Path in the system temp directory for bundle extraction
fn create_temp_bundle_path(ticket: &BlobTicket) -> PathBuf {
    std::env::temp_dir().join(format!("ginseng_bundle_{}", ticket.hash()))
}

/// Determines where to save downloaded files based on the share type
///
/// - Single file: Downloads directory
/// - Multiple files: Timestamped subdirectory in Downloads
/// - Directory: Named subdirectory in Downloads
///
/// # Arguments
///
/// * `metadata` - Share metadata containing the share type
///
/// # Returns
///
/// Path to the target directory for saving files
///
/// # Errors
///
/// Returns an error if the downloads directory cannot be determined
fn determine_target_directory(metadata: &ShareMetadata) -> Result<PathBuf> {
    let downloads_dir = get_downloads_directory()?;

    let target_directory = match &metadata.share_type {
        ShareType::SingleFile => downloads_dir,
        ShareType::MultipleFiles => {
            let timestamp = chrono::Utc::now().timestamp();
            downloads_dir.join(format!("ginseng_files_{}", timestamp))
        }
        ShareType::Directory { name } => downloads_dir.join(name),
    };

    Ok(target_directory)
}

/// Ensures that the parent directory of a file path exists
///
/// Creates all necessary parent directories if they don't exist.
///
/// # Arguments
///
/// * `file_path` - Path to the file whose parent directories should be created
///
/// # Errors
///
/// Returns an error if directory creation fails
async fn ensure_parent_directory_exists(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(())
}

/// Formats node information for display, including ID, addresses, and relay
///
/// # Arguments
///
/// * `endpoint` - Endpoint to extract information from
///
/// # Returns
///
/// Formatted string containing node ID, direct addresses, and relay URL
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
}
