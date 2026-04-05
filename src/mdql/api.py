"""Object-oriented API for MDQL databases and tables.

    from mdql.api import Database, Table

    # Single table
    table = Table("examples/strategies/")
    path = table.insert({
        "title": "My New Strategy",
        "status": "HYPOTHESIS",
        "mechanism": 5,
        "categories": ["funding-rates"],
    })

    # Database
    db = Database("examples/")
    path = db.table("strategies").insert({...})
"""

from __future__ import annotations

import datetime
import re
from pathlib import Path
from typing import Any

from mdql.database import DatabaseConfig, load_database_config
from mdql.errors import MdqlError, ValidationError
from mdql.loader import load_table as _load_table
from mdql.model import Row
from mdql.parser import parse_file
from mdql.schema import MDQL_FILENAME, Schema, load_schema
from mdql.stamp import TIMESTAMP_FIELDS
from mdql.validator import validate_file


def _slugify(text: str, max_length: int = 80) -> str:
    """Convert text to a filename-safe slug."""
    slug = text.lower().strip()
    slug = re.sub(r"[^\w\s-]", "", slug)
    slug = re.sub(r"[\s_]+", "-", slug)
    slug = re.sub(r"-+", "-", slug)
    slug = slug.strip("-")
    if len(slug) > max_length:
        slug = slug[:max_length].rstrip("-")
    return slug


def _format_value(value: Any, field_type: str) -> str:
    """Format a single value for YAML frontmatter."""
    if field_type == "string":
        return f'"{value}"'
    elif field_type == "date":
        if isinstance(value, datetime.date):
            return f'"{value.isoformat()}"'
        return f'"{value}"'
    elif field_type == "string[]":
        if not value:
            return "[]"
        return "\n" + "\n".join(f"  - {item}" for item in value)
    elif field_type == "bool":
        return "true" if value else "false"
    else:  # int, float
        return str(value)


def _serialize_row(data: dict[str, Any], schema: Schema) -> str:
    """Serialize a data dict to markdown file content."""
    today = datetime.date.today().isoformat()

    # --- Frontmatter ---
    fm_lines: list[str] = []
    for name, field_def in schema.frontmatter.items():
        if name in TIMESTAMP_FIELDS:
            continue  # handled below
        if name not in data:
            continue
        formatted = _format_value(data[name], field_def.type)
        if field_def.type == "string[]" and data[name]:
            fm_lines.append(f"{name}:{formatted}")
        else:
            fm_lines.append(f"{name}: {formatted}")

    # Timestamps
    created = data.get("created", today)
    if isinstance(created, datetime.date):
        created = created.isoformat()
    fm_lines.append(f'created: "{created}"')
    fm_lines.append(f'modified: "{today}"')

    content = "---\n" + "\n".join(fm_lines) + "\n---\n"

    # --- H1 ---
    if schema.h1_required:
        if schema.h1_must_equal_frontmatter and schema.h1_must_equal_frontmatter in data:
            h1_text = str(data[schema.h1_must_equal_frontmatter])
        else:
            h1_text = str(data.get("h1", data.get("title", "")))
        content += f"\n# {h1_text}\n"

    # --- Sections ---
    # Required sections get scaffolded; sections with provided content get written
    written_sections: set[str] = set()

    for name, section_def in schema.sections.items():
        section_body = data.get(name, "")
        if section_def.required or section_body:
            content += f"\n## {name}\n\n{section_body}\n"
            written_sections.add(name)

    # Also write any section content provided that isn't in the schema
    for key, value in data.items():
        if (
            key not in schema.frontmatter
            and key not in TIMESTAMP_FIELDS
            and key not in written_sections
            and key not in ("path", "h1")
            and isinstance(value, str)
            and "\n" in value  # heuristic: multi-line strings are section content
        ):
            content += f"\n## {key}\n\n{value}\n"

    return content


def _coerce_value(raw: str, field_type: str) -> Any:
    """Coerce a CLI string value to the appropriate Python type."""
    if field_type == "int":
        return int(raw)
    elif field_type == "float":
        return float(raw)
    elif field_type == "bool":
        return raw.lower() in ("true", "1", "yes")
    elif field_type == "string[]":
        return [item.strip() for item in raw.split(",")]
    else:  # string, date
        return raw


class Table:
    """A single MDQL table backed by a directory with _mdql.md."""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self._schema = load_schema(self.path)

    @property
    def schema(self) -> Schema:
        return self._schema

    @property
    def name(self) -> str:
        return self._schema.table

    def insert(
        self,
        data: dict[str, Any],
        *,
        filename: str | None = None,
    ) -> Path:
        """Create a new row file from a data dict.

        Args:
            data: Field values. Must include all required frontmatter fields
                  (except created/modified which are auto-set).
            filename: Override the auto-generated filename (without .md extension).

        Returns:
            Path to the created file.

        Raises:
            MdqlError: If the filename can't be derived, the file already exists,
                       or the generated file fails validation.
        """
        if filename is None:
            title = data.get("title")
            if title is None:
                raise MdqlError(
                    "Cannot derive filename: provide 'title' in data or pass filename="
                )
            filename = _slugify(str(title))

        if not filename.endswith(".md"):
            filename = filename + ".md"

        filepath = self.path / filename
        if filepath.exists():
            raise MdqlError(f"File already exists: {filename}")

        content = _serialize_row(data, self._schema)
        filepath.write_text(content, encoding="utf-8")

        # Validate the created file
        parsed = parse_file(
            filepath,
            relative_to=self.path,
            normalize_numbered=self._schema.normalize_numbered_headings,
        )
        errors = validate_file(parsed, self._schema)

        if errors:
            filepath.unlink()
            error_msgs = "; ".join(e.message for e in errors)
            raise MdqlError(f"Validation failed: {error_msgs}")

        return filepath

    def load(self) -> tuple[list[Row], list[ValidationError]]:
        """Load and validate all rows.

        Returns:
            (valid_rows, validation_errors)
        """
        _, rows, errors = _load_table(self.path)
        return rows, errors

    def validate(self) -> list[ValidationError]:
        """Validate all files. Returns list of errors (empty = all valid)."""
        _, _, errors = _load_table(self.path)
        return errors


class Database:
    """An MDQL database backed by a directory with a type: database _mdql.md."""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self._config = load_database_config(self.path)
        self._tables: dict[str, Table] = {}
        self._discover_tables()

    def _discover_tables(self) -> None:
        for child in sorted(self.path.iterdir()):
            if child.is_dir() and (child / MDQL_FILENAME).exists():
                t = Table(child)
                self._tables[t.name] = t

    @property
    def name(self) -> str:
        return self._config.name

    @property
    def config(self) -> DatabaseConfig:
        return self._config

    @property
    def tables(self) -> dict[str, Table]:
        return dict(self._tables)

    def table(self, name: str) -> Table:
        """Get a table by name.

        Raises:
            MdqlError: If the table doesn't exist in this database.
        """
        if name not in self._tables:
            available = ", ".join(sorted(self._tables)) or "(none)"
            raise MdqlError(
                f"Table '{name}' not found in database '{self.name}'. "
                f"Available: {available}"
            )
        return self._tables[name]
