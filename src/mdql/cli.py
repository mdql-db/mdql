"""MDQL command-line interface."""

from __future__ import annotations

from pathlib import Path
from typing import Optional

import typer

from mdql.errors import MdqlError
from mdql.loader import load_table
from mdql.projector import format_results
from mdql.schema import load_schema

import yaml


def _is_database_dir(folder: Path) -> bool:
    """Check if a folder contains a _mdql.md with type: database."""
    from mdql.schema import MDQL_FILENAME
    mdql_file = folder / MDQL_FILENAME
    if not mdql_file.exists():
        return False
    try:
        text = mdql_file.read_text(encoding="utf-8")
        lines = text.split("\n")
        if lines and lines[0].strip() == "---":
            for i in range(1, len(lines)):
                if lines[i].strip() == "---":
                    fm = yaml.safe_load("\n".join(lines[1:i]))
                    return isinstance(fm, dict) and fm.get("type") == "database"
    except Exception:
        pass
    return False


app = typer.Typer(
    name="mdql",
    help="A strict Markdown database with SQL-like queries.",
    no_args_is_help=True,
)


@app.command()
def validate(
    folder: Path = typer.Argument(..., help="Path to table folder"),
) -> None:
    """Validate all markdown files in a table folder."""
    try:
        schema, rows, errors = load_table(folder)
    except MdqlError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)

    valid_count = len(rows)
    error_files = {e.file_path for e in errors}
    invalid_count = len(error_files)

    if errors:
        for err in errors:
            typer.echo(str(err), err=True)
        typer.echo(f"\n{valid_count} valid, {invalid_count} invalid", err=True)
        raise typer.Exit(1)
    else:
        typer.echo(f"All {valid_count} files valid in table '{schema.table}'")


@app.command()
def inspect(
    folder: Path = typer.Argument(..., help="Path to table folder"),
    file: Optional[str] = typer.Option(None, "--file", "-f", help="Inspect a single file"),
    format: str = typer.Option("table", "--format", help="Output format: table, json, csv"),
    truncate: int = typer.Option(80, "--truncate", "-t", help="Max chars per cell in table mode"),
) -> None:
    """Inspect normalized rows from a table folder."""
    try:
        schema, rows, errors = load_table(folder)
    except MdqlError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)

    if file:
        rows = [r for r in rows if r["path"] == file]
        if not rows:
            typer.echo(f"File '{file}' not found or invalid", err=True)
            raise typer.Exit(1)

    typer.echo(format_results(rows, output_format=format, truncate=truncate))


@app.command()
def schema(
    folder: Path = typer.Argument(..., help="Path to table or database folder"),
) -> None:
    """Print the effective schema for a table or entire database."""
    from mdql.schema import MDQL_FILENAME

    is_db = _is_database_dir(folder)

    if is_db:
        try:
            from mdql.database import load_database_config
            db_config = load_database_config(folder)
        except MdqlError as e:
            typer.echo(f"Error: {e}", err=True)
            raise typer.Exit(1)

        typer.echo(f"Database: {db_config.name}")
        typer.echo()

        # Find all table subdirectories
        table_dirs = sorted(
            d for d in folder.iterdir()
            if d.is_dir() and (d / MDQL_FILENAME).exists()
        )

        for td in table_dirs:
            try:
                s = load_schema(td)
            except MdqlError as e:
                typer.echo(f"Error loading {td.name}: {e}", err=True)
                continue
            _print_table_schema(s)
            typer.echo()

        if db_config.foreign_keys:
            typer.echo("Foreign keys:")
            for fk in db_config.foreign_keys:
                typer.echo(f"  {fk.from_table}.{fk.from_column} -> {fk.to_table}.{fk.to_column}")
    else:
        try:
            s = load_schema(folder)
        except MdqlError as e:
            typer.echo(f"Error: {e}", err=True)
            raise typer.Exit(1)
        _print_table_schema(s)


def _print_table_schema(s) -> None:
    """Print schema details for a single table."""
    typer.echo(f"Table: {s.table}")
    typer.echo(f"  Primary key: {s.primary_key}")
    typer.echo(f"  H1 required: {s.h1_required}")
    if s.h1_must_equal_frontmatter:
        typer.echo(f"  H1 must equal: frontmatter.{s.h1_must_equal_frontmatter}")

    typer.echo("  Frontmatter:")
    for name, fd in s.frontmatter.items():
        req = "required" if fd.required else "optional"
        enum_str = f" enum={fd.enum}" if fd.enum else ""
        typer.echo(f"    {name}: {fd.type} ({req}){enum_str}")

    if s.sections:
        typer.echo("  Sections:")
        for name, sd in s.sections.items():
            req = "required" if sd.required else "optional"
            typer.echo(f"    {name}: {sd.type} ({req})")

    typer.echo("  Rules:")
    typer.echo(f"    reject_unknown_frontmatter: {s.reject_unknown_frontmatter}")
    typer.echo(f"    reject_unknown_sections: {s.reject_unknown_sections}")
    typer.echo(f"    reject_duplicate_sections: {s.reject_duplicate_sections}")
    typer.echo(f"    normalize_numbered_headings: {s.normalize_numbered_headings}")


@app.command()
def create(
    folder: Path = typer.Argument(..., help="Path to table folder"),
    set_fields: list[str] = typer.Option(
        ..., "--set", "-s", help="Field value as key=value (repeatable)"
    ),
    filename: Optional[str] = typer.Option(None, "--filename", help="Override auto-generated filename"),
) -> None:
    """Create a new row file in a table."""
    from mdql.api import Table, _coerce_value

    try:
        table = Table(folder)
    except MdqlError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)

    # Parse --set key=value pairs
    data: dict = {}
    for pair in set_fields:
        if "=" not in pair:
            typer.echo(f"Error: invalid --set format '{pair}' (expected key=value)", err=True)
            raise typer.Exit(1)
        key, _, raw_value = pair.partition("=")
        key = key.strip()
        raw_value = raw_value.strip()

        # Coerce type from schema
        field_def = table.schema.frontmatter.get(key)
        if field_def:
            try:
                data[key] = _coerce_value(raw_value, field_def.type)
            except (ValueError, TypeError) as e:
                typer.echo(f"Error: cannot parse '{key}={raw_value}' as {field_def.type}: {e}", err=True)
                raise typer.Exit(1)
        else:
            data[key] = raw_value

    try:
        filepath = table.insert(data, filename=filename)
    except MdqlError as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)

    typer.echo(f"Created {filepath.relative_to(folder)}")


@app.command()
def stamp(
    folder: Path = typer.Argument(..., help="Path to table folder"),
) -> None:
    """Add or update created/modified timestamps in all data files."""
    from mdql.stamp import stamp_table

    try:
        results = stamp_table(folder)
    except Exception as e:
        typer.echo(f"Error: {e}", err=True)
        raise typer.Exit(1)

    created_count = sum(1 for _, r in results if r["created_set"])
    modified_count = sum(1 for _, r in results if r["modified_updated"])

    typer.echo(
        f"Stamped {len(results)} files: "
        f"{created_count} created set, {modified_count} modified updated"
    )


@app.command()
def query(
    folder: Path = typer.Argument(..., help="Path to table or database folder"),
    sql: str = typer.Argument(..., help="SQL-like query string"),
    format: str = typer.Option("table", "--format", help="Output format: table, json, csv"),
    truncate: int = typer.Option(80, "--truncate", "-t", help="Max chars per cell in table mode"),
) -> None:
    """Run a SQL-like query against a table or database."""
    from mdql.query_engine import execute_join_query, execute_query
    from mdql.query_parser import parse_query

    try:
        q = parse_query(sql)
    except MdqlError as e:
        typer.echo(f"Query error: {e}", err=True)
        raise typer.Exit(1)

    try:
        if q.join is not None:
            # JOIN query: folder must be a database directory
            from mdql.loader import load_database
            db_config, tables, errors = load_database(folder)
            result_rows, result_columns = execute_join_query(q, tables)
        else:
            # Single-table query
            is_db = _is_database_dir(folder)
            if is_db:
                # Folder is a database dir; find the table subdirectory
                from mdql.loader import load_database
                db_config, tables, errors = load_database(folder)
                if q.table not in tables:
                    typer.echo(f"Error: table '{q.table}' not found in database", err=True)
                    raise typer.Exit(1)
                schema, rows = tables[q.table]
            else:
                schema, rows, errors = load_table(folder)
            result_rows, result_columns = execute_query(q, rows, schema)
    except MdqlError as e:
        typer.echo(f"Query error: {e}", err=True)
        raise typer.Exit(1)

    typer.echo(
        format_results(result_rows, columns=result_columns, output_format=format, truncate=truncate)
    )
