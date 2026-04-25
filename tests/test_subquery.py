"""Tests for subquery support via Database.query()."""

from pathlib import Path

import pytest

from mdql import Database

FIXTURES = Path(__file__).parent / "fixtures" / "subquery_db"


@pytest.fixture
def db():
    return Database(FIXTURES)


class TestBasicSubquery:
    def test_select_star_from_subquery(self, db):
        rows, cols = db.query(
            "SELECT * FROM (SELECT name, category FROM products)"
        )
        assert len(rows) == 4
        assert "name" in cols
        assert "category" in cols

    def test_subquery_preserves_columns(self, db):
        rows, cols = db.query(
            "SELECT * FROM (SELECT name, price FROM products)"
        )
        assert set(cols) == {"name", "price"}
        assert all("category" not in r for r in rows)

    def test_subquery_with_inner_where(self, db):
        rows, cols = db.query(
            "SELECT * FROM (SELECT name, price, category FROM products WHERE category = 'electronics')"
        )
        assert len(rows) == 2
        names = {r["name"] for r in rows}
        assert names == {"Widget A", "Widget B"}

    def test_subquery_with_outer_where(self, db):
        rows, _ = db.query(
            "SELECT name FROM (SELECT name, price FROM products) WHERE price > 75"
        )
        assert len(rows) == 2
        names = {r["name"] for r in rows}
        assert names == {"Widget A", "Widget B"}

    def test_subquery_with_both_where(self, db):
        rows, _ = db.query(
            "SELECT name FROM (SELECT name, price FROM products WHERE price >= 75) WHERE price <= 100"
        )
        assert len(rows) == 2
        names = {r["name"] for r in rows}
        assert names == {"Widget A", "Gadget Y"}


class TestSubqueryWithAggregation:
    def test_subquery_with_group_by(self, db):
        rows, cols = db.query(
            "SELECT category, COUNT(*) FROM (SELECT name, category FROM products) GROUP BY category"
        )
        assert len(rows) == 2
        by_cat = {r["category"]: r["COUNT(*)"] for r in rows}
        assert by_cat["electronics"] == 2
        assert by_cat["tools"] == 2

    def test_subquery_with_inner_group_by(self, db):
        rows, cols = db.query(
            "SELECT * FROM (SELECT category, SUM(price) as total FROM products GROUP BY category)"
        )
        assert len(rows) == 2
        by_cat = {r["category"]: r["total"] for r in rows}
        assert by_cat["electronics"] == 300.0
        assert by_cat["tools"] == 125.0

    def test_subquery_with_inner_group_by_and_outer_where(self, db):
        rows, _ = db.query(
            "SELECT category, total FROM (SELECT category, SUM(price) as total FROM products GROUP BY category) WHERE total > 200"
        )
        assert len(rows) == 1
        assert rows[0]["category"] == "electronics"


class TestSubqueryWithOrderAndLimit:
    def test_subquery_with_order_by(self, db):
        rows, _ = db.query(
            "SELECT name FROM (SELECT name, price FROM products) ORDER BY price DESC"
        )
        assert rows[0]["name"] == "Widget B"
        assert rows[-1]["name"] == "Gadget X"

    def test_subquery_with_limit(self, db):
        rows, _ = db.query(
            "SELECT name FROM (SELECT name, price FROM products ORDER BY price DESC) LIMIT 2"
        )
        assert len(rows) == 2

    def test_subquery_inner_limit(self, db):
        rows, _ = db.query(
            "SELECT * FROM (SELECT name, price FROM products ORDER BY price DESC LIMIT 2)"
        )
        assert len(rows) == 2
