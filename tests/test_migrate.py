"""Tests for field migration operations (rename, drop, merge)."""

from pathlib import Path

import pytest
import yaml

from mdql.api import Table
from mdql.errors import MdqlError
from mdql.migrate import (
    drop_frontmatter_key_in_file,
    drop_section_in_file,
    merge_sections_in_file,
    rename_frontmatter_key_in_file,
    rename_section_in_file,
    update_schema,
)
from mdql.query_parser import (
    AlterDropFieldQuery,
    AlterMergeFieldsQuery,
    AlterRenameFieldQuery,
    parse_query,
)


# ── Helpers ───────────────────────────────────────────────────────────────

SCHEMA_TEMPLATE = """\
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

FILE_WITH_SECTIONS = """\
---
title: "Test doc"
status: "active"
---

## Summary

This is the summary.

## Details

These are the details.
"""

FILE_WITH_NUMBERED = """\
---
title: "Numbered"
---

## 1. Summary

Numbered summary.

## 2. Details

Numbered details.
"""


def _make_table(tmp_path, schema=SCHEMA_TEMPLATE, files=None):
    """Create a table directory with schema and optional data files."""
    (tmp_path / "_mdql.md").write_text(schema)
    for name, content in (files or {}).items():
        (tmp_path / name).write_text(content)
    return tmp_path


# ── File-level: frontmatter rename ────────────────────────────────────────


class TestRenameFrontmatterKey:
    def test_rename_simple_key(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\nstatus: "active"\n---\n\n## Summary\n\nBody.\n')
        assert rename_frontmatter_key_in_file(f, "status", "state") is True
        text = f.read_text()
        assert "state:" in text
        assert "status:" not in text

    def test_rename_preserves_value(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\nstatus: "active"\n---\n')
        rename_frontmatter_key_in_file(f, "status", "state")
        text = f.read_text()
        assert 'state: "active"' in text

    def test_rename_missing_key_returns_false(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\n---\n')
        assert rename_frontmatter_key_in_file(f, "status", "state") is False

    def test_rename_multiline_value(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\ntags:\n  - a\n  - b\n---\n')
        assert rename_frontmatter_key_in_file(f, "tags", "labels") is True
        text = f.read_text()
        assert "labels:" in text
        assert "  - a" in text


# ── File-level: frontmatter drop ──────────────────────────────────────────


class TestDropFrontmatterKey:
    def test_drop_simple_key(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\nstatus: "active"\n---\n')
        assert drop_frontmatter_key_in_file(f, "status") is True
        text = f.read_text()
        assert "status" not in text
        assert "title" in text

    def test_drop_multiline_key(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\ntags:\n  - a\n  - b\nstatus: "x"\n---\n')
        assert drop_frontmatter_key_in_file(f, "tags") is True
        text = f.read_text()
        assert "tags" not in text
        assert "  - a" not in text
        assert 'status: "x"' in text

    def test_drop_missing_key_returns_false(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text('---\ntitle: "hello"\n---\n')
        assert drop_frontmatter_key_in_file(f, "status") is False


# ── File-level: section rename ────────────────────────────────────────────


class TestRenameSectionInFile:
    def test_rename_section(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert rename_section_in_file(f, "Summary", "Overview") is True
        text = f.read_text()
        assert "## Overview" in text
        assert "## Summary" not in text
        assert "This is the summary." in text

    def test_rename_with_normalization(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_NUMBERED)
        assert rename_section_in_file(f, "Summary", "Overview", normalize=True) is True
        text = f.read_text()
        assert "## Overview" in text

    def test_rename_missing_section_returns_false(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert rename_section_in_file(f, "Missing", "New") is False

    def test_rename_preserves_other_sections(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        rename_section_in_file(f, "Summary", "Overview")
        text = f.read_text()
        assert "## Details" in text
        assert "These are the details." in text

    def test_rename_inside_code_fence_not_touched(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(
            '---\ntitle: "x"\n---\n\n## Summary\n\n```\n## Summary\nfake heading\n```\n'
        )
        rename_section_in_file(f, "Summary", "Overview")
        text = f.read_text()
        # The real heading should be renamed
        assert text.count("## Overview") == 1
        # The one inside the fence should remain
        assert "## Summary" in text


# ── File-level: section drop ──────────────────────────────────────────────


class TestDropSectionInFile:
    def test_drop_section(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert drop_section_in_file(f, "Details") is True
        text = f.read_text()
        assert "## Details" not in text
        assert "These are the details." not in text
        assert "## Summary" in text

    def test_drop_first_section(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert drop_section_in_file(f, "Summary") is True
        text = f.read_text()
        assert "## Summary" not in text
        assert "## Details" in text

    def test_drop_missing_returns_false(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert drop_section_in_file(f, "Missing") is False

    def test_drop_with_normalization(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_NUMBERED)
        assert drop_section_in_file(f, "Summary", normalize=True) is True
        text = f.read_text()
        assert "Summary" not in text
        assert "## 2. Details" in text


# ── File-level: section merge ─────────────────────────────────────────────


class TestMergeSectionsInFile:
    def test_merge_two_sections(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert merge_sections_in_file(f, ["Summary", "Details"], "Combined") is True
        text = f.read_text()
        assert "## Combined" in text
        assert "This is the summary." in text
        assert "These are the details." in text
        assert "## Summary" not in text
        assert "## Details" not in text

    def test_merge_fewer_than_two_returns_false(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_SECTIONS)
        assert merge_sections_in_file(f, ["Summary", "Missing"], "X") is False

    def test_merge_with_normalization(self, tmp_path):
        f = tmp_path / "test.md"
        f.write_text(FILE_WITH_NUMBERED)
        assert merge_sections_in_file(
            f, ["Summary", "Details"], "All", normalize=True
        ) is True
        text = f.read_text()
        assert "## All" in text
        assert "Numbered summary." in text
        assert "Numbered details." in text


# ── Schema update ─────────────────────────────────────────────────────────


class TestUpdateSchema:
    def test_rename_section_in_schema(self, tmp_path):
        schema_path = tmp_path / "_mdql.md"
        schema_path.write_text(SCHEMA_TEMPLATE)
        update_schema(schema_path, rename_section=("Summary", "Overview"))
        fm = yaml.safe_load(schema_path.read_text().split("---")[1])
        assert "Overview" in fm["sections"]
        assert "Summary" not in fm["sections"]

    def test_drop_section_in_schema(self, tmp_path):
        schema_path = tmp_path / "_mdql.md"
        schema_path.write_text(SCHEMA_TEMPLATE)
        update_schema(schema_path, drop_section="Details")
        fm = yaml.safe_load(schema_path.read_text().split("---")[1])
        assert "Details" not in fm["sections"]
        assert "Summary" in fm["sections"]

    def test_merge_sections_in_schema(self, tmp_path):
        schema_path = tmp_path / "_mdql.md"
        schema_path.write_text(SCHEMA_TEMPLATE)
        update_schema(schema_path, merge_sections=(["Summary", "Details"], "Combined"))
        fm = yaml.safe_load(schema_path.read_text().split("---")[1])
        assert "Combined" in fm["sections"]
        assert "Summary" not in fm["sections"]
        assert "Details" not in fm["sections"]

    def test_rename_frontmatter_in_schema(self, tmp_path):
        schema_path = tmp_path / "_mdql.md"
        schema_path.write_text(SCHEMA_TEMPLATE)
        update_schema(schema_path, rename_frontmatter=("status", "state"))
        fm = yaml.safe_load(schema_path.read_text().split("---")[1])
        assert "state" in fm["frontmatter"]
        assert "status" not in fm["frontmatter"]

    def test_drop_frontmatter_in_schema(self, tmp_path):
        schema_path = tmp_path / "_mdql.md"
        schema_path.write_text(SCHEMA_TEMPLATE)
        update_schema(schema_path, drop_frontmatter="status")
        fm = yaml.safe_load(schema_path.read_text().split("---")[1])
        assert "status" not in fm["frontmatter"]
        assert "title" in fm["frontmatter"]


# ── Table-level operations ────────────────────────────────────────────────


class TestTableRenameField:
    def test_rename_section_field(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS, "b.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        count = t.rename_field("Summary", "Overview")
        assert count == 2
        assert "Overview" in t.schema.sections
        assert "Summary" not in t.schema.sections
        for f in ["a.md", "b.md"]:
            text = (tmp_path / f).read_text()
            assert "## Overview" in text

    def test_rename_frontmatter_field(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        count = t.rename_field("status", "state")
        assert count == 1
        assert "state" in t.schema.frontmatter
        assert "status" not in t.schema.frontmatter

    def test_rename_unknown_field_raises(self, tmp_path):
        _make_table(tmp_path)
        t = Table(tmp_path)
        with pytest.raises(MdqlError, match="not found in schema"):
            t.rename_field("bogus", "new")


class TestTableDropField:
    def test_drop_section_field(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        count = t.drop_field("Details")
        assert count == 1
        assert "Details" not in t.schema.sections
        text = (tmp_path / "a.md").read_text()
        assert "## Details" not in text

    def test_drop_frontmatter_field(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        count = t.drop_field("status")
        assert count == 1
        assert "status" not in t.schema.frontmatter


class TestTableMergeFields:
    def test_merge_section_fields(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        count = t.merge_fields(["Summary", "Details"], into="Combined")
        assert count == 1
        assert "Combined" in t.schema.sections
        assert "Summary" not in t.schema.sections
        assert "Details" not in t.schema.sections

    def test_merge_frontmatter_raises(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        with pytest.raises(MdqlError, match="Cannot merge frontmatter"):
            t.merge_fields(["title", "status"], into="combined")


# ── ALTER TABLE query parser ──────────────────────────────────────────────


class TestAlterTableParser:
    def test_rename_field(self):
        stmt = parse_query("ALTER TABLE strategies RENAME FIELD 'Summary' TO 'Overview'")
        assert isinstance(stmt, AlterRenameFieldQuery)
        assert stmt.table == "strategies"
        assert stmt.old_name == "Summary"
        assert stmt.new_name == "Overview"

    def test_rename_field_backtick(self):
        stmt = parse_query("ALTER TABLE t RENAME FIELD `Old Name` TO `New Name`")
        assert isinstance(stmt, AlterRenameFieldQuery)
        assert stmt.old_name == "Old Name"
        assert stmt.new_name == "New Name"

    def test_drop_field(self):
        stmt = parse_query("ALTER TABLE strategies DROP FIELD 'Details'")
        assert isinstance(stmt, AlterDropFieldQuery)
        assert stmt.table == "strategies"
        assert stmt.field_name == "Details"

    def test_merge_fields(self):
        stmt = parse_query(
            "ALTER TABLE strategies MERGE FIELDS 'Summary', 'Details' INTO 'Combined'"
        )
        assert isinstance(stmt, AlterMergeFieldsQuery)
        assert stmt.table == "strategies"
        assert stmt.sources == ["Summary", "Details"]
        assert stmt.into == "Combined"

    def test_merge_three_fields(self):
        stmt = parse_query(
            "ALTER TABLE t MERGE FIELDS 'A', 'B', 'C' INTO 'D'"
        )
        assert isinstance(stmt, AlterMergeFieldsQuery)
        assert stmt.sources == ["A", "B", "C"]

    def test_rename_unquoted_idents(self):
        stmt = parse_query("ALTER TABLE t RENAME FIELD status TO state")
        assert isinstance(stmt, AlterRenameFieldQuery)
        assert stmt.old_name == "status"
        assert stmt.new_name == "state"


# ── ALTER TABLE via execute_sql ───────────────────────────────────────────


class TestAlterTableExecution:
    def test_rename_via_sql(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        result = t.execute_sql("ALTER TABLE test RENAME FIELD 'Summary' TO 'Overview'")
        assert "renamed" in result
        assert "Overview" in t.schema.sections

    def test_drop_via_sql(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        result = t.execute_sql("ALTER TABLE test DROP FIELD 'Details'")
        assert "dropped" in result
        assert "Details" not in t.schema.sections

    def test_merge_via_sql(self, tmp_path):
        _make_table(tmp_path, files={"a.md": FILE_WITH_SECTIONS})
        t = Table(tmp_path)
        result = t.execute_sql(
            "ALTER TABLE test MERGE FIELDS 'Summary', 'Details' INTO 'Combined'"
        )
        assert "merged" in result
        assert "Combined" in t.schema.sections
