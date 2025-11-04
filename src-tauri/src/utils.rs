//! Utility functions for file operations and validation

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Validates and converts path strings to canonical absolute paths
///
/// Ensures each path exists and resolves it to an absolute canonical form,
/// which is necessary for consistent file operations.
///
/// # Arguments
///
/// * `paths` - Vector of path strings to validate
///
/// # Returns
///
/// Vector of canonicalized PathBuf instances
///
/// # Errors
///
/// Returns an error if any path does not exist or cannot be canonicalized
pub fn validate_and_canonicalize_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, String> {
    paths
        .iter()
        .map(|path| {
            std::fs::canonicalize(path)
                .map_err(|error| format!("Invalid file path '{}': {}", path, error))
        })
        .collect()
}

/// Extracts the file name from a path, defaulting to "unknown" if extraction fails.
///
/// # Arguments
/// * `file_path` - The path to extract the file name from
///
/// # Returns
/// The file name as a string
pub fn extract_file_name(file_path: &Path) -> String {
    file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Extracts the directory name from a path, defaulting to "folder" if extraction fails.
///
/// # Arguments
/// * `dir_path` - The path to extract the directory name from
///
/// # Returns
/// The directory name as a string
pub fn extract_directory_name(dir_path: &Path) -> String {
    dir_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("folder")
        .to_string()
}

/// Calculates the relative path from a base path to a file path.
///
/// If the file path equals the base path, returns just the file name.
/// Otherwise, strips the base path prefix to get the relative path.
///
/// # Arguments
/// * `file_path` - The target file path
/// * `base_path` - The base path to calculate relative to
///
/// # Returns
/// The relative path as a string
///
/// # Errors
/// Returns an error if the file path is not within the base path
pub fn calculate_relative_path(file_path: &Path, base_path: &Path) -> Result<String> {
    if file_path == base_path {
        Ok(extract_file_name(file_path))
    } else {
        file_path
            .strip_prefix(base_path)
            .map(|path| path.to_str().unwrap_or("unknown").to_string())
            .map_err(|error| anyhow::anyhow!("Failed to calculate relative path: {}", error))
    }
}

/// Calculates the total size of a collection of files.
///
/// # Arguments
/// * `sizes` - Iterator of file sizes in bytes
///
/// # Returns
/// The sum of all file sizes
pub fn calculate_total_size<I>(sizes: I) -> u64
where
    I: Iterator<Item = u64>,
{
    sizes.sum()
}

/// Validates that a collection of paths is not empty.
///
/// # Arguments
/// * `paths` - Slice of paths to validate
///
/// # Returns
/// Ok(()) if paths is not empty
///
/// # Errors
/// Returns an error if the paths collection is empty
pub fn validate_paths_not_empty(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        anyhow::bail!("No files provided");
    }
    Ok(())
}

/// Gets the user's downloads directory with fallbacks.
///
/// Tries in order:
/// 1. System downloads directory (if available)
/// 2. Home directory + "Downloads"
/// 3. Current directory + "ginseng_downloads"
///
/// # Returns
/// Path to the downloads directory
///
/// # Errors
/// Returns an error if no suitable directory can be determined
pub fn get_downloads_directory() -> Result<PathBuf> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|c| c.join("ginseng_downloads"))
        })
        .ok_or_else(|| anyhow::anyhow!("Could not determine downloads directory"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_validate_existing_files() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        File::create(&file_path).unwrap();

        let paths = vec![file_path.to_string_lossy().to_string()];
        let result = validate_and_canonicalize_paths(paths);

        assert!(result.is_ok());
        let canonical_paths = result.unwrap();
        assert_eq!(canonical_paths.len(), 1);
        assert!(canonical_paths[0].is_absolute());
    }

    #[test]
    fn test_validate_nonexistent_file() {
        let paths = vec!["/this/path/does/not/exist.txt".to_string()];
        let result = validate_and_canonicalize_paths(paths);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid file path"));
    }

    #[test]
    fn test_validate_empty_paths() {
        let paths = vec![];
        let result = validate_and_canonicalize_paths(paths);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

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
        assert_eq!(extract_directory_name(Path::new("/path/to/dir")), "dir");
        assert_eq!(extract_directory_name(Path::new("dir")), "dir");
        assert_eq!(extract_directory_name(Path::new("/path/to/")), "to");
    }

    #[test]
    fn test_calculate_relative_path_same_file() {
        let path = Path::new("/home/user/file.txt");
        assert_eq!(calculate_relative_path(path, path).unwrap(), "file.txt");
    }

    #[test]
    fn test_calculate_relative_path_nested() {
        let base = Path::new("/home/user");
        let file = Path::new("/home/user/docs/file.txt");
        assert_eq!(
            calculate_relative_path(file, base).unwrap(),
            "docs/file.txt"
        );
    }

    #[test]
    fn test_calculate_total_size() {
        let sizes = vec![100u64, 200u64, 300u64];
        assert_eq!(calculate_total_size(sizes.into_iter()), 600);
    }

    #[test]
    fn test_calculate_total_size_empty() {
        assert_eq!(calculate_total_size(std::iter::empty::<u64>()), 0);
    }

    #[test]
    fn test_validate_paths_not_empty() {
        let paths = vec![PathBuf::from("/some/path")];
        assert!(validate_paths_not_empty(&paths).is_ok());

        let empty_paths: Vec<PathBuf> = vec![];
        assert!(validate_paths_not_empty(&empty_paths).is_err());
    }

    #[test]
    fn test_get_downloads_directory() {
        // This test just verifies the function doesn't panic
        // The actual result depends on the system
        let result = get_downloads_directory();
        assert!(result.is_ok());
    }
}
