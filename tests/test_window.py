"""Tests for window function support via Database.query()."""

from pathlib import Path

import pytest

from mdql import Database

FIXTURES = Path(__file__).parent / "fixtures" / "subquery_db"


@pytest.fixture
def db():
    return Database(FIXTURES)


class TestRowNumber:
    def test_row_number_basic(self, db):
        rows, cols = db.query(
            "SELECT name, ROW_NUMBER() OVER (ORDER BY price DESC) AS rn FROM products"
        )
        assert len(rows) == 4
        assert "rn" in cols
        by_name = {r["name"]: r["rn"] for r in rows}
        assert by_name["Widget B"] == 1  # price=200
        assert by_name["Widget A"] == 2  # price=100
        assert by_name["Gadget Y"] == 3  # price=75
        assert by_name["Gadget X"] == 4  # price=50

    def test_row_number_with_partition(self, db):
        rows, _ = db.query(
            "SELECT name, ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) AS rn FROM products"
        )
        by_name = {r["name"]: r["rn"] for r in rows}
        assert by_name["Widget B"] == 1  # electronics, price=200
        assert by_name["Widget A"] == 2  # electronics, price=100
        assert by_name["Gadget Y"] == 1  # tools, price=75
        assert by_name["Gadget X"] == 2  # tools, price=50


class TestRank:
    def test_rank_no_ties(self, db):
        rows, _ = db.query(
            "SELECT name, RANK() OVER (ORDER BY price DESC) AS rnk FROM products"
        )
        by_name = {r["name"]: r["rnk"] for r in rows}
        assert by_name["Widget B"] == 1
        assert by_name["Widget A"] == 2
        assert by_name["Gadget Y"] == 3
        assert by_name["Gadget X"] == 4


class TestDenseRank:
    def test_dense_rank_no_ties(self, db):
        rows, _ = db.query(
            "SELECT name, DENSE_RANK() OVER (ORDER BY price DESC) AS dr FROM products"
        )
        by_name = {r["name"]: r["dr"] for r in rows}
        assert by_name["Widget B"] == 1
        assert by_name["Widget A"] == 2
        assert by_name["Gadget Y"] == 3
        assert by_name["Gadget X"] == 4


class TestLagLead:
    def test_lag(self, db):
        rows, _ = db.query(
            "SELECT name, LAG(price, 1) OVER (ORDER BY price ASC) AS prev_price FROM products"
        )
        by_name = {r["name"]: r.get("prev_price") for r in rows}
        assert by_name["Gadget X"] is None  # first, no previous
        assert by_name["Gadget Y"] == 50
        assert by_name["Widget A"] == 75
        assert by_name["Widget B"] == 100

    def test_lead(self, db):
        rows, _ = db.query(
            "SELECT name, LEAD(price, 1) OVER (ORDER BY price ASC) AS next_price FROM products"
        )
        by_name = {r["name"]: r.get("next_price") for r in rows}
        assert by_name["Gadget X"] == 75
        assert by_name["Gadget Y"] == 100
        assert by_name["Widget A"] == 200
        assert by_name["Widget B"] is None  # last, no next


class TestAggregateWindow:
    def test_sum_over_partition(self, db):
        rows, cols = db.query(
            "SELECT name, SUM(price) OVER (PARTITION BY category) AS cat_total FROM products"
        )
        assert len(rows) == 4
        by_name = {r["name"]: r["cat_total"] for r in rows}
        assert by_name["Widget A"] == 300.0  # electronics: 100+200
        assert by_name["Widget B"] == 300.0
        assert by_name["Gadget X"] == 125.0  # tools: 50+75
        assert by_name["Gadget Y"] == 125.0

    def test_count_over_partition(self, db):
        rows, _ = db.query(
            "SELECT name, COUNT(*) OVER (PARTITION BY category) AS cat_count FROM products"
        )
        by_name = {r["name"]: r["cat_count"] for r in rows}
        assert by_name["Widget A"] == 2
        assert by_name["Gadget X"] == 2

    def test_avg_over_partition(self, db):
        rows, _ = db.query(
            "SELECT name, AVG(price) OVER (PARTITION BY category) AS avg_price FROM products"
        )
        by_name = {r["name"]: r["avg_price"] for r in rows}
        assert by_name["Widget A"] == 150.0  # (100+200)/2
        assert by_name["Gadget X"] == 62.5   # (50+75)/2


class TestWindowCombined:
    def test_window_with_where(self, db):
        rows, _ = db.query(
            "SELECT name, ROW_NUMBER() OVER (ORDER BY price DESC) AS rn "
            "FROM products WHERE category = 'electronics'"
        )
        assert len(rows) == 2
        by_name = {r["name"]: r["rn"] for r in rows}
        assert by_name["Widget B"] == 1
        assert by_name["Widget A"] == 2

    def test_window_with_order_by_alias(self, db):
        rows, _ = db.query(
            "SELECT name, ROW_NUMBER() OVER (ORDER BY price DESC) AS rn "
            "FROM products ORDER BY rn"
        )
        assert rows[0]["rn"] == 1
        assert rows[3]["rn"] == 4

    def test_window_with_limit(self, db):
        rows, _ = db.query(
            "SELECT name, ROW_NUMBER() OVER (ORDER BY price DESC) AS rn "
            "FROM products ORDER BY rn LIMIT 2"
        )
        assert len(rows) == 2
        assert rows[0]["name"] == "Widget B"
        assert rows[1]["name"] == "Widget A"
