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


def _serialize_frontmatter(
    data: dict[str, Any],
    schema: Schema,
    *,
    preserve_created: str | None = None,
) -> str:
    """Serialize frontmatter fields to YAML, in schema field order.

    Args:
        preserve_created: If set, use this ISO date string for ``created``
            instead of today (used by replace/update to keep the original).
    """
    today = datetime.date.today().isoformat()

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

    # Timestamps — preserve original created on replace/update
    created = preserve_created or data.get("created", today)
    if isinstance(created, datetime.date):
        created = created.isoformat()
    fm_lines.append(f'created: "{created}"')
    fm_lines.append(f'modified: "{today}"')

    return "---\n" + "\n".join(fm_lines) + "\n---\n"


def _serialize_body(data: dict[str, Any], schema: Schema) -> str:
    """Serialize H1 and sections from a data dict."""
    body = ""

    # H1
    if schema.h1_required:
        if schema.h1_must_equal_frontmatter and schema.h1_must_equal_frontmatter in data:
            h1_text = str(data[schema.h1_must_equal_frontmatter])
        else:
            h1_text = str(data.get("h1", data.get("title", "")))
        body += f"\n# {h1_text}\n"

    # Required sections get scaffolded; sections with provided content get written
    for name, section_def in schema.sections.items():
        section_body = data.get(name, "")
        if section_def.required or section_body:
            body += f"\n## {name}\n\n{section_body}\n"

    return body


def _read_existing(filepath: Path) -> tuple[dict[str, Any], str]:
    """Read an existing markdown file, returning (frontmatter_dict, raw_body).

    The raw_body is everything after the closing ``---``, preserved exactly.
    """
    import yaml

    text = filepath.read_text(encoding="utf-8")
    lines = text.split("\n")

    if not lines or lines[0].strip() != "---":
        raise MdqlError(f"No frontmatter in {filepath.name}")

    end_idx = None
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end_idx = i
            break

    if end_idx is None:
        raise MdqlError(f"Unclosed frontmatter in {filepath.name}")

    fm = yaml.safe_load("\n".join(lines[1:end_idx])) or {}
    raw_body = "\n".join(lines[end_idx + 1:])

    return fm, raw_body


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
        body: str | None = None,
        filename: str | None = None,
        replace: bool = False,
    ) -> Path:
        """Create a new row file (INSERT or INSERT ... ON CONFLICT REPLACE).

        Args:
            data: Frontmatter field values. Must include all required fields
                  (except created/modified which are auto-set).
            body: Pre-formatted markdown body (everything after frontmatter).
                  If provided, sections in data are ignored and body is used
                  verbatim. If omitted, required sections are scaffolded from
                  the schema.
            filename: Override the auto-generated filename (without .md extension).
            replace: If True, overwrite an existing file (preserving its
                     original ``created`` timestamp). If False (default),
                     raise on conflict.

        Returns:
            Path to the created file.

        Raises:
            MdqlError: If the filename can't be derived, the file already exists
                       (and replace=False), or the generated file fails validation.
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

        # Preserve original created timestamp on replace
        preserve_created: str | None = None
        old_content: str | None = None
        if filepath.exists():
            if not replace:
                raise MdqlError(f"File already exists: {filename}")
            old_fm, _ = _read_existing(filepath)
            raw_created = old_fm.get("created")
            if raw_created is not None:
                preserve_created = (
                    raw_created.isoformat()
                    if isinstance(raw_created, datetime.date)
                    else str(raw_created)
                )
            old_content = filepath.read_text(encoding="utf-8")

        content = _serialize_frontmatter(
            data, self._schema, preserve_created=preserve_created
        )
        if body is not None:
            if not body.startswith("\n"):
                content += "\n"
            content += body
            if not body.endswith("\n"):
                content += "\n"
        else:
            content += _serialize_body(data, self._schema)

        filepath.write_text(content, encoding="utf-8")

        # Validate — roll back on failure
        parsed = parse_file(
            filepath,
            relative_to=self.path,
            normalize_numbered=self._schema.normalize_numbered_headings,
        )
        errors = validate_file(parsed, self._schema)

        if errors:
            if old_content is not None:
                filepath.write_text(old_content, encoding="utf-8")
            else:
                filepath.unlink()
            error_msgs = "; ".join(e.message for e in errors)
            raise MdqlError(f"Validation failed: {error_msgs}")

        return filepath

    def update(
        self,
        filename: str,
        data: dict[str, Any],
        *,
        body: str | None = None,
    ) -> Path:
        """Partial update of an existing row (SQL UPDATE).

        Merges ``data`` into existing frontmatter — only the fields you
        provide are changed, everything else is kept. The markdown body
        is preserved unless ``body`` is given.

        Args:
            filename: The row file to update (e.g. ``"funding-rate-fade.md"``).
            data: Frontmatter fields to change. Omitted fields keep their
                  current values.
            body: If provided, replaces the entire markdown body. If omitted,
                  the existing body is preserved.

        Returns:
            Path to the updated file.

        Raises:
            MdqlError: If the file doesn't exist or the result fails validation.
        """
        if not filename.endswith(".md"):
            filename = filename + ".md"

        filepath = self.path / filename
        if not filepath.exists():
            raise MdqlError(f"File not found: {filename}")

        old_fm, old_body = _read_existing(filepath)
        old_content = filepath.read_text(encoding="utf-8")

        # Merge: existing frontmatter + updates
        merged = {**old_fm, **data}

        # Preserve original created
        raw_created = old_fm.get("created")
        preserve_created: str | None = None
        if raw_created is not None:
            preserve_created = (
                raw_created.isoformat()
                if isinstance(raw_created, datetime.date)
                else str(raw_created)
            )

        content = _serialize_frontmatter(
            merged, self._schema, preserve_created=preserve_created
        )
        if body is not None:
            if not body.startswith("\n"):
                content += "\n"
            content += body
            if not body.endswith("\n"):
                content += "\n"
        else:
            content += old_body

        filepath.write_text(content, encoding="utf-8")

        # Validate — roll back on failure
        parsed = parse_file(
            filepath,
            relative_to=self.path,
            normalize_numbered=self._schema.normalize_numbered_headings,
        )
        errors = validate_file(parsed, self._schema)

        if errors:
            filepath.write_text(old_content, encoding="utf-8")
            error_msgs = "; ".join(e.message for e in errors)
            raise MdqlError(f"Validation failed: {error_msgs}")

        return filepath

    def delete(self, filename: str) -> Path:
        """Delete a row file (SQL DELETE).

        Args:
            filename: The row file to delete (e.g. ``"old-strategy.md"``).

        Returns:
            Path to the deleted file (no longer exists on disk).

        Raises:
            MdqlError: If the file doesn't exist.
        """
        if not filename.endswith(".md"):
            filename = filename + ".md"

        filepath = self.path / filename
        if not filepath.exists():
            raise MdqlError(f"File not found: {filename}")

        filepath.unlink()
        return filepath

    def execute_sql(self, sql: str) -> str:
        """Execute a SQL statement (SELECT, INSERT, UPDATE, DELETE).

        Returns a human-readable result string.
        """
        from mdql.query_parser import (
            DeleteQuery,
            InsertQuery,
            Query,
            UpdateQuery,
            parse_query,
        )

        stmt = parse_query(sql)

        if isinstance(stmt, Query):
            return self._exec_select(stmt)
        elif isinstance(stmt, InsertQuery):
            return self._exec_insert(stmt)
        elif isinstance(stmt, UpdateQuery):
            return self._exec_update(stmt)
        elif isinstance(stmt, DeleteQuery):
            return self._exec_delete(stmt)
        else:
            raise MdqlError(f"Unknown statement type: {type(stmt)}")

    def _exec_select(self, query) -> str:
        from mdql.query_engine import execute_query
        from mdql.projector import format_results

        _, rows, _ = _load_table(self.path)
        result_rows, result_columns = execute_query(query, rows, self._schema)
        return format_results(result_rows, columns=result_columns)

    def _exec_insert(self, query) -> str:
        # Build data dict from columns + values, coerce types from schema
        data: dict[str, Any] = {}
        for col, val in zip(query.columns, query.values):
            field_def = self._schema.frontmatter.get(col)
            if field_def and field_def.type == "string[]" and isinstance(val, str):
                data[col] = [v.strip() for v in val.split(",")]
            else:
                data[col] = val
        filepath = self.insert(data)
        return f"INSERT 1 ({filepath.name})"

    def _exec_update(self, query) -> str:
        # Find matching rows via WHERE
        _, rows, _ = _load_table(self.path)

        if query.where:
            from mdql.query_engine import _evaluate
            matching = [r for r in rows if _evaluate(query.where, r, self._schema)]
        else:
            matching = rows

        if not matching:
            return "UPDATE 0"

        data = {col: val for col, val in query.assignments}
        # Coerce string[] from schema
        for col, val in data.items():
            field_def = self._schema.frontmatter.get(col)
            if field_def and field_def.type == "string[]" and isinstance(val, str):
                data[col] = [v.strip() for v in val.split(",")]

        count = 0
        for row in matching:
            self.update(row["path"], data)
            count += 1

        return f"UPDATE {count}"

    def _exec_delete(self, query) -> str:
        _, rows, _ = _load_table(self.path)

        if query.where:
            from mdql.query_engine import _evaluate
            matching = [r for r in rows if _evaluate(query.where, r, self._schema)]
        else:
            matching = rows

        if not matching:
            return "DELETE 0"

        count = 0
        for row in matching:
            self.delete(row["path"])
            count += 1

        return f"DELETE {count}"

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
