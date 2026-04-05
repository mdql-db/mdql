"""Optional Polars integration for MDQL.

Requires polars to be installed.
"""

from __future__ import annotations

import warnings
from pathlib import Path
from typing import TYPE_CHECKING

from mdql.loader import load_table
from mdql.schema import Schema

if TYPE_CHECKING:
    import polars as pl


def to_dataframe(
    rows: list[dict],
    schema: Schema,
) -> "pl.DataFrame":
    """Convert MDQL rows to a Polars DataFrame with proper dtypes.

    - date fields → Date
    - int fields → Int64
    - float fields → Float64
    - bool fields → Boolean
    - string[] fields → List(Utf8)
    - string fields → Utf8
    """
    import polars as pl

    if not rows:
        return pl.DataFrame()

    df = pl.DataFrame(rows)

    casts = {}
    for name, field_def in schema.frontmatter.items():
        if name not in df.columns:
            continue
        if field_def.type == "date":
            casts[name] = pl.Date
        elif field_def.type == "int":
            casts[name] = pl.Int64
        elif field_def.type == "float":
            casts[name] = pl.Float64
        elif field_def.type == "bool":
            casts[name] = pl.Boolean

    if casts:
        df = df.cast(casts, strict=False)

    return df


def load_dataframe(
    folder: str | Path,
    *,
    errors: str = "warn",
) -> "pl.DataFrame":
    """Load a table folder directly into a Polars DataFrame."""
    from mdql.errors import MdqlError

    folder = Path(folder)
    schema, rows, validation_errors = load_table(folder)

    if validation_errors:
        msg = f"{len(validation_errors)} file(s) failed validation in '{schema.table}'"
        if errors == "raise":
            raise MdqlError(msg)
        elif errors == "warn":
            warnings.warn(msg, stacklevel=2)

    return to_dataframe(rows, schema)
