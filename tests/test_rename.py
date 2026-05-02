"""Tests for rename operations: entry rename, table rename, column rename."""

from pathlib import Path

import pytest
import yaml

from mdql.api import Database, Table
from mdql.errors import MdqlError


DB_CONFIG = """\
---
type: database
name: testdb
foreign_keys:
  - from: orders.strategy
    to: strategies.path
---
"""

STRATEGIES_SCHEMA = """\
---
type: schema
table: strategies
primary_key: path
frontmatter:
  title:
    type: string
---
"""

ORDERS_SCHEMA = """\
---
type: schema
table: orders
primary_key: path
frontmatter:
  strategy:
    type: string
  amount:
    type: float
---
"""


def _make_db(tmp_path):
    (tmp_path / "_mdql.md").write_text(DB_CONFIG)

    strats = tmp_path / "strategies"
    strats.mkdir()
    (strats / "_mdql.md").write_text(STRATEGIES_SCHEMA)
    (strats / "alpha.md").write_text("---\ntitle: Alpha\n---\n# Alpha\n")
    (strats / "beta.md").write_text("---\ntitle: Beta\n---\n# Beta\n")

    orders = tmp_path / "orders"
    orders.mkdir()
    (orders / "_mdql.md").write_text(ORDERS_SCHEMA)
    (orders / "order-1.md").write_text("---\nstrategy: alpha.md\namount: 100.0\n---\n# Order 1\n")
    (orders / "order-2.md").write_text("---\nstrategy: alpha.md\namount: 200.0\n---\n# Order 2\n")
    (orders / "order-3.md").write_text("---\nstrategy: beta.md\namount: 50.0\n---\n# Order 3\n")

    return Database(str(tmp_path))


class TestEntryRename:
    def test_rename_file(self, tmp_path):
        db = _make_db(tmp_path)
        msg = db.rename("strategies", "alpha.md", "alpha-strategy.md")
        assert "alpha-strategy.md" in msg
        assert (tmp_path / "strategies" / "alpha-strategy.md").exists()
        assert not (tmp_path / "strategies" / "alpha.md").exists()

    def test_rename_cascades_fk(self, tmp_path):
        db = _make_db(tmp_path)
        db.rename("strategies", "alpha.md", "alpha-strategy.md")

        order1 = yaml.safe_load((tmp_path / "orders" / "order-1.md").read_text().split("---")[1])
        assert order1["strategy"] == "alpha-strategy.md"

        order2 = yaml.safe_load((tmp_path / "orders" / "order-2.md").read_text().split("---")[1])
        assert order2["strategy"] == "alpha-strategy.md"

        order3 = yaml.safe_load((tmp_path / "orders" / "order-3.md").read_text().split("---")[1])
        assert order3["strategy"] == "beta.md"

    def test_rename_nonexistent(self, tmp_path):
        db = _make_db(tmp_path)
        with pytest.raises(MdqlError, match="not found"):
            db.rename("strategies", "nope.md", "new.md")

    def test_rename_target_exists(self, tmp_path):
        db = _make_db(tmp_path)
        with pytest.raises(MdqlError, match="already exists"):
            db.rename("strategies", "alpha.md", "beta.md")


class TestTableRename:
    def test_rename_table(self, tmp_path):
        db = _make_db(tmp_path)
        msg = db.rename_table("strategies", "strats")
        assert "strats" in msg
        assert (tmp_path / "strats").is_dir()
        assert not (tmp_path / "strategies").exists()

    def test_rename_table_updates_schema(self, tmp_path):
        db = _make_db(tmp_path)
        db.rename_table("strategies", "strats")

        schema_text = (tmp_path / "strats" / "_mdql.md").read_text()
        assert "table: strats" in schema_text

    def test_rename_table_updates_fk_config(self, tmp_path):
        db = _make_db(tmp_path)
        db.rename_table("strategies", "strats")

        config_text = (tmp_path / "_mdql.md").read_text()
        assert "to: strats.path" in config_text
        assert "to: strategies.path" not in config_text

    def test_rename_table_nonexistent(self, tmp_path):
        db = _make_db(tmp_path)
        with pytest.raises(MdqlError, match="not found"):
            db.rename_table("nope", "new")

    def test_rename_table_target_exists(self, tmp_path):
        db = _make_db(tmp_path)
        with pytest.raises(MdqlError, match="already exists"):
            db.rename_table("strategies", "orders")


class TestColumnRename:
    def test_rename_field(self, tmp_path):
        db = _make_db(tmp_path)
        strats = db.table("strategies")
        count = strats.rename_field("title", "name")
        assert count == 2

        alpha = yaml.safe_load((tmp_path / "strategies" / "alpha.md").read_text().split("---")[1])
        assert "name" in alpha
        assert "title" not in alpha
