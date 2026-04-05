"""Tests for mdql.stamp."""

from __future__ import annotations

import datetime
from pathlib import Path

import pytest

from mdql.stamp import TIMESTAMP_FIELDS, stamp_file, stamp_table

FIXTURES = Path(__file__).parent / "fixtures"
TODAY = datetime.date(2026, 4, 5)


class TestStampFile:
    def test_adds_both_timestamps(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text('---\ntitle: "Hello"\n---\n\n## Body\n\nContent.\n')

        result = stamp_file(f, now=TODAY)

        assert result["created_set"] is True
        assert result["modified_updated"] is True

        text = f.read_text()
        assert 'created: "2026-04-05"' in text
        assert 'modified: "2026-04-05"' in text

    def test_preserves_existing_created(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text('---\ntitle: "Hello"\ncreated: "2026-01-01"\n---\n\n## Body\n')

        result = stamp_file(f, now=TODAY)

        assert result["created_set"] is False
        assert result["modified_updated"] is True

        text = f.read_text()
        assert 'created: "2026-01-01"' in text
        assert 'modified: "2026-04-05"' in text

    def test_updates_existing_modified(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text(
            '---\ntitle: "Hello"\ncreated: "2026-01-01"\nmodified: "2026-03-01"\n---\n'
        )

        result = stamp_file(f, now=TODAY)

        assert result["created_set"] is False
        assert result["modified_updated"] is True

        text = f.read_text()
        assert 'created: "2026-01-01"' in text
        assert 'modified: "2026-04-05"' in text
        # Old date should be gone
        assert "2026-03-01" not in text

    def test_preserves_body(self, tmp_path):
        f = tmp_path / "note.md"
        original_body = "\n## Hypothesis\n\nSomething interesting.\n\n## Notes\n\nMore stuff.\n"
        f.write_text(f'---\ntitle: "Hello"\n---\n{original_body}')

        stamp_file(f, now=TODAY)

        text = f.read_text()
        assert "Something interesting." in text
        assert "More stuff." in text

    def test_preserves_other_frontmatter(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text('---\ntitle: "Hello"\nstatus: draft\ntags:\n  - python\n---\n')

        stamp_file(f, now=TODAY)

        text = f.read_text()
        assert 'title: "Hello"' in text
        assert "status: draft" in text
        assert "  - python" in text

    def test_no_frontmatter_skips(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text("# Just a heading\n\nNo frontmatter here.\n")

        result = stamp_file(f, now=TODAY)

        assert result["created_set"] is False
        assert result["modified_updated"] is False
        assert f.read_text() == "# Just a heading\n\nNo frontmatter here.\n"

    def test_unclosed_frontmatter_skips(self, tmp_path):
        f = tmp_path / "note.md"
        f.write_text("---\ntitle: broken\nno closing\n")

        result = stamp_file(f, now=TODAY)

        assert result["created_set"] is False
        assert result["modified_updated"] is False


class TestStampTable:
    def test_stamps_all_data_files(self, tmp_path):
        # Create a minimal schema
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\nfrontmatter: {}\n---\n"
        )
        (tmp_path / "a.md").write_text('---\ntitle: "A"\n---\n')
        (tmp_path / "b.md").write_text('---\ntitle: "B"\n---\n')

        results = stamp_table(tmp_path, now=TODAY)

        assert len(results) == 2
        filenames = [name for name, _ in results]
        assert "a.md" in filenames
        assert "b.md" in filenames
        assert "_mdql.md" not in filenames

        for _, r in results:
            assert r["created_set"] is True
            assert r["modified_updated"] is True


class TestValidatorAcceptsTimestamps:
    """Ensure stamped files pass validation even if schema doesn't declare timestamps."""

    def test_timestamps_not_rejected_as_unknown(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "rules:\n  reject_unknown_frontmatter: true\n---\n"
        )
        (tmp_path / "note.md").write_text(
            '---\ntitle: "Hello"\ncreated: "2026-04-05"\nmodified: "2026-04-05"\n---\n'
        )

        from mdql.loader import load_table

        schema, rows, errors = load_table(tmp_path)

        assert len(errors) == 0
        assert len(rows) == 1

    def test_invalid_timestamp_type_rejected(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )
        (tmp_path / "note.md").write_text(
            '---\ntitle: "Hello"\ncreated: "not-a-date"\n---\n'
        )

        from mdql.loader import load_table

        schema, rows, errors = load_table(tmp_path)

        assert len(errors) == 1
        assert "date" in errors[0].message
