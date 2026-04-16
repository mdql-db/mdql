"""Query parser — wraps Rust _native.parse_query."""

from __future__ import annotations

from mdql._native import (
    parse_query as _rust_parse_query,
    Query as _RustQuery,
    Comparison,
    BoolOp,
    OrderSpec,
    JoinInfo,
    InsertQuery,
    UpdateQuery,
    DeleteQuery,
    AlterRenameFieldQuery,
    AlterDropFieldQuery,
    AlterMergeFieldsQuery,
)
from mdql.errors import QueryParseError


class Query:
    """Wrapper around the Rust Query."""

    def __init__(self, rust_query: _RustQuery):
        self._inner = rust_query

    @property
    def columns(self):
        return self._inner.columns

    @property
    def table(self):
        return self._inner.table

    @property
    def table_alias(self):
        return self._inner.table_alias

    @property
    def where_clause(self):
        return self._inner.where_clause

    @property
    def order_by(self):
        return self._inner.order_by

    @property
    def limit(self):
        return self._inner.limit

    @property
    def joins(self):
        return self._inner.joins


def parse_query(sql: str):
    """Parse SQL into an AST object."""
    try:
        result = _rust_parse_query(sql)
    except RuntimeError as e:
        raise QueryParseError(str(e)) from None

    # Wrap SelectQuery in our Query class for .where access
    if isinstance(result, _RustQuery):
        return Query(result)
    return result


__all__ = [
    "parse_query",
    "Query",
    "Comparison",
    "BoolOp",
    "OrderSpec",
    "JoinInfo",
    "InsertQuery",
    "UpdateQuery",
    "DeleteQuery",
    "AlterRenameFieldQuery",
    "AlterDropFieldQuery",
    "AlterMergeFieldsQuery",
]
