"""Object-oriented API for MDQL databases and tables.

Wraps the Rust _native module to provide a Python-compatible interface.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from mdql._native import RustTable, RustDatabase, slugify as _rust_slugify
from mdql.errors import MdqlError


def _slugify(text: str, max_length: int = 80) -> str:
    return _rust_slugify(text, max_length)


class Table:
    """A single MDQL table backed by a directory with _mdql.md."""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self._rust = RustTable(str(self.path))

    @classmethod
    def _from_rust(cls, rust_table: RustTable) -> Table:
        t = cls.__new__(cls)
        t.path = Path(rust_table.path)
        t._rust = rust_table
        return t

    @property
    def schema(self) -> "Schema":
        from mdql.schema import Schema
        return Schema._from_dict(self._rust.schema_data())

    @property
    def name(self) -> str:
        return self._rust.name

    def insert(
        self,
        data: dict[str, Any],
        *,
        body: str | None = None,
        filename: str | None = None,
        replace: bool = False,
    ) -> Path:
        try:
            result_path = self._rust.insert(data, body=body, filename=filename, replace=replace)
            return Path(result_path)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def update(
        self,
        filename: str,
        data: dict[str, Any],
        *,
        body: str | None = None,
    ) -> Path:
        try:
            result_path = self._rust.update(filename, data, body=body)
            return Path(result_path)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def delete(self, filename: str) -> Path:
        try:
            result_path = self._rust.delete(filename)
            return Path(result_path)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def execute_sql(self, sql: str) -> str:
        try:
            return self._rust.execute_sql(sql)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def query(self, sql: str) -> tuple[list[dict], list[str]]:
        """Execute a SELECT query and return structured results."""
        try:
            rows, columns = self._rust.query(sql)
            return list(rows), list(columns)
        except (ValueError, RuntimeError) as e:
            raise MdqlError(str(e)) from None

    def rename_field(self, old_name: str, new_name: str) -> int:
        try:
            return self._rust.rename_field(old_name, new_name)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def drop_field(self, name: str) -> int:
        try:
            return self._rust.drop_field(name)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def merge_fields(self, sources: list[str], *, into: str) -> int:
        try:
            return self._rust.merge_fields(sources, into)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def load(self, *, where: dict | str | None = None) -> tuple[list[dict], list[str]]:
        try:
            rows, errors = self._rust.load(where=where)
            return list(rows), list(errors)
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def update_many(
        self,
        filenames: list[str],
        data: dict[str, Any],
    ) -> list[str]:
        try:
            return list(self._rust.update_many(filenames, data))
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def validate(self) -> list[str]:
        try:
            return list(self._rust.validate())
        except RuntimeError as e:
            raise MdqlError(str(e)) from None

    def __repr__(self) -> str:
        return f"Table({str(self.path)!r})"


class Database:
    """A multi-table MDQL database."""

    def __init__(self, path: str | Path | None = None) -> None:
        if path is None:
            env_path = os.environ.get("MDQL_DATABASE_PATH")
            if env_path:
                path = env_path
            else:
                raise MdqlError(
                    "No path provided and MDQL_DATABASE_PATH not set"
                )
        self.path = Path(path)
        self._rust = RustDatabase(str(self.path))

    @property
    def name(self) -> str:
        return self._rust.name

    @property
    def tables(self) -> list[str]:
        return self._rust.table_names

    def table(self, name: str) -> Table:
        try:
            rust_table = self._rust.table(name)
            return Table._from_rust(rust_table)
        except (ValueError, RuntimeError) as e:
            raise MdqlError(str(e)) from None

    def query(self, sql: str) -> tuple[list[dict], list[str]]:
        """Execute a SQL SELECT query (including JOINs) across all tables."""
        try:
            rows, columns = self._rust.query(sql)
            return list(rows), list(columns)
        except (ValueError, RuntimeError) as e:
            raise MdqlError(str(e)) from None

    def __repr__(self) -> str:
        return f"Database({str(self.path)!r})"
