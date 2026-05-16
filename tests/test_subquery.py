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


class TestWhereInSubquery:
    def test_where_in_subquery_basic(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE category IN (SELECT category FROM products WHERE price > 150)"
        )
        names = {r["name"] for r in rows}
        assert names == {"Widget A", "Widget B"}

    def test_where_in_subquery_no_matches(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE category IN (SELECT category FROM products WHERE price > 999)"
        )
        assert len(rows) == 0

    def test_where_in_subquery_all_match(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE category IN (SELECT category FROM products)"
        )
        assert len(rows) == 4


class TestWhereScalarSubquery:
    def test_where_gt_scalar_subquery(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE price > (SELECT AVG(price) FROM products)"
        )
        names = {r["name"] for r in rows}
        assert names == {"Widget B"}

    def test_where_lt_scalar_subquery(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE price < (SELECT AVG(price) FROM products)"
        )
        names = {r["name"] for r in rows}
        assert names == {"Gadget X", "Gadget Y", "Widget A"}

    def test_where_eq_scalar_subquery(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE price = (SELECT MIN(price) FROM products)"
        )
        assert len(rows) == 1
        assert rows[0]["name"] == "Gadget X"

    def test_where_ge_scalar_subquery(self, db):
        rows, _ = db.query(
            "SELECT name FROM products WHERE price >= (SELECT MAX(price) FROM products)"
        )
        assert len(rows) == 1
        assert rows[0]["name"] == "Widget B"


class TestSelectSubquery:
    def test_scalar_subquery_in_select(self, db):
        rows, cols = db.query(
            "SELECT name, (SELECT COUNT(*) FROM products) as total FROM products LIMIT 1"
        )
        assert rows[0]["total"] == 4

    def test_scalar_subquery_in_select_aggregate(self, db):
        rows, cols = db.query(
            "SELECT name, (SELECT MAX(price) FROM products) as max_price FROM products WHERE name = 'Gadget X'"
        )
        assert len(rows) == 1
        assert rows[0]["max_price"] == 200
