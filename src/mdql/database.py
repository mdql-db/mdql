"""Load and validate _database.md files for multi-table databases."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from mdql.errors import DatabaseConfigError
from mdql.parser import parse_file

DATABASE_FILENAME = "_database.md"


@dataclass
class ForeignKey:
    from_table: str
    from_column: str
    to_table: str
    to_column: str


@dataclass
class DatabaseConfig:
    name: str
    foreign_keys: list[ForeignKey] = field(default_factory=list)


def load_database_config(db_dir: Path) -> DatabaseConfig:
    """Load _database.md from a database directory."""
    db_path = db_dir / DATABASE_FILENAME
    if not db_path.exists():
        raise DatabaseConfigError(f"No {DATABASE_FILENAME} in {db_dir}")

    parsed = parse_file(db_path, relative_to=db_dir)

    if parsed.parse_errors:
        raise DatabaseConfigError(
            f"Cannot parse {DATABASE_FILENAME}: {'; '.join(parsed.parse_errors)}"
        )

    fm = parsed.raw_frontmatter

    if fm.get("type") != "database":
        raise DatabaseConfigError(
            f"{DATABASE_FILENAME}: frontmatter must have 'type: database'"
        )

    name = fm.get("name")
    if not isinstance(name, str):
        raise DatabaseConfigError(
            f"{DATABASE_FILENAME}: frontmatter must have 'name' as a string"
        )

    fks: list[ForeignKey] = []
    for fk_def in fm.get("foreign_keys") or []:
        if not isinstance(fk_def, dict):
            raise DatabaseConfigError(
                f"{DATABASE_FILENAME}: each foreign_key must be a mapping"
            )
        from_spec = fk_def.get("from", "")
        to_spec = fk_def.get("to", "")

        if "." not in from_spec or "." not in to_spec:
            raise DatabaseConfigError(
                f"{DATABASE_FILENAME}: foreign_key 'from' and 'to' must be 'table.column' format"
            )

        from_table, from_col = from_spec.split(".", 1)
        to_table, to_col = to_spec.split(".", 1)

        fks.append(ForeignKey(from_table, from_col, to_table, to_col))

    return DatabaseConfig(name=name, foreign_keys=fks)
