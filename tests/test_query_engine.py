"""Tests for mdql.query_engine."""

import datetime
from pathlib import Path

import pytest

from mdql.loader import load_table
from mdql.query_engine import execute_query
from mdql.query_parser import parse_query

FIXTURES = Path(__file__).parent / "fixtures"


@pytest.fixture
def table_data():
    schema, rows, errors = load_table(FIXTURES / "valid_table")
    return schema, rows


class TestSelectAndProject:
    def test_select_star(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT * FROM notes")
        result, cols = execute_query(q, rows, schema)
        assert len(result) == len(rows)
        assert "path" in cols
        assert "title" in cols

    def test_select_specific_columns(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, author FROM notes")
        result, cols = execute_query(q, rows, schema)
        assert cols == ["title", "author"]

    def test_select_section_column(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, Summary FROM notes")
        result, cols = execute_query(q, rows, schema)
        assert "Summary" in cols
        # At least one row should have Summary content
        assert any(r.get("Summary") for r in result)


class TestWhere:
    def test_equality(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes WHERE author = 'Rasmus'")
        result, _ = execute_query(q, rows, schema)
        assert len(result) == len(rows)  # All have author=Rasmus

    def test_enum_filter(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, status FROM notes WHERE status = 'draft'")
        result, _ = execute_query(q, rows, schema)
        assert all(r["status"] == "draft" for r in result)

    def test_is_null(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, tags FROM notes WHERE tags IS NULL")
        result, _ = execute_query(q, rows, schema)
        assert all(r.get("tags") is None for r in result)

    def test_is_not_null(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, tags FROM notes WHERE tags IS NOT NULL")
        result, _ = execute_query(q, rows, schema)
        assert all(r.get("tags") is not None for r in result)

    def test_like(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes WHERE title LIKE '%tag%'")
        result, _ = execute_query(q, rows, schema)
        assert all("tag" in r["title"].lower() for r in result)

    def test_and(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title, status FROM notes WHERE author = 'Rasmus' AND status = 'draft'")
        result, _ = execute_query(q, rows, schema)
        assert all(r["status"] == "draft" for r in result)


class TestOrderByAndLimit:
    def test_order_by_asc(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes ORDER BY title ASC")
        result, _ = execute_query(q, rows, schema)
        titles = [r["title"] for r in result]
        assert titles == sorted(titles)

    def test_order_by_desc(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes ORDER BY title DESC")
        result, _ = execute_query(q, rows, schema)
        titles = [r["title"] for r in result]
        assert titles == sorted(titles, reverse=True)

    def test_limit(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes LIMIT 2")
        result, _ = execute_query(q, rows, schema)
        assert len(result) == 2

    def test_combined(self, table_data):
        schema, rows = table_data
        q = parse_query("SELECT title FROM notes ORDER BY title DESC LIMIT 1")
        result, _ = execute_query(q, rows, schema)
        assert len(result) == 1


class TestRealStrategyData:
    """Integration tests with the actual strategy files."""

    @pytest.fixture
    def strategies(self):
        examples = Path(__file__).parent.parent / "examples" / "strategies"
        if not examples.exists():
            pytest.skip("examples/strategies not present")
        schema, rows, errors = load_table(examples)
        assert len(rows) > 0, f"No valid rows, errors: {errors}"
        return schema, rows

    def test_top_composite(self, strategies):
        schema, rows = strategies
        q = parse_query("SELECT title, composite FROM strategies ORDER BY composite DESC LIMIT 5")
        result, cols = execute_query(q, rows, schema)
        assert len(result) == 5
        composites = [r["composite"] for r in result]
        assert composites == sorted(composites, reverse=True)

    def test_filter_mechanism(self, strategies):
        schema, rows = strategies
        q = parse_query("SELECT title, mechanism FROM strategies WHERE mechanism > 5")
        result, _ = execute_query(q, rows, schema)
        assert all(r["mechanism"] > 5 for r in result)
        assert len(result) > 0

    def test_section_query(self, strategies):
        schema, rows = strategies
        q = parse_query("SELECT path, Hypothesis FROM strategies WHERE Hypothesis IS NOT NULL LIMIT 5")
        result, cols = execute_query(q, rows, schema)
        assert len(result) <= 5
        assert all(r.get("Hypothesis") is not None for r in result)

    def test_category_like(self, strategies):
        schema, rows = strategies
        q = parse_query("SELECT title, categories FROM strategies WHERE categories LIKE '%defi%'")
        result, _ = execute_query(q, rows, schema)
        assert len(result) > 0
        for r in result:
            assert any("defi" in c for c in r["categories"])

    def test_status_killed(self, strategies):
        schema, rows = strategies
        q = parse_query("SELECT title, status, kill_reason FROM strategies WHERE status = 'KILLED'")
        result, _ = execute_query(q, rows, schema)
        assert len(result) >= 2
        assert all(r["status"] == "KILLED" for r in result)
