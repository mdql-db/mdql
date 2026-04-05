# MDQL

A strict Markdown database with SQL-like queries.

Markdown files with YAML frontmatter are the canonical rows. Frontmatter defines metadata columns, H2 sections define content columns, and a strict schema makes the files queryable while remaining readable by humans and LLMs.

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
---

## Hypothesis

When the perpetual funding rate exceeds 0.05%...

## Entry Rules

Enter on the opposite side of the funding imbalance...
```

- YAML frontmatter fields become metadata columns (`title`, `status`, `mechanism`, ...)
- H2 sections become content columns (`Hypothesis`, `Entry Rules`, ...)
- The `path` (filename) is the implicit primary key
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

## Commands

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

### `mdql schema <folder>`

Print the effective schema. Works on a single table or the whole database:

```bash
uv run mdql schema examples/
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

102 tests covering parser, validator, query engine, CLI, and integration with real data.

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
  cli.py            # typer CLI
```

## License

TBD
