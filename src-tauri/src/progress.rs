//! Parallel progress tracking for multi-file transfers
//!
//! This module provides a tokio-based concurrent progress system that tracks
//! multiple file transfers in parallel with real-time updates.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for a transfer session
pub type TransferId = String;

/// Unique identifier for a file within a transfer
pub type FileId = String;

/// The type of transfer operation being performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferType {
    /// Files are being uploaded (shared)
    Upload,
    /// Files are being downloaded (received)
    Download,
}

/// The current stage of a transfer operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferStage {
    /// Preparing the transfer (collecting files, creating metadata)
    Initializing,
    /// Establishing connection with the peer
    Connecting,
    /// Actively transferring file data
    Transferring,
    /// Completing the transfer (writing final files, cleanup)
    Finalizing,
    /// Transfer completed successfully
    Completed,
    /// Transfer failed with an error
    Failed,
    /// Transfer was cancelled by the user
    Cancelled,
}

/// The current status of an individual file within a transfer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    /// File is queued but transfer hasn't started yet
    Pending,
    /// File is currently being transferred
    Transferring,
    /// File transfer completed successfully
    Completed,
    /// File transfer failed
    Failed,
    /// File was skipped (e.g., already exists)
    Skipped,
}

/// Progress tracking information for a single file within a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    /// Unique identifier for this file
    pub file_id: FileId,
    /// The file name (e.g., "document.pdf")
    pub name: String,
    /// Relative path from the transfer root (e.g., "folder/document.pdf")
    pub relative_path: String,
    /// Total size of the file in bytes
    pub total_bytes: u64,
    /// Number of bytes transferred so far
    pub transferred_bytes: u64,
    /// Current status of this file's transfer
    pub status: FileStatus,
    /// Transfer rate in bytes per second (None if not yet calculated)
    pub transfer_rate: Option<u64>,
    /// Error message if the file transfer failed
    pub error: Option<String>,
}

impl FileProgress {
    /// Creates a new file progress tracker
    ///
    /// # Arguments
    ///
    /// * `name` - The file name
    /// * `relative_path` - The relative path from the transfer root
    /// * `total_bytes` - Total size of the file in bytes
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

    /// Checks if this file's transfer is complete (successfully, failed, or skipped)
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            FileStatus::Completed | FileStatus::Failed | FileStatus::Skipped
        )
    }
}

/// Overall progress tracking for a multi-file transfer operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    /// Unique identifier for this transfer
    pub transfer_id: TransferId,
    /// Type of transfer (upload or download)
    pub transfer_type: TransferType,
    /// Current stage of the transfer
    pub stage: TransferStage,
    /// Total number of files in this transfer
    pub total_files: u64,
    /// Number of files that have completed successfully
    pub completed_files: u64,
    /// Number of files that failed to transfer
    pub failed_files: u64,
    /// Total size of all files in bytes
    pub total_bytes: u64,
    /// Total bytes transferred across all files
    pub transferred_bytes: u64,
    /// Overall transfer rate in bytes per second (None if not yet calculated)
    pub transfer_rate: Option<u64>,
    /// Unix timestamp when the transfer started
    pub start_time: u64,
    /// Estimated time remaining in seconds (None if not yet calculated)
    pub eta_seconds: Option<u64>,
    /// Progress information for each file in the transfer
    pub files: Vec<FileProgress>,
    /// Error message if the transfer failed
    pub error: Option<String>,
}

impl TransferProgress {
    /// Creates a new transfer progress tracker
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Unique identifier for this transfer
    /// * `transfer_type` - Type of transfer (upload or download)
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

    /// Updates transfer rate and ETA based on current progress
    ///
    /// Calculates the overall transfer rate by dividing total transferred bytes
    /// by elapsed time, then estimates the remaining time based on this rate.
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

    /// Recalculates aggregate statistics from individual file progress
    ///
    /// Should be called after updating any file progress to keep totals in sync.
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

/// Events emitted during a transfer for real-time progress updates
///
/// These events are sent through Tauri channels to the frontend for UI updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ProgressEvent {
    /// Transfer has started
    TransferStarted { transfer: TransferProgress },
    /// Overall transfer progress has been updated
    TransferProgress { transfer: TransferProgress },
    /// Individual file progress has been updated
    FileProgress {
        transfer_id: TransferId,
        file: FileProgress,
    },
    /// Transfer has moved to a new stage
    StageChanged {
        transfer_id: TransferId,
        stage: TransferStage,
        message: Option<String>,
    },
    /// Transfer has completed successfully
    TransferCompleted { transfer: TransferProgress },
    /// Transfer has failed
    TransferFailed {
        transfer: TransferProgress,
        error: String,
    },
}

/// Thread-safe progress tracker that can be shared across parallel tasks
///
/// Uses RwLock internally to allow concurrent reads and exclusive writes,
/// enabling multiple tokio tasks to safely update progress in parallel.
#[derive(Clone)]
pub struct ProgressTracker {
    inner: Arc<RwLock<TransferProgress>>,
}

impl ProgressTracker {
    /// Creates a new progress tracker
    ///
    /// # Arguments
    ///
    /// * `transfer_id` - Unique identifier for this transfer
    /// * `transfer_type` - Type of transfer (upload or download)
    pub fn new(transfer_id: String, transfer_type: TransferType) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TransferProgress::new(
                transfer_id,
                transfer_type,
            ))),
        }
    }

    /// Gets a snapshot of the current progress state
    ///
    /// Returns a clone of the current progress, allowing the caller to
    /// inspect progress without holding a lock.
    pub async fn get_snapshot(&self) -> TransferProgress {
        self.inner.read().await.clone()
    }

    /// Updates the current transfer stage
    pub async fn set_stage(&self, stage: TransferStage) {
        let mut inner = self.inner.write().await;
        inner.stage = stage;
    }

    /// Adds a new file to the transfer
    ///
    /// Updates total file count and total bytes accordingly.
    pub async fn add_file(&self, file: FileProgress) {
        let mut inner = self.inner.write().await;
        inner.total_files += 1;
        inner.total_bytes += file.total_bytes;
        inner.files.push(file);
    }

    /// Updates a specific file's progress using a closure
    ///
    /// Finds the file by ID, applies the update function, and recalculates
    /// transfer totals and rates. This is the primary way to update file progress
    /// during parallel transfers.
    ///
    /// # Arguments
    ///
    /// * `file_id` - The ID of the file to update
    /// * `updater` - Closure that modifies the file progress
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

    /// Marks the transfer as failed with an error message
    pub async fn set_error(&self, error: String) {
        let mut inner = self.inner.write().await;
        inner.error = Some(error);
        inner.stage = TransferStage::Failed;
    }

    /// Marks the transfer as completed and updates final rates
    pub async fn complete(&self) {
        let mut inner = self.inner.write().await;
        inner.stage = TransferStage::Completed;
        inner.update_rates();
    }
}

/// Rate limiter for progress updates to prevent flooding the UI with events
///
/// Ensures that progress events are only emitted at a reasonable frequency,
/// typically used to avoid overwhelming the frontend with updates during
/// high-speed transfers.
#[derive(Clone)]
pub struct RateLimiter {
    last_emission: Arc<RwLock<SystemTime>>,
    min_interval: Duration,
}

impl RateLimiter {
    /// Creates a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `min_interval` - Minimum time between emissions
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_emission: Arc::new(RwLock::new(SystemTime::now())),
            min_interval,
        }
    }

    /// Checks if enough time has passed to emit a new event
    ///
    /// If the minimum interval has elapsed, updates the last emission time
    /// and returns true. Otherwise returns false.
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

    /// Forces the next emission to be allowed
    ///
    /// Resets the last emission time to the epoch, ensuring the next
    /// call to `should_emit` will return true.
    pub async fn force_emit(&self) {
        *self.last_emission.write().await = SystemTime::UNIX_EPOCH;
    }
}

/// Formats a byte count into a human-readable string
///
/// Converts bytes to the most appropriate unit (B, KB, MB, GB, TB)
/// with two decimal places of precision.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to format
///
/// # Examples
///
/// ```
/// use ginseng_lib::progress::format_bytes;
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1536), "1.50 KB");
/// assert_eq!(format_bytes(1048576), "1.00 MB");
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
