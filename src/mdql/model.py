"""Convert parsed files to normalized row dicts."""

from __future__ import annotations

import datetime
from typing import Any

from mdql.parser import ParsedFile
from mdql.schema import Schema

Row = dict[str, Any]


def to_row(parsed: ParsedFile, schema: Schema) -> Row:
    """Convert a validated ParsedFile into a flat row dict."""
    row: Row = {"path": parsed.path}

    # Frontmatter fields — coerce types where needed
    for key, value in parsed.raw_frontmatter.items():
        field_def = schema.frontmatter.get(key)
        if field_def and field_def.type == "date" and isinstance(value, str):
            try:
                row[key] = datetime.date.fromisoformat(value)
            except ValueError:
                row[key] = value
        else:
            row[key] = value

    # H1
    if parsed.h1 is not None:
        row["h1"] = parsed.h1

    # Sections
    for section in parsed.sections:
        row[section.normalized_heading] = section.body

    return row
