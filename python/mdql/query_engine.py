"""Query execution — wraps Rust _native.execute_query_rows."""

from __future__ import annotations

from mdql._native import execute_query_rows as _rust_execute_query_rows


def execute_query(query, rows, schema=None):
    """Execute a parsed query against in-memory rows.

    Args:
        query: A Query object (from parse_query) or any object with ._inner.
        rows: list of dicts — the table rows.
        schema: Optional schema (unused — the Rust engine doesn't need it for
                in-memory execution).

    Returns:
        (rows, columns) tuple where rows is list[dict] and columns is list[str].
    """
    sql = _reconstruct_sql(query)
    result_rows, columns = _rust_execute_query_rows(sql, rows)
    return list(result_rows), list(columns)


def _reconstruct_sql(query):
    """Best-effort SQL reconstruction from a Query/wrapper object."""
    inner = getattr(query, '_inner', query)

    parts = ["SELECT"]

    columns = inner.columns if hasattr(inner, 'columns') else query.columns
    if columns == "*":
        parts.append("*")
    elif isinstance(columns, list):
        cols = []
        for c in columns:
            if " " in c:
                cols.append(f"`{c}`")
            else:
                cols.append(c)
        parts.append(", ".join(cols))
    else:
        parts.append(str(columns))

    table = inner.table if hasattr(inner, 'table') else query.table
    parts.append(f"FROM {table}")

    alias = getattr(inner, 'table_alias', None) or getattr(query, 'table_alias', None)
    if alias:
        parts.append(alias)

    join = getattr(inner, 'join', None) or getattr(query, 'join', None)
    if join:
        parts.append(f"JOIN {join.table}")
        if join.alias:
            parts.append(join.alias)
        parts.append(f"ON {join.left_col} = {join.right_col}")

    where_clause = getattr(inner, 'where_clause', None) or getattr(query, 'where_', None)
    if where_clause is None:
        try:
            where_clause = query.where_
        except AttributeError:
            pass
    if where_clause is not None:
        parts.append(f"WHERE {_reconstruct_where(where_clause)}")

    order_by = getattr(inner, 'order_by', None) or getattr(query, 'order_by', None)
    if order_by:
        order_parts = []
        for spec in order_by:
            direction = "DESC" if spec.descending else "ASC"
            col = spec.column if " " not in spec.column else f"`{spec.column}`"
            order_parts.append(f"{col} {direction}")
        parts.append(f"ORDER BY {', '.join(order_parts)}")

    limit = getattr(inner, 'limit', None) or getattr(query, 'limit', None)
    if limit is not None:
        parts.append(f"LIMIT {limit}")

    return " ".join(parts)


def _reconstruct_where(clause):
    from mdql._native import BoolOp, Comparison

    if isinstance(clause, BoolOp):
        left = _reconstruct_where(clause.left)
        right = _reconstruct_where(clause.right)
        return f"{left} {clause.op} {right}"
    elif isinstance(clause, Comparison):
        if clause.op in ("IS NULL", "IS NOT NULL"):
            return f"{clause.column} {clause.op}"
        elif isinstance(clause.value, str):
            return f"{clause.column} {clause.op} '{clause.value}'"
        elif isinstance(clause.value, list):
            vals = ", ".join(
                f"'{v}'" if isinstance(v, str) else str(v) for v in clause.value
            )
            return f"{clause.column} {clause.op} ({vals})"
        elif clause.value is None:
            return f"{clause.column} {clause.op} NULL"
        else:
            return f"{clause.column} {clause.op} {clause.value}"
    return str(clause)
