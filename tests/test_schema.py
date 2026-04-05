"""Tests for mdql.schema."""

from pathlib import Path

import pytest

from mdql.schema import Schema, load_schema
from mdql.errors import SchemaNotFoundError, SchemaInvalidError

FIXTURES = Path(__file__).parent / "fixtures"


class TestLoadSchema:
    def test_load_valid_table_schema(self):
        s = load_schema(FIXTURES / "valid_table")
        assert s.table == "notes"
        assert s.primary_key == "path"
        assert "title" in s.frontmatter
        assert s.frontmatter["title"].type == "string"
        assert s.frontmatter["title"].required is True
        assert s.frontmatter["tags"].type == "string[]"
        assert s.frontmatter["tags"].required is False
        assert s.frontmatter["status"].enum == ["draft", "approved", "archived"]
        assert s.h1_required is False
        assert s.reject_unknown_sections is False
        assert s.normalize_numbered_headings is True

    def test_load_strict_schema(self):
        s = load_schema(FIXTURES / "strict_table")
        assert s.table == "docs"
        assert s.h1_required is True
        assert s.h1_must_equal_frontmatter == "title"
        assert "Summary" in s.sections
        assert s.sections["Summary"].required is True
        assert s.reject_unknown_sections is True

    def test_load_invalid_table_schema(self):
        s = load_schema(FIXTURES / "invalid_table")
        assert s.table == "broken"
        assert s.frontmatter["count"].type == "int"
        assert s.sections["Body"].required is True

    def test_missing_schema_raises(self, tmp_path):
        with pytest.raises(SchemaNotFoundError):
            load_schema(tmp_path)

    def test_invalid_schema_missing_type(self, tmp_path):
        (tmp_path / "_schema.md").write_text("---\ntable: test\n---\n")
        with pytest.raises(SchemaInvalidError, match="type: schema"):
            load_schema(tmp_path)

    def test_invalid_schema_missing_table(self, tmp_path):
        (tmp_path / "_schema.md").write_text("---\ntype: schema\n---\n")
        with pytest.raises(SchemaInvalidError, match="table"):
            load_schema(tmp_path)

    def test_invalid_field_type(self, tmp_path):
        (tmp_path / "_schema.md").write_text(
            "---\ntype: schema\ntable: t\nfrontmatter:\n  x:\n    type: xml\n    required: true\n---\n"
        )
        with pytest.raises(SchemaInvalidError, match="invalid type 'xml'"):
            load_schema(tmp_path)
