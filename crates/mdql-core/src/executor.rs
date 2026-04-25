//! Unified SQL execution — single entry point for CLI, REPL, and web server.

use std::path::Path;

use crate::api::Table;
use crate::database::{ViewDef, is_database_dir, load_database_config, save_database_config};
use crate::errors::{MdqlError, ValidationError};
use crate::model::Row;
use crate::query_engine::{execute_join_query, execute_query};
use crate::query_parser::{Statement, parse_query};

#[derive(Debug)]
pub enum QueryResult {
    Rows { rows: Vec<Row>, columns: Vec<String> },
    Message(String),
}

pub fn execute(path: &Path, sql: &str) -> crate::errors::Result<(QueryResult, Vec<ValidationError>)> {
    let stmt = parse_query(sql)?;
    let is_db = is_database_dir(path);

    match stmt {
        Statement::Select(ref q) => {
            if q.subquery.is_some() || !q.joins.is_empty() || is_db {
                let (_config, tables, errors) = crate::loader::load_database(path)?;
                let (rows, cols) = if let Some(ref sub) = q.subquery {
                    let source_table = &sub.table;
                    let (schema, table_rows) = tables.get(source_table).ok_or_else(|| {
                        MdqlError::QueryExecution(format!(
                            "table '{}' not found in database",
                            source_table
                        ))
                    })?;
                    let (sub_rows, _) = execute_query(sub, table_rows, schema)?;
                    execute_query(q, &sub_rows, schema)?
                } else if !q.joins.is_empty() {
                    execute_join_query(q, &tables)?
                } else {
                    let (schema, rows) = tables.get(&q.table).ok_or_else(|| {
                        MdqlError::QueryExecution(format!(
                            "table '{}' not found in database",
                            q.table
                        ))
                    })?;
                    execute_query(q, rows, schema)?
                };
                Ok((QueryResult::Rows { rows, columns: cols }, errors))
            } else {
                let (schema, rows, errors) = crate::loader::load_table(path)?;
                let (rows, cols) = execute_query(q, &rows, &schema)?;
                Ok((QueryResult::Rows { rows, columns: cols }, errors))
            }
        }
        Statement::CreateView(ref cv) => {
            if !is_db {
                return Err(MdqlError::QueryExecution(
                    "CREATE VIEW requires a database directory".into(),
                ));
            }
            let mut config = load_database_config(path)?;

            let (_config_check, tables, _errors) = crate::loader::load_database(path)?;
            if tables.contains_key(&cv.view_name) {
                return Err(MdqlError::QueryExecution(format!(
                    "Name '{}' already exists as a table or view",
                    cv.view_name
                )));
            }

            if config.views.iter().any(|v| v.name == cv.view_name) {
                return Err(MdqlError::QueryExecution(format!(
                    "View '{}' already exists",
                    cv.view_name
                )));
            }

            let query_str = extract_view_query(sql)?;

            let view_def = ViewDef {
                name: cv.view_name.clone(),
                query: query_str,
            };

            let test_result = crate::loader::load_database(path);
            if let Ok((_cfg, test_tables, _errs)) = test_result {
                let test_view = ViewDef {
                    name: view_def.name.clone(),
                    query: view_def.query.clone(),
                };
                if let Err(e) = super::loader::materialize_view(&test_view, &test_tables) {
                    return Err(MdqlError::QueryExecution(format!(
                        "View query failed validation: {}",
                        e
                    )));
                }
            }

            config.views.push(view_def);
            save_database_config(path, &config)?;
            Ok((
                QueryResult::Message(format!("View '{}' created", cv.view_name)),
                vec![],
            ))
        }
        Statement::DropView(ref dv) => {
            if !is_db {
                return Err(MdqlError::QueryExecution(
                    "DROP VIEW requires a database directory".into(),
                ));
            }
            let mut config = load_database_config(path)?;
            let len_before = config.views.len();
            config.views.retain(|v| v.name != dv.view_name);
            if config.views.len() == len_before {
                return Err(MdqlError::QueryExecution(format!(
                    "View '{}' does not exist",
                    dv.view_name
                )));
            }
            save_database_config(path, &config)?;
            Ok((
                QueryResult::Message(format!("View '{}' dropped", dv.view_name)),
                vec![],
            ))
        }
        ref stmt @ (Statement::Insert(_)
        | Statement::Update(_)
        | Statement::Delete(_)
        | Statement::AlterRename(_)
        | Statement::AlterDrop(_)
        | Statement::AlterMerge(_)) => {
            if is_db {
                let config = load_database_config(path)?;
                let target = stmt.table_name();
                if config.views.iter().any(|v| v.name == target) {
                    return Err(MdqlError::QueryExecution(format!(
                        "Cannot write to view '{}' — views are read-only",
                        target
                    )));
                }
            }
            let table_path = if is_db {
                path.join(stmt.table_name())
            } else {
                path.to_path_buf()
            };
            let mut table = Table::new(&table_path)?;
            let msg = table.execute_sql(sql)?;
            Ok((QueryResult::Message(msg), vec![]))
        }
    }
}

fn extract_view_query(sql: &str) -> crate::errors::Result<String> {
    let upper = sql.to_uppercase();
    let as_keyword = upper.find(" AS ");
    if let Some(pos) = as_keyword {
        let after = &sql[pos + 4..];
        let trimmed = after.trim_start();
        let trimmed_upper = trimmed.to_uppercase();
        if trimmed_upper.starts_with("SELECT") {
            return Ok(trimmed.to_string());
        }
    }
    // Fallback: scan for any whitespace-surrounded AS that precedes SELECT
    let bytes = upper.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i].is_ascii_whitespace()
            && bytes[i + 1] == b'A'
            && bytes[i + 2] == b'S'
            && bytes[i + 3].is_ascii_whitespace()
        {
            let after = &sql[i + 3..];
            let trimmed = after.trim_start();
            let trimmed_upper = trimmed.to_uppercase();
            if trimmed_upper.starts_with("SELECT") {
                return Ok(trimmed.to_string());
            }
        }
        i += 1;
    }
    Err(crate::errors::MdqlError::QueryExecution(
        "CREATE VIEW must contain AS clause followed by SELECT".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_test_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        // Database-level _mdql.md
        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: database\nname: testdb\n---\n",
        )
        .unwrap();

        // Table: strategies
        let strats = dir.path().join("strategies");
        fs::create_dir(&strats).unwrap();
        fs::write(
            strats.join("_mdql.md"),
            "---\ntype: schema\ntable: strategies\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n  status:\n    type: string\n---\n",
        )
        .unwrap();
        fs::write(
            strats.join("alpha.md"),
            "---\ntitle: Alpha\nstatus: LIVE\n---\n# Alpha\n",
        )
        .unwrap();
        fs::write(
            strats.join("beta.md"),
            "---\ntitle: Beta\nstatus: DRAFT\n---\n# Beta\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_create_and_query_view() {
        let dir = make_test_db();
        let (result, _) = execute(
            dir.path(),
            "CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'",
        )
        .unwrap();
        assert!(matches!(result, QueryResult::Message(ref m) if m.contains("created")));

        let (result, _) = execute(dir.path(), "SELECT * FROM live").unwrap();
        if let QueryResult::Rows { rows, columns } = result {
            assert_eq!(rows.len(), 1);
            assert!(columns.contains(&"title".to_string()));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_drop_view() {
        let dir = make_test_db();
        execute(
            dir.path(),
            "CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'",
        )
        .unwrap();

        let (result, _) = execute(dir.path(), "DROP VIEW live").unwrap();
        assert!(matches!(result, QueryResult::Message(ref m) if m.contains("dropped")));

        let err = execute(dir.path(), "SELECT * FROM live");
        assert!(err.is_err());
    }

    #[test]
    fn test_drop_nonexistent_view() {
        let dir = make_test_db();
        let err = execute(dir.path(), "DROP VIEW nonexistent");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_create_view_duplicate_name() {
        let dir = make_test_db();
        execute(
            dir.path(),
            "CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'",
        )
        .unwrap();

        let err = execute(
            dir.path(),
            "CREATE VIEW live AS SELECT * FROM strategies",
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_view_conflicts_with_table() {
        let dir = make_test_db();
        let err = execute(
            dir.path(),
            "CREATE VIEW strategies AS SELECT * FROM strategies",
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_write_to_view_rejected() {
        let dir = make_test_db();
        execute(
            dir.path(),
            "CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'",
        )
        .unwrap();

        let err = execute(
            dir.path(),
            "INSERT INTO live (title, status) VALUES ('Gamma', 'LIVE')",
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("read-only"));
    }

    #[test]
    fn test_create_view_not_database() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: schema\ntable: t\nprimary_key: path\nfrontmatter:\n  x:\n    type: string\n---\n",
        )
        .unwrap();

        let err = execute(
            dir.path(),
            "CREATE VIEW v AS SELECT * FROM t",
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("database directory"));
    }

    #[test]
    fn test_extract_view_query_basic() {
        let q = extract_view_query("CREATE VIEW v AS SELECT * FROM t").unwrap();
        assert!(q.starts_with("SELECT"));
    }

    #[test]
    fn test_extract_view_query_with_column_alias() {
        let q = extract_view_query(
            "CREATE VIEW v AS SELECT token, SUM(size) as sell_size FROM orders GROUP BY token HAVING sell_size > 0"
        ).unwrap();
        assert!(q.starts_with("SELECT"));
        assert!(q.contains("HAVING"));
    }

    #[test]
    fn test_extract_view_query_newline_after_as() {
        let q = extract_view_query("CREATE VIEW v AS\nSELECT * FROM t").unwrap();
        assert!(q.starts_with("SELECT"));
    }

    #[test]
    fn test_create_view_with_aggregate_arithmetic() {
        let dir = make_test_db();
        let result = execute(
            dir.path(),
            "CREATE VIEW v AS SELECT status, COUNT(*) - COUNT(*) as zero FROM strategies GROUP BY status",
        );
        assert!(result.is_ok());
    }
}
