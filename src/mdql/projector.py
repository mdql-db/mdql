"""Format query/inspect results for output."""

from __future__ import annotations

import csv
import io
import json
import datetime
from typing import Any

from tabulate import tabulate

from mdql.model import Row


def format_results(
    rows: list[Row],
    columns: list[str] | None = None,
    output_format: str = "table",
    truncate: int = 80,
) -> str:
    """Format rows for display.

    Args:
        rows: List of row dicts.
        columns: Which columns to show. None = all columns found in rows.
        output_format: "table", "json", or "csv".
        truncate: Max chars per cell in table mode.
    """
    if not rows:
        return "No results."

    if columns is None:
        # Collect all keys preserving first-seen order
        seen: dict[str, None] = {}
        for r in rows:
            for k in r:
                seen.setdefault(k, None)
        columns = list(seen)

    projected = [_project(r, columns) for r in rows]

    if output_format == "json":
        return json.dumps(projected, indent=2, default=_json_default)
    elif output_format == "csv":
        return _to_csv(projected, columns)
    else:
        return _to_table(projected, columns, truncate)


def _project(row: Row, columns: list[str]) -> dict[str, Any]:
    return {c: row.get(c) for c in columns}


def _json_default(obj: Any) -> Any:
    if isinstance(obj, (datetime.date, datetime.datetime)):
        return obj.isoformat()
    raise TypeError(f"Object of type {type(obj).__name__} is not JSON serializable")


def _to_table(rows: list[dict], columns: list[str], truncate: int) -> str:
    table_data = []
    for r in rows:
        table_data.append([_truncate(r.get(c), truncate) for c in columns])
    return tabulate(table_data, headers=columns, tablefmt="simple")


def _to_csv(rows: list[dict], columns: list[str]) -> str:
    buf = io.StringIO()
    writer = csv.DictWriter(buf, fieldnames=columns, extrasaction="ignore")
    writer.writeheader()
    for r in rows:
        writer.writerow({k: _csv_value(r.get(k)) for k in columns})
    return buf.getvalue()


def _csv_value(v: Any) -> str:
    if v is None:
        return ""
    if isinstance(v, list):
        return ";".join(str(x) for x in v)
    if isinstance(v, (datetime.date, datetime.datetime)):
        return v.isoformat()
    return str(v)


def _truncate(value: Any, max_len: int) -> str:
    if value is None:
        return ""
    s = str(value)
    # Replace newlines for table display
    s = s.replace("\n", " ").strip()
    if len(s) > max_len:
        return s[: max_len - 3] + "..."
    return s
