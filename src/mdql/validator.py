"""Validate parsed markdown files against a schema."""

from __future__ import annotations

import datetime
from collections import Counter

from mdql.errors import ValidationError
from mdql.parser import ParsedFile
from mdql.schema import Schema


def validate_file(parsed: ParsedFile, schema: Schema) -> list[ValidationError]:
    """Validate a parsed file against a schema. Returns a list of errors (empty = valid)."""
    errors: list[ValidationError] = []
    fp = parsed.path

    # Parse-level errors
    for msg in parsed.parse_errors:
        errors.append(ValidationError(fp, "parse_error", None, msg))

    # If frontmatter couldn't be parsed, skip field-level checks
    if any(e.error_type == "parse_error" for e in errors):
        return errors

    fm = parsed.raw_frontmatter

    # --- Frontmatter field checks ---
    for name, field_def in schema.frontmatter.items():
        if name not in fm:
            if field_def.required:
                errors.append(
                    ValidationError(
                        fp, "missing_field", name,
                        f"Missing required frontmatter field '{name}'"
                    )
                )
            continue

        value = fm[name]
        type_error = _check_type(value, field_def.type, name)
        if type_error:
            errors.append(ValidationError(fp, "type_mismatch", name, type_error))

        if field_def.enum is not None and value is not None:
            str_val = str(value)
            if str_val not in field_def.enum:
                errors.append(
                    ValidationError(
                        fp, "enum_violation", name,
                        f"Field '{name}' value '{str_val}' not in allowed values: {field_def.enum}"
                    )
                )

    # Unknown frontmatter
    if schema.reject_unknown_frontmatter:
        for key in fm:
            if key not in schema.frontmatter:
                errors.append(
                    ValidationError(
                        fp, "unknown_field", key,
                        f"Unknown frontmatter field '{key}' (not in schema)"
                    )
                )

    # --- H1 checks ---
    if schema.h1_required and parsed.h1 is None:
        errors.append(
            ValidationError(fp, "missing_h1", None, "Missing required H1 heading")
        )

    if (
        schema.h1_must_equal_frontmatter
        and parsed.h1 is not None
        and schema.h1_must_equal_frontmatter in fm
    ):
        expected = str(fm[schema.h1_must_equal_frontmatter])
        if parsed.h1 != expected:
            errors.append(
                ValidationError(
                    fp, "h1_mismatch", None,
                    f"H1 '{parsed.h1}' does not match frontmatter "
                    f"'{schema.h1_must_equal_frontmatter}' (expected '{expected}')",
                    line_number=parsed.h1_line_number,
                )
            )

    # --- Section checks ---
    section_names = [s.normalized_heading for s in parsed.sections]
    section_counter = Counter(section_names)

    # Duplicate sections
    if schema.reject_duplicate_sections:
        for name, count in section_counter.items():
            if count > 1:
                errors.append(
                    ValidationError(
                        fp, "duplicate_section", name,
                        f"Duplicate section '{name}' (appears {count} times)"
                    )
                )

    # Required sections
    for name, section_def in schema.sections.items():
        if section_def.required and name not in section_names:
            errors.append(
                ValidationError(
                    fp, "missing_section", name,
                    f"Missing required section '{name}'"
                )
            )

    # Unknown sections
    if schema.reject_unknown_sections:
        for section in parsed.sections:
            if section.normalized_heading not in schema.sections:
                errors.append(
                    ValidationError(
                        fp, "unknown_section", section.normalized_heading,
                        f"Unknown section '{section.normalized_heading}' (not in schema)",
                        line_number=section.line_number,
                    )
                )

    return errors


def _check_type(value: object, expected_type: str, field_name: str) -> str | None:
    """Check if a value matches the expected schema type. Returns error message or None."""
    if value is None:
        return None

    if expected_type == "string":
        if not isinstance(value, str):
            return f"Field '{field_name}' expected string, got {type(value).__name__}"

    elif expected_type == "int":
        if isinstance(value, bool) or not isinstance(value, int):
            return f"Field '{field_name}' expected int, got {type(value).__name__}"

    elif expected_type == "float":
        if isinstance(value, bool):
            return f"Field '{field_name}' expected float, got bool"
        if not isinstance(value, (int, float)):
            return f"Field '{field_name}' expected float, got {type(value).__name__}"

    elif expected_type == "bool":
        if not isinstance(value, bool):
            return f"Field '{field_name}' expected bool, got {type(value).__name__}"

    elif expected_type == "date":
        if isinstance(value, datetime.date) and not isinstance(value, datetime.datetime):
            return None
        if isinstance(value, str):
            try:
                datetime.date.fromisoformat(value)
                return None
            except ValueError:
                return f"Field '{field_name}' expected date, got string '{value}' (not ISO format)"
        return f"Field '{field_name}' expected date, got {type(value).__name__}"

    elif expected_type == "string[]":
        if not isinstance(value, list):
            return f"Field '{field_name}' expected string[], got {type(value).__name__}"
        for i, item in enumerate(value):
            if not isinstance(item, str):
                return f"Field '{field_name}[{i}]' expected string, got {type(item).__name__}"

    return None
