"""End-to-end 'latest row per group' coverage (issues #56, #57, #58).

These three SQL features each independently express "the newest backtest per
strategy". This locks the canonical patterns at the Python API level.
"""

import pytest
from mdql import Database


def make_db(tmp_path):
    db_dir = tmp_path / "lpg"
    db_dir.mkdir()
    (db_dir / "_mdql.md").write_text("---\ntype: database\nname: lpg\n---\n")

    strats = db_dir / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(
        "---\ntype: schema\ntable: strategies\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n---\n"
    )
    (strats / "alpha.md").write_text("---\ntitle: Alpha\n---\n# Alpha\n")
    (strats / "beta.md").write_text("---\ntitle: Beta\n---\n# Beta\n")

    bt = db_dir / "backtests"
    bt.mkdir()
    (bt / "_mdql.md").write_text(
        "---\ntype: schema\ntable: backtests\nprimary_key: path\n"
        "frontmatter:\n  strategy:\n    type: string\n  result:\n    type: string\n"
        "  modified:\n    type: datetime\n---\n"
    )
    # alpha: old PASS, newer INCONCLUSIVE. beta: single PASS.
    (bt / "a-old.md").write_text(
        '---\nstrategy: alpha.md\nresult: PASS\nmodified: "2026-01-01T00:00:00"\n---\n# a-old\n'
    )
    (bt / "a-new.md").write_text(
        '---\nstrategy: alpha.md\nresult: INCONCLUSIVE\nmodified: "2026-05-01T00:00:00"\n---\n# a-new\n'
    )
    (bt / "b-one.md").write_text(
        '---\nstrategy: beta.md\nresult: PASS\nmodified: "2026-03-01T00:00:00"\n---\n# b-one\n'
    )
    return db_dir


def latest_results(rows):
    return {(r["strategy"], r["result"]) for r in rows}


class TestIssue58InSubqueryDatetime:
    def test_in_subquery_over_datetime(self, tmp_path):
        # Regression for #58: a subquery over a datetime column must not
        # collapse to NULL. Returns the newest row per strategy.
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT strategy, result FROM backtests "
            "WHERE modified IN (SELECT MAX(modified) FROM backtests GROUP BY strategy)"
        )
        assert latest_results(rows) == {
            ("alpha.md", "INCONCLUSIVE"),
            ("beta.md", "PASS"),
        }

    def test_scalar_subquery_over_datetime(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT result FROM backtests "
            "WHERE modified = (SELECT MAX(modified) FROM backtests)"
        )
        assert [r["result"] for r in rows] == ["INCONCLUSIVE"]


class TestIssue56WindowLatestPerGroup:
    def test_row_number_top_per_group(self, tmp_path):
        # Canonical latest-per-group: ROW_NUMBER over a subquery, keep rn = 1.
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT * FROM ("
            "  SELECT strategy, result, "
            "  ROW_NUMBER() OVER (PARTITION BY strategy ORDER BY modified DESC) AS rn "
            "  FROM backtests"
            ") WHERE rn = 1"
        )
        assert latest_results(rows) == {
            ("alpha.md", "INCONCLUSIVE"),
            ("beta.md", "PASS"),
        }


class TestIssue57MultiConditionJoin:
    def test_and_in_on_clause(self, tmp_path):
        # Faithful #57 repro: two-condition ON between real tables.
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT s.title, b.result FROM strategies s "
            "JOIN backtests b ON s.path = b.strategy AND b.result = 'PASS'"
        )
        # alpha has a PASS (a-old), beta has a PASS (b-one); alpha's
        # INCONCLUSIVE is excluded by the second ON condition.
        assert {(r["s.title"], r["b.result"]) for r in rows} == {
            ("Alpha", "PASS"),
            ("Beta", "PASS"),
        }

    def test_or_in_on_clause(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT s.title, b.result FROM strategies s "
            "JOIN backtests b ON s.path = b.strategy "
            "AND (b.result = 'PASS' OR b.result = 'INCONCLUSIVE')"
        )
        # alpha: PASS + INCONCLUSIVE (2), beta: PASS (1) => 3 rows.
        assert len(rows) == 3
