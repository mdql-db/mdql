"""Auto-manage created/modified timestamps in frontmatter.

MDQL reserves two frontmatter fields globally:
  - created: date when the file was first stamped (never overwritten)
  - modified: date when the file was last stamped (always updated)

Both are ISO date strings ("YYYY-MM-DD"). Stamping works at the raw
text level to preserve existing frontmatter formatting.
"""

from __future__ import annotations

import datetime
import re
from pathlib import Path

# Reserved timestamp field names — global across all tables.
TIMESTAMP_FIELDS = {"created", "modified"}

_CREATED_RE = re.compile(r"^created\s*:.*$", re.MULTILINE)
_MODIFIED_RE = re.compile(r"^modified\s*:.*$", re.MULTILINE)


def stamp_file(path: Path, *, now: datetime.date | None = None) -> dict[str, bool]:
    """Add or update timestamp fields in a single markdown file.

    Args:
        path: Path to the markdown file.
        now: Override the current date (useful for tests).

    Returns:
        {"created_set": bool, "modified_updated": bool}
    """
    today = (now or datetime.date.today()).isoformat()
    text = path.read_text(encoding="utf-8")
    lines = text.split("\n")

    if not lines or lines[0].strip() != "---":
        return {"created_set": False, "modified_updated": False}

    # Find closing ---
    end_idx = None
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end_idx = i
            break

    if end_idx is None:
        return {"created_set": False, "modified_updated": False}

    fm_lines = lines[1:end_idx]

    # Check existing fields
    created_idx = None
    modified_idx = None
    for i, line in enumerate(fm_lines):
        stripped = line.lstrip()
        if stripped.startswith("created:") or stripped.startswith("created :"):
            created_idx = i
        elif stripped.startswith("modified:") or stripped.startswith("modified :"):
            modified_idx = i

    created_set = False
    if created_idx is None:
        fm_lines.append(f'created: "{today}"')
        created_set = True

    if modified_idx is not None:
        fm_lines[modified_idx] = f'modified: "{today}"'
    else:
        fm_lines.append(f'modified: "{today}"')

    new_lines = ["---"] + fm_lines + ["---"] + lines[end_idx + 1:]
    path.write_text("\n".join(new_lines), encoding="utf-8")

    return {"created_set": created_set, "modified_updated": True}


def stamp_table(
    folder: Path,
    *,
    now: datetime.date | None = None,
) -> list[tuple[str, dict[str, bool]]]:
    """Stamp all data files in a table folder.

    Returns:
        List of (filename, result) tuples.
    """
    from mdql.schema import MDQL_FILENAME

    results = []
    for md_file in sorted(folder.glob("*.md")):
        if md_file.name == MDQL_FILENAME:
            continue
        result = stamp_file(md_file, now=now)
        results.append((md_file.name, result))

    return results
