"""Timestamp management — wraps Rust _native.stamp_file."""

from __future__ import annotations

import datetime
from pathlib import Path

from mdql._native import stamp_file as _rust_stamp_file

TIMESTAMP_FIELDS = ("created", "modified")


class StampResult:
    def __init__(self, created_set: bool, modified_updated: bool):
        self.created_set = created_set
        self.modified_updated = modified_updated

    def __getitem__(self, key):
        return getattr(self, key)


def stamp_file(path: str | Path, now: datetime.datetime | datetime.date | None = None) -> StampResult:
    """Add or update created/modified timestamps in a file."""
    if isinstance(now, datetime.datetime):
        today_str = now.strftime("%Y-%m-%dT%H:%M:%S")
    elif isinstance(now, datetime.date):
        today_str = now.isoformat()
    else:
        today_str = None
    result = _rust_stamp_file(str(path), today_str)
    return StampResult(result["created_set"], result["modified_updated"])


def stamp_table(folder: str | Path, now: datetime.datetime | datetime.date | None = None) -> list[tuple[str, StampResult]]:
    """Stamp all markdown files in a table folder."""
    folder = Path(folder)
    results = []
    for f in sorted(folder.glob("*.md")):
        if f.name == "_mdql.md":
            continue
        result = stamp_file(f, now)
        results.append((f.name, result))
    return results
