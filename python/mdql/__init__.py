"""MDQL — Markdown Query Language.

Drop-in replacement backed by Rust via PyO3.
"""

from mdql.api import Database, Table
from mdql.schema import Schema

__all__ = ["Database", "Table", "Schema"]
