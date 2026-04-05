"""Orchestrate loading a table folder into validated rows."""

from __future__ import annotations

from pathlib import Path

from mdql.database import DatabaseConfig, load_database_config
from mdql.errors import ValidationError
from mdql.model import Row, to_row
from mdql.parser import parse_file
from mdql.schema import MDQL_FILENAME, Schema, load_schema
from mdql.validator import validate_file

# Files that are not data rows
RESERVED_FILES = {MDQL_FILENAME}


def load_table(
    folder: Path,
) -> tuple[Schema, list[Row], list[ValidationError]]:
    """Load all markdown files in a folder, validate, and return rows.

    Returns:
        (schema, valid_rows, all_errors)
    """
    schema = load_schema(folder)

    md_files = sorted(
        f for f in folder.glob("*.md") if f.name not in RESERVED_FILES
    )

    rows: list[Row] = []
    all_errors: list[ValidationError] = []

    for md_file in md_files:
        parsed = parse_file(
            md_file,
            relative_to=folder,
            normalize_numbered=schema.normalize_numbered_headings,
        )
        errors = validate_file(parsed, schema)

        if errors:
            all_errors.extend(errors)
        else:
            row = to_row(parsed, schema)
            rows.append(row)

    return schema, rows, all_errors


def load_database(
    db_dir: Path,
) -> tuple[DatabaseConfig, dict[str, tuple[Schema, list[Row]]], list[ValidationError]]:
    """Load a multi-table database directory.

    Returns:
        (db_config, {table_name: (schema, rows)}, all_errors)
    """
    db_config = load_database_config(db_dir)

    tables: dict[str, tuple[Schema, list[Row]]] = {}
    all_errors: list[ValidationError] = []

    for child in sorted(db_dir.iterdir()):
        if child.is_dir() and (child / MDQL_FILENAME).exists():
            schema, rows, errors = load_table(child)
            tables[schema.table] = (schema, rows)
            all_errors.extend(errors)

    return db_config, tables, all_errors
