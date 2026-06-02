"""Header/row alignment contract for Database.query (issue #55).

Database.query returns (rows: list[dict], columns: list[str]). Each row dict
must be keyed by exactly `columns`, in the same order: same length, no extra
keys, no missing keys. This covers single-table projections, JOINs (which
internally add unqualified-base aliases to rows), non-existent columns, and
duplicate output names.
"""

import pytest
from mdql.api import Database


def make_db(tmp_path):
    db_dir = tmp_path / "testdb"
    db_dir.mkdir()
    (db_dir / "_mdql.md").write_text(
        "---\ntype: database\nname: testdb\n"
        "foreign_keys:\n  - from: backtests.strategy\n    to: strategies.path\n---\n"
    )

    strats = db_dir / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(
        "---\ntype: schema\ntable: strategies\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  status:\n    type: string\n"
        "  kill_reason:\n    type: string\n---\n"
    )
    # alpha has kill_reason; beta omits it (optional field absent on this row)
    (strats / "alpha.md").write_text(
        "---\ntitle: Alpha\nstatus: LIVE\nkill_reason: superseded\n---\n# Alpha\n"
    )
    (strats / "beta.md").write_text("---\ntitle: Beta\nstatus: DRAFT\n---\n# Beta\n")

    bt = db_dir / "backtests"
    bt.mkdir()
    (bt / "_mdql.md").write_text(
        "---\ntype: schema\ntable: backtests\nprimary_key: path\n"
        "frontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n"
    )
    (bt / "bt-alpha.md").write_text("---\nstrategy: alpha.md\nsharpe: 1.5\n---\n# BT Alpha\n")

    return db_dir


def assert_aligned(rows, cols):
    """Every row dict is keyed by exactly `cols`, in order."""
    for r in rows:
        assert list(r.keys()) == cols, f"row keys {list(r.keys())} != header {cols}"
        assert len(r) == len(cols)


class TestSingleTableAlignment:
    def test_nonexistent_column_is_null_filled_and_aligned(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, cols = db.query(
            "SELECT title, missing_col, status FROM strategies WHERE status = 'LIVE'"
        )
        assert cols == ["title", "missing_col", "status"]
        assert rows
        assert_aligned(rows, cols)
        for r in rows:
            assert r["missing_col"] is None

    def test_select_star_sparse_rows_aligned(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, cols = db.query("SELECT * FROM strategies")
        assert "kill_reason" in cols
        assert_aligned(rows, cols)
        beta = next(r for r in rows if r["title"] == "Beta")
        assert beta["kill_reason"] is None


class TestJoinAlignment:
    def test_join_with_missing_column_aligned(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, cols = db.query(
            "SELECT s.title, missing_col, b.sharpe "
            "FROM strategies s JOIN backtests b ON b.strategy = s.path"
        )
        assert cols == ["s.title", "missing_col", "b.sharpe"]
        assert len(rows) == 1
        # No leaked unqualified-base aliases ('title', 'sharpe') in the dict.
        assert_aligned(rows, cols)
        assert rows[0]["missing_col"] is None
        assert rows[0]["s.title"] == "Alpha"
        assert rows[0]["b.sharpe"] == 1.5


class TestDuplicateColumns:
    def test_duplicate_output_name_raises(self, tmp_path):
        db = Database(make_db(tmp_path))
        with pytest.raises(Exception) as exc:
            db.query("SELECT title, title FROM strategies")
        assert "duplicate output column" in str(exc.value)

    def test_duplicate_via_alias_collision_raises(self, tmp_path):
        db = Database(make_db(tmp_path))
        with pytest.raises(Exception) as exc:
            db.query("SELECT title, status AS title FROM strategies")
        assert "duplicate output column" in str(exc.value)
