//! Sidecar checksum file for tamper detection.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::schema::MDQL_FILENAME;

const CHECKSUMS_FILENAME: &str = "_checksums.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct ChecksumFile {
    pub algorithm: String,
    pub files: BTreeMap<String, String>,
}

impl ChecksumFile {
    fn new() -> Self {
        ChecksumFile {
            algorithm: "xxhash64".to_string(),
            files: BTreeMap::new(),
        }
    }
}

pub fn hash_content(content: &[u8]) -> String {
    format!("{:016x}", xxhash_rust::xxh64::xxh64(content, 0))
}

pub fn load_checksums(table_dir: &Path) -> Option<ChecksumFile> {
    let path = table_dir.join(CHECKSUMS_FILENAME);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_checksums(table_dir: &Path, checksums: &ChecksumFile) -> crate::errors::Result<()> {
    let path = table_dir.join(CHECKSUMS_FILENAME);
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(checksums)
        .map_err(|e| crate::errors::MdqlError::General(e.to_string()))?;
    std::fs::write(&tmp, &content)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn update_checksum(table_dir: &Path, filename: &str, content: &[u8]) -> crate::errors::Result<()> {
    let mut checksums = load_checksums(table_dir).unwrap_or_else(ChecksumFile::new);
    checksums.files.insert(filename.to_string(), hash_content(content));
    save_checksums(table_dir, &checksums)
}

pub fn remove_checksum(table_dir: &Path, filename: &str) -> crate::errors::Result<()> {
    if let Some(mut checksums) = load_checksums(table_dir) {
        checksums.files.remove(filename);
        save_checksums(table_dir, &checksums)?;
    }
    Ok(())
}

pub fn regenerate_checksums(table_dir: &Path) -> crate::errors::Result<usize> {
    let mut checksums = ChecksumFile::new();
    let mut count = 0;

    let mut entries: Vec<_> = std::fs::read_dir(table_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.ends_with(".md") && name_str != MDQL_FILENAME
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let content = std::fs::read(entry.path())?;
        let name = entry.file_name().to_string_lossy().to_string();
        checksums.files.insert(name, hash_content(&content));
        count += 1;
    }

    save_checksums(table_dir, &checksums)?;
    Ok(count)
}

pub fn check_file(table_dir: &Path, filename: &str) -> Option<bool> {
    let checksums = load_checksums(table_dir)?;
    let expected = checksums.files.get(filename)?;
    let file_path = table_dir.join(filename);
    let content = std::fs::read(&file_path).ok()?;
    Some(hash_content(&content) == *expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_content(b"hello world");
        let h2 = hash_content(b"hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn test_hash_different_content() {
        let h1 = hash_content(b"hello");
        let h2 = hash_content(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_roundtrip() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.md"), "---\ntitle: Test\n---\n# Test\n").unwrap();

        update_checksum(dir.path(), "test.md", b"---\ntitle: Test\n---\n# Test\n").unwrap();

        let checksums = load_checksums(dir.path()).unwrap();
        assert_eq!(checksums.algorithm, "xxhash64");
        assert_eq!(checksums.files.len(), 1);
        assert!(checksums.files.contains_key("test.md"));
    }

    #[test]
    fn test_check_file_match() {
        let dir = tempdir().unwrap();
        let content = b"---\ntitle: Test\n---\n# Test\n";
        fs::write(dir.path().join("test.md"), content).unwrap();
        update_checksum(dir.path(), "test.md", content).unwrap();

        assert_eq!(check_file(dir.path(), "test.md"), Some(true));
    }

    #[test]
    fn test_check_file_tampered() {
        let dir = tempdir().unwrap();
        let content = b"---\ntitle: Test\n---\n# Test\n";
        fs::write(dir.path().join("test.md"), content).unwrap();
        update_checksum(dir.path(), "test.md", content).unwrap();

        fs::write(dir.path().join("test.md"), "---\ntitle: Changed\n---\n# Changed\n").unwrap();
        assert_eq!(check_file(dir.path(), "test.md"), Some(false));
    }

    #[test]
    fn test_check_file_no_checksums() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.md"), "content").unwrap();
        assert_eq!(check_file(dir.path(), "test.md"), None);
    }

    #[test]
    fn test_remove_checksum() {
        let dir = tempdir().unwrap();
        update_checksum(dir.path(), "a.md", b"a").unwrap();
        update_checksum(dir.path(), "b.md", b"b").unwrap();

        let checksums = load_checksums(dir.path()).unwrap();
        assert_eq!(checksums.files.len(), 2);

        remove_checksum(dir.path(), "a.md").unwrap();
        let checksums = load_checksums(dir.path()).unwrap();
        assert_eq!(checksums.files.len(), 1);
        assert!(!checksums.files.contains_key("a.md"));
    }

    #[test]
    fn test_regenerate() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("_mdql.md"), "schema").unwrap();
        fs::write(dir.path().join("a.md"), "aaa").unwrap();
        fs::write(dir.path().join("b.md"), "bbb").unwrap();

        let count = regenerate_checksums(dir.path()).unwrap();
        assert_eq!(count, 2);

        let checksums = load_checksums(dir.path()).unwrap();
        assert_eq!(checksums.files.len(), 2);
        assert!(checksums.files.contains_key("a.md"));
        assert!(checksums.files.contains_key("b.md"));
        assert!(!checksums.files.contains_key("_mdql.md"));
    }
}
