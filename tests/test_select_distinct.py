"""SELECT DISTINCT (issue #61).

`SELECT DISTINCT strategy FROM backtests` returned every row with the column
nulled: DISTINCT lexed as an identifier and parsed as a column reference, with
the real column demoted to an implicit alias. Same silent-wrong-result class
as #60.
"""

import pytest
from mdql import Database


def make_db(tmp_path):
    db_dir = tmp_path / "ddb"
    db_dir.mkdir()
    (db_dir / "_mdql.md").write_text("---\ntype: database\nname: ddb\n---\n")
    bt = db_dir / "backtests"
    bt.mkdir()
    (bt / "_mdql.md").write_text(
        "---\ntype: schema\ntable: backtests\nprimary_key: path\n"
        "frontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n"
    )
    # 5 rows, 3 unique strategies
    for name, strat, sharpe in [
        ("b1", "alpha.md", 1.0),
        ("b2", "alpha.md", 1.2),
        ("b3", "beta.md", 0.5),
        ("b4", "beta.md", 0.6),
        ("b5", "gamma.md", 2.0),
    ]:
        (bt / f"{name}.md").write_text(
            f"---\nstrategy: {strat}\nsharpe: {sharpe}\n---\n# {name}\n"
        )
    return db_dir


class TestSelectDistinct:
    def test_distinct_dedupes_and_projects(self, tmp_path):
        # Exact #61 repro shape: values must be real, not None, and deduped.
        db = Database(make_db(tmp_path))
        rows, cols = db.query("SELECT DISTINCT strategy FROM backtests")
        assert cols == ["strategy"]
        assert len(rows) == 3
        assert {r["strategy"] for r in rows} == {"alpha.md", "beta.md", "gamma.md"}
        assert all(r["strategy"] is not None for r in rows)

    def test_distinct_multi_column(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query("SELECT DISTINCT strategy, sharpe FROM backtests")
        assert len(rows) == 5  # all (strategy, sharpe) pairs are unique

    def test_distinct_with_where_order_limit(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT DISTINCT strategy FROM backtests "
            "WHERE sharpe > 0.5 ORDER BY strategy LIMIT 2"
        )
        # dedupe happens before LIMIT: alpha (1.0, 1.2), beta (0.6), gamma (2.0)
        assert [r["strategy"] for r in rows] == ["alpha.md", "beta.md"]

    def test_plain_select_unaffected(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query("SELECT strategy FROM backtests")
        assert len(rows) == 5
