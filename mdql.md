# Strict Markdown DB Prototype Plan

## Domains

marquela.dev
marquela.net
marquela.org

mdql.dev
mdql.io


## Goal

Build a prototype database system where:

- **Markdown files are the canonical storage format**
- **YAML frontmatter maps to metadata columns**
- **Required H1/H2 sections map to content columns**
- **Schema is enforced strictly**
- **Invalid files are rejected**
- Users can run a **small SQL-like query language** over the files
- Any cache or index is **derived only** and can be deleted and rebuilt

This prototype should prove that strict Markdown documents can act as **database rows that are friendly to humans, Git, and LLMs**.

---

## Product thesis

This system is useful because it combines:

- **human-readable documents**
- **structured schema validation**
- **queryable content**
- **LLM-friendly editing format**
- **Git-friendly storage**

The key design decision is:

> Markdown files are the source of truth.  
> The system may build temporary indexes, but it must never require an opaque database file as the canonical store.

---

## Prototype scope

For the first prototype, keep the scope narrow.

### In scope

- One folder = one table
- One Markdown file = one row
- YAML frontmatter only
- One required H1
- Named H2 sections become content fields
- Fixed schema defined in a separate schema file
- Strict validation
- Read-only SQL subset at first
- CLI tool
- Optional in-memory cache
- Optional on-disk derived index/cache, clearly disposable

### Out of scope for v1

- Transactions across multiple files
- Concurrent writers
- Full SQL parser
- Joins across tables
- Complex type system
- Permissions/auth
- Replication/networking
- Full-text ranking
- Binary blobs
- Rich Markdown AST transforms
- GUI

---

## Recommended tech choices

Use a simple stack so the prototype can be built quickly.

- **Language:** Python
- **CLI:** `typer`
- **Parsing Markdown:** `markdown-it-py` or a simple custom section parser
- **Frontmatter:** `python-frontmatter` or `PyYAML`
- **Validation:** `pydantic` or custom validators
- **SQL-like parsing:** `sqlglot` if helpful, otherwise implement a narrow custom parser
- **File watching:** optional, later
- **Testing:** `pytest`

Python is the right prototype language because parsing, validation, and iteration speed matter more than raw performance.

---

## Repository layout

```text
strict-markdown-db/
  README.md
  pyproject.toml
  src/
    smdb/
      __init__.py
      cli.py
      schema.py
      loader.py
      parser.py
      validator.py
      model.py
      query_parser.py
      query_engine.py
      projector.py
      cache.py
      errors.py
  examples/
    notes/
      .smdb.schema.yaml
      001-example.md
      002-example.md
  tests/
    test_parser.py
    test_validator.py
    test_query_engine.py
    test_cli.py
```

---

## Canonical data model

Each Markdown file is one logical row.

### Mapping rules

- File path -> implicit primary key unless overridden
- Frontmatter scalar fields -> metadata columns
- H1 -> title-like field or required display heading
- H2 sections -> content columns
- Section body -> text/markdown value
- Duplicate H2 headings -> invalid unless schema explicitly allows arrays
- Unknown headings -> invalid by default
- Missing required headings -> invalid
- Missing required frontmatter fields -> invalid

---

## Example schema file

Use one schema file per folder/table.

Path:

```text
.smdb.schema.yaml
```

Example:

```yaml
table: notes

primary_key: path

frontmatter:
  title:
    type: string
    required: true
  author:
    type: string
    required: true
  created:
    type: date
    required: true
  tags:
    type: string[]
    required: false
  status:
    type: string
    required: false
    enum: [draft, approved, archived]

h1:
  required: true
  must_equal_frontmatter: title

sections:
  Summary:
    type: markdown
    required: true
  Notes:
    type: markdown
    required: true
  Decision:
    type: markdown
    required: false

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: true
  reject_duplicate_sections: true
```

---

## Example row file

```md
---
title: Example note
author: Rasmus
created: 2026-04-04
tags:
  - db
  - markdown
status: draft
---

# Example note

## Summary
This is a structured Markdown row.

## Notes
This system stores content as Markdown files.

## Decision
Prototype it.
```

---

## Internal logical row representation

After parsing and validation, each file should become a normalized row object like:

```python
{
    "path": "001-example.md",
    "title": "Example note",
    "author": "Rasmus",
    "created": date(2026, 4, 4),
    "tags": ["db", "markdown"],
    "status": "draft",
    "Summary": "This is a structured Markdown row.",
    "Notes": "This system stores content as Markdown files.",
    "Decision": "Prototype it."
}
```

The query engine operates on this logical representation, not directly on raw text.

---

## Core architectural decision

The system has three layers:

### 1. Canonical storage layer
Markdown files and schema files on disk.

### 2. Parse + validate layer
Turns files into normalized row objects or returns validation errors.

### 3. Query layer
Executes SQL-like queries over normalized rows.

Optional:

### 4. Derived cache/index layer
Stores parsed results, hashes, mtimes, and maybe inverted text indexes.  
This layer must always be rebuildable from the canonical Markdown files.

---

## Prototype commands

Implement these CLI commands first.

### `smdb validate`

Validate all files in a folder.

Example:

```bash
smdb validate examples/notes
```

Output:
- success summary
- list of invalid files
- clear error messages

### `smdb inspect`

Parse and show normalized rows.

```bash
smdb inspect examples/notes
```

### `smdb query`

Run a SQL-like query.

```bash
smdb query examples/notes "select title, Summary from notes where author = 'Rasmus'"
```

### `smdb schema`

Print effective schema.

```bash
smdb schema examples/notes
```

---

## Validation behavior

Validation must be deterministic and strict.

### Required checks

- schema file exists
- frontmatter is valid YAML
- required frontmatter fields exist
- frontmatter field types match schema
- no unknown frontmatter keys if forbidden
- H1 exists if required
- H1 matches `frontmatter.title` if configured
- required H2 sections exist
- no unknown H2 headings if forbidden
- no duplicate H2 headings unless allowed
- section ordering rules if any
- markdown file is parseable enough to locate headings

### Error messages should include

- file path
- error type
- exact field/section
- expected vs actual
- optional line number if easy to provide

Example:

```text
001-example.md: missing required section 'Summary'
002-example.md: frontmatter field 'created' expected type date, got string 'yesterday'
003-example.md: duplicate section 'Notes'
```

---

## Query language for v1

Do **not** implement full SQL. Keep it small and reliable.

Support only:

```sql
SELECT column_list
FROM table_name
WHERE simple_predicates
ORDER BY column [ASC|DESC]
LIMIT n
```

### Supported predicates

- `=`
- `!=`
- `LIKE`
- `IN`
- `AND`
- `OR` (optional in v1, okay to skip initially)
- `IS NULL`
- `IS NOT NULL`

### Supported projections

- named columns from frontmatter
- named columns from H2 sections
- `path`

### Example valid queries

```sql
select title, author from notes
select title, Summary from notes where author = 'Rasmus'
select title from notes where status = 'draft' order by created desc
select title from notes where tags like '%db%'
select path, Decision from notes where Decision is not null
```

### Explicitly reject for v1

- joins
- group by
- aggregates
- subqueries
- inserts/updates/deletes
- aliases if they complicate parser
- expressions beyond very small support

---

## Query execution model

For v1, a scan-based engine is fine.

### Execution flow

1. Load schema
2. Find all Markdown files in the folder
3. Parse and validate each file
4. Convert each valid file to normalized row
5. Execute query over in-memory rows
6. Render results as table or JSON

This is acceptable for prototype scale.

### Later optimization

- cache parsed rows by file hash or mtime
- keep heading offsets
- keep precomputed column dictionary
- optionally keep a derived text index

But the first version should work with full scans.

---

## Parsing strategy

Avoid overengineering Markdown parsing early.

### Recommended v1 parser approach

- Parse frontmatter first
- Strip frontmatter region
- Read remaining Markdown line by line
- Detect:
  - first H1
  - H2 headings
  - content under each H2 until next H2 or EOF

This simple parser is probably enough for v1.

You do **not** need a perfect Markdown AST unless:
- heading parsing becomes unreliable
- code fences containing heading-like text cause problems
- you later want richer structured blocks

### Important rule

Preserve section body content as raw Markdown text.  
Do not try to normalize away Markdown formatting in v1.

---

## Schema typing for v1

Keep types intentionally small.

### Frontmatter types

- `string`
- `int`
- `float`
- `bool`
- `date`
- `string[]`

### Section types

For v1:
- `markdown`
- `text`

In practice both may initially be stored as strings, but preserve the distinction in the schema.

---

## H1 handling

Recommended rule for v1:

- H1 is required
- H1 text must equal `frontmatter.title`

Why:
- keeps file readable
- gives LLMs a visible title
- avoids ambiguity

Store the H1 value as `h1` internally if useful, but treat `title` as the canonical query column.

---

## Unknown and duplicate sections

Prototype should default to strictness:

- unknown H2 -> reject
- duplicate H2 -> reject

This keeps the relational model clean.

Later, you can add:
- repeatable sections
- arrays of sections
- nested subsections
- section groups

But do not start there.

---

## Suggested implementation order

### Milestone 1: Schema + parser + validator

Deliverables:
- load schema
- parse one file
- validate one file
- validate folder
- CLI `validate`

Acceptance criteria:
- valid example files pass
- bad files fail with specific errors

### Milestone 2: Normalized row model

Deliverables:
- convert valid file to row dict/object
- `inspect` command
- JSON output

Acceptance criteria:
- frontmatter and sections appear as queryable columns
- path included as implicit column

### Milestone 3: Small query engine

Deliverables:
- parse small SQL subset
- execute select/from/where/order/limit
- render results as table

Acceptance criteria:
- example queries work on example folder
- invalid SQL fails clearly

### Milestone 4: Derived cache

Deliverables:
- skip reparsing unchanged files
- rebuild cache safely
- cache clearly marked non-canonical

Acceptance criteria:
- deleting cache does not lose data
- results remain identical with and without cache

### Milestone 5: LLM-safe editing hooks

Deliverables:
- section replacement helpers
- file formatter
- deterministic writeback

Acceptance criteria:
- replacing one section does not damage other content
- rewritten file remains schema-valid

---

## LLM-specific design goals

This prototype should be intentionally friendly to LLM workflows.

### Why this format works well for LLMs

- frontmatter is explicit metadata
- sections are semantically named
- Markdown is familiar to models
- validation catches structural drift
- diffs stay readable in Git

### Important LLM-friendly features to add

- deterministic formatting
- stable heading order
- section-level patch operations
- meaningful validation errors
- machine-readable schema output

### Strong recommendation

Do not make LLMs rewrite full files unless necessary.

Instead expose helpers like:

- replace section `Summary`
- update frontmatter field `status`
- create missing optional section `Decision`

These operations are much safer and easier to validate.

---

## Write-path design for prototype

Read-only querying is enough for the first milestone, but plan the write path now.

### Write operations for later

- insert row from structured input
- update frontmatter field
- replace section body
- rename section if schema allows
- delete optional section

### Important principle

All writes should round-trip back to valid Markdown files.

That means:
- preserve frontmatter ordering or normalize it deterministically
- preserve section ordering according to schema
- preserve Markdown body content verbatim where possible

---

## File identity and primary key

For v1, use:

- implicit primary key = relative file path

Reason:
- simple
- stable enough for prototype
- avoids duplicate user-defined IDs initially

Later you may support:

- explicit `id` in frontmatter
- file rename tracking
- content-derived IDs

---

## Testing plan

Write tests early. This project is all about determinism.

### Unit tests

- frontmatter parsing
- H1 extraction
- H2 section extraction
- duplicate heading detection
- type validation
- unknown field rejection
- query predicate evaluation

### Integration tests

- validate full example folder
- run query against example folder
- compare output snapshots

### Golden tests

Use a few fixed Markdown files and expected normalized row JSON outputs.

This will make it easy to catch regression in parsing behavior.

---

## Example fixture set

Create these example files:

### Valid
- simple valid note
- valid note missing optional section
- valid note with tags array

### Invalid
- missing frontmatter field
- wrong type for date
- missing H1
- H1 mismatch with title
- missing required H2
- duplicate H2
- unknown H2
- malformed YAML

---

## Performance assumptions

For prototype scale, full scans are acceptable.

Target:
- tens to low thousands of files
- single-user or local-team use
- correctness over speed

This prototype is about proving the model, not matching SQLite performance.

---

## Non-goals

Be explicit about what this system is **not** trying to be yet:

- not a replacement for SQLite/Postgres on performance
- not a distributed database
- not an eventually-consistent document store
- not a full Markdown knowledge graph
- not a perfect Markdown semantic parser

It is a **strict document-row database prototype**.

---

## Future extensions after prototype

Only after the prototype works:

- joins across folders/tables
- explicit foreign keys
- FTS search
- materialized indexes
- repeatable section arrays
- nested subsections
- richer types (`json`, `enum[]`, etc.)
- transactions/journaling
- editor integration
- MCP/tool interface for LLM agents
- Git-aware change tracking
- schema migrations

---

## Suggested README framing

Use language like this:

> Strict Markdown DB is a prototype relational document store where Markdown files with frontmatter are the canonical rows.  
> Frontmatter defines metadata columns, required headings define content columns, and a strict schema makes the files queryable by a SQL-like engine while remaining readable by humans and LLMs.

---

## Codex task framing

Ask Codex to build the prototype in phases, not all at once.

### Recommended prompt structure

1. Build schema loader and validator
2. Build Markdown row parser
3. Build normalized row output
4. Add CLI commands
5. Add SQL-like query support
6. Add tests
7. Add derived cache only after correctness

Also explicitly tell Codex:

- keep canonical storage as Markdown files
- do not introduce SQLite as source of truth
- derived cache is allowed only if disposable
- prefer simple deterministic code over clever abstractions
- prioritize validation quality and tests

---

## Concrete acceptance criteria

The prototype is successful if all of the following are true:

1. A folder with a schema file and valid Markdown rows can be loaded.
2. Invalid files are rejected with clear messages.
3. Frontmatter fields are queryable as columns.
4. H2 sections are queryable as columns.
5. Queries like `select title, Summary from notes where status = 'draft'` work.
6. Deleting any cache does not destroy data or change results.
7. Example files remain readable and editable by humans.
8. An LLM can safely edit a section and the validator can confirm correctness.

---

## First implementation task for Codex

Start here:

1. create project skeleton
2. implement schema loader
3. implement file parser for frontmatter + H1 + H2
4. implement strict validator
5. add `smdb validate`
6. add tests for valid and invalid fixtures

Do **not** start with optimization or full SQL.

---

## Stretch goal after MVP

After read-only querying works, add a safe section update command:

```bash
smdb set-section examples/notes 001-example.md Summary --from-file new_summary.md
```

This is likely one of the most useful LLM-facing operations.

---

## Final guidance

Optimize for these properties in order:

1. correctness
2. determinism
3. readability
4. debuggability
5. LLM-friendliness
6. performance

The prototype should prove one idea clearly:

> strict Markdown documents can function as relational rows when schema, validation, and querying are designed together.

