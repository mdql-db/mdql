"""Schema loading and types — wraps Rust _native.load_schema."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from mdql._native import load_schema as _rust_load_schema
from mdql.errors import SchemaNotFoundError, SchemaInvalidError


class FieldDef:
    """Definition of a frontmatter field."""

    def __init__(self, data: dict):
        self.type = data["type"]
        self.required = data["required"]
        self.enum = data.get("enum")


class SectionDef:
    """Definition of a section."""

    def __init__(self, data: dict):
        self.type = data["type"]
        self.required = data["required"]


class Rules:
    """Schema rules."""

    def __init__(self, data: dict):
        self.reject_unknown_frontmatter = data.get("reject_unknown_frontmatter", False)
        self.reject_unknown_sections = data.get("reject_unknown_sections", False)
        self.reject_duplicate_sections = data.get("reject_duplicate_sections", True)
        self.normalize_numbered_headings = data.get("normalize_numbered_headings", False)


class Schema:
    """MDQL table schema."""

    def __init__(self, table: str, primary_key: str, frontmatter: dict,
                 h1_required: bool, sections: dict, rules: Rules,
                 h1_must_equal_frontmatter: str | None = None):
        self.table = table
        self.primary_key = primary_key
        self.frontmatter = frontmatter
        self.h1_required = h1_required
        self.h1_must_equal_frontmatter = h1_must_equal_frontmatter
        self.sections = sections
        self.rules = rules

    # Convenience accessors for rules
    @property
    def reject_unknown_frontmatter(self):
        return self.rules.reject_unknown_frontmatter

    @property
    def reject_unknown_sections(self):
        return self.rules.reject_unknown_sections

    @property
    def reject_duplicate_sections(self):
        return self.rules.reject_duplicate_sections

    @property
    def normalize_numbered_headings(self):
        return self.rules.normalize_numbered_headings

    @property
    def fields(self) -> dict[str, FieldDef]:
        """All field definitions (alias for frontmatter)."""
        return self.frontmatter

    @property
    def metadata_keys(self) -> list[str]:
        """Return all queryable column names (frontmatter + synthetic)."""
        keys = ["path", "h1", "created", "modified"]
        keys.extend(self.frontmatter.keys())
        return keys

    @classmethod
    def _from_dict(cls, data: dict) -> Schema:
        frontmatter = {
            name: FieldDef(fd) for name, fd in data.get("frontmatter", {}).items()
        }
        sections = {
            name: SectionDef(sd) for name, sd in data.get("sections", {}).items()
        }
        rules = Rules(data.get("rules", {}))
        return cls(
            table=data["table"],
            primary_key=data["primary_key"],
            frontmatter=frontmatter,
            h1_required=data["h1_required"],
            sections=sections,
            rules=rules,
            h1_must_equal_frontmatter=data.get("h1_must_equal_frontmatter"),
        )

    def __repr__(self):
        return f"Schema(table='{self.table}', fields={len(self.frontmatter)})"


def load_schema(folder: str | Path) -> Schema:
    """Load schema from a table directory."""
    try:
        data = _rust_load_schema(str(folder))
        return Schema._from_dict(data)
    except FileNotFoundError as e:
        raise SchemaNotFoundError(str(e)) from None
    except ValueError as e:
        raise SchemaInvalidError(str(e)) from None
    except RuntimeError as e:
        msg = str(e)
        if "not found" in msg.lower() or "no _mdql.md" in msg.lower():
            raise SchemaNotFoundError(msg) from None
        raise SchemaInvalidError(msg) from None
