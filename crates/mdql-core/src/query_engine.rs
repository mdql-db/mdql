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
    execute(query, rows)
}

pub fn execute_join_query(
    query: &SelectQuery,
    tables: &HashMap<String, (Schema, Vec<Row>)>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    let join = query.join.as_ref().ok_or_else(|| {
        MdqlError::QueryExecution("No JOIN clause in query".into())
    })?;

    let left_name = &query.table;
    let right_name = &join.table;

    let (_left_schema, left_rows) = tables.get(left_name.as_str()).ok_or_else(|| {
        MdqlError::QueryExecution(format!("Unknown table '{}'", left_name))
    })?;
    let (_right_schema, right_rows) = tables.get(right_name.as_str()).ok_or_else(|| {
        MdqlError::QueryExecution(format!("Unknown table '{}'", right_name))
    })?;

    // Build alias mapping
    let mut aliases: HashMap<String, String> = HashMap::new();
    aliases.insert(left_name.clone(), left_name.clone());
    aliases.insert(right_name.clone(), right_name.clone());
    if let Some(ref a) = query.table_alias {
        aliases.insert(a.clone(), left_name.clone());
    }
    if let Some(ref a) = join.alias {
        aliases.insert(a.clone(), right_name.clone());
    }

    // Resolve ON columns
    let (left_on_table, left_on_col) = resolve_dotted(&join.left_col, &aliases);
    let (_right_on_table, right_on_col) = resolve_dotted(&join.right_col, &aliases);

    let (join_left_col, join_right_col) = if left_on_table == *left_name {
        (left_on_col, right_on_col)
    } else {
        (right_on_col, left_on_col)
    };

    // Build index on right table
    let mut right_index: HashMap<String, Vec<&Row>> = HashMap::new();
    for r in right_rows {
        if let Some(key) = r.get(&join_right_col) {
            let key_str = key.to_display_string();
            right_index.entry(key_str).or_default().push(r);
        }
    }

    // Perform join
    let left_alias = query.table_alias.as_deref().unwrap_or(left_name);
    let right_alias = join.alias.as_deref().unwrap_or(right_name);

    let mut joined_rows: Vec<Row> = Vec::new();
    for lr in left_rows {
        if let Some(key) = lr.get(&join_left_col) {
            let key_str = key.to_display_string();
            if let Some(matching) = right_index.get(&key_str) {
                for rr in matching {
                    let mut merged = Row::new();
                    for (k, v) in lr {
                        merged.insert(format!("{}.{}", left_alias, k), v.clone());
                    }
                    for (k, v) in *rr {
                        merged.insert(format!("{}.{}", right_alias, k), v.clone());
                    }
                    joined_rows.push(merged);
                }
            }
        }
    }

    execute(query, &joined_rows)
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

    // Resolve column list
    let columns = match &query.columns {
        ColumnList::All => all_columns,
        ColumnList::Named(cols) => cols.clone(),
    };

    // Filter
    let mut result: Vec<Row> = if let Some(ref wc) = query.where_clause {
        rows.iter()
            .filter(|r| evaluate(wc, r))
            .cloned()
            .collect()
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
            join: None,
            where_clause: None,
            order_by: None,
            limit: None,
        };
        let (rows, _cols) = execute(&q, &make_rows()).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_where_gt() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            join: None,
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "count".into(),
                op: ">".into(),
                value: Some(SqlValue::Int(5)),
            })),
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows()).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_order_by_desc() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            join: None,
            where_clause: None,
            order_by: Some(vec![OrderSpec {
                column: "count".into(),
                descending: true,
            }]),
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows()).unwrap();
        assert_eq!(rows[0]["count"], Value::Int(20));
        assert_eq!(rows[2]["count"], Value::Int(5));
    }

    #[test]
    fn test_limit() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            join: None,
            where_clause: None,
            order_by: None,
            limit: Some(2),
        };
        let (rows, _) = execute(&q, &make_rows()).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_like() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            join: None,
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "title".into(),
                op: "LIKE".into(),
                value: Some(SqlValue::String("%lph%".into())),
            })),
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute(&q, &make_rows()).unwrap();
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
            join: None,
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "optional".into(),
                op: "IS NULL".into(),
                value: None,
            })),
            order_by: None,
            limit: None,
        };
        let (result, _) = execute(&q, &rows).unwrap();
        // All rows where optional is NULL or missing
        assert_eq!(result.len(), 3);
    }
}
