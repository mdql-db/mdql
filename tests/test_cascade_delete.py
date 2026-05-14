"""Tests for CASCADE and RESTRICT delete."""

import os
import pytest
import tempfile

from mdql.api import Database, Table
from mdql.errors import MdqlError


def make_cascade_db(tmp_path):
    db_dir = tmp_path / "testdb"
    db_dir.mkdir()

    (db_dir / "_mdql.md").write_text(
        "---\ntype: database\nname: testdb\n"
        "foreign_keys:\n"
        "  - from: backtests.strategy\n"
        "    to: strategies.path\n"
        "---\n"
    )

    strats = db_dir / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(
        "---\ntype: schema\ntable: strategies\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  status:\n    type: string\n---\n"
    )
    (strats / "alpha.md").write_text("---\ntitle: Alpha\nstatus: KILLED\n---\n# Alpha\n")
    (strats / "beta.md").write_text("---\ntitle: Beta\nstatus: LIVE\n---\n# Beta\n")

    bt = db_dir / "backtests"
    bt.mkdir()
    (bt / "_mdql.md").write_text(
        "---\ntype: schema\ntable: backtests\nprimary_key: path\n"
        "frontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n"
    )
    (bt / "bt-alpha.md").write_text("---\nstrategy: alpha.md\nsharpe: 1.5\n---\n# BT Alpha\n")
    (bt / "bt-beta.md").write_text("---\nstrategy: beta.md\nsharpe: 0.8\n---\n# BT Beta\n")

    return db_dir


def make_list_fk_db(tmp_path):
    db_dir = tmp_path / "listdb"
    db_dir.mkdir()

    (db_dir / "_mdql.md").write_text(
        "---\ntype: database\nname: listdb\n"
        "foreign_keys:\n"
        "  - from: strategies.ancestry\n"
        "    to: strategies.path\n"
        "---\n"
    )

    strats = db_dir / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(
        "---\ntype: schema\ntable: strategies\nprimary_key: path\n"
        "frontmatter:\n  title:\n    type: string\n  ancestry:\n    type: string[]\n---\n"
    )
    (strats / "alpha.md").write_text("---\ntitle: Alpha\n---\n# Alpha\n")
    (strats / "beta.md").write_text(
        "---\ntitle: Beta\nancestry:\n  - alpha.md\n  - gamma.md\n---\n# Beta\n"
    )

    return db_dir


class TestCascadeSQL:
    def test_cascade_via_execute(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        result = db.execute("DELETE FROM strategies WHERE status = 'KILLED' CASCADE")
        assert "DELETE 1" in result
        assert "cascade" in result
        assert not (db_dir / "strategies" / "alpha.md").exists()
        assert not (db_dir / "backtests" / "bt-alpha.md").exists()
        assert (db_dir / "strategies" / "beta.md").exists()
        assert (db_dir / "backtests" / "bt-beta.md").exists()

    def test_restrict_blocks_via_execute(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        with pytest.raises(MdqlError, match="RESTRICT"):
            db.execute("DELETE FROM strategies WHERE status = 'KILLED' RESTRICT")
        assert (db_dir / "strategies" / "alpha.md").exists()

    def test_restrict_allows_when_no_deps(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        (db_dir / "backtests" / "bt-alpha.md").unlink()
        db = Database(db_dir)
        result = db.execute("DELETE FROM strategies WHERE status = 'KILLED' RESTRICT")
        assert "DELETE 1" in result
        assert not (db_dir / "strategies" / "alpha.md").exists()

    def test_plain_delete_unchanged(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        result = db.execute("DELETE FROM strategies WHERE status = 'KILLED'")
        assert "DELETE 1" in result
        assert not (db_dir / "strategies" / "alpha.md").exists()
        assert (db_dir / "backtests" / "bt-alpha.md").exists()


class TestCascadePythonAPI:
    def test_cascade_delete(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        result = db.delete("strategies", "status = 'KILLED'", cascade=True)
        assert "DELETE 1" in result
        assert not (db_dir / "strategies" / "alpha.md").exists()
        assert not (db_dir / "backtests" / "bt-alpha.md").exists()

    def test_restrict_delete(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        with pytest.raises(MdqlError, match="RESTRICT"):
            db.delete("strategies", "status = 'KILLED'", restrict=True)

    def test_dry_run_cascade(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        plan = db.delete("strategies", "status = 'KILLED'", cascade=True, dry_run=True)
        assert isinstance(plan, dict)
        assert len(plan["primary_deletes"]) == 1
        assert len(plan["cascade_actions"]) == 1
        assert "DELETE backtests/bt-alpha.md" in plan["cascade_actions"][0]
        assert (db_dir / "strategies" / "alpha.md").exists()

    def test_dry_run_restrict_shows_violations(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        plan = db.delete("strategies", "status = 'KILLED'", restrict=True, dry_run=True)
        assert isinstance(plan, dict)
        assert len(plan["restrict_violations"]) > 0
        assert (db_dir / "strategies" / "alpha.md").exists()

    def test_cascade_and_restrict_errors(self, tmp_path):
        db_dir = make_cascade_db(tmp_path)
        db = Database(db_dir)
        with pytest.raises(MdqlError, match="Cannot use both"):
            db.delete("strategies", "status = 'KILLED'", cascade=True, restrict=True)


class TestListFKPrune:
    def test_cascade_prunes_list(self, tmp_path):
        db_dir = make_list_fk_db(tmp_path)
        db = Database(db_dir)
        result = db.execute("DELETE FROM strategies WHERE path = 'alpha.md' CASCADE")
        assert "DELETE 1" in result
        assert "pruned" in result.lower()
        beta_content = (db_dir / "strategies" / "beta.md").read_text()
        assert "alpha.md" not in beta_content
        assert "gamma.md" in beta_content


class TestDeleteQueryMode:
    def test_parse_cascade_mode(self):
        from mdql.query_parser import parse_query
        from mdql._native import DeleteQuery
        q = parse_query("DELETE FROM t WHERE x = 1 CASCADE")
        assert isinstance(q, DeleteQuery)
        assert q.mode == "CASCADE"

    def test_parse_restrict_mode(self):
        from mdql.query_parser import parse_query
        from mdql._native import DeleteQuery
        q = parse_query("DELETE FROM t WHERE x = 1 RESTRICT")
        assert isinstance(q, DeleteQuery)
        assert q.mode == "RESTRICT"

    def test_parse_default_mode(self):
        from mdql.query_parser import parse_query
        from mdql._native import DeleteQuery
        q = parse_query("DELETE FROM t WHERE x = 1")
        assert isinstance(q, DeleteQuery)
        assert q.mode == "DEFAULT"
