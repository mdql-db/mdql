"""ACID transaction primitives — wraps Rust _native txn functions."""

from __future__ import annotations

import contextlib
from pathlib import Path

from mdql._native import (
    atomic_write as _rust_atomic_write,
    recover_journal as _rust_recover,
    RustTableTransaction,
)

JOURNAL_FILENAME = ".mdql_journal"
LOCK_FILENAME = ".mdql_lock"
TMP_SUFFIX = ".mdql_tmp"


def atomic_write(path: str | Path, content: str) -> None:
    """Write content atomically via tempfile + rename."""
    _rust_atomic_write(str(path), content)


def recover_journal(folder: str | Path) -> bool:
    """Recover from a crash journal if one exists."""
    from mdql.errors import JournalRecoveryError
    try:
        return _rust_recover(str(folder))
    except RuntimeError as e:
        raise JournalRecoveryError(str(e)) from None


class TableTransaction:
    """Multi-file transaction with journal-based recovery."""

    def __init__(self, folder: str | Path, operation: str):
        self._inner = RustTableTransaction(str(folder), operation)

    def backup(self, path: str | Path):
        self._inner.backup(str(path))

    def record_create(self, path: str | Path):
        self._inner.record_create(str(path))

    def record_delete(self, path: str | Path, content: str):
        self._inner.record_delete(str(path), content)

    def commit(self):
        self._inner.commit()

    def rollback(self):
        self._inner.rollback()


@contextlib.contextmanager
def table_lock(folder: str | Path):
    """Acquire an exclusive lock on a table directory.

    Note: The Rust implementation uses RAII-based locking through the Table API.
    This Python wrapper provides a context manager for direct use.
    """
    import fcntl
    lock_path = Path(folder) / LOCK_FILENAME
    lock_path.touch()
    f = open(lock_path, "w")
    try:
        fcntl.flock(f.fileno(), fcntl.LOCK_EX)
        yield
    finally:
        fcntl.flock(f.fileno(), fcntl.LOCK_UN)
        f.close()


@contextlib.contextmanager
def multi_file_txn(folder: str | Path, operation: str):
    """Multi-file transaction with journal-based recovery.

    Note: Uses the Rust atomic_write and journal recovery internally.
    """
    import json
    folder = Path(folder)
    journal_path = folder / JOURNAL_FILENAME
    journal = {
        "operation": operation,
        "backups": {},
        "created": [],
    }

    class Txn:
        def backup(self, path: Path):
            if path.exists():
                journal["backups"][str(path)] = path.read_text()
                _write_journal()

        def record_create(self, path: Path):
            journal["created"].append(str(path))
            _write_journal()

        def record_delete(self, path: Path, content: str):
            journal["backups"][str(path)] = content
            _write_journal()

    def _write_journal():
        atomic_write(journal_path, json.dumps(journal))

    _write_journal()
    txn = Txn()
    try:
        yield txn
    except Exception:
        # Rollback
        for path_str, content in journal["backups"].items():
            atomic_write(path_str, content)
        for path_str in journal["created"]:
            p = Path(path_str)
            if p.exists():
                p.unlink()
        raise
    finally:
        if journal_path.exists():
            journal_path.unlink()
