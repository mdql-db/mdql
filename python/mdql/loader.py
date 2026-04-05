"""Table loading — wraps Rust _native.load_table."""

from __future__ import annotations

from pathlib import Path

from mdql._native import load_table as _rust_load_table
from mdql.errors import ValidationError
from mdql.schema import Schema


def _parse_validation_error(error_str: str) -> ValidationError:
    """Convert a Rust validation error string into a ValidationError object."""
    return ValidationError(
        file_path="",
        error_type="validation",
        message=error_str,
    )


def load_table(folder: str | Path) -> tuple:
    """Load all markdown files in a table folder.

    Returns (Schema, list[dict], list[ValidationError]).
    """
    schema_data, rows, errors = _rust_load_table(str(folder))
    schema = Schema._from_dict(schema_data)
    validation_errors = [_parse_validation_error(e) for e in errors]
    return schema, list(rows), validation_errors
