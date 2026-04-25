"""Markdown parser — wraps Rust _native.parse_file."""

from __future__ import annotations

import datetime
import re
from pathlib import Path
from typing import Any

from mdql._native import parse_file as _rust_parse_file, normalize_heading

_DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def _coerce_unquoted_dates(fm: dict, raw_lines: list[str]) -> dict:
    """Convert date-like strings to datetime.date if unquoted in raw YAML."""
    if not isinstance(fm, dict):
        return fm
    # Build set of keys whose values are unquoted dates
    unquoted_date_keys = set()
    for line in raw_lines:
        if ":" not in line:
            continue
        key, _, val = line.partition(":")
        key = key.strip()
        val = val.strip()
        if _DATE_RE.match(val) and key in fm:
            unquoted_date_keys.add(key)

    for key in unquoted_date_keys:
        val = fm[key]
        if isinstance(val, str) and _DATE_RE.match(val):
            try:
                y, m, d = val.split("-")
                fm[key] = datetime.date(int(y), int(m), int(d))
            except (ValueError, TypeError):
                pass
    return fm


class Section:
    """A parsed H2 section."""

    def __init__(self, data: dict):
        self.raw_heading = data["raw_heading"]
        self.heading = data["normalized_heading"]
        self.normalized_heading = data["normalized_heading"]
        self.body = data["body"]
        self.line_number = data["line_number"]

    def __repr__(self):
        return f"Section(heading='{self.heading}')"


class ParsedFile:
    """Result of parsing a markdown file."""

    def __init__(self, data: dict, raw_yaml_lines: list[str] | None = None):
        self.path = data["path"]
        self.raw_frontmatter = data["raw_frontmatter"]
        if raw_yaml_lines and isinstance(self.raw_frontmatter, dict):
            _coerce_unquoted_dates(self.raw_frontmatter, raw_yaml_lines)
        self.h1 = data["h1"]
        self.h1_line_number = data.get("h1_line_number")
        self.sections = [Section(s) for s in data.get("sections", [])]
        self.parse_errors = data.get("parse_errors", [])

    def __repr__(self):
        return f"ParsedFile(path='{self.path}', sections={len(self.sections)})"


def _extract_yaml_lines(filepath: Path) -> list[str]:
    """Extract raw YAML lines from frontmatter."""
    try:
        text = filepath.read_text(encoding="utf-8")
    except Exception:
        return []
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        return []
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            return lines[1:i]
    return []


def parse_file(
    path: str | Path,
    folder: str | Path | None = None,
    normalize: bool = False,
    *,
    relative_to: str | Path | None = None,
    normalize_numbered: bool | None = None,
) -> ParsedFile:
    """Parse a markdown file into structured data."""
    rel = folder or relative_to
    if normalize_numbered is not None:
        normalize = normalize_numbered
    data = _rust_parse_file(
        str(path),
        str(rel) if rel else None,
        normalize,
    )
    p = Path(path)
    raw_yaml_lines = _extract_yaml_lines(p)
    pf = ParsedFile(data, raw_yaml_lines=raw_yaml_lines)
    if rel is not None:
        pf._folder = Path(rel) if not isinstance(rel, Path) else rel
    elif p.is_absolute():
        pf._folder = p.parent
    return pf


__all__ = ["ParsedFile", "Section", "parse_file", "normalize_heading"]
