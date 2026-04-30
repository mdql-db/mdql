"""Tests for sidecar checksum tamper detection."""

from pathlib import Path

import pytest

from mdql.api import Table
from mdql.migrate import regenerate_checksums


SCHEMA = """\
---
type: schema
table: items
primary_key: path
frontmatter:
  title:
    type: string
---
"""


def _make_table(tmp_path):
    (tmp_path / "_mdql.md").write_text(SCHEMA)
    t = Table(str(tmp_path))
    t.insert({"title": "Alpha"}, body="# Alpha\n", filename="alpha.md")
    t.insert({"title": "Beta"}, body="# Beta\n", filename="beta.md")
    return t


class TestRegenerateChecksums:
    def test_creates_checksum_file(self, tmp_path):
        t = _make_table(tmp_path)
        count = regenerate_checksums(str(tmp_path))
        assert count == 2
        assert (tmp_path / "_checksums.json").exists()

    def test_regenerate_idempotent(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))
        regenerate_checksums(str(tmp_path))
        import json
        data = json.loads((tmp_path / "_checksums.json").read_text())
        assert len(data["files"]) == 2


class TestWriteUpdatesChecksum:
    def test_insert_updates_checksum(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))

        t.insert({"title": "Gamma"}, body="# Gamma\n", filename="gamma.md")

        import json
        data = json.loads((tmp_path / "_checksums.json").read_text())
        assert "gamma.md" in data["files"]
        assert len(data["files"]) == 3

    def test_delete_removes_checksum(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))

        t.delete("alpha.md")

        import json
        data = json.loads((tmp_path / "_checksums.json").read_text())
        assert "alpha.md" not in data["files"]
        assert len(data["files"]) == 1

    def test_update_changes_checksum(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))

        import json
        before = json.loads((tmp_path / "_checksums.json").read_text())
        old_hash = before["files"]["alpha.md"]

        t.update("alpha.md", {"title": "Alpha Updated"}, body="# Alpha Updated\n")

        after = json.loads((tmp_path / "_checksums.json").read_text())
        assert after["files"]["alpha.md"] != old_hash


class TestTamperDetection:
    def test_unmodified_no_flag(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))

        rows, _ = t.load()
        for row in rows:
            assert "_modified_externally" not in row

    def test_tampered_file_flagged(self, tmp_path):
        t = _make_table(tmp_path)
        regenerate_checksums(str(tmp_path))

        (tmp_path / "alpha.md").write_text("---\ntitle: Hacked\n---\n# Hacked\n")

        rows, _ = t.load()
        alpha = next(r for r in rows if r["path"] == "alpha.md")
        assert alpha.get("_modified_externally") is True

    def test_no_checksums_no_flag(self, tmp_path):
        t = _make_table(tmp_path)
        rows, _ = t.load()
        for row in rows:
            assert "_modified_externally" not in row
