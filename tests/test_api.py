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


    def test_insert_with_body(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "rules:\n  reject_unknown_sections: false\n---\n"
        )

        raw_body = "\n## Hypothesis\n\nWhen funding rates spike...\n\n## Entry Rules\n\n1. Wait for confirmation\n2. Enter short\n"

        table = Table(tmp_path)
        filepath = table.insert(
            {"title": "Body Test"},
            body=raw_body,
        )

        content = filepath.read_text()
        # Frontmatter is generated from data
        assert 'title: "Body Test"' in content
        assert "created:" in content
        # Body is passed through verbatim
        assert "## Hypothesis" in content
        assert "When funding rates spike..." in content
        assert "## Entry Rules" in content
        assert "1. Wait for confirmation" in content

    def test_insert_body_overrides_section_data(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: docs\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "sections:\n"
            "  Summary:\n    type: markdown\n    required: true\n---\n"
        )

        table = Table(tmp_path)
        filepath = table.insert(
            {"title": "Override Test", "Summary": "ignored"},
            body="\n## Summary\n\nReal content from external source.\n",
        )

        content = filepath.read_text()
        assert "Real content from external source." in content
        assert "ignored" not in content

    def test_insert_body_validates(self, tmp_path):
        """Body that violates schema (missing required section) is rejected."""
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: docs\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n"
            "sections:\n"
            "  Summary:\n    type: markdown\n    required: true\n---\n"
        )

        table = Table(tmp_path)
        with pytest.raises(MdqlError, match="Validation failed"):
            table.insert(
                {"title": "No Summary"},
                body="\n## Wrong Section\n\nThis isn't Summary.\n",
            )


SIMPLE_SCHEMA = (
    "---\ntype: schema\ntable: notes\n"
    "frontmatter:\n"
    "  title:\n    type: string\n    required: true\n"
    "  status:\n    type: string\n    required: false\n"
    "  priority:\n    type: int\n    required: false\n"
    "h1:\n  required: false\n"
    "rules:\n  reject_unknown_sections: false\n---\n"
)


class TestReplace:
    def test_replace_overwrites(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Original", "status": "draft"})
        table.insert(
            {"title": "Replaced", "status": "approved"},
            filename="original",
            replace=True,
        )

        content = (tmp_path / "original.md").read_text()
        assert 'title: "Replaced"' in content
        assert 'status: "approved"' in content

    def test_replace_preserves_created(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "First"})
        original = (tmp_path / "first.md").read_text()
        original_created = next(
            line for line in original.splitlines() if line.startswith("created:")
        )

        table.insert(
            {"title": "Second Version"},
            filename="first",
            replace=True,
        )

        replaced = (tmp_path / "first.md").read_text()
        replaced_created = next(
            line for line in replaced.splitlines() if line.startswith("created:")
        )
        assert original_created == replaced_created

    def test_replace_false_raises_on_conflict(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Exists"})
        with pytest.raises(MdqlError, match="already exists"):
            table.insert({"title": "Exists"}, replace=False)

    def test_replace_rolls_back_on_validation_failure(self, tmp_path):
        schema = (
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  status:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )
        (tmp_path / "_mdql.md").write_text(schema)
        table = Table(tmp_path)

        table.insert({"title": "Good", "status": "ok"})
        original = (tmp_path / "good.md").read_text()

        with pytest.raises(MdqlError, match="Validation failed"):
            # Missing required 'status' — should fail and roll back
            table.insert({"title": "Bad"}, filename="good", replace=True)

        # Original file should be restored
        assert (tmp_path / "good.md").read_text() == original

    def test_replace_with_body(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Note"}, body="\n## Old\n\nOld content.\n")
        table.insert(
            {"title": "Note"},
            body="\n## New\n\nNew content.\n",
            filename="note",
            replace=True,
        )

        content = (tmp_path / "note.md").read_text()
        assert "New content." in content
        assert "Old content." not in content


class TestUpdate:
    def test_update_changes_single_field(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Note", "status": "draft", "priority": 3})
        table.update("note.md", {"status": "approved"})

        content = (tmp_path / "note.md").read_text()
        assert 'status: "approved"' in content
        assert 'title: "Note"' in content  # preserved
        assert "priority: 3" in content  # preserved

    def test_update_preserves_body(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert(
            {"title": "Note"},
            body="\n## Hypothesis\n\nImportant content.\n",
        )
        table.update("note.md", {"status": "approved"})

        content = (tmp_path / "note.md").read_text()
        assert "Important content." in content

    def test_update_replaces_body_when_given(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert(
            {"title": "Note"},
            body="\n## Old\n\nOld stuff.\n",
        )
        table.update(
            "note.md",
            {},
            body="\n## New\n\nNew stuff.\n",
        )

        content = (tmp_path / "note.md").read_text()
        assert "New stuff." in content
        assert "Old stuff." not in content

    def test_update_preserves_created(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Note"})
        original = (tmp_path / "note.md").read_text()
        original_created = next(
            line for line in original.splitlines() if line.startswith("created:")
        )

        table.update("note.md", {"status": "done"})

        updated = (tmp_path / "note.md").read_text()
        updated_created = next(
            line for line in updated.splitlines() if line.startswith("created:")
        )
        assert original_created == updated_created

    def test_update_bumps_modified(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Note"})
        table.update("note.md", {"status": "done"})

        content = (tmp_path / "note.md").read_text()
        assert "modified:" in content

    def test_update_nonexistent_raises(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        with pytest.raises(MdqlError, match="File not found"):
            table.update("nope.md", {"status": "draft"})

    def test_update_rolls_back_on_validation_failure(self, tmp_path):
        schema = (
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  count:\n    type: int\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )
        (tmp_path / "_mdql.md").write_text(schema)
        table = Table(tmp_path)

        table.insert({"title": "Note", "count": 5})
        original = (tmp_path / "note.md").read_text()

        with pytest.raises(MdqlError, match="Validation failed"):
            # Setting count to a string should fail type validation
            table.update("note.md", {"count": "not-a-number"})

        assert (tmp_path / "note.md").read_text() == original

    def test_update_without_extension(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.insert({"title": "Note"})
        table.update("note", {"status": "done"})

        content = (tmp_path / "note.md").read_text()
        assert 'status: "done"' in content


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
