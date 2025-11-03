# Parallel File Transfer with Streaming Progress

## Overview

Successfully refactored the file transfer system to support **true parallel processing** with **real-time streaming progress** using iroh's blob protocol progress handles.

## Key Changes

### 1. Parallel Upload Processing

**Before:**
- Files were processed sequentially in a `for` loop
- Progress only showed completion of each file
- No byte-by-byte progress during hashing

**After:**
- Files processed concurrently using `futures::stream::for_each_concurrent`
- Bounded concurrency: `min(8, num_cpus)`
- Real-time progress during file hashing via `AddProgressItem` stream
- Progress events: `Size`, `CopyProgress`, `OutboardProgress`, `CopyDone`, `Done`

### 2. Parallel Download Processing

**Before:**
- Files downloaded sequentially
- No incremental progress updates

**After:**
- Files downloaded concurrently with concurrency limit of 6
- Real-time progress via `DownloadProgressItem` stream  
- Progress events: `Progress(bytes)`, `TryProvider`, `PartComplete`, `Error`

### 3. Streaming Progress Architecture

#### Upload Progress Flow
```rust
add_path().stream() 
→ AddProgressItem::Size(total_bytes)
→ AddProgressItem::CopyProgress(bytes_copied) [~every 1MB]
→ AddProgressItem::OutboardProgress(bytes_hashed) [during hash computation]
→ AddProgressItem::CopyDone
→ AddProgressItem::Done(hash)
```

#### Download Progress Flow
```rust
download().stream()
→ DownloadProgressItem::TryProvider
→ DownloadProgressItem::Progress(total_bytes_downloaded) [incremental]
→ DownloadProgressItem::PartComplete
→ Stream ends → export to filesystem
```

### 4. Progress Aggregation

- Each file task updates shared `ProgressTracker` (thread-safe via `Arc<RwLock>`)
- Combined progress calculated from:
  - Upload: `max(copy_progress, outboard_progress).min(total_bytes)`
  - Download: `total_bytes.min(file_size)`
- Rate-limited UI updates (100ms) to prevent flooding
- Force-emit on file completion for immediate feedback

## Parallelization Strategy

### Decision: Bounded Concurrency (No Batching)

**Why this approach:**
- ✅ Simple to implement and maintain
- ✅ Naturally prevents resource exhaustion
- ✅ Works well for any number of files (1 to 10,000+)
- ✅ CPU and I/O naturally shared across concurrent tasks
- ❌ No need for complex batching logic

**Rejected alternatives:**
- ❌ Unbounded parallelism: Would exhaust resources with many files
- ❌ Batching: Adds complexity without meaningful benefit
- ❌ Single aggregator task: Unnecessary - per-task updates work well

### Concurrency Limits

- **Upload**: `min(8, num_cpus)` - CPU-bound (hashing)
- **Download**: `6` - Network-bound
- Can be tuned based on profiling

## Code Changes

### New Dependencies
```toml
num_cpus = "1.16"
# Already had: futures = "0.3"
```

### New Functions

#### `upload_one_file()`
Handles single file upload with streaming progress:
- Streams `AddProgressItem` events
- Updates tracker on `CopyProgress` and `OutboardProgress`
- Sends `FileInfo` to collection channel on completion

#### `download_one_file()`  
Handles single file download with streaming progress:
- Streams `DownloadProgressItem` events
- Updates tracker on `Progress(bytes)`
- Exports to filesystem after download completes

### Modified Functions

#### `share_files_parallel()`
- Changed from sequential `for` loop to `stream::for_each_concurrent`
- Files collected via `mpsc` channel from concurrent tasks
- Progress updates stream to UI in real-time

#### `download_files_parallel()`
- Changed from sequential downloads to concurrent
- Stream-based progress for each file
- Export happens after download stream completes

## Performance Implications

### Before
- **Sequential processing**: 1 file at a time
- **Progress updates**: Only on file completion
- **Large files**: No feedback during hashing/download

### After  
- **Parallel processing**: Up to 8 uploads / 6 downloads concurrently
- **Progress updates**: Every ~1MB during copy, real-time during hash/download
- **Large files**: Smooth progress bars showing actual bytes transferred

### Expected Improvements
- **Upload speed**: ~3-6x faster with multiple files (limited by hashing CPU)
- **Download speed**: ~4-6x faster (limited by network and peer bandwidth)
- **UX**: Much better with incremental progress on large files

## Thread Safety

- `ProgressTracker`: `Arc<RwLock<TransferProgress>>` - safe for concurrent updates
- `RateLimiter`: `Arc<RwLock<SystemTime>>` - prevents update flooding
- `Channel<ProgressEvent>`: Wrapped in `Arc` for sharing across tasks
- File collection: `mpsc::unbounded_channel` for gathering results

## Error Handling

- Per-file errors logged but don't stop other files
- `AddProgressItem::Error` and `DownloadProgressItem::Error` properly handled
- Failed files marked with `FileStatus::Failed` (future enhancement)
- Overall transfer continues even if individual files fail

## Testing Recommendations

1. **Single large file** (>100MB): Verify smooth progress during hash/download
2. **Many small files** (100+): Verify concurrency and no UI lag
3. **Mixed sizes**: Ensure proper progress aggregation
4. **Network interruption**: Test error handling during download
5. **Disk full**: Test export failure handling

## Future Enhancements

### Possible Optimizations
1. **Adaptive concurrency**: Tune based on system load
2. **Priority queue**: Process larger files first for better perceived performance
3. **Cancellation**: Add ability to cancel transfers mid-stream
4. **Pause/Resume**: Leverage iroh's partial download support
5. **Aggregator task**: If lock contention becomes an issue (unlikely)

### Advanced Features
1. **Bandwidth throttling**: Limit transfer speed
2. **Retry logic**: Auto-retry failed files
3. **Deduplication**: Skip files already in blob store
4. **Delta sync**: Only transfer changed portions

## Breaking Changes

None - the public API remains identical:
- `share_files_parallel(channel, paths)` - same signature
- `download_files_parallel(channel, ticket)` - same signature
- Progress events - same structure

## Migration Notes

This is a drop-in replacement. The CLI versions (`share_files_cli`, `download_files_cli`) remain unchanged and don't use streaming progress.

## References

- [iroh-blobs AddProgress docs](https://docs.rs/iroh-blobs/0.96.0/iroh_blobs/api/blobs/enum.AddProgressItem.html)
- [iroh-blobs DownloadProgress docs](https://docs.rs/iroh-blobs/0.96.0/iroh_blobs/api/downloader/enum.DownloadProgressItem.html)
- [Tokio for_each_concurrent](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.for_each_concurrent)
