"""Optional pandas integration for MDQL.

Requires pandas to be installed. pandas is not a dependency of mdql core.
"""

from __future__ import annotations

import warnings
from pathlib import Path
from typing import TYPE_CHECKING

from mdql.loader import load_table
from mdql.model import Row
from mdql.schema import Schema

if TYPE_CHECKING:
    import pandas as pd


def to_dataframe(
    rows: list[Row],
    schema: Schema,
) -> pd.DataFrame:
    """Convert MDQL rows to a pandas DataFrame with proper dtypes.

    - date fields → datetime64[ns] (via pd.to_datetime)
    - int fields → Int64 (nullable integer)
    - float fields → Float64 (nullable float)
    - bool fields → boolean (nullable bool)
    - string[] fields → kept as lists
    - string fields → string dtype
    """
    import pandas as pd

    df = pd.DataFrame(rows)

    if df.empty:
        return df

    for name, field_def in schema.frontmatter.items():
        if name not in df.columns:
            continue

        if field_def.type == "date":
            df[name] = pd.to_datetime(df[name], errors="coerce")
        elif field_def.type == "int":
            df[name] = pd.to_numeric(df[name], errors="coerce").astype("Int64")
        elif field_def.type == "float":
            df[name] = pd.to_numeric(df[name], errors="coerce").astype("Float64")
        elif field_def.type == "bool":
            df[name] = df[name].astype("boolean")
        elif field_def.type == "string":
            df[name] = df[name].astype("string")

    return df


def load_dataframe(
    folder: str | Path,
    *,
    errors: str = "warn",
) -> pd.DataFrame:
    """Load a table folder directly into a pandas DataFrame.

    Args:
        folder: Path to a table directory containing _mdql.md.
        errors: How to handle validation errors.
            "warn" (default) — emit a warning and return only valid rows.
            "raise" — raise if any file fails validation.
            "ignore" — silently skip invalid files.

    Returns:
        DataFrame with proper dtypes derived from the schema.
    """
    from mdql.errors import MdqlError

    folder = Path(folder)
    schema, rows, validation_errors = load_table(folder)

    if validation_errors:
        error_files = sorted({e.file_path for e in validation_errors})
        msg = (
            f"{len(error_files)} file(s) failed validation in "
            f"'{schema.table}': {', '.join(error_files)}"
        )
        if errors == "raise":
            raise MdqlError(msg)
        elif errors == "warn":
            warnings.warn(msg, stacklevel=2)

    return to_dataframe(rows, schema)
