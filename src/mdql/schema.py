"""Load and validate _schema.md files.

Schema files are markdown files where the structured config lives in
YAML frontmatter and the body serves as human-readable documentation.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from mdql.errors import SchemaInvalidError, SchemaNotFoundError
from mdql.parser import parse_file

SCHEMA_FILENAME = "_schema.md"

VALID_FIELD_TYPES = {"string", "int", "float", "bool", "date", "string[]"}
VALID_SECTION_TYPES = {"markdown", "text"}


@dataclass
class FieldDef:
    type: str
    required: bool
    enum: list[str] | None = None


@dataclass
class SectionDef:
    type: str  # "markdown" or "text"
    required: bool


@dataclass
class Schema:
    table: str
    primary_key: str
    frontmatter: dict[str, FieldDef]
    h1_required: bool
    h1_must_equal_frontmatter: str | None
    sections: dict[str, SectionDef]
    reject_unknown_frontmatter: bool
    reject_unknown_sections: bool
    reject_duplicate_sections: bool
    normalize_numbered_headings: bool


def load_schema(folder: Path) -> Schema:
    """Load _schema.md from a table folder and return a Schema."""
    schema_path = folder / SCHEMA_FILENAME
    if not schema_path.exists():
        raise SchemaNotFoundError(f"No {SCHEMA_FILENAME} in {folder}")

    parsed = parse_file(schema_path, relative_to=folder)

    if parsed.parse_errors:
        raise SchemaInvalidError(
            f"Cannot parse {SCHEMA_FILENAME}: {'; '.join(parsed.parse_errors)}"
        )

    fm = parsed.raw_frontmatter
    _validate_meta_schema(fm, schema_path)

    # Build field definitions
    frontmatter_defs: dict[str, FieldDef] = {}
    for name, spec in (fm.get("frontmatter") or {}).items():
        if not isinstance(spec, dict):
            raise SchemaInvalidError(
                f"{SCHEMA_FILENAME}: frontmatter.{name} must be a mapping"
            )
        ftype = spec.get("type", "string")
        if ftype not in VALID_FIELD_TYPES:
            raise SchemaInvalidError(
                f"{SCHEMA_FILENAME}: frontmatter.{name} has invalid type '{ftype}'. "
                f"Valid types: {', '.join(sorted(VALID_FIELD_TYPES))}"
            )
        enum_vals = spec.get("enum")
        if enum_vals is not None and not isinstance(enum_vals, list):
            raise SchemaInvalidError(
                f"{SCHEMA_FILENAME}: frontmatter.{name}.enum must be a list"
            )
        frontmatter_defs[name] = FieldDef(
            type=ftype,
            required=bool(spec.get("required", False)),
            enum=[str(v) for v in enum_vals] if enum_vals else None,
        )

    # Build section definitions
    section_defs: dict[str, SectionDef] = {}
    for name, spec in (fm.get("sections") or {}).items():
        if not isinstance(spec, dict):
            raise SchemaInvalidError(
                f"{SCHEMA_FILENAME}: sections.{name} must be a mapping"
            )
        stype = spec.get("type", "markdown")
        if stype not in VALID_SECTION_TYPES:
            raise SchemaInvalidError(
                f"{SCHEMA_FILENAME}: sections.{name} has invalid type '{stype}'"
            )
        section_defs[name] = SectionDef(
            type=stype,
            required=bool(spec.get("required", False)),
        )

    # H1 config
    h1_config = fm.get("h1") or {}
    h1_required = bool(h1_config.get("required", True))
    h1_must_equal = h1_config.get("must_equal_frontmatter")

    # Rules
    rules = fm.get("rules") or {}

    return Schema(
        table=fm["table"],
        primary_key=fm.get("primary_key", "path"),
        frontmatter=frontmatter_defs,
        h1_required=h1_required,
        h1_must_equal_frontmatter=h1_must_equal,
        sections=section_defs,
        reject_unknown_frontmatter=bool(rules.get("reject_unknown_frontmatter", True)),
        reject_unknown_sections=bool(rules.get("reject_unknown_sections", True)),
        reject_duplicate_sections=bool(rules.get("reject_duplicate_sections", True)),
        normalize_numbered_headings=bool(
            rules.get("normalize_numbered_headings", False)
        ),
    )


def _validate_meta_schema(fm: dict, path: Path) -> None:
    """Validate that frontmatter has the required meta-schema fields."""
    if fm.get("type") != "schema":
        raise SchemaInvalidError(
            f"{path}: frontmatter must have 'type: schema'"
        )
    if not isinstance(fm.get("table"), str):
        raise SchemaInvalidError(
            f"{path}: frontmatter must have 'table' as a string"
        )
    fm_fields = fm.get("frontmatter")
    if fm_fields is not None and not isinstance(fm_fields, dict):
        raise SchemaInvalidError(
            f"{path}: 'frontmatter' must be a mapping"
        )
    sections = fm.get("sections")
    if sections is not None and not isinstance(sections, dict):
        raise SchemaInvalidError(
            f"{path}: 'sections' must be a mapping"
        )
