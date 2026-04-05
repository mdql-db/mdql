//! Per-file mtime-based caching for parsed rows.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use crate::model::Row;

/// Cached row with its file mtime at parse time.
#[derive(Debug)]
struct CachedRow {
    mtime: SystemTime,
    row: Row,
}

/// Per-table cache that tracks file mtimes to avoid re-parsing unchanged files.
#[derive(Debug)]
pub struct TableCache {
    rows: HashMap<String, CachedRow>,
    table_mtime: Option<SystemTime>,
}

impl TableCache {
    pub fn new() -> Self {
        TableCache {
            rows: HashMap::new(),
            table_mtime: None,
        }
    }

    /// Get a cached row if the file hasn't been modified since caching.
    pub fn get(&self, path: &str, current_mtime: SystemTime) -> Option<&Row> {
        self.rows.get(path).and_then(|cached| {
            if cached.mtime == current_mtime {
                Some(&cached.row)
            } else {
                None
            }
        })
    }

    /// Store a row in the cache with its file mtime.
    pub fn put(&mut self, path: String, mtime: SystemTime, row: Row) {
        self.rows.insert(path, CachedRow { mtime, row });
    }

    /// Remove a cached entry (e.g., after delete).
    pub fn remove(&mut self, path: &str) {
        self.rows.remove(path);
    }

    /// Check if the table directory has been modified since last cache update.
    pub fn is_stale(&self, table_dir: &Path) -> bool {
        let current = dir_mtime(table_dir);
        match (self.table_mtime, current) {
            (Some(cached), Some(now)) => cached != now,
            (None, _) => true, // Never cached
            (_, None) => true, // Can't read dir mtime
        }
    }

    /// Update the cached table-directory mtime.
    pub fn set_table_mtime(&mut self, table_dir: &Path) {
        self.table_mtime = dir_mtime(table_dir);
    }

    /// Clear all cached entries.
    pub fn invalidate_all(&mut self) {
        self.rows.clear();
        self.table_mtime = None;
    }

    /// Number of cached rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// All cached paths.
    pub fn cached_paths(&self) -> Vec<&str> {
        self.rows.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for TableCache {
    fn default() -> Self {
        Self::new()
    }
}

fn dir_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
}

/// Get the mtime of a file, returning None if it can't be read.
pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Value;
    use std::time::Duration;

    #[test]
    fn test_cache_hit_and_miss() {
        let mut cache = TableCache::new();
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let row = Row::from([
            ("path".into(), Value::String("test.md".into())),
            ("title".into(), Value::String("Hello".into())),
        ]);

        cache.put("test.md".into(), mtime, row);

        // Hit: same mtime
        assert!(cache.get("test.md", mtime).is_some());

        // Miss: different mtime
        let later = mtime + Duration::from_secs(1);
        assert!(cache.get("test.md", later).is_none());
    }

    #[test]
    fn test_remove() {
        let mut cache = TableCache::new();
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let row = Row::from([("path".into(), Value::String("test.md".into()))]);
        cache.put("test.md".into(), mtime, row);
        assert_eq!(cache.len(), 1);

        cache.remove("test.md");
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_invalidate_all() {
        let mut cache = TableCache::new();
        let mtime = SystemTime::UNIX_EPOCH;
        cache.put("a.md".into(), mtime, Row::new());
        cache.put("b.md".into(), mtime, Row::new());
        assert_eq!(cache.len(), 2);

        cache.invalidate_all();
        assert!(cache.is_empty());
    }
}
