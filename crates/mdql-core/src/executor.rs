//! Unified SQL execution — single entry point for CLI, REPL, and web server.

use std::path::Path;

use crate::api::Table;
use crate::cascade;
use crate::database::{ViewDef, is_database_dir, load_database_config, save_database_config};
use crate::errors::{MdqlError, ValidationError};
use crate::model::Row;
use crate::query_ast::*;
use crate::query_engine::{execute_join_query, execute_query};
use crate::query_parser::{Statement, parse_query};
use crate::schema::Schema;

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
            let has_ctes = !q.ctes.is_empty();
            let has_subqueries = query_has_subqueries(q);
            let needs_db = has_ctes || has_subqueries || q.subquery.is_some() || !q.joins.is_empty() || is_db;

            if has_ctes && !is_db {
                return Err(MdqlError::QueryExecution(
                    "CTEs (WITH) require a database directory".into(),
                ));
            }

            if needs_db {
                let (_config, mut tables, errors) = crate::loader::load_database(path)?;

                for cte in &q.ctes {
                    let (rows, cols) = materialize_cte(&cte.query, &tables)?;
                    let schema = crate::loader::build_view_schema(&cte.name, &cols, &rows);
                    tables.insert(cte.name.clone(), (schema, rows));
                }

                let mut q = q.clone();
                if has_subqueries {
                    materialize_subqueries(&mut q, &tables)?;
                }

                let (rows, cols) = if let Some(ref sub) = q.subquery {
                    let source_table = &sub.table;
                    let (schema, table_rows) = tables.get(source_table).ok_or_else(|| {
                        MdqlError::QueryExecution(format!(
                            "table '{}' not found in database",
                            source_table
                        ))
                    })?;
                    execute_query(&q, table_rows, schema)?
                } else if !q.joins.is_empty() {
                    execute_join_query(&q, &tables)?
                } else {
                    let (schema, rows) = tables.get(&q.table).ok_or_else(|| {
                        MdqlError::QueryExecution(format!(
                            "table '{}' not found in database",
                            q.table
                        ))
                    })?;
                    execute_query(&q, rows, schema)?
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
        Statement::Delete(ref dq) if dq.mode != DeleteMode::Default => {
            if !is_db {
                return Err(MdqlError::QueryExecution(
                    "CASCADE/RESTRICT requires a database directory".into(),
                ));
            }
            let config = load_database_config(path)?;
            if config.views.iter().any(|v| v.name == dq.table) {
                return Err(MdqlError::QueryExecution(format!(
                    "Cannot write to view '{}' — views are read-only",
                    dq.table
                )));
            }
            let (_cfg, tables, errors) = crate::loader::load_database(path)?;
            let (_, rows) = tables.get(&dq.table).ok_or_else(|| {
                MdqlError::QueryExecution(format!("table '{}' not found in database", dq.table))
            })?;
            let matched_filenames: Vec<String> = if let Some(ref wc) = dq.where_clause {
                rows.iter()
                    .filter(|r| crate::query_engine::evaluate(wc, r))
                    .filter_map(|r| r.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect()
            } else {
                rows.iter()
                    .filter_map(|r| r.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect()
            };

            match dq.mode {
                DeleteMode::Cascade => {
                    let plan = cascade::build_cascade_plan(
                        &dq.table, &matched_filenames, &config, &tables,
                    );
                    let msg = cascade::execute_cascade_plan(&plan, path)?;
                    Ok((QueryResult::Message(msg), errors))
                }
                DeleteMode::Restrict => {
                    let plan = cascade::build_restrict_plan(
                        &dq.table, &matched_filenames, &config, &tables,
                    );
                    if !plan.restrict_violations.is_empty() {
                        let violations = plan.restrict_violations.join("\n  ");
                        return Err(MdqlError::QueryExecution(format!(
                            "RESTRICT: cannot delete — {} dependent references:\n  {}",
                            plan.restrict_violations.len(),
                            violations,
                        )));
                    }
                    let table_path = path.join(&dq.table);
                    let table = Table::new(&table_path)?;
                    let msg = table.exec_delete_matched(&matched_filenames)?;
                    Ok((QueryResult::Message(msg), errors))
                }
                DeleteMode::Default => unreachable!(),
            }
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

pub fn materialize_cte(
    query: &crate::query_ast::SelectQuery,
    tables: &std::collections::HashMap<String, (crate::schema::Schema, Vec<Row>)>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    if let Some(ref sub) = query.subquery {
        let (_, sub_rows) = tables.get(&sub.table).ok_or_else(|| {
            MdqlError::QueryExecution(format!("table '{}' not found in database", sub.table))
        })?;
        let (sub_rows, _) = execute_query(sub, sub_rows, &tables.get(&sub.table).unwrap().0)?;
        execute_query(query, &sub_rows, &tables.get(&sub.table).unwrap().0)
    } else if !query.joins.is_empty() {
        execute_join_query(query, tables)
    } else {
        let (schema, rows) = tables.get(&query.table).ok_or_else(|| {
            MdqlError::QueryExecution(format!("table '{}' not found in database", query.table))
        })?;
        execute_query(query, rows, schema)
    }
}

type Tables = std::collections::HashMap<String, (Schema, Vec<Row>)>;

fn query_has_subqueries(q: &SelectQuery) -> bool {
    if let Some(ref wc) = q.where_clause {
        if where_has_subquery(wc) { return true; }
    }
    if let ColumnList::Named(ref exprs) = q.columns {
        for se in exprs {
            match se {
                SelectExpr::Expr { expr, .. } => {
                    if expr_has_subquery(expr) { return true; }
                }
                SelectExpr::Aggregate { arg_expr: Some(e), .. } => {
                    if expr_has_subquery(e) { return true; }
                }
                _ => {}
            }
        }
    }
    false
}

fn where_has_subquery(wc: &WhereClause) -> bool {
    match wc {
        WhereClause::BoolOp(bop) => where_has_subquery(&bop.left) || where_has_subquery(&bop.right),
        WhereClause::Comparison(cmp) => {
            cmp.left_expr.as_ref().map_or(false, |e| expr_has_subquery(e))
                || cmp.right_expr.as_ref().map_or(false, |e| expr_has_subquery(e))
        }
    }
}

fn expr_has_subquery(expr: &Expr) -> bool {
    match expr {
        Expr::Subquery(_) => true,
        Expr::BinaryOp { left, right, .. } => expr_has_subquery(left) || expr_has_subquery(right),
        Expr::UnaryMinus(inner) => expr_has_subquery(inner),
        Expr::Case { whens, else_expr } => {
            whens.iter().any(|(c, e)| where_has_subquery(c) || expr_has_subquery(e))
                || else_expr.as_ref().map_or(false, |e| expr_has_subquery(e))
        }
        _ => false,
    }
}

pub fn materialize_subqueries(
    query: &mut SelectQuery,
    tables: &Tables,
) -> crate::errors::Result<()> {
    if let Some(ref mut wc) = query.where_clause {
        materialize_in_where(wc, tables)?;
    }
    if let ColumnList::Named(ref mut exprs) = query.columns {
        for se in exprs.iter_mut() {
            match se {
                SelectExpr::Expr { ref mut expr, .. } => {
                    materialize_in_expr(expr, tables)?;
                }
                SelectExpr::Aggregate { ref mut arg_expr, .. } => {
                    if let Some(ref mut e) = arg_expr {
                        materialize_in_expr(e, tables)?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn materialize_in_where(wc: &mut WhereClause, tables: &Tables) -> crate::errors::Result<()> {
    match wc {
        WhereClause::BoolOp(ref mut bop) => {
            materialize_in_where(&mut bop.left, tables)?;
            materialize_in_where(&mut bop.right, tables)?;
        }
        WhereClause::Comparison(ref mut cmp) => {
            if let Some(ref mut expr) = cmp.left_expr {
                materialize_in_expr(expr, tables)?;
            }
            if let Some(ref mut expr) = cmp.right_expr {
                if let Expr::Subquery(ref sq) = expr {
                    let (rows, _cols) = materialize_cte(sq, tables)?;
                    if cmp.op == CmpOp::In {
                        let values: Vec<SqlValue> = rows.iter()
                            .filter_map(|r| r.values().next())
                            .map(|v| value_to_sql_value(v))
                            .collect();
                        cmp.value = Some(SqlValue::List(values.clone()));
                        cmp.right_expr = None;
                    } else {
                        let val = rows.first()
                            .and_then(|r| r.values().next())
                            .map(|v| value_to_sql_value(v))
                            .unwrap_or(SqlValue::Null);
                        *expr = Expr::Literal(val);
                    }
                } else {
                    materialize_in_expr(expr, tables)?;
                }
            }
        }
    }
    Ok(())
}

fn materialize_in_expr(expr: &mut Expr, tables: &Tables) -> crate::errors::Result<()> {
    match expr {
        Expr::Subquery(ref sq) => {
            let (rows, _cols) = materialize_cte(sq, tables)?;
            let val = rows.first()
                .and_then(|r| r.values().next())
                .map(|v| value_to_sql_value(v))
                .unwrap_or(SqlValue::Null);
            *expr = Expr::Literal(val);
        }
        Expr::BinaryOp { ref mut left, ref mut right, .. } => {
            materialize_in_expr(left, tables)?;
            materialize_in_expr(right, tables)?;
        }
        Expr::UnaryMinus(ref mut inner) => {
            materialize_in_expr(inner, tables)?;
        }
        Expr::Case { ref mut whens, ref mut else_expr } => {
            for (ref mut cond, ref mut result) in whens.iter_mut() {
                materialize_in_where(cond, tables)?;
                materialize_in_expr(result, tables)?;
            }
            if let Some(ref mut e) = else_expr {
                materialize_in_expr(e, tables)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn value_to_sql_value(v: &crate::model::Value) -> SqlValue {
    match v {
        crate::model::Value::String(s) => SqlValue::String(s.clone()),
        crate::model::Value::Int(n) => SqlValue::Int(*n),
        crate::model::Value::Float(f) => SqlValue::Float(*f),
        crate::model::Value::Bool(b) => SqlValue::Int(if *b { 1 } else { 0 }),
        _ => SqlValue::Null,
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
    use crate::model::Value;
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

    // ── Issue #44: HAVING in CREATE VIEW ──

    #[test]
    fn test_create_view_with_having() {
        let dir = make_test_db();
        // Create a view with HAVING — both statuses have cnt=1, so HAVING cnt > 0 keeps both
        let (result, _) = execute(
            dir.path(),
            "CREATE VIEW popular AS SELECT status, COUNT(*) as cnt FROM strategies GROUP BY status HAVING cnt > 0",
        )
        .unwrap();
        assert!(matches!(result, QueryResult::Message(ref m) if m.contains("created")));

        // Query the view to confirm it works
        let (result, _) = execute(dir.path(), "SELECT * FROM popular").unwrap();
        if let QueryResult::Rows { rows, columns } = result {
            assert!(columns.contains(&"status".to_string()));
            assert!(columns.contains(&"cnt".to_string()));
            // Both LIVE and DRAFT have count 1, both > 0
            assert_eq!(rows.len(), 2);
        } else {
            panic!("Expected Rows, got {:?}", result);
        }
    }

    #[test]
    fn test_extract_view_query_tab_after_as() {
        let q = extract_view_query("CREATE VIEW v AS\tSELECT * FROM t").unwrap();
        assert!(q.starts_with("SELECT"));
        assert!(q.contains("FROM t"));
    }

    fn make_join_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: database\nname: testdb\n---\n",
        )
        .unwrap();

        let strats = dir.path().join("strategies");
        fs::create_dir(&strats).unwrap();
        fs::write(
            strats.join("_mdql.md"),
            "---\ntype: schema\ntable: strategies\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n---\n",
        )
        .unwrap();
        fs::write(strats.join("alpha.md"), "---\ntitle: Alpha\n---\n# Alpha\n").unwrap();
        fs::write(strats.join("beta.md"), "---\ntitle: Beta\n---\n# Beta\n").unwrap();
        fs::write(strats.join("gamma.md"), "---\ntitle: Gamma\n---\n# Gamma\n").unwrap();

        let bt = dir.path().join("backtests");
        fs::create_dir(&bt).unwrap();
        fs::write(
            bt.join("_mdql.md"),
            "---\ntype: schema\ntable: backtests\nprimary_key: path\nfrontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n",
        )
        .unwrap();
        fs::write(bt.join("bt-alpha.md"), "---\nstrategy: alpha.md\nsharpe: 1.5\n---\n# BT Alpha\n").unwrap();

        dir
    }

    #[test]
    fn test_inner_join() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT s.title, b.sharpe FROM strategies s JOIN backtests b ON b.strategy = s.path",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("s.title").unwrap(), &Value::String("Alpha".into()));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_left_join() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT s.title, b.sharpe FROM strategies s LEFT JOIN backtests b ON b.strategy = s.path",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 3);
            let alpha = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Alpha".into()))).unwrap();
            assert_eq!(alpha.get("b.sharpe"), Some(&Value::Float(1.5)));
            let beta = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Beta".into()))).unwrap();
            assert_eq!(beta.get("b.sharpe"), Some(&Value::Null));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_left_join_in_view() {
        let dir = make_join_db();
        execute(
            dir.path(),
            "CREATE VIEW overview AS SELECT s.title, b.sharpe FROM strategies s LEFT JOIN backtests b ON b.strategy = s.path",
        )
        .unwrap();
        let (result, _) = execute(dir.path(), "SELECT * FROM overview").unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 3);
        } else {
            panic!("Expected Rows");
        }
    }

    fn make_compound_join_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: database\nname: testdb\n---\n",
        )
        .unwrap();

        let strats = dir.path().join("strategies");
        fs::create_dir(&strats).unwrap();
        fs::write(
            strats.join("_mdql.md"),
            "---\ntype: schema\ntable: strategies\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n---\n",
        )
        .unwrap();
        fs::write(strats.join("alpha.md"), "---\ntitle: Alpha\n---\n# Alpha\n").unwrap();
        fs::write(strats.join("beta.md"), "---\ntitle: Beta\n---\n# Beta\n").unwrap();

        let bt = dir.path().join("backtests");
        fs::create_dir(&bt).unwrap();
        fs::write(
            bt.join("_mdql.md"),
            "---\ntype: schema\ntable: backtests\nprimary_key: path\nfrontmatter:\n  strategy:\n    type: string\n  mode:\n    type: string\n  sharpe:\n    type: float\n---\n",
        )
        .unwrap();
        fs::write(bt.join("bt-alpha-paper.md"), "---\nstrategy: alpha.md\nmode: PAPER\nsharpe: 1.5\n---\n# BT\n").unwrap();
        fs::write(bt.join("bt-alpha-live.md"), "---\nstrategy: alpha.md\nmode: LIVE\nsharpe: 1.2\n---\n# BT\n").unwrap();
        fs::write(bt.join("bt-beta-paper.md"), "---\nstrategy: beta.md\nmode: PAPER\nsharpe: 0.8\n---\n# BT\n").unwrap();

        dir
    }

    #[test]
    fn test_join_compound_and() {
        let dir = make_compound_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT s.title, b.sharpe FROM strategies s JOIN backtests b ON b.strategy = s.path AND b.mode = 'PAPER'",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 2);
            let alpha = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Alpha".into()))).unwrap();
            assert_eq!(alpha.get("b.sharpe"), Some(&Value::Float(1.5)));
            let beta = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Beta".into()))).unwrap();
            assert_eq!(beta.get("b.sharpe"), Some(&Value::Float(0.8)));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_left_join_compound() {
        let dir = make_compound_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT s.title, b.sharpe FROM strategies s LEFT JOIN backtests b ON b.strategy = s.path AND b.mode = 'LIVE'",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 2);
            let alpha = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Alpha".into()))).unwrap();
            assert_eq!(alpha.get("b.sharpe"), Some(&Value::Float(1.2)));
            let beta = rows.iter().find(|r| r.get("s.title") == Some(&Value::String("Beta".into()))).unwrap();
            assert_eq!(beta.get("b.sharpe"), Some(&Value::Null));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_join_compound_with_comparison() {
        let dir = make_compound_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT s.title, b.sharpe FROM strategies s JOIN backtests b ON b.strategy = s.path AND b.sharpe > 1.0",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| {
                if let Some(Value::Float(s)) = r.get("b.sharpe") { *s > 1.0 } else { false }
            }));
        } else {
            panic!("Expected Rows");
        }
    }

    fn make_cascade_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: database\nname: testdb\nforeign_keys:\n  - from: backtests.strategy\n    to: strategies.path\n---\n",
        )
        .unwrap();

        let strats = dir.path().join("strategies");
        fs::create_dir(&strats).unwrap();
        fs::write(
            strats.join("_mdql.md"),
            "---\ntype: schema\ntable: strategies\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n  status:\n    type: string\n---\n",
        )
        .unwrap();
        fs::write(strats.join("alpha.md"), "---\ntitle: Alpha\nstatus: KILLED\n---\n# Alpha\n").unwrap();
        fs::write(strats.join("beta.md"), "---\ntitle: Beta\nstatus: LIVE\n---\n# Beta\n").unwrap();

        let bt = dir.path().join("backtests");
        fs::create_dir(&bt).unwrap();
        fs::write(
            bt.join("_mdql.md"),
            "---\ntype: schema\ntable: backtests\nprimary_key: path\nfrontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n",
        )
        .unwrap();
        fs::write(bt.join("bt-alpha.md"), "---\nstrategy: alpha.md\nsharpe: 1.5\n---\n# BT Alpha\n").unwrap();
        fs::write(bt.join("bt-beta.md"), "---\nstrategy: beta.md\nsharpe: 0.8\n---\n# BT Beta\n").unwrap();

        dir
    }

    #[test]
    fn test_cascade_delete() {
        let dir = make_cascade_db();
        let (result, _) = execute(
            dir.path(),
            "DELETE FROM strategies WHERE status = 'KILLED' CASCADE",
        )
        .unwrap();
        if let QueryResult::Message(msg) = result {
            assert!(msg.contains("DELETE 1"));
            assert!(msg.contains("cascade"));
        } else {
            panic!("Expected Message");
        }
        assert!(!dir.path().join("strategies/alpha.md").exists());
        assert!(!dir.path().join("backtests/bt-alpha.md").exists());
        assert!(dir.path().join("strategies/beta.md").exists());
        assert!(dir.path().join("backtests/bt-beta.md").exists());
    }

    #[test]
    fn test_restrict_delete_blocks() {
        let dir = make_cascade_db();
        let err = execute(
            dir.path(),
            "DELETE FROM strategies WHERE status = 'KILLED' RESTRICT",
        );
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("RESTRICT"));
        assert!(dir.path().join("strategies/alpha.md").exists());
    }

    #[test]
    fn test_restrict_delete_allows_no_dependents() {
        let dir = make_cascade_db();
        fs::remove_file(dir.path().join("backtests/bt-alpha.md")).unwrap();

        let (result, _) = execute(
            dir.path(),
            "DELETE FROM strategies WHERE status = 'KILLED' RESTRICT",
        )
        .unwrap();
        if let QueryResult::Message(msg) = result {
            assert!(msg.contains("DELETE 1"));
        } else {
            panic!("Expected Message");
        }
        assert!(!dir.path().join("strategies/alpha.md").exists());
    }

    #[test]
    fn test_cascade_default_unchanged() {
        let dir = make_cascade_db();
        let (result, _) = execute(
            dir.path(),
            "DELETE FROM strategies WHERE status = 'KILLED'",
        )
        .unwrap();
        if let QueryResult::Message(msg) = result {
            assert!(msg.contains("DELETE 1"));
        } else {
            panic!("Expected Message");
        }
        assert!(!dir.path().join("strategies/alpha.md").exists());
        assert!(dir.path().join("backtests/bt-alpha.md").exists());
    }

    // ── CTE (WITH) integration tests ──

    #[test]
    fn test_cte_basic() {
        let dir = make_test_db();
        let (result, _) = execute(
            dir.path(),
            "WITH live AS (SELECT * FROM strategies WHERE status = 'LIVE') SELECT * FROM live",
        )
        .unwrap();
        if let QueryResult::Rows { rows, columns } = result {
            assert_eq!(rows.len(), 1);
            assert!(columns.contains(&"title".to_string()));
            assert_eq!(rows[0].get("title"), Some(&Value::String("Alpha".into())));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_cte_with_where_on_cte() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "WITH bt AS (SELECT * FROM backtests WHERE sharpe > 1.0) SELECT * FROM bt",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("sharpe"), Some(&Value::Float(1.5)));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_cte_multi_with_join() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "WITH s AS (SELECT * FROM strategies), b AS (SELECT * FROM backtests) SELECT s.title, b.sharpe FROM s JOIN b ON b.strategy = s.path",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("s.title"), Some(&Value::String("Alpha".into())));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_cte_with_aggregation() {
        let dir = make_test_db();
        let (result, _) = execute(
            dir.path(),
            "WITH counts AS (SELECT status, COUNT(*) AS cnt FROM strategies GROUP BY status) SELECT * FROM counts WHERE cnt > 0",
        )
        .unwrap();
        if let QueryResult::Rows { rows, columns } = result {
            assert!(columns.contains(&"status".to_string()));
            assert!(columns.contains(&"cnt".to_string()));
            assert!(rows.len() >= 1);
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_cte_chained() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "WITH good AS (SELECT * FROM backtests WHERE sharpe > 1.0), matched AS (SELECT s.title, g.sharpe FROM strategies s JOIN good g ON g.strategy = s.path) SELECT * FROM matched",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
        } else {
            panic!("Expected Rows");
        }
    }

    // ── Subquery integration tests ──

    #[test]
    fn test_where_in_subquery() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT * FROM strategies WHERE path IN (SELECT strategy FROM backtests)",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("title"), Some(&Value::String("Alpha".into())));
        } else {
            panic!("Expected Rows");
        }
    }

    fn make_multi_bt_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("_mdql.md"),
            "---\ntype: database\nname: testdb\n---\n",
        )
        .unwrap();

        let strats = dir.path().join("strategies");
        fs::create_dir(&strats).unwrap();
        fs::write(
            strats.join("_mdql.md"),
            "---\ntype: schema\ntable: strategies\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n---\n",
        )
        .unwrap();
        fs::write(strats.join("alpha.md"), "---\ntitle: Alpha\n---\n# Alpha\n").unwrap();
        fs::write(strats.join("beta.md"), "---\ntitle: Beta\n---\n# Beta\n").unwrap();

        let bt = dir.path().join("backtests");
        fs::create_dir(&bt).unwrap();
        fs::write(
            bt.join("_mdql.md"),
            "---\ntype: schema\ntable: backtests\nprimary_key: path\nfrontmatter:\n  strategy:\n    type: string\n  sharpe:\n    type: float\n---\n",
        )
        .unwrap();
        fs::write(bt.join("bt-alpha.md"), "---\nstrategy: alpha.md\nsharpe: 2.0\n---\n# BT\n").unwrap();
        fs::write(bt.join("bt-beta.md"), "---\nstrategy: beta.md\nsharpe: 0.5\n---\n# BT\n").unwrap();

        dir
    }

    #[test]
    fn test_where_scalar_subquery() {
        let dir = make_multi_bt_db();
        // AVG(sharpe) = (2.0 + 0.5) / 2 = 1.25 → only bt-alpha (2.0) passes
        let (result, _) = execute(
            dir.path(),
            "SELECT * FROM backtests WHERE sharpe > (SELECT AVG(sharpe) FROM backtests)",
        )
        .unwrap();
        if let QueryResult::Rows { rows, .. } = result {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].get("sharpe"), Some(&Value::Float(2.0)));
        } else {
            panic!("Expected Rows");
        }
    }

    #[test]
    fn test_select_scalar_subquery() {
        let dir = make_join_db();
        let (result, _) = execute(
            dir.path(),
            "SELECT title, (SELECT COUNT(*) FROM backtests) AS bt_count FROM strategies",
        )
        .unwrap();
        if let QueryResult::Rows { rows, columns } = result {
            assert!(columns.contains(&"bt_count".to_string()));
            for row in &rows {
                assert_eq!(row.get("bt_count"), Some(&Value::Int(1)));
            }
        } else {
            panic!("Expected Rows");
        }
    }
}
