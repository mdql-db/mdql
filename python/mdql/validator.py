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

    # Rust FFI re-parses from disk, so it needs the table folder path.
    folder = getattr(parsed, '_folder', None)
    if folder is None:
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
