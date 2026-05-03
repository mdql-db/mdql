"""Tests for dict fields with nested list and dict values."""

from pathlib import Path

import yaml

from mdql.api import Table
from mdql.errors import MdqlError


SCHEMA = """\
---
type: schema
table: items
primary_key: path
frontmatter:
  title:
    type: string
  params:
    type: dict
---
"""


def _make_table(tmp_path):
    (tmp_path / "_mdql.md").write_text(SCHEMA)
    return Table(str(tmp_path))


class TestDictWithLists:
    def test_insert_dict_with_list_value(self, tmp_path):
        t = _make_table(tmp_path)
        t.insert(
            {"title": "Test", "params": {"blocked_tokens": ["ZK", "W"]}},
            body="# Test\n",
        )
        content = (tmp_path / "test.md").read_text()
        fm = yaml.safe_load(content.split("---")[1])
        assert fm["params"]["blocked_tokens"] == ["ZK", "W"]

    def test_insert_dict_with_empty_list(self, tmp_path):
        t = _make_table(tmp_path)
        t.insert(
            {"title": "Test", "params": {"tags": []}},
            body="# Test\n",
        )
        content = (tmp_path / "test.md").read_text()
        fm = yaml.safe_load(content.split("---")[1])
        assert fm["params"]["tags"] == []

    def test_insert_dict_with_mixed_values(self, tmp_path):
        t = _make_table(tmp_path)
        t.insert(
            {
                "title": "Test",
                "params": {
                    "threshold": 0.5,
                    "blocked_tokens": ["ZK", "W"],
                    "enabled": True,
                },
            },
            body="# Test\n",
        )
        content = (tmp_path / "test.md").read_text()
        fm = yaml.safe_load(content.split("---")[1])
        assert fm["params"]["threshold"] == 0.5
        assert fm["params"]["blocked_tokens"] == ["ZK", "W"]
        assert fm["params"]["enabled"] is True

    def test_update_dict_with_list_value(self, tmp_path):
        t = _make_table(tmp_path)
        t.insert(
            {"title": "Test", "params": {"tokens": ["BTC"]}},
            body="# Test\n",
        )
        t.update("test.md", {"params": {"tokens": ["BTC", "ETH"]}})
        content = (tmp_path / "test.md").read_text()
        fm = yaml.safe_load(content.split("---")[1])
        assert fm["params"]["tokens"] == ["BTC", "ETH"]

    def test_load_dict_with_list_value(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "item.md").write_text(
            "---\ntitle: Item\nparams:\n  tokens:\n    - ZK\n    - W\n---\n# Item\n"
        )
        rows, errors = t.load()
        assert len(rows) == 1
        params = rows[0]["params"]
        assert params["tokens"] == ["ZK", "W"]

    def test_roundtrip_dict_with_list(self, tmp_path):
        t = _make_table(tmp_path)
        original = {"blocked": ["ZK", "W", "ARB"], "max_size": 1000}
        t.insert(
            {"title": "RT", "params": original},
            body="# RT\n",
        )
        rows, _ = t.load()
        assert len(rows) == 1
        params = rows[0]["params"]
        assert params["blocked"] == ["ZK", "W", "ARB"]
        assert params["max_size"] == 1000

    def test_validate_dict_with_list(self, tmp_path):
        t = _make_table(tmp_path)
        (tmp_path / "item.md").write_text(
            "---\ntitle: Item\nparams:\n  tokens:\n    - ZK\n    - W\n---\n# Item\n"
        )
        errors = t.validate()
        assert len(errors) == 0
