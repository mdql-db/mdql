"""Parse markdown files into structured representations.

Handles frontmatter extraction, H1/H2 detection, code fence tracking,
and numbered heading normalization.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path

import yaml

from mdql.errors import ParseError


@dataclass
class Section:
    """One H2 section from a markdown file."""

    raw_heading: str  # e.g. "1. Hypothesis"
    normalized_heading: str  # e.g. "Hypothesis"
    body: str  # raw markdown content under the heading
    line_number: int


@dataclass
class ParsedFile:
    """Result of parsing one markdown file."""

    path: str  # relative to table folder
    raw_frontmatter: dict
    h1: str | None
    h1_line_number: int | None
    sections: list[Section]
    parse_errors: list[str] = field(default_factory=list)


_NUMBERED_HEADING_RE = re.compile(r"^\d+\.\s+")
_FENCE_OPEN_RE = re.compile(r"^(`{3,}|~{3,})")
_H1_RE = re.compile(r"^#\s+(.+)$")
_H2_RE = re.compile(r"^##\s+(.+)$")


def normalize_heading(raw: str) -> str:
    """Strip leading numbered prefix like '1. ' from a heading."""
    return _NUMBERED_HEADING_RE.sub("", raw).strip()


def parse_file(
    path: Path,
    *,
    relative_to: Path | None = None,
    normalize_numbered: bool = True,
) -> ParsedFile:
    """Parse a markdown file into a ParsedFile structure.

    Args:
        path: Absolute or relative path to the markdown file.
        relative_to: If given, store path relative to this directory.
        normalize_numbered: Strip leading number prefixes from H2 headings.
    """
    rel_path = str(path.relative_to(relative_to)) if relative_to else str(path)

    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as e:
        raise ParseError(f"Cannot read {rel_path}: {e}") from e

    lines = text.split("\n")

    # --- Parse frontmatter ---
    raw_frontmatter: dict = {}
    body_start = 0
    parse_errors: list[str] = []

    if lines and lines[0].strip() == "---":
        closing = None
        for i in range(1, len(lines)):
            if lines[i].strip() == "---":
                closing = i
                break

        if closing is None:
            parse_errors.append("Unclosed frontmatter (no closing '---')")
            body_start = 1
        else:
            fm_text = "\n".join(lines[1:closing])
            try:
                parsed_yaml = yaml.safe_load(fm_text)
                if parsed_yaml is None:
                    raw_frontmatter = {}
                elif isinstance(parsed_yaml, dict):
                    raw_frontmatter = parsed_yaml
                else:
                    parse_errors.append(
                        f"Frontmatter is not a mapping (got {type(parsed_yaml).__name__})"
                    )
            except yaml.YAMLError as e:
                parse_errors.append(f"Malformed YAML in frontmatter: {e}")
            body_start = closing + 1
    else:
        parse_errors.append("No frontmatter found (file must start with '---')")

    # --- Parse body: H1, H2 sections ---
    h1: str | None = None
    h1_line_number: int | None = None
    sections: list[Section] = []

    in_fence = False
    fence_char: str | None = None  # '`' or '~'
    fence_width: int = 0

    current_heading: str | None = None
    current_heading_normalized: str | None = None
    current_heading_line: int | None = None
    current_body_lines: list[str] = []

    def _finalize_section() -> None:
        nonlocal current_heading, current_heading_normalized, current_heading_line, current_body_lines
        if current_heading is not None:
            body = "\n".join(current_body_lines).strip()
            sections.append(
                Section(
                    raw_heading=current_heading,
                    normalized_heading=current_heading_normalized or current_heading,
                    body=body,
                    line_number=current_heading_line or 0,
                )
            )
            current_heading = None
            current_heading_normalized = None
            current_heading_line = None
            current_body_lines = []

    for i in range(body_start, len(lines)):
        line = lines[i]
        # 1-indexed line number
        line_num = i + 1

        # --- Code fence tracking ---
        fence_match = _FENCE_OPEN_RE.match(line)
        if fence_match:
            marker = fence_match.group(1)
            char = marker[0]
            width = len(marker)

            if not in_fence:
                in_fence = True
                fence_char = char
                fence_width = width
                if current_heading is not None:
                    current_body_lines.append(line)
                continue
            elif char == fence_char and width >= fence_width and line.strip() == marker:
                # Closing fence: same char, at least same width, nothing else on line
                in_fence = False
                fence_char = None
                fence_width = 0
                if current_heading is not None:
                    current_body_lines.append(line)
                continue

        if in_fence:
            if current_heading is not None:
                current_body_lines.append(line)
            continue

        # --- H1 detection ---
        h1_match = _H1_RE.match(line)
        if h1_match:
            if h1 is None:
                h1 = h1_match.group(1).strip()
                h1_line_number = line_num
            else:
                parse_errors.append(
                    f"Duplicate H1 at line {line_num} (first was at line {h1_line_number})"
                )
            continue

        # --- H2 detection ---
        h2_match = _H2_RE.match(line)
        if h2_match:
            _finalize_section()
            raw_h = h2_match.group(1).strip()
            current_heading = raw_h
            current_heading_normalized = (
                normalize_heading(raw_h) if normalize_numbered else raw_h
            )
            current_heading_line = line_num
            current_body_lines = []
            continue

        # --- Regular content ---
        if current_heading is not None:
            current_body_lines.append(line)

    _finalize_section()

    return ParsedFile(
        path=rel_path,
        raw_frontmatter=raw_frontmatter,
        h1=h1,
        h1_line_number=h1_line_number,
        sections=sections,
        parse_errors=parse_errors,
    )
