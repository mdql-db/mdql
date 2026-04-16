//! Unified SQL execution — single entry point for CLI, REPL, and web server.

use std::path::Path;

use crate::api::Table;
use crate::database::is_database_dir;
use crate::errors::{MdqlError, ValidationError};
use crate::model::Row;
use crate::query_engine::{execute_join_query, execute_query};
use crate::query_parser::{Statement, parse_query};

pub enum QueryResult {
    Rows { rows: Vec<Row>, columns: Vec<String> },
    Message(String),
}

pub fn execute(path: &Path, sql: &str) -> crate::errors::Result<(QueryResult, Vec<ValidationError>)> {
    let stmt = parse_query(sql)?;
    let is_db = is_database_dir(path);

    match stmt {
        Statement::Select(ref q) => {
            if !q.joins.is_empty() || is_db {
                let (_config, tables, errors) = crate::loader::load_database(path)?;
                let (rows, cols) = if !q.joins.is_empty() {
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
        _ => {
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
