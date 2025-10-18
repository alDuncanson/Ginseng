#[derive(serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct FileInfo {
    pub path: String,
}

#[derive(serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct ProcessFilesResponse {
    pub total: usize,
    pub processed: usize,
    pub files: Vec<FileInfo>,
}

pub fn file_info(path: String) -> FileInfo {
    FileInfo { path }
}

pub fn process_paths(paths: Vec<String>) -> ProcessFilesResponse {
    let mut processed = 0usize;
    let files: Vec<FileInfo> = paths
        .into_iter()
        .map(|p| {
            if std::fs::metadata(&p).is_ok() {
                processed += 1;
            }
            file_info(p)
        })
        .collect();
    ProcessFilesResponse {
        total: files.len(),
        processed,
        files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_file(size: usize) -> String {
        let dir = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("tether_test_core_{}.bin", ts));
        fs::write(&path, vec![0u8; size]).expect("write temp file");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn empty_input_returns_zeroes() {
        let resp = process_paths(vec![]);
        assert_eq!(resp.total, 0);
        assert_eq!(resp.processed, 0);
        assert!(resp.files.is_empty());
    }

    #[test]
    fn single_existing_is_counted() {
        let p = tmp_file(1);
        let resp = process_paths(vec![p.clone()]);
        assert_eq!(resp.total, 1);
        assert_eq!(resp.processed, 1);
        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.files[0].path, p);
    }

    #[test]
    fn single_missing_is_not_processed() {
        let dir = std::env::temp_dir();
        let missing = dir
            .join("tether_core_missing.bin")
            .to_string_lossy()
            .into_owned();
        let resp = process_paths(vec![missing.clone()]);
        assert_eq!(resp.total, 1);
        assert_eq!(resp.processed, 0);
        assert_eq!(resp.files[0].path, missing);
    }

    #[test]
    fn mix_existing_and_missing() {
        let exists = tmp_file(8);
        let dir = std::env::temp_dir();
        let missing = dir
            .join("tether_core_missing2.bin")
            .to_string_lossy()
            .into_owned();
        let resp = process_paths(vec![exists.clone(), missing.clone()]);
        assert_eq!(resp.total, 2);
        assert_eq!(resp.processed, 1);
        assert_eq!(
            resp.files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<_>>(),
            vec![exists, missing]
        );
    }

    #[test]
    fn duplicates_are_counted_individually() {
        let p = tmp_file(2);
        let resp = process_paths(vec![p.clone(), p.clone()]);
        assert_eq!(resp.total, 2);
        assert_eq!(resp.processed, 2);
    }
}
