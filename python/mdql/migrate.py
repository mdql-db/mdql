"""Field migration — wraps Rust _native migrate functions."""

from __future__ import annotations

from pathlib import Path

from mdql._native import (
    rename_frontmatter_key_in_file as _rust_rename_fm,
    drop_frontmatter_key_in_file as _rust_drop_fm,
    rename_section_in_file as _rust_rename_sec,
    drop_section_in_file as _rust_drop_sec,
    merge_sections_in_file as _rust_merge_secs,
    update_schema as _rust_update_schema,
)


def rename_frontmatter_key_in_file(path: str | Path, old_key: str, new_key: str) -> bool:
    return _rust_rename_fm(str(path), old_key, new_key)


def drop_frontmatter_key_in_file(path: str | Path, key: str) -> bool:
    return _rust_drop_fm(str(path), key)


def rename_section_in_file(
    path: str | Path, old_name: str, new_name: str, normalize: bool = False,
) -> bool:
    return _rust_rename_sec(str(path), old_name, new_name, normalize)


def drop_section_in_file(path: str | Path, name: str, normalize: bool = False) -> bool:
    return _rust_drop_sec(str(path), name, normalize)


def merge_sections_in_file(
    path: str | Path, sources: list[str], into: str, normalize: bool = False,
) -> bool:
    return _rust_merge_secs(str(path), sources, into, normalize)


def update_schema(
    schema_path: str | Path,
    rename_frontmatter: tuple[str, str] | None = None,
    drop_frontmatter: str | None = None,
    rename_section: tuple[str, str] | None = None,
    drop_section: str | None = None,
    merge_sections: tuple[list[str], str] | None = None,
) -> None:
    _rust_update_schema(
        str(schema_path),
        rename_fm_old=rename_frontmatter[0] if rename_frontmatter else None,
        rename_fm_new=rename_frontmatter[1] if rename_frontmatter else None,
        drop_fm=drop_frontmatter,
        rename_sec_old=rename_section[0] if rename_section else None,
        rename_sec_new=rename_section[1] if rename_section else None,
        drop_sec=drop_section,
        merge_sources=merge_sections[0] if merge_sections else None,
        merge_into=merge_sections[1] if merge_sections else None,
    )
