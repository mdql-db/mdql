# MDQL

A database where every entry is a markdown file and every change is a readable diff.

MDQL turns folders of markdown files into a schema-validated, queryable database. Frontmatter fields are metadata columns. H2 sections are content columns. The files are the database — there is nothing else. Every file reads like a normal markdown document, but you get full SQL: SELECT, INSERT, UPDATE, DELETE, JOINs across multiple tables, ORDER BY, aggregation, computed expressions, and CASE WHEN.

Your database lives in git. Every insert, update, and migration is a readable diff. Branching, merging, and rollback come free.

## Install

```bash
cargo install mdql          # from source via Cargo
brew install mdql-db/tap/mdql  # macOS / Linux via Homebrew
pip install mdql             # Python bindings
```

## Quick start

```bash
mdql validate examples/strategies/
# All 100 files valid in table 'strategies'

mdql query examples/strategies/ \
  "SELECT title, composite FROM strategies ORDER BY composite DESC LIMIT 5"
```

```
title                                                                composite
-------------------------------------------------------------------  ---------
Bridge Inflow to Destination Chain → DEX Liquidity Pressure                500
DeFi Protocol TVL Step-Change → Governance Token Repricing Lag             500
Lending Protocol Daily Interest Accrual Liquidation Threshold Creep        500
USDC Circle Business-Day Redemption Queue — Weekend Premium Decay          490
Cascading Liquidation Chain — Second-Order Collateral Asset Short          480
```

## Why MDQL

- **Zero infrastructure.** No server, no Docker, no connection strings. `git clone` and you have the database. `rm -rf` and it's gone.
- **Data review via pull requests.** Data changes go through the same PR review process as code. A reviewer reads the diff of an INSERT the way they read a code change.
- **Branch-level isolation.** An agent works on a feature branch, inserts and updates entries freely, and the main database is untouched until merge. Multiple agents work in parallel without coordination.
- **No serialization boundary.** The storage format is the readable format. An LLM sees a well-structured markdown document, not a JSON blob or SQL dump.
- **Graceful degradation.** If you stop using MDQL tomorrow, you still have a folder of valid markdown files. No proprietary format to export from.
- **Section-level content columns.** Long-form structured prose — a hypothesis, a methodology, kill criteria — is a first-class queryable column. `SELECT Hypothesis FROM strategies WHERE status = 'LIVE'`.
- **Every unix tool still works.** `grep -r "funding" strategies/` works. `wc -l strategies/*.md` works. `diff` works.
- **Self-documenting schemas.** The schema file is a markdown document. Its body explains the fields, conventions, and rationale. An LLM reading `_mdql.md` gets both the machine-readable schema and the human context for why fields exist.
- **Schema migrations are diffs.** `ALTER TABLE RENAME FIELD` rewrites every file. The migration shows up as a git diff.
- **Audit trail for free.** `git blame strategies/bad-debt-socialization-event-token-short.md` tells you who changed what and when.

## Directory structure

```
my-project/
  _mdql.md                    # type: database — config + foreign keys
  strategies/
    _mdql.md                  # type: schema — table schema + docs
    bad-debt-socialization-event-token-short.md
    aave-utilization-kink-rate-spike-borrow-unwind-short.md
    ...
  backtests/
    _mdql.md                  # type: schema
    bt-bad-debt-socialization-binance.md
    ...
  src/                        # no _mdql.md — invisible to MDQL
  docs/                       # no _mdql.md — invisible to MDQL
```

A `_mdql.md` file marks a directory as part of an MDQL database. The `type` field in frontmatter determines what it is — `database` at the root, `schema` in each table folder. Directories without `_mdql.md` are ignored, so MDQL coexists with any project structure.

## How it works

One folder = one table. One markdown file = one row.

A row file looks like this:

```markdown
---
title: "Bad Debt Socialization Event — Token Short"
status: HYPOTHESIS
mechanism: 7
categories:
  - defi-protocol
  - lending
created: "2026-04-03"
modified: "2026-04-05"
---

## Hypothesis

When an on-chain lending protocol accumulates bad debt that exceeds
its reserve buffer, the smart contract mints governance tokens...

## Structural Mechanism

The protocol's shortfall module triggers an auction...
```

- YAML frontmatter fields are metadata columns (`title`, `status`, `mechanism`, ...)
- H2 sections are content columns (`Hypothesis`, `Structural Mechanism`, ...)
- The `path` (filename) is the implicit primary key
- `created` and `modified` are reserved timestamp fields, auto-managed by `mdql stamp`
- All columns are queryable with SQL

## `_mdql.md` files

Every MDQL-managed directory has a `_mdql.md` file. The `type` field in frontmatter says what kind.

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

Supported types: `string`, `int`, `float`, `bool`, `date`, `datetime`, `string[]`

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

The markdown body in both cases is documentation — ignored by the engine, useful for humans and LLMs.

## Foreign key validation

Foreign keys defined in the database config are validated automatically. No setup required.

**At load time:** Every call to `load_database()` checks all FK constraints. If `backtests.strategy` references a file that does not exist in `strategies.path`, the error is returned alongside the data. CLI commands (`query`, `validate`, `repl`) print FK warnings to stderr.

**In the REPL:** A filesystem watcher runs in the background. If you rename or delete a file in another terminal, the REPL detects the change within 500ms and prints any new FK violations.

**In the web UI:** Same filesystem watcher runs as a background task. FK errors are available at `GET /api/fk-errors`.

**With `mdql validate`:** When pointed at a database directory (not just a single table), reports per-table schema validation summaries followed by FK violations:

```bash
mdql validate examples/
```

```
Table 'strategies': 100 files valid
Table 'backtests': 18 files valid
Foreign key violations:
  backtests/bt-broken.md: strategy = 'nonexistent.md' not found in strategies
```

NULL FK values are not violations — a backtest with no strategy set is valid.

## Python API

```bash
pip install mdql
```

### Database and Table

```python
from mdql import Database, Table

db = Database("examples/")
strategies = db.table("strategies")
```

### SELECT with JOINs

`Database.query()` runs SQL across all tables in the database, including multi-table JOINs.

```python
rows, columns = db.query(
    "SELECT s.title, b.sharpe, b.status "
    "FROM strategies s "
    "JOIN backtests b ON b.strategy = s.path"
)
# rows: list of dicts, one per result row
# columns: list of column names
```

### Single-table queries

`Table.query()` runs a SELECT query on one table and returns structured results.

```python
rows, columns = strategies.query(
    "SELECT status, COUNT(*) AS cnt FROM strategies GROUP BY status"
)
# rows: list of dicts
# columns: list of column names

# Computed expressions and CASE WHEN
rows, columns = strategies.query(
    "SELECT title, mechanism * safety score, "
    "CASE WHEN mechanism >= 7 THEN 'high' ELSE 'low' END tier "
    "FROM strategies ORDER BY score DESC"
)

# Conditional aggregation
rows, columns = strategies.query(
    "SELECT SUM(CASE WHEN status = 'LIVE' THEN 1 ELSE 0 END) live_count, "
    "COUNT(*) total FROM strategies"
)
```

### Load rows with filtering

`Table.load()` returns all rows, optionally filtered by a dict of field values.

```python
# All rows
rows, errors = strategies.load()

# Filtered by dict — equality matching
rows, errors = strategies.load(where={"status": "LIVE"})

# Filtered by SQL WHERE string — full operator support
rows, errors = strategies.load(where="mechanism >= 7 AND status = 'HYPOTHESIS'")
rows, errors = strategies.load(where="categories LIKE '%defi%'")
```

The `where` parameter accepts a dict (equality matching) or a SQL WHERE string (supports `=`, `!=`, `<`, `>`, `<=`, `>=`, `LIKE`, `IN`, `IS NULL`, `AND`, `OR`). `errors` contains any schema validation issues found during loading.

### INSERT

```python
# Create a new row — filename derived from title
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
# Returns: Path to created file (e.g. my-new-strategy.md)
# created/modified timestamps set automatically
# required sections scaffolded as empty ## headings
# validated against schema before writing

# With pre-formatted body (e.g. from Claude output)
strategies.insert(
    {"title": "Another Strategy", "status": "HYPOTHESIS", ...},
    body=raw_markdown,  # placed verbatim after frontmatter
)

# Overwrite existing file, preserve created timestamp
strategies.insert(
    {"title": "Revised Strategy", "status": "BACKTESTING", ...},
    filename="my-new-strategy",
    replace=True,
)
```

### UPDATE

```python
# Partial merge — only the fields you pass are changed
strategies.update("my-new-strategy.md", {"status": "KILLED", "kill_reason": "No edge"})

# Update body only
strategies.update("my-new-strategy.md", {}, body=new_markdown)
```

### Bulk UPDATE

`Table.update_many()` updates the same fields across multiple files.

```python
updated_paths = strategies.update_many(
    ["file-a.md", "file-b.md", "file-c.md"],
    {"status": "KILLED"},
)
# Returns: list of paths that were updated
```

### DELETE

```python
strategies.delete("my-new-strategy.md")
```

### Schema operations

```python
table = Table("examples/strategies/")

table.rename_field("Summary", "Overview")     # section or frontmatter
table.drop_field("Details")                   # section or frontmatter
table.merge_fields(["Entry Rules", "Exit Rules"], into="Trading Rules")  # sections only
```

### Validation

```python
errors = strategies.validate()
# Returns: list of validation errors (schema + FK)
```

All writes are validated against the schema and rolled back on failure. The `created` timestamp is always preserved on `replace` and `update`; `modified` is always set to today.

## CLI commands

### `mdql query <folder> "<sql>"`

Run SQL against a table or database. Supports `SELECT`, `INSERT INTO`, `UPDATE SET`, `DELETE FROM`, `ALTER TABLE`, and `JOIN`.

```bash
# Filter and sort
mdql query examples/strategies/ \
  "SELECT title FROM strategies WHERE mechanism > 5 ORDER BY composite DESC LIMIT 5"

# Query section content
mdql query examples/strategies/ \
  "SELECT path, Hypothesis FROM strategies WHERE Hypothesis IS NOT NULL LIMIT 3"

# Category search (LIKE works on arrays)
mdql query examples/strategies/ \
  "SELECT title FROM strategies WHERE categories LIKE '%defi%'"

# Output as JSON
mdql query examples/strategies/ \
  "SELECT title, composite FROM strategies LIMIT 3" --format json
```

Supported WHERE operators: `=`, `!=`, `<`, `>`, `<=`, `>=`, `LIKE`, `IN`, `IS NULL`, `IS NOT NULL`, `AND`, `OR`

Column names with spaces use backticks: `` SELECT `Structural Mechanism` FROM strategies ``

### Computed expressions

Arithmetic expressions (`+`, `-`, `*`, `/`, `%`) work in SELECT, WHERE, and ORDER BY. Supports parentheses, unary minus, and mixed int/float coercion.

```bash
# Computed columns with aliases
mdql query examples/strategies/ \
  "SELECT title, mechanism * safety total_score FROM strategies ORDER BY total_score DESC LIMIT 5"

# Expressions in WHERE
mdql query examples/strategies/ \
  "SELECT title FROM strategies WHERE mechanism + implementation > 10"

# Parenthesized expressions
mdql query examples/strategies/ \
  "SELECT title, (mechanism + implementation) / 2 avg_score FROM strategies"
```

Integer division truncates (`7 / 2 = 3`). Division by zero returns NULL. NULL propagates through all arithmetic.

### Column aliases

Columns can be aliased with `AS` or by placing the alias directly after the expression (implicit alias). ORDER BY can reference SELECT aliases.

```bash
# Explicit alias with AS
mdql query examples/ \
  "SELECT s.title AS name, b.sharpe AS ratio FROM strategies s JOIN backtests b ON b.strategy = s.path"

# Implicit alias (no AS keyword)
mdql query examples/ \
  "SELECT s.composite comp, b.edge_vs_random edge FROM strategies s JOIN backtests b ON b.strategy = s.path ORDER BY edge DESC"
```

### CASE WHEN

CASE WHEN expressions work anywhere a value is expected — in SELECT, WHERE, ORDER BY, and inside aggregate functions.

```bash
# Categorize rows
mdql query examples/strategies/ \
  "SELECT title, CASE WHEN mechanism >= 7 THEN 'high' WHEN mechanism >= 4 THEN 'medium' ELSE 'low' END rating FROM strategies"

# Conditional aggregation
mdql query examples/strategies/ \
  "SELECT COUNT(*) total, SUM(CASE WHEN mechanism >= 7 THEN 1 ELSE 0 END) high_mechanism FROM strategies"
```

### JOINs

Point at the database directory (parent of table folders) for cross-table queries. Supports two or more tables:

```bash
# Two-table JOIN
mdql query examples/ \
  "SELECT s.title, b.sharpe, b.status
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path"

# Multi-table JOIN
mdql query my-db/ \
  "SELECT s.title, b.result, c.verdict
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path
   JOIN critiques c ON c.strategy = s.path"
```

### SQL write operations

```bash
# INSERT
mdql query examples/strategies/ \
  "INSERT INTO strategies (title, status, mechanism, implementation, safety, frequency, composite, categories, pipeline_stage)
   VALUES ('New Strategy', 'HYPOTHESIS', 5, 4, 7, 3, 420, 'exchange-structure', 'Pre-backtest')"

# UPDATE
mdql query examples/strategies/ \
  "UPDATE strategies SET status = 'KILLED', kill_reason = 'No edge' WHERE path = 'new-strategy.md'"

# DELETE
mdql query examples/strategies/ \
  "DELETE FROM strategies WHERE path = 'new-strategy.md'"
```

For `string[]` columns, pass comma-separated values in a single string: `'funding-rates,defi'`.

### ALTER TABLE — field migrations

Rename, drop, or merge fields across all files in a table. Works for both frontmatter fields and sections. The schema `_mdql.md` is updated automatically.

```bash
mdql query examples/strategies/ \
  "ALTER TABLE strategies RENAME FIELD 'Summary' TO 'Overview'"
# ALTER TABLE — renamed 'Summary' to 'Overview' in 42 files

mdql query examples/strategies/ \
  "ALTER TABLE strategies DROP FIELD 'Details'"

mdql query examples/strategies/ \
  "ALTER TABLE strategies MERGE FIELDS 'Entry Rules', 'Exit Rules' INTO 'Trading Rules'"
```

Field names can be single-quoted (`'Name'`), backtick-quoted (`` `Name With Spaces` ``), or bare identifiers.

### `mdql rename <db-folder> <table> <old-name> <new-name>`

Rename a file within a table. Automatically updates all foreign key references in other tables that point to the old filename.

```bash
mdql rename examples/ strategies bad-debt-socialization-event-token-short.md bad-debt-token-short.md
# Renamed strategies/bad-debt-socialization-event-token-short.md → bad-debt-token-short.md
# Updated 3 references in backtests
```

### `mdql create <folder> --set key=value`

Create a new row file. Field types are coerced from the schema (e.g. `--set mechanism=5` becomes int).

```bash
mdql create examples/strategies/ \
  -s 'title=My New Strategy' \
  -s 'status=HYPOTHESIS' \
  -s 'mechanism=5' \
  -s 'implementation=4' \
  -s 'safety=7' \
  -s 'frequency=3' \
  -s 'composite=420' \
  -s 'categories=exchange-structure' \
  -s 'pipeline_stage=Pre-backtest (step 2 of 9)'
```

For `string[]` fields, use comma-separated values: `-s 'categories=funding-rates,defi'`

### `mdql validate <folder>`

Validate all markdown files against the schema. Works on a single table or a database directory.

```bash
mdql validate examples/strategies/
# All 100 files valid in table 'strategies'
```

Invalid files get clear error messages:

```
missing-field.md: Missing required frontmatter field 'count'
wrong-type-date.md: Field 'created' expected datetime (ISO 8601), got string 'yesterday'
duplicate-section.md: Duplicate section 'Body' (appears 2 times)
```

When pointed at a database directory, also reports foreign key violations (see [Foreign key validation](#foreign-key-validation)).

### `mdql inspect <folder>`

Show normalized rows.

```bash
mdql inspect examples/strategies/ -f bad-debt-socialization-event-token-short.md --format json
```

### `mdql stamp <folder>`

Add or update `created` and `modified` timestamps in all data files.

```bash
mdql stamp examples/strategies/
# Stamped 100 files: 0 created set, 100 modified updated
```

- `created` is set to the current ISO 8601 timestamp if missing, never overwritten
- `modified` is always updated to the current ISO 8601 timestamp
- Both are ISO datetime strings (`"YYYY-MM-DDTHH:MM:SS"`) in frontmatter
- These fields are reserved — schemas don't need to declare them, and they are never rejected as unknown fields

### `mdql schema <folder>`

Print the effective schema. Works on a single table or the whole database.

```bash
mdql schema examples/
```

### `mdql repl <folder>`

Open an interactive REPL for running queries. Supports tab completion for table names, column names, and SQL keywords.

```bash
mdql repl examples/
```

When pointed at a database directory, runs a background filesystem watcher that prints FK violations to stderr if files change on disk while the REPL is open.

### `mdql client <folder>`

Open a browser-based UI for running queries. Starts a local web server with a query editor.

```bash
mdql client examples/
```

The web server exposes a REST API:
- `POST /api/query` — execute SQL
- `GET /api/fk-errors` — current foreign key violations (updated by background watcher)

## Multi-agent setup

MDQL is a single-writer, filesystem-based database. When multiple agents or processes need to read and write the same data, point them all at the same directory. MDQL's `flock` locking serializes writes automatically.

For multi-agent setups, keep the database in its own directory (and optionally its own git repo for audit trail), separate from application code:

```
~/repos/
  my-project/         # application code — branched freely
  my-project-db/      # MDQL database — shared by all agents
    _mdql.md
    strategies/
    orders/
```

### `MDQL_DATABASE_PATH`

Set the `MDQL_DATABASE_PATH` environment variable so agents and CLI commands find the database without hardcoding paths.

```bash
export MDQL_DATABASE_PATH=~/repos/my-project-db

# CLI commands fall back to this when no folder is given
mdql validate
mdql repl
```

```python
from mdql import Database

# Reads MDQL_DATABASE_PATH when no path is given
db = Database()
```

An explicit path always takes precedence: `Database("/other/path")` and `mdql validate /other/path` ignore the env var.

## Pandas integration

```bash
pip install mdql[pandas]
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

Schema types map to pandas dtypes:

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

All write operations are process-safe. Three layers of protection:

**Atomic writes.** Every file write goes through a temp-file-then-rename path. If the process crashes mid-write, the original file is untouched.

**Table locking.** Write operations acquire an exclusive `fcntl.flock` per table. Two processes writing to the same table serialize rather than corrupt each other's files.

**Write-ahead journal.** Multi-file operations (`ALTER TABLE`, batch `UPDATE`/`DELETE`, `stamp`) write a journal before making changes. If the process crashes mid-operation, the next `Table()` construction detects the journal and rolls back all partial changes automatically.

```python
# Safe even if the process is killed mid-way:
table.rename_field("Summary", "Overview")  # touches 100 files + schema
# On crash: next Table("strategies/") auto-recovers from journal
```

## Running tests

```bash
# Rust tests
cargo test

# Python tests (requires maturin develop first)
pytest
```

## Project structure

```
crates/
  mdql-core/        # core library: parser, schema, validator, query engine,
                     # indexes, caching, full-text search, ACID transactions,
                     # FK validation, filesystem watcher
  mdql/             # CLI binary: validate, query, create, inspect, schema,
                     # stamp, rename, repl (with autocomplete), client (web UI)
  mdql-web/         # browser UI: axum REST server + embedded SPA
python/
  src/lib.rs        # PyO3 bindings (Rust → Python)
  mdql/             # Python wrapper package (thin layer over Rust)
tests/              # Python test suite
examples/           # example data (strategies, backtests)
```

## License

AGPL-3.0. Commercial licenses available — see [LICENSE.md](LICENSE.md).
