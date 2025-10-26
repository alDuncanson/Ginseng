use anyhow::Result;
use iroh::{endpoint::Connection, protocol::Router, Endpoint, RelayMode};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol, Hash};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShareMetadata {
    pub files: Vec<FileInfo>,
    pub share_type: ShareType,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        let endpoint = create_endpoint().await?;
        let store = MemStore::new();
        let blobs = BlobsProtocol::new(&store, None);
        let router = create_router(&endpoint, &blobs);

        Ok(Self {
            endpoint,
            store,
            blobs,
            _router: router,
        })
    }

    pub async fn share_files(&self, paths: Vec<PathBuf>) -> Result<String> {
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

    pub async fn download_files(&self, ticket_str: String) -> Result<(ShareMetadata, PathBuf)> {
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

    pub async fn node_info(&self) -> Result<String> {
        format_node_info(&self.endpoint)
    }
}

async fn create_endpoint() -> Result<Endpoint> {
    Endpoint::builder()
        .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
        .relay_mode(RelayMode::Default)
        .discovery_n0()
        .bind()
        .await
        .map_err(Into::into)
}

fn create_router(endpoint: &Endpoint, blobs: &BlobsProtocol) -> Router {
    iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_blobs::protocol::ALPN, blobs.clone())
        .spawn()
}

fn validate_paths_not_empty(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        anyhow::bail!("No files provided");
    }
    Ok(())
}

async fn create_share_metadata(blobs: &BlobsProtocol, paths: &[PathBuf]) -> Result<ShareMetadata> {
    if paths.len() == 1 {
        create_single_path_metadata(blobs, &paths[0]).await
    } else {
        create_multiple_files_metadata(blobs, paths).await
    }
}

async fn create_single_path_metadata(blobs: &BlobsProtocol, path: &Path) -> Result<ShareMetadata> {
    let canonical_path = fs::canonicalize(path).await?;

    if canonical_path.is_file() {
        create_single_file_metadata(blobs, &canonical_path).await
    } else if canonical_path.is_dir() {
        create_directory_metadata(blobs, &canonical_path).await
    } else {
        anyhow::bail!("Path is neither a file nor a directory")
    }
}

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

async fn create_directory_metadata(
    blobs: &BlobsProtocol,
    dir_path: &Path,
) -> Result<ShareMetadata> {
    let directory_name = extract_directory_name(dir_path);
    let file_infos = collect_directory_files(blobs, dir_path).await?;
    let total_size = calculate_total_size(&file_infos);

    Ok(ShareMetadata {
        files: file_infos,
        share_type: ShareType::Directory {
            name: directory_name,
        },
        total_size,
    })
}

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

    let total_size = calculate_total_size(&file_infos);

    Ok(ShareMetadata {
        files: file_infos,
        share_type: ShareType::MultipleFiles,
        total_size,
    })
}

async fn validate_all_paths_are_files(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        let canonical_path = fs::canonicalize(path).await?;
        if !canonical_path.is_file() {
            anyhow::bail!("All paths must be files when sharing multiple items");
        }
    }
    Ok(())
}

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

fn extract_file_name(file_path: &Path) -> String {
    file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_directory_name(dir_path: &Path) -> String {
    dir_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("folder")
        .to_string()
}

fn calculate_relative_path(file_path: &Path, base_path: &Path) -> Result<String> {
    if file_path == base_path {
        Ok(extract_file_name(file_path))
    } else {
        file_path
            .strip_prefix(base_path)
            .map(|p| p.to_str().unwrap_or("unknown").to_string())
            .map_err(Into::into)
    }
}

async fn get_file_size(file_path: &Path) -> Result<u64> {
    fs::metadata(file_path)
        .await
        .map(|m| m.len())
        .map_err(Into::into)
}

async fn store_file_as_blob(blobs: &BlobsProtocol, file_path: &Path) -> Result<String> {
    blobs
        .store()
        .add_path(file_path)
        .await
        .map(|tag| tag.hash.to_string())
        .map_err(Into::into)
}

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

fn calculate_total_size(file_infos: &[FileInfo]) -> u64 {
    file_infos.iter().map(|f| f.size).sum()
}

async fn store_metadata_as_blob(blobs: &BlobsProtocol, metadata: &ShareMetadata) -> Result<String> {
    let metadata_json = serde_json::to_string(metadata)?;
    store_json_as_blob(blobs, &metadata_json).await
}

async fn store_bundle_as_blob(
    blobs: &BlobsProtocol,
    bundle: &ShareBundle,
) -> Result<(Hash, iroh_blobs::BlobFormat)> {
    let bundle_json = serde_json::to_string(bundle)?;
    blobs
        .store()
        .add_bytes(bundle_json.into_bytes())
        .await
        .map(|tag| (tag.hash, tag.format))
        .map_err(Into::into)
}

async fn store_json_as_blob(blobs: &BlobsProtocol, json: &str) -> Result<String> {
    blobs
        .store()
        .add_bytes(json.as_bytes().to_vec())
        .await
        .map(|tag| tag.hash.to_string())
        .map_err(Into::into)
}

fn create_share_ticket(
    endpoint: &Endpoint,
    bundle_hash: &Hash,
    bundle_format: &iroh_blobs::BlobFormat,
) -> Result<String> {
    let node_addr = endpoint.node_addr();
    let ticket = BlobTicket::new(node_addr, *bundle_hash, *bundle_format);
    Ok(ticket.to_string())
}

fn parse_ticket(ticket_str: &str) -> Result<BlobTicket> {
    ticket_str.parse().map_err(Into::into)
}

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

async fn establish_connection(endpoint: &Endpoint, ticket: &BlobTicket) -> Result<Connection> {
    endpoint
        .connect(ticket.node_addr().clone(), iroh_blobs::protocol::ALPN)
        .await
        .map_err(Into::into)
}

async fn download_blob(endpoint: &Endpoint, store: &MemStore, ticket: &BlobTicket) -> Result<()> {
    let downloader = store.downloader(endpoint);
    downloader
        .download(ticket.hash(), Some(ticket.node_addr().node_id))
        .await
}

async fn parse_bundle_from_blob(blobs: &BlobsProtocol, ticket: &BlobTicket) -> Result<ShareBundle> {
    let temp_bundle_path = create_temp_bundle_path(ticket);
    blobs.export(ticket.hash(), &temp_bundle_path).await?;

    let bundle_json = fs::read_to_string(&temp_bundle_path).await?;
    let bundle = serde_json::from_str(&bundle_json)?;

    fs::remove_file(&temp_bundle_path).await?;
    Ok(bundle)
}

fn create_temp_bundle_path(ticket: &BlobTicket) -> PathBuf {
    std::env::temp_dir().join(format!("ginseng_bundle_{}", ticket.hash()))
}

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

fn get_downloads_directory() -> Result<PathBuf> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .or_else(|| std::env::current_dir().ok().map(|c| c.join("downloads")))
        .ok_or_else(|| anyhow::anyhow!("Could not determine downloads directory"))
}

async fn download_all_files(
    endpoint: &Endpoint,
    blobs: &BlobsProtocol,
    metadata: &ShareMetadata,
    target_dir: &Path,
    ticket: &BlobTicket,
) -> Result<()> {
    let downloader = blobs.store().downloader(endpoint);

    // Download all files first to ensure they're available
    for file_info in &metadata.files {
        let file_hash: Hash = file_info
            .hash
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid hash for file '{}': {}", file_info.name, e))?;

        downloader
            .download(file_hash, Some(ticket.node_addr().node_id))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to download file '{}' ({}): {}",
                    file_info.name,
                    file_hash,
                    e
                )
            })?;
    }

    // Then export all files to their target locations
    for file_info in &metadata.files {
        export_individual_file(blobs, file_info, target_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to export file '{}': {}", file_info.name, e))?;
    }

    Ok(())
}

async fn export_individual_file(
    blobs: &BlobsProtocol,
    file_info: &FileInfo,
    target_dir: &Path,
) -> Result<()> {
    let file_hash: Hash = file_info
        .hash
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid hash for file '{}': {}", file_info.name, e))?;
    let target_file_path = target_dir.join(&file_info.relative_path);

    ensure_parent_directory_exists(&target_file_path)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to create directory for '{}': {}",
                file_info.relative_path,
                e
            )
        })?;

    blobs
        .export(file_hash, &target_file_path)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to export '{}' to '{}': {}",
                file_info.name,
                target_file_path.display(),
                e
            )
        })?;

    Ok(())
}

async fn ensure_parent_directory_exists(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    Ok(())
}

fn format_node_info(endpoint: &Endpoint) -> Result<String> {
    let node_id = endpoint.node_id();
    let endpoint_addr = endpoint.node_addr();

    Ok(format!(
        "Node ID: {}\nDirect addresses: {:?}\nRelay URL: {:?}",
        node_id,
        endpoint_addr.direct_addresses().collect::<Vec<_>>(),
        endpoint_addr.relay_url()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_file_name() {
        assert_eq!(
            extract_file_name(Path::new("/path/to/file.txt")),
            "file.txt"
        );
        assert_eq!(extract_file_name(Path::new("file.txt")), "file.txt");
        assert_eq!(extract_file_name(Path::new("/path/to/")), "to");
    }

    #[test]
    fn test_extract_directory_name() {
        assert_eq!(
            extract_directory_name(Path::new("/path/to/folder")),
            "folder"
        );
        assert_eq!(extract_directory_name(Path::new("folder")), "folder");
        assert_eq!(extract_directory_name(Path::new("/")), "folder");
    }

    #[test]
    fn test_calculate_relative_path_same_file() {
        let path = Path::new("/path/to/file.txt");
        let result = calculate_relative_path(path, path).unwrap();
        assert_eq!(result, "file.txt");
    }

    #[test]
    fn test_calculate_relative_path_nested() {
        let file_path = Path::new("/base/dir/subdir/file.txt");
        let base_path = Path::new("/base/dir");
        let result = calculate_relative_path(file_path, base_path).unwrap();
        assert_eq!(result, "subdir/file.txt");
    }

    #[test]
    fn test_calculate_total_size() {
        let files = vec![
            FileInfo {
                name: "file1".to_string(),
                relative_path: "file1".to_string(),
                size: 100,
                hash: "hash1".to_string(),
            },
            FileInfo {
                name: "file2".to_string(),
                relative_path: "file2".to_string(),
                size: 200,
                hash: "hash2".to_string(),
            },
        ];

        assert_eq!(calculate_total_size(&files), 300);
    }

    #[test]
    fn test_calculate_total_size_empty() {
        assert_eq!(calculate_total_size(&[]), 0);
    }

    #[test]
    fn test_validate_paths_not_empty() {
        let empty_paths: Vec<PathBuf> = vec![];
        assert!(validate_paths_not_empty(&empty_paths).is_err());

        let non_empty_paths = vec![PathBuf::from("test")];
        assert!(validate_paths_not_empty(&non_empty_paths).is_ok());
    }

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
        let ticket: BlobTicket = ticket_str.parse().unwrap_or_else(|_| {
            let temp_dir = TempDir::new().unwrap();
            let temp_file = temp_dir.path().join("temp_ticket");
            std::fs::write(&temp_file, "dummy").unwrap();

            let dummy_hash = iroh_blobs::Hash::new([0u8; 32]);
            let dummy_node_id = iroh::NodeId::from_bytes(&[1u8; 32]).unwrap();
            let dummy_addr = iroh::NodeAddr::new(dummy_node_id);
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
