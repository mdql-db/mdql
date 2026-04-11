"""Tests that Rust CLI and Python API produce identical results.

These tests run the same operations through both the Rust CLI (via subprocess)
and the Python API, then compare the outputs to catch any divergence.
"""

import json
import subprocess
from pathlib import Path

import pytest

from mdql.api import Database, Table
from mdql.loader import load_table
from mdql.schema import load_schema

FIXTURES = Path(__file__).parent / "fixtures"
EXAMPLES = Path(__file__).parent.parent / "examples"

MDQL_BIN = Path(__file__).parent.parent / "target" / "debug" / "mdql"


def run_cli(args: list[str]) -> str:
    """Run the mdql CLI and return stdout."""
    result = subprocess.run(
        [str(MDQL_BIN)] + args,
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        pytest.fail(f"CLI failed: {result.stderr}")
    return result.stdout


def cli_query_json(folder: str, sql: str) -> list[dict]:
    """Run a CLI query with --format json and parse the result."""
    output = run_cli(["query", folder, sql, "--format", "json"])
    return json.loads(output)


@pytest.fixture(autouse=True)
def require_cli():
    """Skip all tests if the CLI binary hasn't been built."""
    if not MDQL_BIN.exists():
        pytest.skip("mdql CLI not built — run `cargo build` first")


class TestSchemaAlignment:
    """Schema loading produces the same structure in both Rust and Python."""

    def test_schema_filename(self):
        """Both use _mdql.md as the schema filename."""
        from mdql._native import RustTable
        t = RustTable(str(FIXTURES / "valid_table"))
        assert t.name == "notes"
        # CLI should also find the schema
        output = run_cli(["validate", str(FIXTURES / "valid_table")])
        assert "valid" in output.lower()

    def test_schema_fields_match(self):
        """Python schema fields match what CLI reports."""
        schema = load_schema(FIXTURES / "valid_table")
        cli_output = run_cli(["schema", str(FIXTURES / "valid_table")])
        for field_name in schema.frontmatter:
            assert field_name in cli_output

    def test_schema_table_name(self):
        """Table name matches between Python and CLI."""
        schema = load_schema(FIXTURES / "valid_table")
        cli_output = run_cli(["schema", str(FIXTURES / "valid_table")])
        assert schema.table in cli_output


class TestQueryAlignment:
    """Same SQL query returns the same data from CLI and Python API."""

    def test_select_star_row_count(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, _ = table.query("SELECT * FROM notes")
        cli_rows = cli_query_json(str(FIXTURES / "valid_table"), "SELECT * FROM notes")
        assert len(py_rows) == len(cli_rows)

    def test_select_columns_match(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, py_cols = table.query("SELECT title, author FROM notes")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT title, author FROM notes",
        )
        assert py_cols == ["title", "author"]
        assert set(cli_rows[0].keys()) == {"title", "author"}

    def test_select_values_match(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, _ = table.query("SELECT title FROM notes ORDER BY title ASC")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT title FROM notes ORDER BY title ASC",
        )
        py_titles = [r["title"] for r in py_rows]
        cli_titles = [r["title"] for r in cli_rows]
        assert py_titles == cli_titles

    def test_where_filter_match(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, _ = table.query(
            "SELECT title FROM notes WHERE status = 'draft' ORDER BY title ASC"
        )
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT title FROM notes WHERE status = 'draft' ORDER BY title ASC",
        )
        py_titles = [r["title"] for r in py_rows]
        cli_titles = [r["title"] for r in cli_rows]
        assert py_titles == cli_titles

    def test_limit_match(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, _ = table.query("SELECT title FROM notes LIMIT 2")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT title FROM notes LIMIT 2",
        )
        assert len(py_rows) == len(cli_rows) == 2

    def test_order_by_desc_match(self):
        table = Table(FIXTURES / "valid_table")
        py_rows, _ = table.query("SELECT title FROM notes ORDER BY title DESC")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT title FROM notes ORDER BY title DESC",
        )
        py_titles = [r["title"] for r in py_rows]
        cli_titles = [r["title"] for r in cli_rows]
        assert py_titles == cli_titles


class TestLoadAlignment:
    """Table.load() matches CLI row counts and data."""

    def test_row_count(self):
        py_schema, py_rows, py_errors = load_table(FIXTURES / "valid_table")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT * FROM notes",
        )
        assert len(py_rows) == len(cli_rows)

    def test_field_values(self):
        """Spot-check that field values are identical."""
        py_schema, py_rows, _ = load_table(FIXTURES / "valid_table")
        cli_rows = cli_query_json(
            str(FIXTURES / "valid_table"),
            "SELECT path, title, author FROM notes ORDER BY path ASC",
        )
        py_sorted = sorted(py_rows, key=lambda r: r["path"])
        for py_row, cli_row in zip(py_sorted, cli_rows):
            assert py_row["title"] == cli_row["title"]
            assert py_row["author"] == cli_row["author"]


class TestValidationAlignment:
    """Validation produces consistent results."""

    def test_valid_table(self):
        table = Table(FIXTURES / "valid_table")
        py_errors = table.validate()
        cli_output = run_cli(["validate", str(FIXTURES / "valid_table")])
        if len(py_errors) == 0:
            assert "valid" in cli_output.lower()

    def test_invalid_table(self):
        table = Table(FIXTURES / "invalid_table")
        py_errors = table.validate()
        result = subprocess.run(
            [str(MDQL_BIN), "validate", str(FIXTURES / "invalid_table")],
            capture_output=True, text=True,
        )
        # Both should report errors
        assert len(py_errors) > 0
        assert result.returncode != 0 or "error" in result.stderr.lower() or "invalid" in result.stdout.lower()


class TestWriteAlignment:
    """Insert/update through Python produces files the CLI can read."""

    def test_python_insert_cli_reads(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: items\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  count:\n    type: int\n    required: false\n"
            "h1:\n  required: false\n---\n"
        )
        table = Table(tmp_path)
        table.insert({"title": "Test Item", "count": 42})

        # CLI should be able to query this file
        cli_rows = cli_query_json(
            str(tmp_path),
            "SELECT title, count FROM items",
        )
        assert len(cli_rows) == 1
        assert cli_rows[0]["title"] == "Test Item"
        assert cli_rows[0]["count"] == 42

    def test_python_insert_cli_validates(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: items\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )
        table = Table(tmp_path)
        table.insert({"title": "Valid Item"})

        output = run_cli(["validate", str(tmp_path)])
        assert "valid" in output.lower()

    def test_cli_insert_python_reads(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(
            "---\ntype: schema\ntable: items\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  count:\n    type: int\n    required: false\n"
            "h1:\n  required: false\n---\n"
        )
        run_cli([
            "query", str(tmp_path),
            "INSERT INTO items (title, count) VALUES ('CLI Item', 99)",
        ])

        table = Table(tmp_path)
        rows, errors = table.load()
        assert len(rows) == 1
        assert rows[0]["title"] == "CLI Item"
        assert rows[0]["count"] == 99


@pytest.mark.skipif(
    not (EXAMPLES / "_mdql.md").exists(),
    reason="example data not present",
)
class TestExamplesAlignment:
    """Alignment tests on the real example data."""

    def test_strategies_row_count(self):
        table = Table(EXAMPLES / "strategies")
        py_rows, _ = table.load()
        cli_rows = cli_query_json(
            str(EXAMPLES / "strategies"),
            "SELECT * FROM strategies",
        )
        assert len(py_rows) == len(cli_rows)

    def test_database_query_match(self):
        db = Database(EXAMPLES)
        py_rows, py_cols = db.query(
            "SELECT title, composite FROM strategies ORDER BY composite DESC LIMIT 5"
        )
        cli_rows = cli_query_json(
            str(EXAMPLES),
            "SELECT title, composite FROM strategies ORDER BY composite DESC LIMIT 5",
        )
        py_titles = [r["title"] for r in py_rows]
        cli_titles = [r["title"] for r in cli_rows]
        assert py_titles == cli_titles
