# CLAUDE.md

## Build and test

```bash
# Rust tests
cargo test --workspace

# Python: rebuild bindings then run tests
PATH=".venv/bin:$PATH" maturin develop
.venv/bin/python -m pytest tests/ -x -q
```

## Release workflow

Every change follows this sequence — do not skip steps or stop partway:

1. **Implement** the change (Rust in `crates/`, Python in `python/`, tests in `tests/`)
2. **Write tests** covering the new behavior
3. **Run tests** — all Rust (`cargo test --workspace`) and Python (`.venv/bin/python -m pytest tests/ -x -q`) tests must pass
4. **Update README.md** if the change affects user-facing behavior
5. **Commit** with a descriptive message (no AI attribution)
6. **Bump version** in three files: `Cargo.toml` (workspace), `python/Cargo.toml` (version + dep), `pyproject.toml`
7. **Commit** the version bump separately
8. **Push** development, merge to main, push main
9. **Tag** with `v*` (e.g. `git tag v0.5.22`) and push the tag — this triggers CI release to crates.io + PyPI
10. **Wait for CI** — `gh run watch <id> --exit-status`
11. **Update Homebrew** — get SHA256 of tarball, update `mdql-db/homebrew-tap` via GitHub API, pull tap locally, `brew reinstall mdql`
12. **Install in zunid** — the zunid venv is uv-managed (no pip): `cd ~/repos/zunid && uv pip install --no-cache --reinstall "mdql==<version>" --index-url https://pypi.org/simple` (pin the exact version — plain `--upgrade` can miss a release published seconds earlier due to PyPI propagation lag)
13. **Refresh the cargo CLI** — `cargo install --path crates/mdql --force` (the binary at `~/.cargo/bin/mdql` is on PATH and shadows the Homebrew build)
14. **Close the GitHub issue** if applicable

## Version bump locations

- `Cargo.toml` line ~11: `version = "x.y.z"` (workspace)
- `python/Cargo.toml` lines ~3 and ~13: package version and mdql-core dependency version
- `pyproject.toml` line ~7: `version = "x.y.z"`

## Branches

- `development` — default branch on GitHub, primary work branch
- `main` — kept in sync with development, tags are created on main

## Project structure

- `crates/mdql-core/` — core Rust library (parser, schema, validator, query engine, API)
- `crates/mdql/` — CLI binary
- `crates/mdql-web/` — web UI server
- `python/src/lib.rs` — PyO3 bindings
- `python/mdql/` — Python wrapper package
- `tests/` — Python test suite
