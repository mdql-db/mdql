"""File validation — wraps Rust _native.validate_file."""

from __future__ import annotations

from pathlib import Path

from mdql._native import validate_file as _rust_validate_file
from mdql.errors import ValidationError


def validate_file(parsed, schema) -> list[ValidationError]:
    """Validate a parsed file against its schema.

    parsed: a ParsedFile object (has .path attribute, relative to table folder)
    schema: a Schema object (has .table attribute) — we derive the folder
    """
    path = parsed.path if hasattr(parsed, 'path') else parsed["path"]

    # We need the folder path. Infer from the ParsedFile: its path is relative
    # to the table folder, and the file was parsed from there.
    # The _rust_validate_file needs the folder path to re-load the schema.
    # We need to figure out the folder. The parsed file has _folder if set,
    # or we can get it from the schema.
    folder = getattr(parsed, '_folder', None)
    if folder is None:
        # The path in ParsedFile is relative (e.g. "simple.md").
        # We can't derive the folder from just a relative path and a Schema.
        # But the _rust_validate_file actually re-parses from folder/path,
        # so we need to know the folder somehow.
        # For backward compat, store it on ParsedFile during parse_file().
        raise ValueError(
            "Cannot determine table folder for validation. "
            "Make sure parse_file was called with a folder/relative_to argument."
        )

    errors_data = _rust_validate_file({"path": path}, str(folder))

    return [
        ValidationError(
            file_path=e.get("file_path", path),
            error_type=e.get("error_type", "unknown"),
            message=e.get("message", ""),
            field=e.get("field"),
            line_number=e.get("line_number"),
        )
        for e in errors_data
    ]
