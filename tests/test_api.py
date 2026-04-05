"""Tests for mdql.api — Table, Database, and insert."""

from pathlib import Path

import pytest

from mdql.api import Database, Table, _slugify
from mdql.errors import MdqlError

FIXTURES = Path(__file__).parent / "fixtures"
EXAMPLES = Path(__file__).parent.parent / "examples"


class TestSlugify:
    def test_simple(self):
        assert _slugify("Funding Rate Fade") == "funding-rate-fade"

    def test_special_chars(self):
        assert _slugify("AAVE: Health Factor 1.0!") == "aave-health-factor-10"

    def test_multiple_spaces(self):
        assert _slugify("too   many  spaces") == "too-many-spaces"

    def test_truncation(self):
        slug = _slugify("a" * 100, max_length=20)
        assert len(slug) <= 20


class TestTable:
    def test_init(self):
        table = Table(FIXTURES / "valid_table")
        assert table.name == "notes"
        assert table.path == FIXTURES / "valid_table"

    def test_load(self):
        table = Table(FIXTURES / "valid_table")
        rows, errors = table.load()
        assert len(rows) > 0
        assert len(errors) == 0

    def test_validate(self):
        table = Table(FIXTURES / "invalid_table")
        errors = table.validate()
        assert len(errors) > 0

    def test_insert(self, tmp_path):
        # Set up a minimal table
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  priority:\n    type: int\n    required: false\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({"title": "My First Note", "priority": 3})

        assert filepath.exists()
        assert filepath.name == "my-first-note.md"

        content = filepath.read_text()
        assert 'title: "My First Note"' in content
        assert "priority: 3" in content
        assert "created:" in content
        assert "modified:" in content

    def test_insert_with_list_field(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: items\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  tags:\n    type: string[]\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({
            "title": "Tagged Item",
            "tags": ["python", "testing"],
        })

        content = filepath.read_text()
        assert "  - python" in content
        assert "  - testing" in content

    def test_insert_custom_filename(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({"title": "Hello"}, filename="custom-name")

        assert filepath.name == "custom-name.md"

    def test_insert_duplicate_raises(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        table.insert({"title": "First"})

        with pytest.raises(MdqlError, match="already exists"):
            table.insert({"title": "First"})

    def test_insert_missing_required_field_raises(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  status:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        with pytest.raises(MdqlError, match="Validation failed"):
            table.insert({"title": "Missing Status"})

    def test_insert_no_title_no_filename_raises(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  count:\n    type: int\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        with pytest.raises(MdqlError, match="Cannot derive filename"):
            table.insert({"count": 5})

    def test_insert_scaffolds_required_sections(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: docs\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "sections:\n"
            "  Summary:\n    type: markdown\n    required: true\n"
            "  Notes:\n    type: markdown\n    required: false\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({"title": "Doc With Sections"})

        content = filepath.read_text()
        assert "## Summary" in content
        assert "## Notes" not in content  # not required, not provided

    def test_insert_with_section_content(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: docs\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "sections:\n"
            "  Summary:\n    type: markdown\n    required: true\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({
            "title": "With Content",
            "Summary": "This is the summary text.",
        })

        content = filepath.read_text()
        assert "## Summary" in content
        assert "This is the summary text." in content

    def test_insert_with_enum_field(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: items\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  status:\n    type: string\n    required: true\n"
            "    enum: [draft, approved]\n"
            "h1:\n  required: false\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert({"title": "Enum Test", "status": "draft"})
        assert filepath.exists()

    def test_created_file_passes_validation(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  priority:\n    type: int\n    required: true\n"
            "  tags:\n    type: string[]\n    required: false\n"
            "h1:\n  required: false\n"
            "rules:\n  reject_unknown_frontmatter: true\n---\n"
        )

        table = Table(tmp_path)
        table.insert({
            "title": "Validated Note",
            "priority": 5,
            "tags": ["a", "b"],
        })

        # Re-validate the whole table
        errors = table.validate()
        assert len(errors) == 0


class TestDatabase:
    @pytest.mark.skipif(
        not (EXAMPLES / "_mdql.md").exists(),
        reason="example data not present",
    )
    def test_init(self):
        db = Database(EXAMPLES)
        assert db.name == "zunid"
        assert "strategies" in db.tables
        assert "backtests" in db.tables

    @pytest.mark.skipif(
        not (EXAMPLES / "_mdql.md").exists(),
        reason="example data not present",
    )
    def test_table_accessor(self):
        db = Database(EXAMPLES)
        table = db.table("strategies")
        assert table.name == "strategies"

    @pytest.mark.skipif(
        not (EXAMPLES / "_mdql.md").exists(),
        reason="example data not present",
    )
    def test_unknown_table_raises(self):
        db = Database(EXAMPLES)
        with pytest.raises(MdqlError, match="not found"):
            db.table("nonexistent")


class TestCreateRowIntegration:
    @pytest.mark.skipif(
        not (EXAMPLES / "_mdql.md").exists(),
        reason="example data not present",
    )
    def test_create_strategy_via_database(self, tmp_path):
        """Create a strategy through the Database API, mimicking real usage."""
        import shutil

        # Copy the examples database to tmp for a writable test
        test_db = tmp_path / "db"
        shutil.copytree(EXAMPLES, test_db)

        db = Database(test_db)
        filepath = db.table("strategies").insert({
            "title": "Test Strategy From API",
            "status": "HYPOTHESIS",
            "mechanism": 5,
            "implementation": 4,
            "safety": 7,
            "frequency": 3,
            "composite": 420,
            "categories": ["exchange-structure"],
            "pipeline_stage": "Pre-backtest (step 2 of 9)",
        })

        assert filepath.exists()
        assert filepath.name == "test-strategy-from-api.md"

        # Verify it validates with the real schema
        errors = db.table("strategies").validate()
        assert not any(
            e.file_path == "test-strategy-from-api.md" for e in errors
        )
