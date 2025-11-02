# Parallel Progress Event System

This document describes the newly implemented parallel progress tracking system for Ginseng file transfers.

## Overview

A **tokio-based concurrent progress event system** that provides real-time updates during multi-file uploads and downloads. The system uses:

- **Event-driven architecture** with typed progress events
- **Thread-safe progress tracking** via `Arc<RwLock<T>>`
- **Rate limiting** to prevent UI overwhelming
- **Streaming updates** for file-by-file progress

## Architecture

### Backend (Rust/Tokio)

#### Core Components

**`src-tauri/src/progress.rs`**
- `ProgressTracker`: Thread-safe wrapper around transfer state
- `TransferProgress`: Overall transfer metrics (bytes, files, ETA, etc.)
- `FileProgress`: Individual file progress with status
- `ProgressEvent`: Tagged enum of all event types
- `RateLimiter`: Prevents excessive updates (100ms intervals)

**Event Types:**
```rust
pub enum ProgressEvent {
    TransferStarted { transfer: TransferProgress },
    TransferProgress { transfer: TransferProgress },
    FileProgress { transfer_id, file: FileProgress },
    StageChanged { transfer_id, stage, message },
    TransferCompleted { transfer: TransferProgress },
    TransferFailed { transfer, error },
}
```

**Transfer Stages:**
- `Initializing` - Validating paths, counting files
- `Connecting` - Establishing peer connections
- `Transferring` - Active data transfer
- `Finalizing` - Writing to disk, cleanup
- `Completed` - Success
- `Failed` - Error occurred

#### Implementation

**`GinsengCore::share_files_parallel()`**
1. Creates `ProgressTracker` with unique transfer ID
2. Scans all files and initializes progress entries
3. Processes files sequentially (parallel version ready for future)
4. Emits `FileProgress` events as each file completes
5. Rate-limits overall `TransferProgress` updates
6. Finalizes and emits `TransferCompleted`

**`GinsengCore::download_files_parallel()`**
1. Parses ticket and fetches metadata
2. Initializes file progress for all files
3. Downloads files with per-file progress updates
4. Exports to filesystem after download
5. Updates overall progress with rate limiting

### Frontend (React/TypeScript)

#### Components

**`src/types/progress.ts`**
- TypeScript types matching Rust structs
- Utility functions: `formatBytes()`, `formatDuration()`, `calculateProgress()`

**`src/components/ParallelProgress.tsx`**
- `ParallelProgress`: Full progress display with file list
- `FileProgressItem`: Individual file progress bars
- Compact and expanded views
- Color-coded status indicators

**`src/components/FileTransfer.tsx`**
- Manages `Channel<ProgressEvent>` for both uploads/downloads
- State management for `uploadProgress` and `downloadProgress`
- Event handlers update state based on event type
- Renders `ParallelProgress` component when active

#### Usage Pattern

```typescript
const channel = new Channel<ProgressEvent>();

channel.onmessage = (event: ProgressEvent) => {
  switch (event.event) {
    case "transferStarted":
    case "transferProgress":
      setUploadProgress(event.data.transfer);
      break;
    case "transferCompleted":
      setUploadProgress(event.data.transfer);
      toast.success("Upload complete!");
      break;
    case "transferFailed":
      toast.error(event.data.error);
      break;
  }
};

await invoke("share_files_parallel", { channel, paths });
```

## Commands

### New Tauri Commands

**`share_files_parallel(channel, paths)`**
- Takes `Channel<ProgressEvent>` for streaming updates
- Returns ticket string on completion
- Emits progress events throughout transfer

**`download_files_parallel(channel, ticket)`**
- Downloads with progress tracking
- Returns `DownloadResult` with metadata and path
- Emits per-file and overall progress

### Legacy Commands (Still Available)

**`share_files(channel, paths)`** - Old simple progress (kept for CLI)
**`download_files(ticket)`** - No progress tracking (kept for CLI)

## CLI Support

CLI commands use non-progress versions:
- `GinsengCore::share_files_cli()`
- `GinsengCore::download_files_cli()`

No channels required, simple console output.

## Key Features

### ✅ Real-time Progress
- File-by-file progress updates
- Overall transfer metrics (bytes, speed, ETA)
- Stage transitions (connecting → transferring → finalizing)

### ✅ Rate Limiting
- Updates throttled to 100ms intervals
- Prevents UI thrashing on fast transfers
- Important events always sent (start, complete, error)

### ✅ Error Handling
- Per-file error tracking
- Overall transfer failure events
- Error messages propagated to UI

### ✅ Concurrent Architecture
- Thread-safe `ProgressTracker` with `Arc<RwLock<T>>`
- Ready for true parallel file processing (currently sequential due to lifetime constraints)
- Clean separation between tracking and I/O

## Future Enhancements

### True Parallel Processing
Currently files are processed sequentially. To enable parallel:
1. Use `tokio::task::spawn_blocking()` for CPU-bound operations
2. Use `futures::stream::buffer_unordered()` for async operations
3. Careful lifetime management with Arc clones

### Streaming Progress
Integrate with iroh's download progress streams:
```rust
let mut stream = download_progress.stream().await?;
while let Some(event) = stream.next().await {
    // Update file progress based on bytes downloaded
}
```

### Cancelation Support
Add `TransferCancelled` event and cancelation tokens:
```rust
let cancel_token = CancellationToken::new();
// Pass to transfer operations
// Emit TransferCancelled on cancel
```

## Testing

**Manual Testing:**
1. Select multiple large files
2. Click "Generate Share Ticket"
3. Observe real-time file-by-file progress
4. Check transfer rate and ETA updates
5. Test download with ticket

**What to Verify:**
- Progress bars update smoothly
- File statuses change (pending → transferring → completed)
- Overall metrics match individual file progress
- Error handling (invalid ticket, network issues)
- Rate limiting (no UI lag)

## Performance Characteristics

- **Memory**: O(n) where n = number of files (stores all FileProgress in memory)
- **Update Rate**: Max 10 updates/second (100ms rate limit)
- **Concurrency**: Currently sequential, designed for parallel
- **Network**: No overhead, uses existing iroh streams

## Migration Notes

**For Existing Code:**
- Old `share_files()` and `download_files()` commands still work
- New parallel commands are opt-in
- Frontend detects and uses parallel version by default
- CLI uses non-progress versions

**Breaking Changes:**
- None - fully backward compatible
