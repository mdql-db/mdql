# MDQL

A database where every entry is a markdown file and every change is a readable diff.

MDQL turns folders of markdown files into a schema-validated, queryable database. Frontmatter fields are metadata columns. H2 sections are content columns. The files are the database — there is nothing else. Every file reads like a normal markdown document, but you get full SQL: SELECT, INSERT, UPDATE, DELETE, JOINs across multiple tables, CTEs, subqueries, window functions, views, ORDER BY, aggregation, computed expressions, and CASE WHEN.

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
created: "2026-04-03T14:22:01"
modified: "2026-04-05T09:15:33"
---

## Hypothesis

When an on-chain lending protocol accumulates bad debt that exceeds
its reserve buffer, the smart contract mints governance tokens...

## Structural Mechanism

The protocol's shortfall module triggers an auction...
```

- **Frontmatter** = metadata columns (`title`, `status`, `mechanism`, ...) — typed, validated, queryable
- **H2 sections** = content columns (`Hypothesis`, `Structural Mechanism`, ...) — queryable long-form text
- **H1** = decorative only. Not queryable, not stored as a column. Present for human readability in editors and GitHub rendering (standard markdown convention: one H1 per document as the title)
- **Loose body** (text not under any H2) = rejected. All content must live under an `## Heading` to be queryable. This prevents silent data loss.
- The `path` (filename) is the implicit primary key
- `created` and `modified` are reserved datetime fields (ISO 8601, e.g. `"2026-04-03T14:22:01"`), auto-managed by `mdql stamp`
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

Supported types: `string`, `int`, `float`, `bool`, `date`, `datetime`, `string[]`, `dict`

The `dict` type stores a YAML mapping. Values can be scalars, lists, or nested dicts. Use dot-access in queries: `SELECT params.entry_days FROM strategies`.

```yaml
params:
  threshold: 0.5
  blocked_tokens:
    - ZK
    - W
  enabled: true
```

### Database config (`type: database`)

```markdown
---
type: database
name: zunid

foreign_keys:
  - from: backtests.strategy
    to: strategies.path

views:
  - name: live_strategies
    query: "SELECT * FROM strategies WHERE status = 'LIVE'"
  - name: strategy_performance
    query: "SELECT s.title, b.sharpe FROM strategies s JOIN backtests b ON b.strategy = s.path"
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

## Views

Views are named queries that act as virtual tables. Define them with standard SQL, query them like any other table.

### Creating and dropping views

```bash
# Create a view (persisted in _mdql.md)
mdql query examples/ "CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'"

# Query the view like a regular table
mdql query examples/ "SELECT title, mechanism FROM live ORDER BY mechanism DESC"

# Drop a view
mdql query examples/ "DROP VIEW live"

# Views support the full query syntax: GROUP BY, HAVING, aggregate arithmetic
mdql query examples/ "CREATE VIEW positions AS SELECT token, SUM(CASE WHEN side = 'BUY' THEN size ELSE 0 END) - SUM(CASE WHEN side = 'SELL' THEN size ELSE 0 END) as net FROM orders GROUP BY token HAVING net > 0"
```

Views require a database directory (not a single table folder). They are stored in the `views:` section of the database-level `_mdql.md` and re-executed dynamically on each query — no cached data on disk.

### Restrictions

- **Read-only.** `INSERT INTO`, `UPDATE`, and `DELETE FROM` a view return a clear error.
- **No view-to-view references.** A view query cannot reference another view.
- **Name conflicts.** A view cannot have the same name as a physical table.

### Python API

```python
db = Database("examples/")

# Create and drop views
db.execute("CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'")
db.execute("DROP VIEW live")

# Query a view (same as querying a table)
rows, columns = db.query("SELECT * FROM live")

# List view names
db.view_names  # ['live', ...]
```

### Schema display

```bash
mdql schema examples/
# ...
# Views:
#   live = SELECT * FROM strategies WHERE status = 'LIVE'
```

## CTEs (Common Table Expressions)

Use `WITH ... AS` to define temporary named result sets within a query. CTEs are materialized in order, so later CTEs can reference earlier ones.

```bash
# Single CTE
mdql query examples/ "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') SELECT * FROM live ORDER BY title"

# Multiple CTEs with JOIN
mdql query examples/ "WITH s AS (SELECT * FROM strategies WHERE status = 'LIVE'), b AS (SELECT * FROM backtests WHERE sharpe > 1.0) SELECT s.title, b.sharpe FROM s JOIN b ON b.strategy = s.path"

# Chained CTEs — second CTE references the first
mdql query examples/ "WITH good AS (SELECT * FROM backtests WHERE sharpe > 1.0), matched AS (SELECT s.title, g.sharpe FROM strategies s JOIN good g ON g.strategy = s.path) SELECT * FROM matched"

# CTE with aggregation
mdql query examples/ "WITH counts AS (SELECT status, COUNT(*) AS cnt FROM strategies GROUP BY status) SELECT * FROM counts WHERE cnt > 1"
```

```python
# Python API
rows, columns = db.query(
    "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') "
    "SELECT * FROM live ORDER BY title"
)
```

CTEs require a database directory. A CTE can shadow a physical table name — the CTE version takes precedence within the query.

## Subqueries

Subqueries work in WHERE clauses (both IN and scalar comparisons), SELECT expressions, and FROM position (derived tables).

```bash
# WHERE IN subquery — filter rows based on another query's results
mdql query examples/ "SELECT title FROM strategies WHERE status IN (SELECT status FROM strategies WHERE composite > 400)"

# WHERE scalar subquery — compare against a single value
mdql query examples/ "SELECT title, sharpe FROM backtests WHERE sharpe > (SELECT AVG(sharpe) FROM backtests)"

# FROM subquery (derived table) — use a query result as the source table
mdql query examples/ "SELECT title FROM (SELECT title, composite FROM strategies WHERE status = 'LIVE') ORDER BY composite DESC LIMIT 5"
```

```python
# Python API — WHERE IN subquery
rows, columns = db.query(
    "SELECT name FROM products "
    "WHERE category IN (SELECT category FROM products WHERE price > 150)"
)

# Scalar subquery in SELECT
rows, columns = db.query(
    "SELECT name, (SELECT MAX(price) FROM products) as max_price FROM products"
)
```

Subqueries are pre-materialized: they execute first and their results replace the subquery node with literal values before the outer query runs. Subqueries over `date`/`datetime` columns compare correctly (e.g. `WHERE modified IN (SELECT MAX(modified) FROM backtests GROUP BY strategy)`).

Because subqueries are pre-materialized once, **correlated** subqueries (referencing an outer row's columns, e.g. `WHERE b.created = (SELECT MAX(b2.created) FROM backtests b2 WHERE b2.strategy = b.strategy)`) and **tuple** `IN` (`WHERE (strategy, created) IN (SELECT ...)`) are not supported. Use a window function or an `IN` subquery for latest-per-group instead (see below).

## Window Functions

Window functions compute values across a set of rows related to the current row, without collapsing them. Supported: `ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `LAG()`, `LEAD()`, and aggregate functions (`SUM`, `COUNT`, `AVG`, `MIN`, `MAX`) with `OVER`.

```bash
# ROW_NUMBER — assign a unique rank to each row
mdql query examples/ "SELECT title, ROW_NUMBER() OVER (ORDER BY composite DESC) AS rank FROM strategies LIMIT 5"

# RANK with PARTITION BY — rank within groups
mdql query examples/ "SELECT title, status, RANK() OVER (PARTITION BY status ORDER BY composite DESC) AS status_rank FROM strategies"

# LAG — access previous row's value
mdql query examples/ "SELECT title, composite, LAG(composite, 1) OVER (ORDER BY composite DESC) AS prev_composite FROM strategies"

# Aggregate window — SUM per partition without collapsing rows
mdql query examples/ "SELECT title, status, COUNT(*) OVER (PARTITION BY status) AS status_count FROM strategies"
```

```python
# Python API
rows, columns = db.query(
    "SELECT name, ROW_NUMBER() OVER (ORDER BY price DESC) AS rank FROM products"
)
```

Window functions run after WHERE, GROUP BY, and HAVING, but before ORDER BY and LIMIT.

### Latest row per group

A common need is the newest row per group (e.g. the latest backtest per strategy) so stale historical rows don't skew results. Two ways:

```bash
# 1. Window function — number rows per group, keep the first
mdql query examples/ \
  "SELECT * FROM (
     SELECT strategy, sharpe, ROW_NUMBER() OVER (PARTITION BY strategy ORDER BY created DESC) AS rn
     FROM backtests
   ) WHERE rn = 1"

# 2. IN subquery — match the max timestamp per group
mdql query examples/ \
  "SELECT strategy, sharpe FROM backtests
   WHERE created IN (SELECT MAX(created) FROM backtests GROUP BY strategy)"
```

(JOINing directly to a `GROUP BY` subquery — a derived table in the JOIN clause — is not yet supported; use one of the two forms above.)

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

`Database.query()` runs SQL across all tables in the database, including INNER JOIN and LEFT JOIN.

```python
rows, columns = db.query(
    "SELECT s.title, b.sharpe, b.status "
    "FROM strategies s "
    "JOIN backtests b ON b.strategy = s.path"
)

# LEFT JOIN keeps all left rows, fills NULLs for unmatched
rows, columns = db.query(
    "SELECT s.title, b.sharpe "
    "FROM strategies s "
    "LEFT JOIN backtests b ON b.strategy = s.path"
)

# Compound ON conditions — filter during join
rows, columns = db.query(
    "SELECT s.title, b.sharpe "
    "FROM strategies s "
    "LEFT JOIN backtests b ON b.strategy = s.path AND b.mode = 'PAPER'"
)
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
# Delete a single file
strategies.delete("my-new-strategy.md")

# CASCADE — delete row and all FK-dependent rows (backtests, events, etc.)
db.delete("strategies", "status = 'KILLED'", cascade=True)

# RESTRICT — error if any FK-dependent rows exist
db.delete("strategies", "path = 'alpha.md'", restrict=True)

# Dry-run — preview the cascade/restrict plan without executing
plan = db.delete("strategies", "status = 'KILLED'", cascade=True, dry_run=True)
# Returns dict: {"primary_deletes": [...], "cascade_actions": [...], "restrict_violations": [...]}

# Via SQL
db.execute("DELETE FROM strategies WHERE status = 'KILLED' CASCADE")
db.execute("DELETE FROM strategies WHERE path = 'alpha.md' RESTRICT")
```

CASCADE walks the FK graph via BFS. Scalar FKs (e.g. `backtests.strategy -> strategies.path`) trigger dependent row deletion. List FKs (e.g. `string[]` columns) prune the deleted value from the list without deleting the row. RESTRICT checks for any dependent references and blocks the delete if found.

### Rename

```python
# Rename an entry — cascades FK references in other tables
db.rename("strategies", "old-name.md", "new-name.md")

# Rename a table — updates schema + FK config
db.rename_table("strategies", "strats")
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

### `mdql --version`

Print the installed version.

```bash
mdql --version
# mdql 0.5.25
```

### `mdql query <folder> "<sql>"`

Run SQL against a table or database. Supports `SELECT`, `INSERT INTO`, `UPDATE SET`, `DELETE FROM`, `ALTER TABLE`, `JOIN`, `CREATE VIEW`, and `DROP VIEW`.

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

Boolean columns compare against the literals `true`/`false` (case-insensitive): `WHERE enabled = true`. They also work in INSERT/UPDATE values: `SET enabled = false`.

`SELECT DISTINCT` deduplicates on the projected columns (before ORDER BY and LIMIT):

```bash
mdql query examples/ "SELECT DISTINCT strategy FROM backtests"
```

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

### GROUP BY, HAVING, and aggregation

```bash
# Count by status
mdql query examples/strategies/ \
  "SELECT status, COUNT(*) cnt FROM strategies GROUP BY status"

# HAVING filters groups after aggregation
mdql query examples/strategies/ \
  "SELECT status, COUNT(*) cnt FROM strategies GROUP BY status HAVING COUNT(*) > 10"
```

Aggregate arithmetic — combine aggregates with `+`, `-`, `*`, `/`:

```bash
# Net position per token
mdql query examples/ \
  "SELECT token, SUM(CASE WHEN side = 'BUY' THEN size ELSE 0 END) - SUM(CASE WHEN side = 'SELL' THEN size ELSE 0 END) as net FROM orders GROUP BY token"

# Average via SUM/COUNT
mdql query examples/ \
  "SELECT SUM(mechanism) / COUNT(*) as avg_mechanism FROM strategies"
```

Supported aggregate functions: `COUNT(*)`, `COUNT(col)`, `SUM(expr)`, `AVG(expr)`, `MIN(expr)`, `MAX(expr)`.

### Subqueries

Use a subquery in the `FROM` clause to compute derived values in a single query:

```bash
mdql query examples/ \
  "SELECT token, sell_size, buy_size FROM (SELECT token, SUM(CASE WHEN side = 'SELL' THEN size ELSE 0 END) as sell_size, SUM(CASE WHEN side = 'BUY' THEN size ELSE 0 END) as buy_size FROM orders GROUP BY token) WHERE sell_size > buy_size"
```

Subqueries support the full SELECT syntax including WHERE, GROUP BY, HAVING, and ORDER BY.

### Date arithmetic

```bash
# Rows created in the last 30 days
mdql query examples/strategies/ \
  "SELECT title, created FROM strategies WHERE created >= CURRENT_DATE - INTERVAL 30 DAYS"

# Days since creation
mdql query examples/strategies/ \
  "SELECT title, DATEDIFF(CURRENT_DATE, created) days_old FROM strategies ORDER BY days_old DESC LIMIT 5"

# Future date calculation
mdql query examples/strategies/ \
  "SELECT title, modified + INTERVAL 7 DAY review_due FROM strategies"
```

- `CURRENT_DATE` — today's date
- `CURRENT_TIMESTAMP` — current datetime
- `DATEDIFF(date1, date2)` — returns number of days between two dates (date1 - date2)
- `date + INTERVAL N DAY` / `date - INTERVAL N DAYS` — add or subtract days from a date or datetime

#### Date literals

Dates may be written quoted or bare — `WHERE created >= 2026-01-01` and `WHERE created >= '2026-01-01'` are the same query. Bare dates must be zero-padded ISO (`YYYY-MM-DD`, optionally `THH:MM[:SS]` or a space before the time); `2026-1-1` is a parse error rather than a wrong answer, since dates compare as strings. Arithmetic still needs spaces: `2026 - 1 - 1` is the number 2024.

### JOINs

Point at the database directory (parent of table folders) for cross-table queries. Supports INNER JOIN and LEFT JOIN with two or more tables:

```bash
# Two-table JOIN
mdql query examples/ \
  "SELECT s.title, b.sharpe, b.status
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path"

# LEFT JOIN — keeps all left rows, fills NULLs for unmatched right rows
mdql query examples/ \
  "SELECT s.title, b.sharpe
   FROM strategies s
   LEFT JOIN backtests b ON b.strategy = s.path"

# Multi-table JOIN (mix INNER and LEFT)
mdql query my-db/ \
  "SELECT s.title, b.result, c.verdict
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path
   LEFT JOIN critiques c ON c.strategy = s.path"

# Compound ON conditions (AND/OR)
mdql query examples/ \
  "SELECT s.title, b.sharpe
   FROM strategies s
   LEFT JOIN backtests b ON b.strategy = s.path AND b.mode = 'PAPER'"

mdql query examples/ \
  "SELECT s.title, b.sharpe
   FROM strategies s
   JOIN backtests b ON b.strategy = s.path AND b.sharpe > 1.0"
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

# CASCADE DELETE — removes row and all FK-dependent rows
mdql query examples/ \
  "DELETE FROM strategies WHERE status = 'KILLED' CASCADE"
# DELETE 3 (cascade: 7 deleted, 2 list refs pruned)

# RESTRICT DELETE — errors if dependent rows exist
mdql query examples/ \
  "DELETE FROM strategies WHERE path = 'alpha.md' RESTRICT"

# Dry-run — preview what would happen without executing
mdql query examples/ --dry-run \
  "DELETE FROM strategies WHERE status = 'KILLED' CASCADE"
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

### `mdql rename-table <db-folder> <old-name> <new-name>`

Rename a table directory. Updates the schema `table:` field and all foreign key references in the database config.

```bash
mdql rename-table examples/ strategies strats
# RENAME TABLE strategies → strats
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
loose-note.md: Body content not under an H2 section is not allowed; wrap in ## heading
```

Files with body content not under an H2 heading are rejected (the row is excluded from query results). All prose must be wrapped in `## Heading` sections to be queryable. This prevents silent data loss where text exists on disk but is invisible to MDQL queries.

When pointed at a database directory, also reports foreign key violations (see [Foreign key validation](#foreign-key-validation)).

Exits non-zero on any validation error, so it works as a CI gate or git pre-commit hook:

```bash
# Silent mode for CI — exit code only, no output
mdql validate --quiet

# Explicit --strict flag (same behavior as default, accepted for clarity)
mdql validate --strict examples/
```

**Pre-commit hook** (`.git/hooks/pre-commit`):

```bash
#!/bin/sh
mdql validate --quiet
```

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

### `mdql checksums <folder>`

Generate or verify sidecar checksums for tamper detection. Each table gets a `_checksums.json` file tracking xxHash64 of every row file.

```bash
# Generate checksums from current file state
mdql checksums examples/strategies/ --regenerate

# Verify files against stored checksums
mdql checksums examples/strategies/ --verify
```

When checksums exist, MDQL automatically updates them on insert/update/delete. On load, files that don't match their checksum are flagged with `_modified_externally: true` in the row data.

```python
from mdql.migrate import regenerate_checksums

regenerate_checksums("examples/strategies/")

rows, errors = strategies.load()
modified = [r for r in rows if r.get("_modified_externally")]
```

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
| `datetime` | `datetime64[ns]`   |
| `string[]` | Python lists       |
| `dict`     | Python dicts       |

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
