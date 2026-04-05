"""Tests for SQL write operations: INSERT INTO, UPDATE SET, DELETE FROM."""

from pathlib import Path

import pytest

from mdql.api import Table
from mdql.errors import MdqlError, QueryParseError
from mdql.query_parser import (
    DeleteQuery,
    InsertQuery,
    UpdateQuery,
    parse_query,
)

SIMPLE_SCHEMA = (
    "---\ntype: schema\ntable: notes\n"
    "frontmatter:\n"
    "  title:\n    type: string\n    required: true\n"
    "  status:\n    type: string\n    required: false\n"
    "  priority:\n    type: int\n    required: false\n"
    "  tags:\n    type: string[]\n    required: false\n"
    "h1:\n  required: false\n"
    "rules:\n  reject_unknown_sections: false\n---\n"
)


# ── Parser tests ──────────────────────────────────────────────────────


class TestParseInsert:
    def test_basic(self):
        q = parse_query("INSERT INTO notes (title, status) VALUES ('Hello', 'draft')")
        assert isinstance(q, InsertQuery)
        assert q.table == "notes"
        assert q.columns == ["title", "status"]
        assert q.values == ["Hello", "draft"]

    def test_with_number(self):
        q = parse_query("INSERT INTO notes (title, priority) VALUES ('Hello', 5)")
        assert isinstance(q, InsertQuery)
        assert q.values == ["Hello", 5]

    def test_column_count_mismatch(self):
        with pytest.raises(QueryParseError, match="Column count"):
            parse_query("INSERT INTO notes (title) VALUES ('a', 'b')")

    def test_null_value(self):
        q = parse_query("INSERT INTO notes (title, status) VALUES ('Hello', NULL)")
        assert q.values == ["Hello", None]


class TestParseUpdate:
    def test_basic(self):
        q = parse_query("UPDATE notes SET status = 'approved'")
        assert isinstance(q, UpdateQuery)
        assert q.table == "notes"
        assert q.assignments == [("status", "approved")]
        assert q.where is None

    def test_multiple_assignments(self):
        q = parse_query("UPDATE notes SET status = 'done', priority = 1")
        assert isinstance(q, UpdateQuery)
        assert q.assignments == [("status", "done"), ("priority", 1)]

    def test_with_where(self):
        q = parse_query("UPDATE notes SET status = 'killed' WHERE path = 'old.md'")
        assert isinstance(q, UpdateQuery)
        assert q.where is not None

    def test_with_complex_where(self):
        q = parse_query(
            "UPDATE notes SET status = 'killed' WHERE priority > 3 AND status = 'draft'"
        )
        assert isinstance(q, UpdateQuery)
        assert q.where is not None


class TestParseDelete:
    def test_basic(self):
        q = parse_query("DELETE FROM notes")
        assert isinstance(q, DeleteQuery)
        assert q.table == "notes"
        assert q.where is None

    def test_with_where(self):
        q = parse_query("DELETE FROM notes WHERE path = 'old.md'")
        assert isinstance(q, DeleteQuery)
        assert q.where is not None

    def test_with_complex_where(self):
        q = parse_query("DELETE FROM notes WHERE status = 'draft' AND priority < 3")
        assert isinstance(q, DeleteQuery)
        assert q.where is not None


# ── Execution tests via Table.execute_sql ─────────────────────────────


class TestExecuteInsert:
    def test_insert_creates_file(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        result = table.execute_sql(
            "INSERT INTO notes (title, status) VALUES ('My Note', 'draft')"
        )

        assert "INSERT 1" in result
        assert (tmp_path / "my-note.md").exists()

        content = (tmp_path / "my-note.md").read_text()
        assert 'title: "My Note"' in content
        assert 'status: "draft"' in content
        assert "created:" in content
        assert "modified:" in content

    def test_insert_with_int(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql(
            "INSERT INTO notes (title, priority) VALUES ('Urgent', 10)"
        )

        content = (tmp_path / "urgent.md").read_text()
        assert "priority: 10" in content

    def test_insert_string_array_coercion(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql(
            "INSERT INTO notes (title, tags) VALUES ('Tagged', 'python,testing')"
        )

        content = (tmp_path / "tagged.md").read_text()
        assert "  - python" in content
        assert "  - testing" in content

    def test_insert_duplicate_fails(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('First')")
        with pytest.raises(MdqlError, match="already exists"):
            table.execute_sql("INSERT INTO notes (title) VALUES ('First')")

    def test_insert_validates(self, tmp_path):
        schema = (
            "---\ntype: schema\ntable: notes\n"
            "frontmatter:\n"
            "  title:\n    type: string\n    required: true\n"
            "  count:\n    type: int\n    required: true\n"
            "h1:\n  required: false\n---\n"
        )
        (tmp_path / "_mdql.md").write_text(schema)
        table = Table(tmp_path)

        with pytest.raises(MdqlError, match="Validation failed"):
            table.execute_sql("INSERT INTO notes (title) VALUES ('Missing Count')")


class TestExecuteUpdate:
    def test_update_single_row(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title, status) VALUES ('Note', 'draft')")
        result = table.execute_sql(
            "UPDATE notes SET status = 'approved' WHERE path = 'note.md'"
        )

        assert "UPDATE 1" in result
        content = (tmp_path / "note.md").read_text()
        assert 'status: "approved"' in content
        assert 'title: "Note"' in content  # preserved

    def test_update_multiple_rows(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title, status) VALUES ('A', 'draft')")
        table.execute_sql("INSERT INTO notes (title, status) VALUES ('B', 'draft')")
        table.execute_sql("INSERT INTO notes (title, status) VALUES ('C', 'approved')")

        result = table.execute_sql(
            "UPDATE notes SET status = 'archived' WHERE status = 'draft'"
        )

        assert "UPDATE 2" in result
        assert 'status: "archived"' in (tmp_path / "a.md").read_text()
        assert 'status: "archived"' in (tmp_path / "b.md").read_text()
        assert 'status: "approved"' in (tmp_path / "c.md").read_text()

    def test_update_no_match(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('A')")
        result = table.execute_sql(
            "UPDATE notes SET status = 'x' WHERE path = 'nonexistent.md'"
        )

        assert "UPDATE 0" in result

    def test_update_without_where(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('A')")
        table.execute_sql("INSERT INTO notes (title) VALUES ('B')")

        result = table.execute_sql("UPDATE notes SET priority = 5")

        assert "UPDATE 2" in result
        assert "priority: 5" in (tmp_path / "a.md").read_text()
        assert "priority: 5" in (tmp_path / "b.md").read_text()


class TestExecuteDelete:
    def test_delete_single_row(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('Keep')")
        table.execute_sql("INSERT INTO notes (title) VALUES ('Remove')")

        result = table.execute_sql("DELETE FROM notes WHERE path = 'remove.md'")

        assert "DELETE 1" in result
        assert (tmp_path / "keep.md").exists()
        assert not (tmp_path / "remove.md").exists()

    def test_delete_with_condition(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title, status) VALUES ('A', 'draft')")
        table.execute_sql("INSERT INTO notes (title, status) VALUES ('B', 'draft')")
        table.execute_sql("INSERT INTO notes (title, status) VALUES ('C', 'approved')")

        result = table.execute_sql("DELETE FROM notes WHERE status = 'draft'")

        assert "DELETE 2" in result
        assert not (tmp_path / "a.md").exists()
        assert not (tmp_path / "b.md").exists()
        assert (tmp_path / "c.md").exists()

    def test_delete_no_match(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('Keep')")
        result = table.execute_sql("DELETE FROM notes WHERE path = 'nope.md'")

        assert "DELETE 0" in result
        assert (tmp_path / "keep.md").exists()

    def test_delete_without_where(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title) VALUES ('A')")
        table.execute_sql("INSERT INTO notes (title) VALUES ('B')")

        result = table.execute_sql("DELETE FROM notes")

        assert "DELETE 2" in result
        assert not (tmp_path / "a.md").exists()
        assert not (tmp_path / "b.md").exists()

    def test_delete_nonexistent_file_raises(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        with pytest.raises(MdqlError, match="File not found"):
            table.delete("nope.md")


class TestExecuteSelect:
    def test_select_via_execute_sql(self, tmp_path):
        (tmp_path / "_mdql.md").write_text(SIMPLE_SCHEMA)
        table = Table(tmp_path)

        table.execute_sql("INSERT INTO notes (title, priority) VALUES ('A', 10)")
        table.execute_sql("INSERT INTO notes (title, priority) VALUES ('B', 5)")

        result = table.execute_sql(
            "SELECT title, priority FROM notes ORDER BY priority DESC"
        )

        assert "A" in result
        assert "B" in result
