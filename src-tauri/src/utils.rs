use std::path::{Path, PathBuf};

pub fn absolute_path<P: AsRef<Path>>(p: P) -> Result<PathBuf, String> {
    std::path::absolute(p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_resolves() {
        let cwd = std::env::current_dir().unwrap();
        let abs = absolute_path(".").unwrap();
        assert_eq!(abs, cwd);
    }
}
