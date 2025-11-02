//! Parallel progress tracking for multi-file transfers
//!
//! This module provides a tokio-based concurrent progress system that tracks
//! multiple file transfers in parallel with real-time updates.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

pub type TransferId = String;
pub type FileId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferType {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransferStage {
    Initializing,
    Connecting,
    Transferring,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Pending,
    Transferring,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    pub file_id: FileId,
    pub name: String,
    pub relative_path: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub status: FileStatus,
    pub transfer_rate: Option<u64>,
    pub error: Option<String>,
}

impl FileProgress {
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

    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            FileStatus::Completed | FileStatus::Failed | FileStatus::Skipped
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: TransferId,
    pub transfer_type: TransferType,
    pub stage: TransferStage,
    pub total_files: u64,
    pub completed_files: u64,
    pub failed_files: u64,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub transfer_rate: Option<u64>,
    pub start_time: u64,
    pub eta_seconds: Option<u64>,
    pub files: Vec<FileProgress>,
    pub error: Option<String>,
}

impl TransferProgress {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum ProgressEvent {
    TransferStarted {
        transfer: TransferProgress,
    },
    TransferProgress {
        transfer: TransferProgress,
    },
    FileProgress {
        transfer_id: TransferId,
        file: FileProgress,
    },
    StageChanged {
        transfer_id: TransferId,
        stage: TransferStage,
        message: Option<String>,
    },
    TransferCompleted {
        transfer: TransferProgress,
    },
    TransferFailed {
        transfer: TransferProgress,
        error: String,
    },
}

/// Thread-safe progress tracker that can be shared across parallel tasks
#[derive(Clone)]
pub struct ProgressTracker {
    inner: Arc<RwLock<TransferProgress>>,
}

impl ProgressTracker {
    pub fn new(transfer_id: String, transfer_type: TransferType) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TransferProgress::new(
                transfer_id,
                transfer_type,
            ))),
        }
    }

    pub async fn get_snapshot(&self) -> TransferProgress {
        self.inner.read().await.clone()
    }

    pub async fn set_stage(&self, stage: TransferStage) {
        let mut inner = self.inner.write().await;
        inner.stage = stage;
    }

    pub async fn add_file(&self, file: FileProgress) {
        let mut inner = self.inner.write().await;
        inner.total_files += 1;
        inner.total_bytes += file.total_bytes;
        inner.files.push(file);
    }

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

    pub async fn set_error(&self, error: String) {
        let mut inner = self.inner.write().await;
        inner.error = Some(error);
        inner.stage = TransferStage::Failed;
    }

    pub async fn complete(&self) {
        let mut inner = self.inner.write().await;
        inner.stage = TransferStage::Completed;
        inner.update_rates();
    }
}

/// Rate limiter for progress updates
#[derive(Clone)]
pub struct RateLimiter {
    last_emission: Arc<RwLock<SystemTime>>,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_emission: Arc::new(RwLock::new(SystemTime::now())),
            min_interval,
        }
    }

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

    pub async fn force_emit(&self) {
        *self.last_emission.write().await = SystemTime::UNIX_EPOCH;
    }
}

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
