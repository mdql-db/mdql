//! Execute parsed queries over in-memory rows.

use std::cmp::Ordering;
use std::collections::HashMap;

use regex::Regex;

use crate::errors::MdqlError;
use crate::model::{Row, Value};
use crate::query_parser::*;
use crate::schema::Schema;

pub fn execute_query(
    query: &SelectQuery,
    rows: &[Row],
    _schema: &Schema,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    execute(query, rows, None)
}

/// Execute a query with optional B-tree index and FTS searcher.
pub fn execute_query_indexed(
    query: &SelectQuery,
    rows: &[Row],
    schema: &Schema,
    index: Option<&crate::index::TableIndex>,
    searcher: Option<&crate::search::TableSearcher>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    // Pre-compute FTS results for any LIKE clauses on section columns
    let fts_results = if let (Some(ref wc), Some(searcher)) = (&query.where_clause, searcher) {
        collect_fts_results(wc, schema, searcher)
    } else {
        HashMap::new()
    };

    execute_with_fts(query, rows, index, &fts_results)
}

/// Collect FTS results for LIKE comparisons on section columns.
/// Returns a map from (column, pattern) → set of matching paths.
fn collect_fts_results(
    clause: &WhereClause,
    schema: &Schema,
    searcher: &crate::search::TableSearcher,
) -> HashMap<(String, String), std::collections::HashSet<String>> {
    let mut results = HashMap::new();
    collect_fts_results_inner(clause, schema, searcher, &mut results);
    results
}

fn collect_fts_results_inner(
    clause: &WhereClause,
    schema: &Schema,
    searcher: &crate::search::TableSearcher,
    results: &mut HashMap<(String, String), std::collections::HashSet<String>>,
) {
    match clause {
        WhereClause::Comparison(cmp) => {
            if (cmp.op == "LIKE" || cmp.op == "NOT LIKE") && schema.sections.contains_key(&cmp.column) {
                if let Some(SqlValue::String(pattern)) = &cmp.value {
                    // Strip SQL wildcards for Tantivy query
                    let search_term = pattern.replace('%', " ").replace('_', " ").trim().to_string();
                    if !search_term.is_empty() {
                        if let Ok(paths) = searcher.search(&search_term, Some(&cmp.column)) {
                            let key = (cmp.column.clone(), pattern.clone());
                            results.insert(key, paths.into_iter().collect());
                        }
                    }
                }
            }
        }
        WhereClause::BoolOp(bop) => {
            collect_fts_results_inner(&bop.left, schema, searcher, results);
            collect_fts_results_inner(&bop.right, schema, searcher, results);
        }
    }
}

type FtsResults = HashMap<(String, String), std::collections::HashSet<String>>;

fn execute_with_fts(
    query: &SelectQuery,
    rows: &[Row],
    index: Option<&crate::index::TableIndex>,
    fts: &FtsResults,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    // Determine available columns
    let mut all_columns: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in rows {
        for k in r.keys() {
            if seen.insert(k.clone()) {
                all_columns.push(k.clone());
            }
        }
    }

    let columns = match &query.columns {
        ColumnList::All => all_columns,
        ColumnList::Named(cols) => cols.clone(),
    };

    // Filter — try index first, fall back to full scan
    let mut result: Vec<Row> = if let Some(ref wc) = query.where_clause {
        let candidate_paths = index.and_then(|idx| try_index_filter(wc, idx));
        if let Some(paths) = candidate_paths {
            rows.iter()
                .filter(|r| {
                    r.get("path")
                        .and_then(|v| v.as_str())
                        .map_or(false, |p| paths.contains(p))
                })
                .filter(|r| evaluate_with_fts(wc, r, fts))
                .cloned()
                .collect()
        } else {
            rows.iter()
                .filter(|r| evaluate_with_fts(wc, r, fts))
                .cloned()
                .collect()
        }
    } else {
        rows.to_vec()
    };

    // Sort
    if let Some(ref order_by) = query.order_by {
        sort_rows(&mut result, order_by);
    }

    // Limit
    if let Some(limit) = query.limit {
        result.truncate(limit as usize);
    }

    Ok((result, columns))
}

fn evaluate_with_fts(clause: &WhereClause, row: &Row, fts: &FtsResults) -> bool {
    match clause {
        WhereClause::BoolOp(bop) => {
            let left = evaluate_with_fts(&bop.left, row, fts);
            match bop.op.as_str() {
                "AND" => left && evaluate_with_fts(&bop.right, row, fts),
                "OR" => left || evaluate_with_fts(&bop.right, row, fts),
                _ => false,
            }
        }
        WhereClause::Comparison(cmp) => {
            // Check if we have FTS results for this comparison
            if cmp.op == "LIKE" || cmp.op == "NOT LIKE" {
                if let Some(SqlValue::String(pattern)) = &cmp.value {
                    let key = (cmp.column.clone(), pattern.clone());
                    if let Some(matching_paths) = fts.get(&key) {
                        let row_path = row.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let matched = matching_paths.contains(row_path);
                        return if cmp.op == "LIKE" { matched } else { !matched };
                    }
                }
            }
            evaluate_comparison(cmp, row)
        }
    }
}

pub fn execute_join_query(
    query: &SelectQuery,
    tables: &HashMap<String, (Schema, Vec<Row>)>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    if query.joins.is_empty() {
        return Err(MdqlError::QueryExecution("No JOIN clause in query".into()));
    }

    let left_name = &query.table;
    let left_alias = query.table_alias.as_deref().unwrap_or(left_name);

    // Build alias→table mapping for all tables
    let mut aliases: HashMap<String, String> = HashMap::new();
    aliases.insert(left_name.clone(), left_name.clone());
    if let Some(ref a) = query.table_alias {
        aliases.insert(a.clone(), left_name.clone());
    }
    for join in &query.joins {
        aliases.insert(join.table.clone(), join.table.clone());
        if let Some(ref a) = join.alias {
            aliases.insert(a.clone(), join.table.clone());
        }
    }

    // Start with the left table rows, prefixed with alias
    let (_left_schema, left_rows) = tables.get(left_name.as_str()).ok_or_else(|| {
        MdqlError::QueryExecution(format!("Unknown table '{}'", left_name))
    })?;

    let mut current_rows: Vec<Row> = left_rows
        .iter()
        .map(|r| {
            let mut prefixed = Row::new();
            for (k, v) in r {
                prefixed.insert(format!("{}.{}", left_alias, k), v.clone());
            }
            prefixed
        })
        .collect();

    // Process each JOIN sequentially
    for join in &query.joins {
        let right_name = &join.table;
        let right_alias = join.alias.as_deref().unwrap_or(right_name);

        let (_right_schema, right_rows) = tables.get(right_name.as_str()).ok_or_else(|| {
            MdqlError::QueryExecution(format!("Unknown table '{}'", right_name))
        })?;

        // Resolve ON columns to determine which is left vs right
        let (on_left_table, on_left_col) = resolve_dotted(&join.left_col, &aliases);
        let (on_right_table, on_right_col) = resolve_dotted(&join.right_col, &aliases);

        // Figure out which ON column refers to the new right table
        let (left_key, right_key) = if on_right_table == *right_name {
            // left_col is from the left side, right_col is from the right table
            let left_alias_for_col = reverse_alias(&on_left_table, &aliases, query, &query.joins);
            (format!("{}.{}", left_alias_for_col, on_left_col), on_right_col)
        } else {
            // right_col is from the left side, left_col is from the right table
            let right_alias_for_col = reverse_alias(&on_right_table, &aliases, query, &query.joins);
            (format!("{}.{}", right_alias_for_col, on_right_col), on_left_col)
        };

        // Build index on right table
        let mut right_index: HashMap<String, Vec<&Row>> = HashMap::new();
        for r in right_rows {
            if let Some(key) = r.get(&right_key) {
                let key_str = key.to_display_string();
                right_index.entry(key_str).or_default().push(r);
            }
        }

        // Join current rows with right table
        let mut next_rows: Vec<Row> = Vec::new();
        for lr in &current_rows {
            if let Some(key) = lr.get(&left_key) {
                let key_str = key.to_display_string();
                if let Some(matching) = right_index.get(&key_str) {
                    for rr in matching {
                        let mut merged = lr.clone();
                        for (k, v) in *rr {
                            merged.insert(format!("{}.{}", right_alias, k), v.clone());
                        }
                        next_rows.push(merged);
                    }
                }
            }
        }
        current_rows = next_rows;
    }

    execute(query, &current_rows, None)
}

/// Given a table name, find the alias used for it.
fn reverse_alias(
    table_name: &str,
    aliases: &HashMap<String, String>,
    query: &SelectQuery,
    joins: &[JoinClause],
) -> String {
    // Check if the FROM table matches
    if query.table == table_name {
        return query.table_alias.as_deref().unwrap_or(&query.table).to_string();
    }
    // Check join tables
    for j in joins {
        if j.table == table_name {
            return j.alias.as_deref().unwrap_or(&j.table).to_string();
        }
    }
    // Fall back: check if table_name is itself an alias
    if aliases.contains_key(table_name) {
        return table_name.to_string();
    }
    table_name.to_string()
}

fn resolve_dotted(col: &str, aliases: &HashMap<String, String>) -> (String, String) {
    if let Some((alias, column)) = col.split_once('.') {
        let table = aliases.get(alias).cloned().unwrap_or_else(|| alias.to_string());
        (table, column.to_string())
    } else {
        (String::new(), col.to_string())
    }
}

fn execute(
    query: &SelectQuery,
    rows: &[Row],
    index: Option<&crate::index::TableIndex>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    let empty_fts = HashMap::new();
    execute_with_fts(query, rows, index, &empty_fts)
}

pub fn evaluate(clause: &WhereClause, row: &Row) -> bool {
    match clause {
        WhereClause::BoolOp(bop) => {
            let left = evaluate(&bop.left, row);
            match bop.op.as_str() {
                "AND" => left && evaluate(&bop.right, row),
                "OR" => left || evaluate(&bop.right, row),
                _ => false,
            }
        }
        WhereClause::Comparison(cmp) => evaluate_comparison(cmp, row),
    }
}

fn evaluate_comparison(cmp: &Comparison, row: &Row) -> bool {
    let actual = row.get(&cmp.column);

    if cmp.op == "IS NULL" {
        return actual.map_or(true, |v| v.is_null());
    }
    if cmp.op == "IS NOT NULL" {
        return actual.map_or(false, |v| !v.is_null());
    }

    let actual = match actual {
        Some(v) if !v.is_null() => v,
        _ => return false,
    };

    let expected = match &cmp.value {
        Some(v) => v,
        None => return false,
    };

    match cmp.op.as_str() {
        "=" => eq_match(actual, expected),
        "!=" => !eq_match(actual, expected),
        "<" => compare_values(actual, expected) == Some(Ordering::Less),
        ">" => compare_values(actual, expected) == Some(Ordering::Greater),
        "<=" => matches!(compare_values(actual, expected), Some(Ordering::Less | Ordering::Equal)),
        ">=" => matches!(compare_values(actual, expected), Some(Ordering::Greater | Ordering::Equal)),
        "LIKE" => like_match(actual, expected),
        "NOT LIKE" => !like_match(actual, expected),
        "IN" => {
            if let SqlValue::List(items) = expected {
                items.iter().any(|v| eq_match(actual, v))
            } else {
                eq_match(actual, expected)
            }
        }
        _ => false,
    }
}

fn coerce_sql_to_value(sql_val: &SqlValue, target: &Value) -> Value {
    match sql_val {
        SqlValue::Null => Value::Null,
        SqlValue::String(s) => {
            match target {
                Value::Int(_) => s.parse::<i64>().map(Value::Int).unwrap_or(Value::String(s.clone())),
                Value::Float(_) => s.parse::<f64>().map(Value::Float).unwrap_or(Value::String(s.clone())),
                Value::Date(_) => {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .map(Value::Date)
                        .unwrap_or(Value::String(s.clone()))
                }
                _ => Value::String(s.clone()),
            }
        }
        SqlValue::Int(n) => {
            match target {
                Value::Float(_) => Value::Float(*n as f64),
                _ => Value::Int(*n),
            }
        }
        SqlValue::Float(f) => Value::Float(*f),
        SqlValue::List(_) => Value::Null, // Lists handled separately
    }
}

fn eq_match(actual: &Value, expected: &SqlValue) -> bool {
    // Special handling for lists (e.g., categories)
    if let Value::List(items) = actual {
        if let SqlValue::String(s) = expected {
            return items.contains(s);
        }
    }

    let coerced = coerce_sql_to_value(expected, actual);
    actual == &coerced
}

fn like_match(actual: &Value, pattern: &SqlValue) -> bool {
    let pattern_str = match pattern {
        SqlValue::String(s) => s,
        _ => return false,
    };

    // Convert SQL LIKE to regex
    let mut regex_str = String::from("(?is)^");
    for ch in pattern_str.chars() {
        match ch {
            '%' => regex_str.push_str(".*"),
            '_' => regex_str.push('.'),
            c => {
                if regex::escape(&c.to_string()) != c.to_string() {
                    regex_str.push_str(&regex::escape(&c.to_string()));
                } else {
                    regex_str.push(c);
                }
            }
        }
    }
    regex_str.push('$');

    let re = match Regex::new(&regex_str) {
        Ok(r) => r,
        Err(_) => return false,
    };

    match actual {
        Value::List(items) => items.iter().any(|item| re.is_match(item)),
        _ => re.is_match(&actual.to_display_string()),
    }
}

fn compare_values(actual: &Value, expected: &SqlValue) -> Option<Ordering> {
    let coerced = coerce_sql_to_value(expected, actual);
    actual.partial_cmp(&coerced).map(|o| o)
}

/// Convert a SqlValue to a Value for index lookups (without a target type for coercion).
fn sql_value_to_index_value(sv: &SqlValue) -> Value {
    match sv {
        SqlValue::String(s) => {
            // Try date
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Value::Date(d);
            }
            Value::String(s.clone())
        }
        SqlValue::Int(n) => Value::Int(*n),
        SqlValue::Float(f) => Value::Float(*f),
        SqlValue::Null => Value::Null,
        SqlValue::List(_) => Value::Null,
    }
}

/// Try to use B-tree indexes to narrow the candidate row set.
/// Returns Some(paths) if the entire WHERE clause could be resolved via index,
/// or None if a full scan is needed.
fn try_index_filter(
    clause: &WhereClause,
    index: &crate::index::TableIndex,
) -> Option<std::collections::HashSet<String>> {
    match clause {
        WhereClause::Comparison(cmp) => {
            if !index.has_index(&cmp.column) {
                return None;
            }
            match cmp.op.as_str() {
                "=" => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_eq(&cmp.column, &val);
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                "<" => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    // exclusive upper bound: use range with max < val
                    // lookup_range is inclusive, so we get all <= val then remove exact matches
                    let range_paths = index.lookup_range(&cmp.column, None, Some(&val));
                    let eq_paths: std::collections::HashSet<&str> = index.lookup_eq(&cmp.column, &val).into_iter().collect();
                    Some(range_paths.into_iter().filter(|p| !eq_paths.contains(p)).map(|s| s.to_string()).collect())
                }
                ">" => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let range_paths = index.lookup_range(&cmp.column, Some(&val), None);
                    let eq_paths: std::collections::HashSet<&str> = index.lookup_eq(&cmp.column, &val).into_iter().collect();
                    Some(range_paths.into_iter().filter(|p| !eq_paths.contains(p)).map(|s| s.to_string()).collect())
                }
                "<=" => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_range(&cmp.column, None, Some(&val));
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                ">=" => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_range(&cmp.column, Some(&val), None);
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                "IN" => {
                    if let Some(SqlValue::List(items)) = &cmp.value {
                        let vals: Vec<Value> = items.iter().map(sql_value_to_index_value).collect();
                        let paths = index.lookup_in(&cmp.column, &vals);
                        Some(paths.into_iter().map(|s| s.to_string()).collect())
                    } else {
                        None
                    }
                }
                _ => None, // LIKE, IS NULL, etc. can't use index
            }
        }
        WhereClause::BoolOp(bop) => {
            let left = try_index_filter(&bop.left, index);
            let right = try_index_filter(&bop.right, index);
            match bop.op.as_str() {
                "AND" => {
                    match (left, right) {
                        (Some(l), Some(r)) => Some(l.intersection(&r).cloned().collect()),
                        (Some(l), None) => Some(l), // narrow with left, scan-verify right
                        (None, Some(r)) => Some(r),
                        (None, None) => None,
                    }
                }
                "OR" => {
                    match (left, right) {
                        (Some(l), Some(r)) => Some(l.union(&r).cloned().collect()),
                        _ => None, // Can't use index if either side needs full scan
                    }
                }
                _ => None,
            }
        }
    }
}

fn sort_rows(rows: &mut Vec<Row>, specs: &[OrderSpec]) {
    rows.sort_by(|a, b| {
        for spec in specs {
            let va = a.get(&spec.column);
            let vb = b.get(&spec.column);

            // NULLs sort last
            let ordering = match (va, vb) {
                (None, None) | (Some(Value::Null), Some(Value::Null)) => Ordering::Equal,
                (None, _) | (Some(Value::Null), _) => Ordering::Greater,
                (_, None) | (_, Some(Value::Null)) => Ordering::Less,
                (Some(a_val), Some(b_val)) => {
                    a_val.partial_cmp(b_val).unwrap_or(Ordering::Equal)
                }
            };

            let ordering = if spec.descending {
                ordering.reverse()
            } else {
                ordering
            };

            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    });
}

/// Convert a SqlValue to our model Value (for use in insert/update).
pub fn sql_value_to_value(sql_val: &SqlValue) -> Value {
    match sql_val {
        SqlValue::Null => Value::Null,
        SqlValue::String(s) => Value::String(s.clone()),
        SqlValue::Int(n) => Value::Int(*n),
        SqlValue::Float(f) => Value::Float(*f),
        SqlValue::List(items) => {
            let strings: Vec<String> = items
                .iter()
                .filter_map(|v| match v {
                    SqlValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            Value::List(strings)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rows() -> Vec<Row> {
        vec![
            Row::from([
                ("path".into(), Value::String("a.md".into())),
                ("title".into(), Value::String("Alpha".into())),
                ("count".into(), Value::Int(10)),
            ]),
            Row::from([
                ("path".into(), Value::String("b.md".into())),
                ("title".into(), Value::String("Beta".into())),
                ("count".into(), Value::Int(5)),
            ]),
            Row::from([
                ("path".into(), Value::String("c.md".into())),
                ("title".into(), Value::String("Gamma".into())),
                ("count".into(), Value::Int(20)),
            ]),
        ]
    }

    #[test]
    fn test_select_all() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: None,
            order_by: None,
            limit: None,
        };
        let (rows, _cols) = execute(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_where_gt() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "count".into(),
                op: ">".into(),
                value: Some(SqlValue::Int(5)),
            })),
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_order_by_desc() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: None,
            order_by: Some(vec![OrderSpec {
                column: "count".into(),
                descending: true,
            }]),
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows(), None).unwrap();
        assert_eq!(rows[0]["count"], Value::Int(20));
        assert_eq!(rows[2]["count"], Value::Int(5));
    }

    #[test]
    fn test_limit() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: None,
            order_by: None,
            limit: Some(2),
        };
        let (rows, _) = execute(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_like() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "title".into(),
                op: "LIKE".into(),
                value: Some(SqlValue::String("%lph%".into())),
            })),
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["title"], Value::String("Alpha".into()));
    }

    #[test]
    fn test_is_null() {
        let mut rows = make_rows();
        rows[1].insert("optional".into(), Value::Null);

        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "optional".into(),
                op: "IS NULL".into(),
                value: None,
            })),
            order_by: None,
            limit: None,
        };
        let (result, _) = execute(&q, &rows, None).unwrap();
        // All rows where optional is NULL or missing
        assert_eq!(result.len(), 3);
    }
}
