"""Tests for CTE (WITH ... AS) queries."""

import pytest
from mdql.api import Database


def make_cte_db(tmp_path):
    db_dir = tmp_path / "testdb"
    db_dir.mkdir()

    (db_dir / "_mdql.md").write_text(
        "---\ntype: database\nname: testdb\n---\n"
    )

    strats = db_dir / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(
        "---\ntype: schema\ntable: strategies\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  status:\n    type: string\n---\n"
    )
    (strats / "alpha.md").write_text("---\ntitle: Alpha\nstatus: LIVE\n---\n# Alpha\n")
    (strats / "beta.md").write_text("---\ntitle: Beta\nstatus: DRAFT\n---\n# Beta\n")
    (strats / "gamma.md").write_text("---\ntitle: Gamma\nstatus: LIVE\n---\n# Gamma\n")

    bt = db_dir / "backtests"
    bt.mkdir()
    (bt / "_mdql.md").write_text(
        "---\ntype: schema\ntable: backtests\nprimary_key: path\n"
        "frontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n"
    )
    (bt / "bt-alpha.md").write_text("---\nstrategy: alpha.md\nsharpe: 1.5\n---\n# BT Alpha\n")
    (bt / "bt-gamma.md").write_text("---\nstrategy: gamma.md\nsharpe: 0.8\n---\n# BT Gamma\n")

    return db_dir


class TestCTEBasic:
    def test_simple_cte(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, cols = db.query(
            "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') "
            "SELECT * FROM live"
        )
        assert len(rows) == 2
        titles = {r["title"] for r in rows}
        assert titles == {"Alpha", "Gamma"}

    def test_cte_with_filter(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') "
            "SELECT * FROM live WHERE title = 'Alpha'"
        )
        assert len(rows) == 1
        assert rows[0]["title"] == "Alpha"

    def test_cte_with_aggregation(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH counts AS (SELECT status, COUNT(*) AS cnt FROM strategies GROUP BY status) "
            "SELECT * FROM counts ORDER BY cnt DESC"
        )
        assert len(rows) == 2
        assert rows[0]["cnt"] == 2  # LIVE has 2
        assert rows[1]["cnt"] == 1  # DRAFT has 1


class TestCTEMultiple:
    def test_two_ctes_with_join(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH s AS (SELECT * FROM strategies WHERE status = 'LIVE'), "
            "b AS (SELECT * FROM backtests WHERE sharpe > 1.0) "
            "SELECT s.title, b.sharpe FROM s JOIN b ON b.strategy = s.path"
        )
        assert len(rows) == 1
        assert rows[0]["s.title"] == "Alpha"
        assert rows[0]["b.sharpe"] == 1.5


class TestCTEChained:
    def test_cte_references_earlier_cte(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH good AS (SELECT * FROM backtests WHERE sharpe > 1.0), "
            "matched AS (SELECT s.title, g.sharpe FROM strategies s JOIN good g ON g.strategy = s.path) "
            "SELECT * FROM matched"
        )
        assert len(rows) == 1
        assert rows[0]["s.title"] == "Alpha"


class TestCTEEdgeCases:
    def test_cte_same_name_as_table(self, tmp_path):
        """CTE should shadow the real table."""
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH strategies AS (SELECT * FROM strategies WHERE status = 'LIVE') "
            "SELECT * FROM strategies"
        )
        assert len(rows) == 2
        titles = {r["title"] for r in rows}
        assert titles == {"Alpha", "Gamma"}

    def test_cte_with_limit(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') "
            "SELECT * FROM live LIMIT 1"
        )
        assert len(rows) == 1

    def test_cte_order_by(self, tmp_path):
        db_dir = make_cte_db(tmp_path)
        db = Database(db_dir)
        rows, _ = db.query(
            "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') "
            "SELECT * FROM live ORDER BY title ASC"
        )
        assert len(rows) == 2
        assert rows[0]["title"] == "Alpha"
        assert rows[1]["title"] == "Gamma"
