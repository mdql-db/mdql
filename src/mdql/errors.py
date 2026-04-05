from __future__ import annotations

from dataclasses import dataclass, field


class MdqlError(Exception):
    """Base exception for all mdql errors."""


class SchemaNotFoundError(MdqlError):
    """Raised when _mdql.md is missing from a table directory."""


class SchemaInvalidError(MdqlError):
    """Raised when _mdql.md (type: schema) fails meta-schema validation."""


class ParseError(MdqlError):
    """Raised when a markdown file cannot be parsed at all."""


class QueryParseError(MdqlError):
    """Raised when a SQL-like query cannot be parsed."""


class QueryExecutionError(MdqlError):
    """Raised when query execution fails (e.g. unknown column)."""


class DatabaseConfigError(MdqlError):
    """Raised when _mdql.md (type: database) is missing or invalid."""


@dataclass
class ValidationError:
    """One validation finding for a file."""

    file_path: str
    error_type: str
    field: str | None
    message: str
    line_number: int | None = None

    def __str__(self) -> str:
        loc = f":{self.line_number}" if self.line_number else ""
        return f"{self.file_path}{loc}: {self.message}"
