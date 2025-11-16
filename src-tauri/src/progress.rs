//! Parallel progress tracking for multi-file transfers.
//!
//! This module provides a tokio-based concurrent progress tracking system that enables
//! real-time monitoring of multi-file transfer operations. It's designed to handle
//! parallel file transfers with minimal overhead while providing detailed progress
//! information to the UI.
//!
//! # Architecture
//!
//! The progress system consists of three main components:
//!
//! 1. **ProgressTracker**: Thread-safe aggregator of transfer state
//!    - Uses `Arc<RwLock<TransferProgress>>` for concurrent access
//!    - Maintains overall statistics (totals, rates, ETAs)
//!    - Allows multiple parallel tasks to update progress safely
//!
//! 2. **Progress Events**: One-way communication to frontend via Tauri channels
//!    - Sends snapshots of current state as events
//!    - Fire-and-forget pattern (no state storage)
//!    - Consumed by the UI for real-time display
//!
//! 3. **RateLimiter**: Prevents UI flooding with excessive updates
//!    - Enforces minimum time between progress emissions
//!    - Reduces overhead during high-speed transfers
//!
//! # Usage Pattern
//!
//! ```ignore
//! // Create a tracker for a new transfer
//! let tracker = ProgressTracker::new(transfer_id, TransferType::Upload);
//!
//! // Add files to track
//! for file in files {
//!     tracker.add_file(FileProgress::new(name, path, size)).await;
//! }
//!
//! // In parallel transfer tasks, update individual files
//! tracker.update_file(&file_id, |file| {
//!     file.transferred_bytes = new_bytes;
//! }).await;
//!
//! // Periodically emit progress events (with rate limiting)
//! if rate_limiter.should_emit().await {
//!     channel.send(ProgressEvent::TransferProgress {
//!         transfer: tracker.get_snapshot().await,
//!     });
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for a transfer session.
///
/// Currently implemented as a String (typically a UUID), this allows
/// correlation of progress events and tracking data across the system.
pub type TransferId = String;

/// Unique identifier for a file within a transfer.
///
/// Used to correlate file-level progress updates with specific files
/// in the transfer's file list.
pub type FileId = String;

/// The type of transfer operation being performed.
///
/// Distinguishes between outbound (upload/share) and inbound (download/receive)
/// transfers for UI display and metrics tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferType {
    /// Files are being uploaded (shared) to a peer
    Upload,
    /// Files are being downloaded (received) from a peer
    Download,
}

/// The current stage of a transfer operation.
///
/// Represents the high-level lifecycle of a transfer from initialization
/// through completion or failure. Used for UI state management and determining
/// what actions are valid at any given time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferStage {
    /// Preparing the transfer: collecting files, calculating sizes, creating metadata
    Initializing,
    /// Establishing peer-to-peer connection with the remote node
    Connecting,
    /// Actively transferring file data between peers
    Transferring,
    /// Completing the transfer: finalizing files, running cleanup operations
    Finalizing,
    /// Transfer completed successfully - all files transferred
    Completed,
    /// Transfer failed with an error - see error field for details
    Failed,
    /// Transfer was cancelled by the user before completion
    Cancelled,
}

/// The current status of an individual file within a transfer.
///
/// Tracks the lifecycle of each file independently, allowing parallel
/// transfers where files can be in different states simultaneously.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    /// File is queued but transfer hasn't started yet
    Pending,
    /// File is currently being transferred with active data flow
    Transferring,
    /// File transfer completed successfully
    Completed,
    /// File transfer failed - see error field for details
    Failed,
    /// File was skipped (e.g., already exists, user excluded)
    Skipped,
}

/// Progress tracking information for a single file within a transfer.
///
/// Contains all metrics and metadata needed to display per-file progress
/// in the UI. Each file in a multi-file transfer maintains its own
/// `FileProgress` instance, updated independently during parallel transfers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    /// Unique identifier for this file (UUID)
    pub file_id: FileId,
    /// The file name without path (e.g., "document.pdf")
    pub name: String,
    /// Relative path from the transfer root (e.g., "folder/document.pdf")
    pub relative_path: String,
    /// Total size of the file in bytes
    pub total_bytes: u64,
    /// Number of bytes transferred so far
    pub transferred_bytes: u64,
    /// Current status of this file's transfer
    pub status: FileStatus,
    /// Per-file transfer rate in bytes per second (None if not yet calculated)
    pub transfer_rate: Option<u64>,
    /// Error message if the file transfer failed (None if successful or in progress)
    pub error: Option<String>,
}

impl FileProgress {
    /// Creates a new file progress tracker with a unique ID.
    ///
    /// Initializes the file in `Pending` status with zero bytes transferred.
    /// The file ID is auto-generated as a UUID v4.
    ///
    /// # Arguments
    ///
    /// * `name` - The file name without path (e.g., "document.pdf")
    /// * `relative_path` - The relative path from the transfer root (e.g., "folder/document.pdf")
    /// * `total_bytes` - Total size of the file in bytes
    ///
    /// # Returns
    ///
    /// A new `FileProgress` instance ready to track transfer progress
    pub fn new(name: String, relative_path: String, total_bytes: u64) -> Self {
        Self {
            file_id: Uuid::new_v4().to_string(),
            name,
            relative_path,
            total_bytes,
            transferred_bytes: 0,
            status: FileStatus::Pending,
            transfer_rate: None,
            error: None,
        }
    }

    /// Checks if this file's transfer has reached a terminal state.
    ///
    /// A file is considered complete if it's in any of the terminal states:
    /// `Completed` (success), `Failed` (error), or `Skipped` (intentionally not transferred).
    ///
    /// # Returns
    ///
    /// `true` if the file is in a terminal state, `false` if still pending or transferring
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            FileStatus::Completed | FileStatus::Failed | FileStatus::Skipped
        )
    }
}

/// Overall progress tracking for a multi-file transfer operation.
///
/// Aggregates progress across all files in a transfer, providing both
/// individual file-level details and overall transfer statistics. This is
/// the primary data structure sent to the UI for progress display.
///
/// All byte counts and rates are calculated by aggregating individual file
/// progress and updated atomically via the `ProgressTracker`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    /// Unique identifier for this transfer (UUID)
    pub transfer_id: TransferId,
    /// Type of transfer (upload or download)
    pub transfer_type: TransferType,
    /// Current stage of the transfer lifecycle
    pub stage: TransferStage,
    /// Total number of files in this transfer
    pub total_files: u64,
    /// Number of files that have completed successfully
    pub completed_files: u64,
    /// Number of files that failed to transfer
    pub failed_files: u64,
    /// Total size of all files combined in bytes
    pub total_bytes: u64,
    /// Total bytes transferred across all files so far
    pub transferred_bytes: u64,
    /// Overall transfer rate in bytes per second (None if not yet calculated)
    pub transfer_rate: Option<u64>,
    /// Unix timestamp (seconds since epoch) when the transfer started
    pub start_time: u64,
    /// Estimated time remaining in seconds (None if not yet calculated)
    pub eta_seconds: Option<u64>,
    /// Progress information for each file in the transfer
    pub files: Vec<FileProgress>,
    /// Error message if the overall transfer failed (None if successful or in progress)
    pub error: Option<String>,
}

impl TransferProgress {
    /// Creates a new transfer progress tracker in the `Initializing` stage.
    ///
    /// Initializes all counters to zero and records the current time as the
    /// start time. Files should be added via `ProgressTracker::add_file()`.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Unique identifier for this transfer (typically a UUID)
    /// * `transfer_type` - Type of transfer (upload or download)
    ///
    /// # Returns
    ///
    /// A new `TransferProgress` instance with empty file list and zero counters
    pub fn new(transfer_id: TransferId, transfer_type: TransferType) -> Self {
        Self {
            transfer_id,
            transfer_type,
            stage: TransferStage::Initializing,
            total_files: 0,
            completed_files: 0,
            failed_files: 0,
            total_bytes: 0,
            transferred_bytes: 0,
            transfer_rate: None,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            eta_seconds: None,
            files: Vec::new(),
            error: None,
        }
    }

    /// Updates transfer rate and ETA based on current progress.
    ///
    /// Calculates the overall transfer rate by dividing total transferred bytes
    /// by elapsed time since `start_time`, then estimates the remaining time
    /// based on this rate. Should be called after `recalculate_totals()`.
    ///
    /// The rate calculation uses simple averaging over the entire transfer duration.
    /// For transfers with variable speed, this provides a reasonable overall estimate
    /// but may not reflect current instantaneous speed.
    pub fn update_rates(&mut self) {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.start_time);

        if elapsed > 0 && self.transferred_bytes > 0 {
            self.transfer_rate = Some(self.transferred_bytes / elapsed);

            if let Some(rate) = self.transfer_rate {
                if rate > 0 {
                    let remaining = self.total_bytes.saturating_sub(self.transferred_bytes);
                    self.eta_seconds = Some(remaining / rate);
                }
            }
        }
    }

    /// Recalculates aggregate statistics from individual file progress.
    ///
    /// Sums up transferred bytes across all files and counts files by status.
    /// This should be called after updating any individual file's progress to
    /// keep the overall statistics in sync.
    ///
    /// Automatically called by `ProgressTracker::update_file()`, so manual
    /// calls are only needed if modifying files directly.
    pub fn recalculate_totals(&mut self) {
        self.transferred_bytes = self.files.iter().map(|f| f.transferred_bytes).sum();
        self.completed_files = self
            .files
            .iter()
            .filter(|f| f.status == FileStatus::Completed)
            .count() as u64;
        self.failed_files = self
            .files
            .iter()
            .filter(|f| f.status == FileStatus::Failed)
            .count() as u64;
    }
}

/// Events emitted during a transfer for real-time progress updates.
///
/// These events are sent through Tauri IPC channels to the frontend where
/// they drive UI updates. They represent state change notifications rather
/// than state storage - the `ProgressTracker` maintains the authoritative state.
///
/// The `#[serde(tag = "event", content = "data")]` attribute creates a tagged
/// enum representation in JSON, making it easy for the frontend to dispatch
/// on event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ProgressEvent {
    /// Transfer has started - initial state snapshot
    TransferStarted { transfer: TransferProgress },
    /// Overall transfer progress has been updated - periodic snapshots during transfer
    TransferProgress { transfer: TransferProgress },
    /// Individual file progress has been updated - granular per-file updates
    FileProgress {
        transfer_id: TransferId,
        file: FileProgress,
    },
    /// Transfer has moved to a new stage in its lifecycle
    StageChanged {
        transfer_id: TransferId,
        stage: TransferStage,
        message: Option<String>,
    },
    /// Transfer has completed successfully - final state
    TransferCompleted { transfer: TransferProgress },
    /// Transfer has failed - terminal error state
    TransferFailed {
        transfer: TransferProgress,
        error: String,
    },
}

/// Thread-safe progress tracker that can be shared across parallel tasks.
///
/// Wraps `TransferProgress` in `Arc<RwLock<_>>` to enable safe concurrent access
/// from multiple async tasks. This is the primary interface for updating transfer
/// progress during parallel file operations.
///
/// # Concurrency Model
///
/// - Multiple readers can access progress snapshots simultaneously
/// - Writers get exclusive access to update state atomically
/// - All update methods are async to work with tokio's cooperative scheduling
/// - Updates automatically recalculate totals and rates
///
/// # Cloning
///
/// This struct is cheaply cloneable (only increments Arc reference count),
/// allowing it to be passed to multiple parallel tasks without copying data.
#[derive(Clone)]
pub struct ProgressTracker {
    inner: Arc<RwLock<TransferProgress>>,
}

impl ProgressTracker {
    /// Creates a new progress tracker for a transfer.
    ///
    /// Initializes the tracker in the `Initializing` stage with zero counters.
    /// The tracker is ready to accept files via `add_file()`.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Unique identifier for this transfer (typically a UUID)
    /// * `transfer_type` - Type of transfer (upload or download)
    ///
    /// # Returns
    ///
    /// A new `ProgressTracker` instance that can be cloned and shared across tasks
    pub fn new(transfer_id: String, transfer_type: TransferType) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TransferProgress::new(
                transfer_id,
                transfer_type,
            ))),
        }
    }

    /// Gets a snapshot of the current progress state.
    ///
    /// Returns a clone of the current `TransferProgress`, allowing the caller to
    /// inspect progress without holding the lock. This is useful for emitting
    /// progress events - take a snapshot, release the lock, then send the event.
    ///
    /// # Returns
    ///
    /// Cloned `TransferProgress` representing the current state
    pub async fn get_snapshot(&self) -> TransferProgress {
        self.inner.read().await.clone()
    }

    /// Updates the current transfer stage.
    ///
    /// Transitions the transfer to a new lifecycle stage. Common transitions:
    /// `Initializing → Transferring → Completed/Failed`
    ///
    /// # Arguments
    ///
    /// * `stage` - The new stage to transition to
    pub async fn set_stage(&self, stage: TransferStage) {
        let mut inner = self.inner.write().await;
        inner.stage = stage;
    }

    /// Adds a new file to the transfer.
    ///
    /// Increments total file count and total bytes, then adds the file to
    /// the files list. Should be called during the `Initializing` stage
    /// before transfer begins.
    ///
    /// # Arguments
    ///
    /// * `file` - File progress tracker to add to this transfer
    pub async fn add_file(&self, file: FileProgress) {
        let mut inner = self.inner.write().await;
        inner.total_files += 1;
        inner.total_bytes += file.total_bytes;
        inner.files.push(file);
    }

    /// Updates a specific file's progress using a closure.
    ///
    /// Finds the file by ID, applies the update function, and automatically
    /// recalculates aggregate totals and transfer rates. This is the primary
    /// method for updating progress during parallel file transfers.
    ///
    /// The update is atomic - the closure runs while holding the write lock,
    /// ensuring consistent state even with concurrent updates from other tasks.
    ///
    /// # Arguments
    ///
    /// * `file_id` - The ID of the file to update
    /// * `updater` - Closure that modifies the file progress (e.g., updating bytes transferred)
    ///
    /// # Example
    ///
    /// ```ignore
    /// tracker.update_file(&file_id, |file| {
    ///     file.transferred_bytes = new_bytes;
    ///     file.status = FileStatus::Transferring;
    /// }).await;
    /// ```
    pub async fn update_file<F>(&self, file_id: &str, updater: F)
    where
        F: FnOnce(&mut FileProgress),
    {
        let mut inner = self.inner.write().await;
        if let Some(file) = inner.files.iter_mut().find(|f| f.file_id == file_id) {
            updater(file);
            inner.recalculate_totals();
            inner.update_rates();
        }
    }

    /// Marks the transfer as failed with an error message.
    ///
    /// Sets the error message and transitions to the `Failed` stage.
    /// This is a terminal state - no further progress updates should occur.
    ///
    /// # Arguments
    ///
    /// * `error` - Human-readable error message describing what went wrong
    pub async fn set_error(&self, error: String) {
        let mut inner = self.inner.write().await;
        inner.error = Some(error);
        inner.stage = TransferStage::Failed;
    }

    /// Marks the transfer as completed and updates final rates.
    ///
    /// Transitions to the `Completed` stage and calculates final transfer
    /// statistics. This is a terminal state - no further progress updates
    /// should occur after calling this.
    pub async fn complete(&self) {
        let mut inner = self.inner.write().await;
        inner.stage = TransferStage::Completed;
        inner.update_rates();
    }
}

/// Rate limiter for progress updates to prevent flooding the UI with events.
///
/// Enforces a minimum time interval between progress event emissions to avoid
/// overwhelming the frontend during high-speed transfers. This is especially
/// important when transferring many small files or very large files with
/// high-frequency progress callbacks.
///
/// # Design
///
/// Uses `Arc<RwLock<SystemTime>>` to track the last emission time across
/// multiple async tasks. The minimum interval is fixed at creation time.
///
/// A typical interval is 16-50ms (matching 20-60 FPS UI refresh rates).
#[derive(Clone)]
pub struct RateLimiter {
    last_emission: Arc<RwLock<SystemTime>>,
    min_interval: Duration,
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified minimum interval.
    ///
    /// # Arguments
    ///
    /// * `min_interval` - Minimum duration between allowed emissions (e.g., `Duration::from_millis(16)`)
    ///
    /// # Returns
    ///
    /// A new `RateLimiter` instance initialized with current time
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Limit to ~60 FPS (16.67ms per frame)
    /// let limiter = RateLimiter::new(Duration::from_millis(16));
    /// ```
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_emission: Arc::new(RwLock::new(SystemTime::now())),
            min_interval,
        }
    }

    /// Checks if enough time has passed to emit a new event.
    ///
    /// If the minimum interval has elapsed since the last emission,
    /// updates the last emission time to now and returns `true`.
    /// Otherwise returns `false` without updating state.
    ///
    /// # Returns
    ///
    /// `true` if emission is allowed (and state is updated), `false` if still rate-limited
    ///
    /// # Example
    ///
    /// ```ignore
    /// if rate_limiter.should_emit().await {
    ///     channel.send(progress_event);
    /// }
    /// ```
    pub async fn should_emit(&self) -> bool {
        let now = SystemTime::now();
        let last = *self.last_emission.read().await;

        if now.duration_since(last).unwrap_or_default() >= self.min_interval {
            *self.last_emission.write().await = now;
            true
        } else {
            false
        }
    }

    /// Forces the next emission to be allowed immediately.
    ///
    /// Resets the last emission time to the Unix epoch, guaranteeing that
    /// the next call to `should_emit()` will return `true`. Useful for
    /// ensuring important events (like completion) are always sent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Always send completion event regardless of rate limiting
    /// rate_limiter.force_emit().await;
    /// channel.send(ProgressEvent::TransferCompleted { ... });
    /// ```
    pub async fn force_emit(&self) {
        *self.last_emission.write().await = SystemTime::UNIX_EPOCH;
    }
}

/// Formats a byte count into a human-readable string.
///
/// Converts bytes to the most appropriate unit (B, KB, MB, GB, TB)
/// with two decimal places of precision using binary units (1024 bytes = 1 KB).
///
/// # Arguments
///
/// * `bytes` - The number of bytes to format
///
/// # Returns
///
/// A formatted string with the byte count and appropriate unit
///
/// # Examples
///
/// ```
/// use ginseng_lib::progress::format_bytes;
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1536), "1.50 KB");
/// assert_eq!(format_bytes(1048576), "1.00 MB");
/// assert_eq!(format_bytes(1073741824), "1.00 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
