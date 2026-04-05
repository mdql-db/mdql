"""Tests for mdql.validator."""

from pathlib import Path

import pytest

from mdql.loader import load_table
from mdql.parser import parse_file
from mdql.schema import load_schema
from mdql.validator import validate_file

FIXTURES = Path(__file__).parent / "fixtures"


class TestValidTable:
    """All files in valid_table should pass validation."""

    @pytest.fixture
    def schema(self):
        return load_schema(FIXTURES / "valid_table")

    def test_simple(self, schema):
        p = parse_file(FIXTURES / "valid_table" / "simple.md", relative_to=FIXTURES / "valid_table")
        errors = validate_file(p, schema)
        assert errors == []

    def test_with_tags(self, schema):
        p = parse_file(FIXTURES / "valid_table" / "with-tags.md", relative_to=FIXTURES / "valid_table")
        errors = validate_file(p, schema)
        assert errors == []

    def test_numbered_headings(self, schema):
        p = parse_file(
            FIXTURES / "valid_table" / "numbered-headings.md",
            relative_to=FIXTURES / "valid_table",
            normalize_numbered=schema.normalize_numbered_headings,
        )
        errors = validate_file(p, schema)
        assert errors == []

    def test_code_fence(self, schema):
        p = parse_file(FIXTURES / "valid_table" / "with-code-fence.md", relative_to=FIXTURES / "valid_table")
        errors = validate_file(p, schema)
        assert errors == []

    def test_unknown_section_allowed(self, schema):
        p = parse_file(FIXTURES / "valid_table" / "unknown-section.md", relative_to=FIXTURES / "valid_table")
        errors = validate_file(p, schema)
        assert errors == []  # reject_unknown_sections is false


class TestStrictTable:
    @pytest.fixture
    def schema(self):
        return load_schema(FIXTURES / "strict_table")

    def test_valid_with_h1(self, schema):
        p = parse_file(FIXTURES / "strict_table" / "valid-with-h1.md", relative_to=FIXTURES / "strict_table")
        errors = validate_file(p, schema)
        assert errors == []

    def test_missing_h1(self, schema):
        p = parse_file(FIXTURES / "strict_table" / "missing-h1.md", relative_to=FIXTURES / "strict_table")
        errors = validate_file(p, schema)
        types = [e.error_type for e in errors]
        assert "missing_h1" in types

    def test_mismatched_h1(self, schema):
        p = parse_file(FIXTURES / "strict_table" / "mismatched-h1.md", relative_to=FIXTURES / "strict_table")
        errors = validate_file(p, schema)
        types = [e.error_type for e in errors]
        assert "h1_mismatch" in types

    def test_unknown_section_rejected(self, schema):
        p = parse_file(FIXTURES / "strict_table" / "unknown-section.md", relative_to=FIXTURES / "strict_table")
        errors = validate_file(p, schema)
        types = [e.error_type for e in errors]
        assert "unknown_section" in types


class TestInvalidTable:
    @pytest.fixture
    def schema(self):
        return load_schema(FIXTURES / "invalid_table")

    def test_missing_required_field(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "missing-field.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "missing_field" and e.field == "count" for e in errors)

    def test_wrong_type_int(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "wrong-type-int.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "type_mismatch" and e.field == "count" for e in errors)

    def test_wrong_type_date(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "wrong-type-date.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "type_mismatch" and e.field == "created" for e in errors)

    def test_duplicate_section(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "duplicate-section.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "duplicate_section" for e in errors)

    def test_unknown_section(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "unknown-section.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "unknown_section" for e in errors)

    def test_missing_required_section(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "missing-section.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "missing_section" and e.field == "Body" for e in errors)

    def test_malformed_yaml(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "malformed-yaml.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "parse_error" for e in errors)

    def test_unknown_frontmatter(self, schema):
        p = parse_file(FIXTURES / "invalid_table" / "unknown-frontmatter.md", relative_to=FIXTURES / "invalid_table")
        errors = validate_file(p, schema)
        assert any(e.error_type == "unknown_field" and e.field == "extra_field" for e in errors)


class TestLoader:
    def test_load_valid_table(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        assert schema.table == "notes"
        assert len(rows) > 0
        assert all(isinstance(r, dict) for r in rows)
        assert all("path" in r for r in rows)

    def test_load_invalid_table_collects_errors(self):
        schema, rows, errors = load_table(FIXTURES / "invalid_table")
        assert len(errors) > 0

    def test_row_has_frontmatter_fields(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        simple = next(r for r in rows if r["path"] == "simple.md")
        assert simple["title"] == "Simple note"
        assert simple["author"] == "Rasmus"
        assert simple["status"] == "draft"

    def test_row_has_section_content(self):
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        simple = next(r for r in rows if r["path"] == "simple.md")
        assert "Summary" in simple
        assert "simple note for testing" in simple["Summary"]

    def test_date_coercion(self):
        import datetime
        schema, rows, errors = load_table(FIXTURES / "valid_table")
        tagged = next(r for r in rows if r["path"] == "with-tags.md")
        # Quoted date string should be coerced to datetime.date
        assert isinstance(tagged["created"], datetime.date)
