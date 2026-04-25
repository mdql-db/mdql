"""Database config — for compatibility with existing imports."""

from dataclasses import dataclass, field


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


def load_database_config(db_dir):
    """Load database config — delegates to Rust internally via Database."""
    from mdql.api import Database
    db = Database(db_dir)
    return DatabaseConfig(name=db.name)
