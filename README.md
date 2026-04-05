# MDQL

A database where every entry is a markdown file and every change is a readable diff.

Think Obsidian, but for LLMs and pipelines. MDQL turns folders of markdown files into a schema-validated, queryable database. Frontmatter fields are metadata columns. H2 sections are content columns. The files are the database — there's nothing else. Every file reads like a normal markdown document, but you get full SQL: SELECT, INSERT, UPDATE, DELETE, JOINs, ORDER BY, aggregation.

**Version controlled by default.** Your database lives in git. Every insert, update, and migration is a readable diff. Branching, merging, and rollback come free.

**Use it how you want.** Run SQL in the interactive REPL. Use it as a Python ORM. Script it from the CLI. Or just `cat`, `grep`, and `awk` the files directly — they're plain markdown, always.

**Flexible schema.** Fields can be required or optional, typed or freeform. Sections can be strictly enforced or left open. You choose how tight the guardrails are per table.

**Relational.** Define foreign keys between tables and JOIN across them. A backtests table can reference a strategies table, and you query across both with standard SQL joins.

**Built for LLMs.** AI agents can read and write the data natively. No serialization layer, no ORM translation, no API calls. The schema, the data, and the queries are all human-readable text that fits naturally into any context window.

### Why MDQL

- **Zero infrastructure.** No server, no Docker, no connection strings. `git clone` and you have the database. `rm -rf` and it's gone.
- **Data review via pull requests.** Data changes go through the same PR review process as code. A reviewer reads the diff of an INSERT the way they read a code change. CI can validate schema compliance before merge.
- **Branch-level isolation.** An agent works on a feature branch, inserts and updates entries freely, and the main database is untouched until merge. Multiple agents work in parallel without coordination.
- **No serialization boundary.** Most databases require translating between the storage format and what humans or LLMs actually read. Here the storage format IS the readable format. An LLM sees a well-structured markdown document, not a JSON blob or SQL dump.
- **Graceful degradation.** If you stop using MDQL tomorrow, you still have a folder of perfectly valid markdown files. No proprietary format to export from. The data outlives the tool.
- **Section-level content columns.** Unlike key-value stores, long-form structured prose — a hypothesis, a methodology, kill criteria — is a first-class queryable column. `SELECT Hypothesis FROM strategies WHERE status = 'LIVE'`.
- **Every unix tool still works.** `grep -r "funding" strategies/` works. `wc -l strategies/*.md` works. `diff` works. MDQL adds structure on top of plain text; it doesn't replace it.
- **Self-documenting schemas.** The schema file is a markdown document. Its body explains the fields, conventions, and rationale. An LLM reading `_mdql.md` gets both the machine-readable schema and the human context for why fields exist.
- **Schema migrations are diffs.** `ALTER TABLE RENAME FIELD` rewrites every file. The migration shows up as a git diff — you can review what changed in every entry, not just trust a migration script ran correctly.
- **Audit trail for free.** `git blame strategies/funding-rate-fade.md` tells you who changed what and when. `git log --oneline strategies/` is a changelog. No separate audit logging needed.
- **Scales down to one file.** A table with 3 entries is 3 files and a schema. No minimum viable size. Useful from day one, not just at scale.
- **LLM context efficiency.** A single entry is a self-contained markdown file that fits in any context window. No need to reconstruct context from normalized tables — the document IS the context.

```
my-project/
  _mdql.md                    # type: database — config + foreign keys
  strategies/
    _mdql.md                  # type: schema — table schema + docs
    altcoin-btc-lag-trade.md
    funding-rate-fade.md
    ...
  backtests/
    _mdql.md                  # type: schema
    bt-funding-rate-fade-binance.md
    ...
  src/                        # no _mdql.md — invisible to MDQL
  docs/                       # no _mdql.md — invisible to MDQL
```

A `_mdql.md` file marks a directory as part of an MDQL database. The `type` field in frontmatter determines what it is — `database` at the root, `schema` in each table folder. Directories without `_mdql.md` are ignored, so MDQL coexists with any project structure.

## Quick start

```bash
uv sync
uv run mdql validate examples/strategies/
# All 159 files valid in table 'strategies'

uv run mdql query examples/strategies/ \
  "SELECT title, composite FROM strategies ORDER BY composite DESC LIMIT 5"
```

```
title                                                                 composite
------------------------------------------------------------------  -----------
Perp-Spot Basis Convergence                                                3024
Funding Rate Fade                                                          2592
Hyperliquid OI-Weighted Funding Imbalance Across Correlated Assets         2520
Hyperliquid Funding Settlement Anticipation Drift                          2400
Hyperliquid Mark-Index Divergence Forced Convergence                       2352
```

## How it works

**One folder = one table. One markdown file = one row.**

A row file looks like this:

```markdown
---
title: "Funding Rate Fade"
status: HYPOTHESIS
mechanism: 6
categories:
  - funding-rates
created: "2026-04-04"
modified: "2026-04-05"
---

## Hypothesis

When the perpetual funding rate exceeds 0.05%...

## Entry Rules

Enter on the opposite side of the funding imbalance...
```

- YAML frontmatter fields become metadata columns (`title`, `status`, `mechanism`, ...)
- H2 sections become content columns (`Hypothesis`, `Entry Rules`, ...)
- The `path` (filename) is the implicit primary key
- `created` and `modified` are reserved timestamp fields, auto-managed by `mdql stamp`
- All columns are queryable with SQL-like syntax

## `_mdql.md` files

Every MDQL-managed directory has a `_mdql.md` file. Like `__init__.py` in Python, it marks the directory as part of the database. The `type` field in frontmatter says what kind:

### Table schema (`type: schema`)

```markdown
---
type: schema
table: strategies
primary_key: path

frontmatter:
  title:
    type: string
    required: true
  mechanism:
    type: int
    required: true
  categories:
    type: string[]
    required: true

h1:
  required: false

sections: {}

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: false
  reject_duplicate_sections: true
---

# strategies

Documentation about this table goes here.
```

Supported types: `string`, `int`, `float`, `bool`, `date`, `string[]`

### Database config (`type: database`)

```markdown
---
type: database
name: zunid

foreign_keys:
  - from: backtests.strategy
    to: strategies.path
---

# zunid

Trading strategy research database.
```

The markdown body in both cases is documentation — ignored by the engine, valuable for humans and LLMs.

## Python API

```python
from mdql import Database, Table

db = Database("examples/")
strategies = db.table("strategies")

# INSERT — create a new row, fail if exists
strategies.insert({
    "title": "My New Strategy",
    "status": "HYPOTHESIS",
    "mechanism": 5,
    "implementation": 4,
    "safety": 7,
    "frequency": 3,
    "composite": 420,
    "categories": ["exchange-structure"],
    "pipeline_stage": "Pre-backtest (step 2 of 9)",
})
# → my-new-strategy.md (filename derived from title)
#   created/modified timestamps set automatically
#   required sections scaffolded as empty ## headings
#   validated against schema before writing

# INSERT with pre-formatted body (e.g. from Claude output)
strategies.insert(
    {"title": "Another Strategy", "status": "HYPOTHESIS", ...},
    body=raw_markdown,  # verbatim after frontmatter
)

# INSERT ... ON CONFLICT REPLACE — overwrite, preserve created timestamp
strategies.insert(
    {"title": "Revised Strategy", "status": "BACKTESTING", ...},
    filename="my-new-strategy",
    replace=True,
)

# UPDATE — partial merge, only change what you pass
strategies.update("my-new-strategy.md", {"status": "KILLED", "kill_reason": "No edge"})
# existing frontmatter and body preserved, only status/kill_reason changed

# UPDATE with new body
strategies.update("my-new-strategy.md", {}, body=new_markdown)

# Read
rows, errors = strategies.load()
validation_errors = strategies.validate()
```

All writes are validated against the schema and rolled back on failure. The `created` timestamp is always preserved on `replace` and `update`; `modified` is always bumped.

## Commands

### `mdql create <folder> --set key=value`

Create a new row file. Field types are coerced from the schema (e.g. `--set mechanism=5` becomes int).

```bash
uv run mdql create examples/strategies/ \
  -s 'title=My New Strategy' \
  -s 'status=HYPOTHESIS' \
  -s 'mechanism=5' \
  -s 'implementation=4' \
  -s 'safety=7' \
  -s 'frequency=3' \
  -s 'composite=420' \
  -s 'categories=exchange-structure' \
  -s 'pipeline_stage=Pre-backtest (step 2 of 9)'
# Created my-new-strategy.md
```

For `string[]` fields, use comma-separated values: `-s 'categories=funding-rates,defi'`

### SQL write operations

The `query` command supports full CRUD — not just SELECT:

```bash
# INSERT
uv run mdql query examples/strategies/ \
  "INSERT INTO strategies (title, status, mechanism, implementation, safety, frequency, composite, categories, pipeline_stage)
   VALUES ('New Strategy', 'HYPOTHESIS', 5, 4, 7, 3, 420, 'exchange-structure', 'Pre-backtest')"
# INSERT 1 (new-strategy.md)

# UPDATE — change specific fields, body and other fields preserved
uv run mdql query examples/strategies/ \
  "UPDATE strategies SET status = 'KILLED', kill_reason = 'No edge' WHERE path = 'new-strategy.md'"
# UPDATE 1

# DELETE
uv run mdql query examples/strategies/ \
  "DELETE FROM strategies WHERE path = 'new-strategy.md'"
# DELETE 1
```

All write operations go through schema validation. For `string[]` columns, pass comma-separated values in a single string: `'funding-rates,defi'`.

### ALTER TABLE — field migrations

Rename, drop, or merge fields across all files in a table. Works for both frontmatter fields and sections. The schema `_mdql.md` is updated automatically.

```bash
# Rename a section across all files
uv run mdql query examples/strategies/ \
  "ALTER TABLE strategies RENAME FIELD 'Summary' TO 'Overview'"
# ALTER TABLE — renamed 'Summary' to 'Overview' in 42 files

# Rename a frontmatter field
uv run mdql query examples/strategies/ \
  "ALTER TABLE strategies RENAME FIELD 'status' TO 'state'"

# Drop a field (section or frontmatter)
uv run mdql query examples/strategies/ \
  "ALTER TABLE strategies DROP FIELD 'Details'"

# Merge multiple sections into one
uv run mdql query examples/strategies/ \
  "ALTER TABLE strategies MERGE FIELDS 'Entry Rules', 'Exit Rules' INTO 'Trading Rules'"
```

Field names can be single-quoted (`'Name'`), backtick-quoted (`` `Name With Spaces` ``), or bare identifiers.

The same operations are available via the Python API:

```python
table = Table("examples/strategies/")

table.rename_field("Summary", "Overview")     # section or frontmatter
table.drop_field("Details")                   # section or frontmatter
table.merge_fields(["Entry Rules", "Exit Rules"], into="Trading Rules")  # sections only
```

### `mdql validate <folder>`

Validate all markdown files against the schema.

```bash
uv run mdql validate examples/strategies/
# All 159 files valid in table 'strategies'
```

Invalid files get clear error messages:

```
missing-field.md: Missing required frontmatter field 'count'
wrong-type-date.md: Field 'created' expected date, got string 'yesterday'
duplicate-section.md: Duplicate section 'Body' (appears 2 times)
```

### `mdql query <folder> "<sql>"`

Run SQL-like queries. Supports `SELECT`, `FROM`, `WHERE`, `ORDER BY`, `LIMIT`, and `JOIN`.

```bash
# Filter and sort
uv run mdql query examples/strategies/ \
  "SELECT title FROM strategies WHERE mechanism > 5 ORDER BY composite DESC LIMIT 5"

# Query section content
uv run mdql query examples/strategies/ \
  "SELECT path, Hypothesis FROM strategies WHERE Hypothesis IS NOT NULL LIMIT 3"

# Category search (LIKE works on arrays)
uv run mdql query examples/strategies/ \
  "SELECT title FROM strategies WHERE categories LIKE '%defi%'"

# Output as JSON
uv run mdql query examples/strategies/ \
  "SELECT title, composite FROM strategies LIMIT 3" --format json
```

Supported statements: `SELECT`, `INSERT INTO`, `UPDATE SET`, `DELETE FROM`, `ALTER TABLE`

Supported WHERE operators: `=`, `!=`, `<`, `>`, `<=`, `>=`, `LIKE`, `IN`, `IS NULL`, `IS NOT NULL`, `AND`, `OR`

Column names with spaces use backticks: `` SELECT `Structural Mechanism` FROM strategies ``

### `mdql query <db-folder> "<join-sql>"`

Point at the database directory (parent of table folders) for cross-table queries:

```bash
uv run mdql query examples/ \
  "SELECT s.title, b.sharpe, b.status
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path"
```

```
s.title                                          b.sharpe  b.status
---------------------------------------------  ----------  ------------
Strategy Specification: Altcoin BTC Lag Trade         0.6  inconclusive
Funding Rate Fade                                     1.4  pass
Perp-Spot Basis Convergence                          -0.8  fail
```

### `mdql inspect <folder>`

Show normalized rows.

```bash
uv run mdql inspect examples/strategies/ -f funding-rate-fade.md --format json
```

### `mdql stamp <folder>`

Add or update `created` and `modified` timestamps in all data files.

```bash
uv run mdql stamp examples/strategies/
# Stamped 159 files: 0 created set, 159 modified updated
```

- `created` is set to today's date if missing, never overwritten
- `modified` is always updated to today's date
- Both are ISO date strings (`"YYYY-MM-DD"`) in frontmatter
- These fields are reserved globally — schemas don't need to declare them, and they're never rejected as unknown fields

### `mdql schema <folder>`

Print the effective schema. Works on a single table or the whole database:

```bash
uv run mdql schema examples/
```

## Pandas integration

MDQL has optional pandas support. Install with:

```bash
uv pip install mdql[pandas]
```

### One-liner

```python
from mdql.pandas import load_dataframe

df = load_dataframe("examples/strategies/")
```

### Two-step (when you already have rows)

```python
from mdql.loader import load_table
from mdql.pandas import to_dataframe

schema, rows, errors = load_table("examples/strategies/")
df = to_dataframe(rows, schema)
```

Schema types map to pandas dtypes automatically:

| MDQL type  | pandas dtype       |
|------------|--------------------|
| `string`   | `string`           |
| `int`      | `Int64` (nullable) |
| `float`    | `Float64` (nullable) |
| `bool`     | `boolean` (nullable) |
| `date`     | `datetime64[ns]`   |
| `string[]` | Python lists       |

Validation errors are handled via the `errors` parameter: `"warn"` (default), `"raise"`, or `"ignore"`.

## ACID compliance

All write operations are process-safe. MDQL provides three layers of protection:

**Atomic writes.** Every file write goes through a temp-file-then-rename path. If the process crashes mid-write, the original file is untouched.

**Table locking.** Write operations acquire an exclusive `fcntl.flock` per table. Two processes writing to the same table will serialize rather than corrupt each other's files.

**Write-ahead journal.** Multi-file operations (`ALTER TABLE`, batch `UPDATE`/`DELETE`, `stamp`) write a journal before making changes. If the process crashes mid-operation, the next `Table()` construction detects the journal and rolls back all partial changes automatically.

```python
# This is safe even if the process is killed mid-way:
table.rename_field("Summary", "Overview")  # touches 160 files + schema
# On crash: next Table("strategies/") auto-recovers from journal
```

## Design principles

1. **Markdown files are the source of truth.** No opaque database files. Any index or cache is derived and disposable.
2. **Strict validation.** Invalid files are rejected with clear errors. No silent data corruption.
3. **Config is markdown too.** `_mdql.md` files use the same format they enforce — YAML frontmatter for structure, markdown body for documentation.
4. **Coexists with any project.** Only directories with `_mdql.md` are part of the database. Everything else is invisible.
5. **LLM-friendly.** Deterministic formatting, section-level granularity, meaningful error messages. LLMs can read, edit, and query these files natively.
6. **Git-friendly.** Every change is a readable diff. No binary blobs.

## Running tests

```bash
uv run pytest
```

251 tests covering parser, validator, query engine, SQL CRUD, field migrations, ACID transactions, CLI, API, timestamps, pandas integration, and integration with real data.

## Project structure

```
src/mdql/
  parser.py         # markdown -> ParsedFile (frontmatter, H1, H2 sections)
  schema.py         # load _mdql.md (type: schema) -> Schema
  validator.py      # validate ParsedFile against Schema
  model.py          # ParsedFile -> Row dict
  loader.py         # orchestrate: folder -> rows
  database.py       # load _mdql.md (type: database) -> DatabaseConfig
  query_parser.py   # SQL subset -> Query AST (recursive descent)
  query_engine.py   # execute queries over in-memory rows
  projector.py      # format output (table/json/csv)
  pandas.py         # optional pandas integration (load_dataframe, to_dataframe)
  stamp.py          # auto-manage created/modified timestamps
  migrate.py        # field migration (rename, drop, merge) across files
  txn.py            # ACID primitives (atomic write, table lock, journal)
  api.py            # object-oriented API (Table, Database, insert)
  cli.py            # typer CLI
```

## License

TBD
