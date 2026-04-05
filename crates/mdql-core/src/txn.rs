//! Process-level ACID primitives for MDQL.
//!
//! Three layers:
//! 1. `atomic_write` — crash-safe single-file write via temp+rename
//! 2. `table_lock` — exclusive per-table lock via flock
//! 3. `multi_file_txn` — write-ahead journal for multi-file operations

use std::fs;
use std::io::Write;
use std::path::Path;

use fs2::FileExt;

use crate::errors::MdqlError;

pub const LOCK_FILENAME: &str = ".mdql_lock";
pub const JOURNAL_FILENAME: &str = ".mdql_journal";
pub const TMP_SUFFIX: &str = ".mdql_tmp";

// ── Atomic single-file write ─────────────────────────────────────────────

/// Write content to path atomically via temp file + rename.
pub fn atomic_write(path: &Path, content: &str) -> crate::errors::Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(content.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| {
        MdqlError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    Ok(())
}

// ── Table-level lock ──────────────────────────────────────────────────────

/// RAII guard for an exclusive table lock.
pub struct TableLock {
    _file: fs::File,
}

impl TableLock {
    pub fn acquire(table_dir: &Path) -> crate::errors::Result<Self> {
        let lock_path = table_dir.join(LOCK_FILENAME);
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        file.lock_exclusive()?;
        Ok(TableLock { _file: file })
    }
}

impl Drop for TableLock {
    fn drop(&mut self) {
        let _ = self._file.unlock();
    }
}

// ── Write-ahead journal ──────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct JournalEntry {
    pub action: String,  // "modify", "create", "delete"
    pub path: String,
    pub backup: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Journal {
    pub version: u32,
    pub operation: String,
    pub started_at: String,
    pub entries: Vec<JournalEntry>,
}

pub struct TableTransaction {
    _table_dir: std::path::PathBuf,
    journal_path: std::path::PathBuf,
    journal: Journal,
}

impl TableTransaction {
    pub fn new(table_dir: &Path, operation: &str) -> crate::errors::Result<Self> {
        let journal_path = table_dir.join(JOURNAL_FILENAME);
        let journal = Journal {
            version: 1,
            operation: operation.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            entries: Vec::new(),
        };

        let t = TableTransaction {
            _table_dir: table_dir.to_path_buf(),
            journal_path,
            journal,
        };
        t.flush()?;
        Ok(t)
    }

    pub fn backup(&mut self, path: &Path) -> crate::errors::Result<()> {
        let content = fs::read_to_string(path)?;
        self.journal.entries.push(JournalEntry {
            action: "modify".to_string(),
            path: path.to_string_lossy().to_string(),
            backup: Some(content),
        });
        self.flush()
    }

    pub fn record_create(&mut self, path: &Path) -> crate::errors::Result<()> {
        self.journal.entries.push(JournalEntry {
            action: "create".to_string(),
            path: path.to_string_lossy().to_string(),
            backup: None,
        });
        self.flush()
    }

    pub fn record_delete(&mut self, path: &Path, content: &str) -> crate::errors::Result<()> {
        self.journal.entries.push(JournalEntry {
            action: "delete".to_string(),
            path: path.to_string_lossy().to_string(),
            backup: Some(content.to_string()),
        });
        self.flush()
    }

    pub fn rollback(&self) -> crate::errors::Result<()> {
        for entry in self.journal.entries.iter().rev() {
            let path = Path::new(&entry.path);
            match entry.action.as_str() {
                "modify" => {
                    if let Some(ref backup) = entry.backup {
                        let _ = atomic_write(path, backup);
                    }
                }
                "create" => {
                    if path.exists() {
                        let _ = fs::remove_file(path);
                    }
                }
                "delete" => {
                    if let Some(ref backup) = entry.backup {
                        let _ = atomic_write(path, backup);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn commit(&self) -> crate::errors::Result<()> {
        let _ = fs::remove_file(&self.journal_path);
        Ok(())
    }

    fn flush(&self) -> crate::errors::Result<()> {
        let content = serde_json::to_string(&self.journal)
            .map_err(|e| MdqlError::General(e.to_string()))?;
        atomic_write(&self.journal_path, &content)
    }
}

/// Context manager equivalent — runs a closure within a transaction.
/// On success, commits. On error, rolls back and re-raises.
pub fn with_multi_file_txn<F>(
    table_dir: &Path,
    operation: &str,
    f: F,
) -> crate::errors::Result<()>
where
    F: FnOnce(&mut TableTransaction) -> crate::errors::Result<()>,
{
    let mut txn = TableTransaction::new(table_dir, operation)?;
    match f(&mut txn) {
        Ok(()) => {
            txn.commit()?;
            Ok(())
        }
        Err(e) => {
            let _ = txn.rollback();
            let _ = txn.commit(); // Clean up journal after rollback
            Err(e)
        }
    }
}

/// If a journal exists from a crashed transaction, roll back.
/// Returns true if recovery was performed.
pub fn recover_journal(table_dir: &Path) -> crate::errors::Result<bool> {
    let journal_path = table_dir.join(JOURNAL_FILENAME);
    if !journal_path.exists() {
        cleanup_tmp_files(table_dir);
        return Ok(false);
    }

    let text = match fs::read_to_string(&journal_path) {
        Ok(t) => t,
        Err(e) => {
            let corrupt_path = journal_path.with_extension("corrupt");
            let _ = fs::rename(&journal_path, &corrupt_path);
            return Err(MdqlError::JournalRecovery(format!(
                "Corrupt journal in {}, renamed to {}: {}",
                table_dir.display(),
                corrupt_path.file_name().unwrap_or_default().to_string_lossy(),
                e
            )));
        }
    };

    let journal: Journal = match serde_json::from_str(&text) {
        Ok(j) => j,
        Err(e) => {
            let corrupt_path = journal_path.with_extension("corrupt");
            let _ = fs::rename(&journal_path, &corrupt_path);
            return Err(MdqlError::JournalRecovery(format!(
                "Corrupt journal in {}, renamed to {}: {}",
                table_dir.display(),
                corrupt_path.file_name().unwrap_or_default().to_string_lossy(),
                e
            )));
        }
    };

    // Roll back in reverse
    for entry in journal.entries.iter().rev() {
        let path = Path::new(&entry.path);
        match entry.action.as_str() {
            "modify" => {
                if let Some(ref backup) = entry.backup {
                    let _ = atomic_write(path, backup);
                }
            }
            "create" => {
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
            "delete" => {
                if let Some(ref backup) = entry.backup {
                    let _ = atomic_write(path, backup);
                }
            }
            _ => {}
        }
    }

    let _ = fs::remove_file(&journal_path);
    cleanup_tmp_files(table_dir);
    Ok(true)
}

fn cleanup_tmp_files(table_dir: &Path) {
    if let Ok(entries) = fs::read_dir(table_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().ends_with(TMP_SUFFIX) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}
