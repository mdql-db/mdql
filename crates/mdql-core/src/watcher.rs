//! Filesystem watcher that detects FK violations when files change on disk.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::errors::{MdqlError, ValidationError};

/// Watches a database directory for file changes and re-validates foreign keys.
pub struct FkWatcher {
    _watcher: RecommendedWatcher,
    errors_rx: mpsc::Receiver<Vec<ValidationError>>,
}

impl FkWatcher {
    /// Start watching a database directory. On any .md file change,
    /// re-runs FK validation and sends results on an internal channel.
    pub fn start(db_path: PathBuf) -> Result<Self, MdqlError> {
        let (tx, rx) = mpsc::channel();

        let watcher_db_path = db_path.clone();
        let mut last_run = Instant::now() - Duration::from_secs(10);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(_) => return,
            };

            // Only react to file changes (create, modify, rename, remove)
            let dominated = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            );
            if !dominated {
                return;
            }

            // Only react to .md files (ignore .lock, .journal, .swp, .tmp, etc.)
            let has_md = event.paths.iter().any(|p| {
                p.extension().and_then(|e| e.to_str()) == Some("md")
            });
            if !has_md {
                return;
            }

            // Debounce: skip if we validated within the last 500ms
            let now = Instant::now();
            if now.duration_since(last_run) < Duration::from_millis(500) {
                return;
            }
            last_run = now;

            // Re-validate
            if let Ok((_config, _tables, errors)) =
                crate::loader::load_database(&watcher_db_path)
            {
                let fk_errors: Vec<_> = errors
                    .into_iter()
                    .filter(|e| e.error_type == crate::errors::ValidationErrorKind::FkViolation || e.error_type == crate::errors::ValidationErrorKind::FkMissingTable)
                    .collect();
                let _ = tx.send(fk_errors);
            }
        })
        .map_err(|e| MdqlError::General(format!("Failed to start file watcher: {}", e)))?;

        watcher
            .watch(&db_path, RecursiveMode::Recursive)
            .map_err(|e| MdqlError::General(format!("Failed to watch directory: {}", e)))?;

        Ok(FkWatcher {
            _watcher: watcher,
            errors_rx: rx,
        })
    }

    /// Non-blocking: drain any pending FK validation results.
    /// Returns the most recent set of FK errors, or None if no changes detected.
    pub fn poll(&self) -> Option<Vec<ValidationError>> {
        let mut latest = None;
        // Drain all pending messages, keep only the most recent
        while let Ok(errors) = self.errors_rx.try_recv() {
            latest = Some(errors);
        }
        latest
    }
}
