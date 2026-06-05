"""Boolean literals in SQL (issue #60).

`WHERE enabled = true` parsed `true` as a reference to a nonexistent column,
so the comparison evaluated against NULL and silently returned 0 rows. These
lock the fix end-to-end through the Python binding: read, write, round-trip.
"""

import pytest
from mdql import Database


def make_db(tmp_path):
    db_dir = tmp_path / "booldb"
    db_dir.mkdir()
    (db_dir / "_mdql.md").write_text("---\ntype: database\nname: booldb\n---\n")

    alloc = db_dir / "allocations"
    alloc.mkdir()
    (alloc / "_mdql.md").write_text(
        "---\ntype: schema\ntable: allocations\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  strategy:\n    type: string\n"
        "  allocation_pct:\n    type: float\n  enabled:\n    type: bool\n---\n"
    )
    (alloc / "a.md").write_text(
        "---\ntitle: A\nstrategy: a.md\nallocation_pct: 40.0\nenabled: true\n---\n# A\n"
    )
    (alloc / "b.md").write_text(
        "---\ntitle: B\nstrategy: b.md\nallocation_pct: 30.0\nenabled: false\n---\n# B\n"
    )
    (alloc / "c.md").write_text(
        "---\ntitle: C\nstrategy: c.md\nallocation_pct: 35.0\nenabled: true\n---\n# C\n"
    )
    return db_dir


class TestBooleanWhere:
    def test_eq_true_returns_enabled_rows(self, tmp_path):
        # Exact #60 repro shape.
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT strategy, allocation_pct FROM allocations WHERE enabled = true"
        )
        assert {r["strategy"] for r in rows} == {"a.md", "c.md"}

    def test_eq_false(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query("SELECT strategy FROM allocations WHERE enabled = false")
        assert {r["strategy"] for r in rows} == {"b.md"}

    def test_neq_and_case_insensitive(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query("SELECT strategy FROM allocations WHERE enabled != TRUE")
        assert {r["strategy"] for r in rows} == {"b.md"}
        rows, _ = db.query("SELECT strategy FROM allocations WHERE enabled = False")
        assert {r["strategy"] for r in rows} == {"b.md"}

    def test_bool_with_other_conditions(self, tmp_path):
        db = Database(make_db(tmp_path))
        rows, _ = db.query(
            "SELECT SUM(allocation_pct) AS total FROM allocations "
            "WHERE enabled = true AND allocation_pct > 35"
        )
        assert rows[0]["total"] == 40.0


class TestBooleanWrites:
    def test_update_set_bool_literal(self, tmp_path):
        db_dir = make_db(tmp_path)
        db = Database(db_dir)
        db.execute("UPDATE allocations SET enabled = false WHERE strategy = 'a.md'")
        text = (db_dir / "allocations" / "a.md").read_text()
        assert "enabled: false" in text  # written as YAML bool, not string
        rows, _ = db.query("SELECT strategy FROM allocations WHERE enabled = true")
        assert {r["strategy"] for r in rows} == {"c.md"}

    def test_insert_with_bool_literal(self, tmp_path):
        db_dir = make_db(tmp_path)
        db = Database(db_dir)
        db.execute(
            "INSERT INTO allocations (title, strategy, allocation_pct, enabled) "
            "VALUES ('Delta', 'd.md', 10.0, true)"
        )
        text = (db_dir / "allocations" / "delta.md").read_text()
        assert "enabled: true" in text
        rows, _ = db.query("SELECT strategy FROM allocations WHERE enabled = true")
        assert "d.md" in {r["strategy"] for r in rows}
