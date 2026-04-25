"""Tests for mdql.query_parser."""

import pytest

from mdql.query_parser import (
    BoolOp,
    Comparison,
    OrderSpec,
    Query,
    parse_query,
)
from mdql.errors import QueryParseError


class TestBasicQueries:
    def test_select_star(self):
        q = parse_query("SELECT * FROM notes")
        assert q.columns == "*"
        assert q.table == "notes"

    def test_select_columns(self):
        q = parse_query("SELECT title, author FROM notes")
        assert q.columns == ["title", "author"]
        assert q.table == "notes"

    def test_case_insensitive_keywords(self):
        q = parse_query("select title from notes")
        assert q.columns == ["title"]
        assert q.table == "notes"

    def test_backtick_column(self):
        q = parse_query("SELECT `Structural Mechanism` FROM strategies")
        assert q.columns == ["Structural Mechanism"]


class TestWhere:
    def test_equality(self):
        q = parse_query("SELECT title FROM notes WHERE author = 'Rasmus'")
        assert isinstance(q.where_clause, Comparison)
        assert q.where_clause.column == "author"
        assert q.where_clause.op == "="
        assert q.where_clause.value == "Rasmus"

    def test_not_equal(self):
        q = parse_query("SELECT title FROM notes WHERE status != 'draft'")
        assert q.where_clause.op == "!="

    def test_numeric_comparison(self):
        q = parse_query("SELECT title FROM s WHERE mechanism > 5")
        assert q.where_clause.op == ">"
        assert q.where_clause.value == 5

    def test_like(self):
        q = parse_query("SELECT title FROM notes WHERE title LIKE '%test%'")
        assert q.where_clause.op == "LIKE"
        assert q.where_clause.value == "%test%"

    def test_in(self):
        q = parse_query("SELECT title FROM notes WHERE status IN ('draft', 'approved')")
        assert q.where_clause.op == "IN"
        assert q.where_clause.value == ["draft", "approved"]

    def test_is_null(self):
        q = parse_query("SELECT title FROM notes WHERE tags IS NULL")
        assert q.where_clause.op == "IS NULL"

    def test_is_not_null(self):
        q = parse_query("SELECT title FROM notes WHERE tags IS NOT NULL")
        assert q.where_clause.op == "IS NOT NULL"

    def test_and(self):
        q = parse_query("SELECT title FROM s WHERE mechanism > 5 AND safety > 3")
        assert isinstance(q.where_clause, BoolOp)
        assert q.where_clause.op == "AND"

    def test_or(self):
        q = parse_query("SELECT title FROM s WHERE status = 'LIVE' OR status = 'KILLED'")
        assert isinstance(q.where_clause, BoolOp)
        assert q.where_clause.op == "OR"

    def test_and_binds_tighter_than_or(self):
        q = parse_query("SELECT * FROM s WHERE a = 1 OR b = 2 AND c = 3")
        # Should parse as: a=1 OR (b=2 AND c=3)
        assert isinstance(q.where_clause, BoolOp)
        assert q.where_clause.op == "OR"
        assert isinstance(q.where_clause.right, BoolOp)
        assert q.where_clause.right.op == "AND"


class TestOrderByAndLimit:
    def test_order_by(self):
        q = parse_query("SELECT title FROM s ORDER BY composite DESC")
        assert q.order_by == [OrderSpec("composite", True)]

    def test_order_by_asc(self):
        q = parse_query("SELECT title FROM s ORDER BY title ASC")
        assert q.order_by == [OrderSpec("title", False)]

    def test_order_by_default_asc(self):
        q = parse_query("SELECT title FROM s ORDER BY title")
        assert q.order_by == [OrderSpec("title", False)]

    def test_limit(self):
        q = parse_query("SELECT title FROM s LIMIT 10")
        assert q.limit == 10

    def test_combined(self):
        q = parse_query(
            "SELECT title, composite FROM s WHERE mechanism > 3 ORDER BY composite DESC LIMIT 5"
        )
        assert q.columns == ["title", "composite"]
        assert q.where_clause is not None
        assert q.order_by == [OrderSpec("composite", True)]
        assert q.limit == 5


class TestJoin:
    def test_basic_join(self):
        q = parse_query(
            "SELECT s.title, b.title FROM strategies s "
            "JOIN backtests b ON b.strategy = s.path"
        )
        assert q.table == "strategies"
        assert q.table_alias == "s"
        assert len(q.joins) == 1
        assert q.joins[0].table == "backtests"
        assert q.joins[0].alias == "b"
        assert q.joins[0].left_col == "b.strategy"
        assert q.joins[0].right_col == "s.path"

    def test_join_with_where(self):
        q = parse_query(
            "SELECT s.title, b.sharpe FROM strategies s "
            "JOIN backtests b ON b.strategy = s.path "
            "WHERE b.status = 'pass'"
        )
        assert len(q.joins) == 1
        assert q.where_clause is not None

    def test_join_with_order_limit(self):
        q = parse_query(
            "SELECT s.title FROM strategies s "
            "JOIN backtests b ON b.strategy = s.path "
            "ORDER BY b.sharpe DESC LIMIT 5"
        )
        assert len(q.joins) == 1
        assert q.order_by is not None
        assert q.limit == 5


class TestErrors:
    def test_empty_query(self):
        with pytest.raises(QueryParseError):
            parse_query("")

    def test_missing_from(self):
        with pytest.raises(QueryParseError):
            parse_query("SELECT title")

    def test_trailing_junk(self):
        with pytest.raises(QueryParseError):
            parse_query("SELECT title FROM notes foo bar")

    def test_table_alias(self):
        q = parse_query("SELECT title FROM notes n")
        assert q.table == "notes"
        assert q.table_alias == "n"


class TestAggregateArithmetic:
    def test_sum_division(self):
        q = parse_query(
            "SELECT token, SUM(sell) / SUM(buy) as ratio FROM orders GROUP BY token"
        )
        assert q.columns == ["token", "ratio"]

    def test_sum_subtraction(self):
        q = parse_query("SELECT SUM(a) - SUM(b) as net FROM t")
        assert "net" in q.columns

    def test_aggregate_multiplication(self):
        q = parse_query("SELECT COUNT(*) * 2 as doubled FROM t")
        assert q.columns == ["doubled"]

    def test_nested_case_in_aggregate_arithmetic(self):
        q = parse_query(
            "SELECT SUM(CASE WHEN x = 1 THEN a ELSE 0 END) / COUNT(*) as ratio FROM t"
        )
        assert q.columns == ["ratio"]


class TestSubquery:
    def test_basic_subquery(self):
        q = parse_query("SELECT x FROM (SELECT x, y FROM t) LIMIT 5")
        assert q.columns == ["x"]
        assert q.limit == 5

    def test_subquery_with_group_by(self):
        q = parse_query(
            "SELECT token, sell FROM (SELECT token, SUM(size) as sell FROM orders GROUP BY token)"
        )
        assert q.columns == ["token", "sell"]


class TestCreateView:
    def test_create_view_with_arithmetic(self):
        from mdql._native import CreateViewQuery

        result = parse_query(
            "CREATE VIEW v AS SELECT token, SUM(sell) / SUM(buy) as ratio FROM orders GROUP BY token"
        )
        assert isinstance(result, CreateViewQuery)
        assert result.view_name == "v"

    def test_create_view_with_having(self):
        from mdql._native import CreateViewQuery

        result = parse_query(
            "CREATE VIEW v AS SELECT status, COUNT(*) as cnt FROM t GROUP BY status HAVING cnt > 5"
        )
        assert isinstance(result, CreateViewQuery)
        assert result.view_name == "v"
