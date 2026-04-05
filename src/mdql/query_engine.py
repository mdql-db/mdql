"""Execute parsed queries over in-memory rows."""

from __future__ import annotations

import datetime
import re
from typing import Any

from mdql.errors import QueryExecutionError
from mdql.model import Row
from mdql.query_parser import BoolOp, Comparison, OrderSpec, Query, WhereClause
from mdql.schema import Schema


def execute_query(
    query: Query,
    rows: list[Row],
    schema: Schema,
) -> tuple[list[Row], list[str]]:
    """Execute a single-table query and return (result_rows, column_names)."""
    return _execute(query, rows, schema)


def execute_join_query(
    query: Query,
    tables: dict[str, tuple[Schema, list[Row]]],
) -> tuple[list[Row], list[str]]:
    """Execute a query with JOIN across multiple tables."""
    if query.join is None:
        raise QueryExecutionError("No JOIN clause in query")

    # Resolve table names (could be actual names or aliases)
    left_name = query.table
    right_name = query.join.table

    if left_name not in tables:
        raise QueryExecutionError(f"Unknown table '{left_name}'")
    if right_name not in tables:
        raise QueryExecutionError(f"Unknown table '{right_name}'")

    left_schema, left_rows = tables[left_name]
    right_schema, right_rows = tables[right_name]

    # Build alias mapping: alias -> table_name
    aliases: dict[str, str] = {}
    aliases[left_name] = left_name
    aliases[right_name] = right_name
    if query.table_alias:
        aliases[query.table_alias] = left_name
    if query.join.alias:
        aliases[query.join.alias] = right_name

    # Resolve ON columns (e.g., "b.strategy" -> table=backtests, col=strategy)
    left_on_table, left_on_col = _resolve_dotted(query.join.left_col, aliases)
    right_on_table, right_on_col = _resolve_dotted(query.join.right_col, aliases)

    # Determine which side is which
    if left_on_table == left_name:
        join_left_col, join_right_col = left_on_col, right_on_col
    else:
        join_left_col, join_right_col = right_on_col, left_on_col

    # Build index on right table for nested-loop join
    right_index: dict[Any, list[Row]] = {}
    for r in right_rows:
        key = r.get(join_right_col)
        if key is not None:
            right_index.setdefault(key, []).append(r)

    # Perform join
    joined_rows: list[Row] = []
    left_alias = query.table_alias or left_name
    right_alias = query.join.alias or right_name

    for lr in left_rows:
        key = lr.get(join_left_col)
        if key is None:
            continue
        for rr in right_index.get(key, []):
            # Merge: prefix columns with alias
            merged: Row = {}
            for k, v in lr.items():
                merged[f"{left_alias}.{k}"] = v
            for k, v in rr.items():
                merged[f"{right_alias}.{k}"] = v
            joined_rows.append(merged)

    # Use left schema for filtering (could be either)
    return _execute(query, joined_rows, left_schema)


def _resolve_dotted(col: str, aliases: dict[str, str]) -> tuple[str, str]:
    """Resolve 'alias.column' to (table_name, column)."""
    if "." in col:
        alias, column = col.split(".", 1)
        table = aliases.get(alias, alias)
        return table, column
    return "", col


def _execute(
    query: Query,
    rows: list[Row],
    schema: Schema,
) -> tuple[list[Row], list[str]]:
    """Core execution: filter, sort, limit, project."""
    # Determine available columns from data
    all_columns: dict[str, None] = {}
    for r in rows:
        for k in r:
            all_columns.setdefault(k, None)

    # Resolve column list
    if query.columns == "*":
        columns = list(all_columns)
    else:
        columns = query.columns

    # Filter
    if query.where:
        rows = [r for r in rows if _evaluate(query.where, r, schema)]

    # Sort
    if query.order_by:
        rows = _sort(rows, query.order_by)

    # Limit
    if query.limit is not None:
        rows = rows[: query.limit]

    return rows, columns


def _evaluate(clause: WhereClause, row: Row, schema: Schema) -> bool:
    if isinstance(clause, BoolOp):
        left = _evaluate(clause.left, row, schema)
        if clause.op == "AND":
            return left and _evaluate(clause.right, row, schema)
        else:  # OR
            return left or _evaluate(clause.right, row, schema)

    assert isinstance(clause, Comparison)
    col = clause.column
    actual = row.get(col)

    if clause.op == "IS NULL":
        return actual is None
    if clause.op == "IS NOT NULL":
        return actual is not None

    if actual is None:
        return False  # NULL compared to anything is False

    expected = _coerce(clause.value, actual)

    if clause.op == "=":
        return _eq(actual, expected)
    if clause.op == "!=":
        return not _eq(actual, expected)
    if clause.op == "<":
        return _compare(actual, expected) < 0
    if clause.op == ">":
        return _compare(actual, expected) > 0
    if clause.op == "<=":
        return _compare(actual, expected) <= 0
    if clause.op == ">=":
        return _compare(actual, expected) >= 0
    if clause.op == "LIKE":
        return _like(actual, expected)
    if clause.op == "NOT LIKE":
        return not _like(actual, expected)
    if clause.op == "IN":
        if isinstance(expected, list):
            return any(_eq(actual, _coerce(v, actual)) for v in expected)
        return _eq(actual, expected)

    raise QueryExecutionError(f"Unknown operator: {clause.op}")


def _eq(actual: Any, expected: Any) -> bool:
    """Equality with list-aware matching (e.g., 'defi-protocol' = categories)."""
    if isinstance(actual, list):
        return expected in actual
    return actual == expected


def _like(actual: Any, pattern: Any) -> bool:
    """SQL LIKE: % = any chars, _ = one char. Works on strings and lists."""
    if not isinstance(pattern, str):
        return False

    # Convert SQL LIKE pattern to regex
    regex = "^"
    for ch in pattern:
        if ch == "%":
            regex += ".*"
        elif ch == "_":
            regex += "."
        else:
            regex += re.escape(ch)
    regex += "$"
    compiled = re.compile(regex, re.IGNORECASE | re.DOTALL)

    if isinstance(actual, list):
        return any(compiled.match(str(item)) for item in actual)
    return bool(compiled.match(str(actual)))


def _coerce(value: Any, target: Any) -> Any:
    """Coerce a query literal to match the target column's type."""
    if value is None:
        return None

    if isinstance(value, list):
        return value  # IN lists stay as-is

    if isinstance(target, int) and not isinstance(target, bool):
        if isinstance(value, (int, float)):
            return int(value)
        if isinstance(value, str):
            try:
                return int(value)
            except ValueError:
                pass

    if isinstance(target, float):
        if isinstance(value, (int, float)):
            return float(value)
        if isinstance(value, str):
            try:
                return float(value)
            except ValueError:
                pass

    if isinstance(target, datetime.date) and isinstance(value, str):
        try:
            return datetime.date.fromisoformat(value)
        except ValueError:
            pass

    return value


def _compare(a: Any, b: Any) -> int:
    """Three-way comparison. Returns <0, 0, >0."""
    try:
        if a < b:
            return -1
        if a > b:
            return 1
        return 0
    except TypeError:
        # Fallback: compare as strings
        sa, sb = str(a), str(b)
        if sa < sb:
            return -1
        if sa > sb:
            return 1
        return 0


def _sort(rows: list[Row], specs: list[OrderSpec]) -> list[Row]:
    """Sort rows by multiple columns."""
    import functools

    def cmp(a: Row, b: Row) -> int:
        for spec in specs:
            va = a.get(spec.column)
            vb = b.get(spec.column)
            # NULLs sort last
            if va is None and vb is None:
                continue
            if va is None:
                return 1
            if vb is None:
                return -1
            c = _compare(va, vb)
            if spec.descending:
                c = -c
            if c != 0:
                return c
        return 0

    return sorted(rows, key=functools.cmp_to_key(cmp))
