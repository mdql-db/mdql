"""Tests for ACID transaction primitives (txn.py)."""

import json
import multiprocessing
import time
from pathlib import Path

import pytest

from mdql.api import Table
from mdql.errors import JournalRecoveryError
from mdql.txn import (
    JOURNAL_FILENAME,
    LOCK_FILENAME,
    TMP_SUFFIX,
    atomic_write,
    multi_file_txn,
    recover_journal,
    table_lock,
)


# ── Helpers ───────────────────────────────────────────────────────────────

SCHEMA = """\
---
type: schema
table: test
primary_key: path
frontmatter:
  title:
    type: string
    required: true
  status:
    type: string
    required: false
h1:
  required: false
sections:
  Summary:
    type: markdown
    required: false
  Details:
    type: markdown
    required: false
rules:
  reject_unknown_frontmatter: false
  reject_unknown_sections: false
  reject_duplicate_sections: true
  normalize_numbered_headings: false
---
"""

FILE_A = """\
---
title: "File A"
status: "active"
---

## Summary

A summary.

## Details

A details.
"""

FILE_B = """\
---
title: "File B"
status: "draft"
---

## Summary

B summary.
"""


def _make_table(tmp_path, files=None):
    (tmp_path / "_mdql.md").write_text(SCHEMA)
    for name, content in (files or {}).items():
        (tmp_path / name).write_text(content)
    return tmp_path


# ── atomic_write ──────────────────────────────────────────────────────────


class TestAtomicWrite:
    def test_creates_new_file(self, tmp_path):
        p = tmp_path / "new.md"
        atomic_write(p, "hello world")
        assert p.read_text() == "hello world"

    def test_replaces_existing_file(self, tmp_path):
        p = tmp_path / "existing.md"
        p.write_text("old content")
        atomic_write(p, "new content")
        assert p.read_text() == "new content"

    def test_no_temp_files_left(self, tmp_path):
        p = tmp_path / "clean.md"
        atomic_write(p, "content")
        tmp_files = list(tmp_path.glob(f"*{TMP_SUFFIX}"))
        assert tmp_files == []

    def test_preserves_unicode(self, tmp_path):
        p = tmp_path / "unicode.md"
        atomic_write(p, "Hello \u2603 snowman \U0001f680 rocket")
        assert "\u2603" in p.read_text()
        assert "\U0001f680" in p.read_text()


# ── table_lock ────────────────────────────────────────────────────────────


def _lock_worker(folder_str: str, result_file: str):
    """Worker process that acquires a lock, writes a timestamp, and releases."""
    folder = Path(folder_str)
    with table_lock(folder):
        Path(result_file).write_text(str(time.monotonic()))
        time.sleep(0.5)


class TestTableLock:
    def test_lock_creates_lockfile(self, tmp_path):
        with table_lock(tmp_path):
            assert (tmp_path / LOCK_FILENAME).exists()

    def test_sequential_lock_does_not_deadlock(self, tmp_path):
        with table_lock(tmp_path):
            pass
        with table_lock(tmp_path):
            pass

    def test_lock_provides_mutual_exclusion(self, tmp_path):
        """Two processes with the same lock should serialize."""
        result_a = str(tmp_path / "result_a.txt")
        result_b = str(tmp_path / "result_b.txt")

        p1 = multiprocessing.Process(
            target=_lock_worker, args=(str(tmp_path), result_a)
        )
        p2 = multiprocessing.Process(
            target=_lock_worker, args=(str(tmp_path), result_b)
        )

        p1.start()
        time.sleep(0.1)
        p2.start()

        p1.join(timeout=5)
        p2.join(timeout=5)

        # Both should have completed
        assert Path(result_a).exists()
        assert Path(result_b).exists()

        t1 = float(Path(result_a).read_text())
        t2 = float(Path(result_b).read_text())

        assert abs(t2 - t1) >= 0.3


# ── multi_file_txn ────────────────────────────────────────────────────────


class TestMultiFileTxn:
    def test_commit_deletes_journal(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})
        with multi_file_txn(tmp_path, "test op") as txn:
            txn.backup(tmp_path / "a.md")
            atomic_write(tmp_path / "a.md", "changed")
        assert not (tmp_path / JOURNAL_FILENAME).exists()
        assert (tmp_path / "a.md").read_text() == "changed"

    def test_rollback_on_exception(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A, "b.md": FILE_B})
        original_a = FILE_A
        original_b = FILE_B

        with pytest.raises(RuntimeError, match="boom"):
            with multi_file_txn(tmp_path, "test op") as txn:
                txn.backup(tmp_path / "a.md")
                atomic_write(tmp_path / "a.md", "changed A")
                txn.backup(tmp_path / "b.md")
                atomic_write(tmp_path / "b.md", "changed B")
                raise RuntimeError("boom")

        # Both files should be restored
        assert (tmp_path / "a.md").read_text() == original_a
        assert (tmp_path / "b.md").read_text() == original_b
        # Journal should be cleaned up
        assert not (tmp_path / JOURNAL_FILENAME).exists()

    def test_rollback_created_file(self, tmp_path):
        _make_table(tmp_path)
        new_file = tmp_path / "new.md"

        with pytest.raises(RuntimeError):
            with multi_file_txn(tmp_path, "test") as txn:
                atomic_write(new_file, "new content")
                txn.record_create(new_file)
                raise RuntimeError("fail")

        assert not new_file.exists()

    def test_rollback_deleted_file(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})

        with pytest.raises(RuntimeError):
            with multi_file_txn(tmp_path, "test") as txn:
                content = (tmp_path / "a.md").read_text()
                txn.record_delete(tmp_path / "a.md", content)
                (tmp_path / "a.md").unlink()
                raise RuntimeError("fail")

        assert (tmp_path / "a.md").exists()
        assert (tmp_path / "a.md").read_text() == FILE_A

    def test_journal_contents(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})
        journal_path = tmp_path / JOURNAL_FILENAME

        # Manually create a transaction to inspect the journal mid-flight
        from mdql.txn import TableTransaction
        txn = TableTransaction(tmp_path, "test op")
        txn.backup(tmp_path / "a.md")

        journal = json.loads(journal_path.read_text())
        assert journal["version"] == 1
        assert journal["operation"] == "test op"
        assert len(journal["entries"]) == 1
        assert journal["entries"][0]["action"] == "modify"
        assert journal["entries"][0]["backup"] == FILE_A

        txn.commit()
        assert not journal_path.exists()


# ── recover_journal ───────────────────────────────────────────────────────


class TestRecoverJournal:
    def test_no_journal_returns_false(self, tmp_path):
        assert recover_journal(tmp_path) is False

    def test_recovers_modified_files(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})

        # Simulate a crash: write journal, modify file, don't commit
        journal = {
            "version": 1,
            "operation": "crashed op",
            "started_at": "2026-04-05T00:00:00",
            "entries": [
                {"action": "modify", "path": str(tmp_path / "a.md"), "backup": FILE_A}
            ],
        }
        (tmp_path / JOURNAL_FILENAME).write_text(json.dumps(journal))
        atomic_write(tmp_path / "a.md", "corrupted content")

        assert recover_journal(tmp_path) is True
        assert (tmp_path / "a.md").read_text() == FILE_A
        assert not (tmp_path / JOURNAL_FILENAME).exists()

    def test_recovers_created_files(self, tmp_path):
        new_file = tmp_path / "new.md"
        new_file.write_text("should be deleted")

        journal = {
            "version": 1,
            "operation": "crashed insert",
            "started_at": "2026-04-05T00:00:00",
            "entries": [
                {"action": "create", "path": str(new_file), "backup": None}
            ],
        }
        (tmp_path / JOURNAL_FILENAME).write_text(json.dumps(journal))

        recover_journal(tmp_path)
        assert not new_file.exists()

    def test_recovers_deleted_files(self, tmp_path):
        journal = {
            "version": 1,
            "operation": "crashed delete",
            "started_at": "2026-04-05T00:00:00",
            "entries": [
                {"action": "delete", "path": str(tmp_path / "a.md"), "backup": FILE_A}
            ],
        }
        (tmp_path / JOURNAL_FILENAME).write_text(json.dumps(journal))

        recover_journal(tmp_path)
        assert (tmp_path / "a.md").exists()
        assert (tmp_path / "a.md").read_text() == FILE_A

    def test_corrupt_journal_raises(self, tmp_path):
        (tmp_path / JOURNAL_FILENAME).write_text("not valid json{{{")
        with pytest.raises(JournalRecoveryError):
            recover_journal(tmp_path)
        # Should be renamed to .corrupt
        assert (tmp_path / ".mdql_journal.corrupt").exists()

    def test_table_init_recovers(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})

        journal = {
            "version": 1,
            "operation": "crashed rename",
            "started_at": "2026-04-05T00:00:00",
            "entries": [
                {"action": "modify", "path": str(tmp_path / "a.md"), "backup": FILE_A}
            ],
        }
        (tmp_path / JOURNAL_FILENAME).write_text(json.dumps(journal))
        atomic_write(tmp_path / "a.md", "corrupted")

        # Table construction should trigger recovery
        t = Table(tmp_path)
        assert (tmp_path / "a.md").read_text() == FILE_A
        assert not (tmp_path / JOURNAL_FILENAME).exists()

    def test_cleans_up_orphaned_tmp_files(self, tmp_path):
        orphan = tmp_path / f"something{TMP_SUFFIX}"
        orphan.write_text("leftover")
        recover_journal(tmp_path)
        assert not orphan.exists()


# ── Integration: ACID in Table operations ─────────────────────────────────


class TestAcidIntegration:
    def test_rename_field_is_atomic(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A, "b.md": FILE_B})
        t = Table(tmp_path)
        t.rename_field("Summary", "Overview")

        # No journal left
        assert not (tmp_path / JOURNAL_FILENAME).exists()
        # Change applied
        assert "## Overview" in (tmp_path / "a.md").read_text()
        assert "## Overview" in (tmp_path / "b.md").read_text()

    def test_drop_field_is_atomic(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})
        t = Table(tmp_path)
        t.drop_field("Details")
        assert not (tmp_path / JOURNAL_FILENAME).exists()
        assert "## Details" not in (tmp_path / "a.md").read_text()

    def test_insert_uses_atomic_write(self, tmp_path):
        _make_table(tmp_path)
        t = Table(tmp_path)
        t.insert({"title": "New Entry"})
        # No temp files
        assert list(tmp_path.glob(f"*{TMP_SUFFIX}")) == []

    def test_update_uses_atomic_write(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A})
        t = Table(tmp_path)
        t.update("a.md", {"status": "archived"})
        assert list(tmp_path.glob(f"*{TMP_SUFFIX}")) == []

    def test_batch_update_is_atomic(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A, "b.md": FILE_B})
        t = Table(tmp_path)
        result = t.execute_sql("UPDATE test SET status = 'done'")
        assert "UPDATE 2" in result
        assert not (tmp_path / JOURNAL_FILENAME).exists()

    def test_batch_delete_is_atomic(self, tmp_path):
        _make_table(tmp_path, {"a.md": FILE_A, "b.md": FILE_B})
        t = Table(tmp_path)
        result = t.execute_sql("DELETE FROM test WHERE status = 'draft'")
        assert "DELETE 1" in result
        assert not (tmp_path / JOURNAL_FILENAME).exists()
        # File B (draft) deleted, File A (active) preserved
        assert (tmp_path / "a.md").exists()
        assert not (tmp_path / "b.md").exists()
