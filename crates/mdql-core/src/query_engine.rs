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
    if let Some(ref sub) = query.subquery {
        let (sub_rows, _sub_cols) = execute_inner(sub, rows, None)?;
        return execute_inner(query, &sub_rows, None);
    }
    execute_inner(query, rows, None)
}

#[allow(dead_code)]
pub(crate) fn execute_query_indexed(
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

#[allow(dead_code)]
fn collect_fts_results(
    clause: &WhereClause,
    schema: &Schema,
    searcher: &crate::search::TableSearcher,
) -> HashMap<(String, String), std::collections::HashSet<String>> {
    let mut results = HashMap::new();
    collect_fts_results_inner(clause, schema, searcher, &mut results);
    results
}

#[allow(dead_code)]
fn collect_fts_results_inner(
    clause: &WhereClause,
    schema: &Schema,
    searcher: &crate::search::TableSearcher,
    results: &mut HashMap<(String, String), std::collections::HashSet<String>>,
) {
    match clause {
        WhereClause::Comparison(cmp) => {
            if (cmp.op == CmpOp::Like || cmp.op == CmpOp::NotLike) && schema.sections.contains_key(&cmp.column) {
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

    // Check if query has aggregates
    let has_aggregates = match &query.columns {
        ColumnList::Named(exprs) => exprs.iter().any(|e| e.is_aggregate()),
        _ => false,
    };

    // Output column names
    let columns: Vec<String> = match &query.columns {
        ColumnList::All => all_columns,
        ColumnList::Named(exprs) => exprs.iter().map(|e| e.output_name()).collect(),
    };

    // Filter — try index first, fall back to full scan
    let filtered: Vec<Row> = if let Some(ref wc) = query.where_clause {
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

    // Aggregate if needed
    let mut result = if has_aggregates || query.group_by.is_some() {
        let exprs = match &query.columns {
            ColumnList::Named(exprs) => exprs.clone(),
            _ => return Err(MdqlError::QueryExecution(
                "SELECT * with GROUP BY is not supported".into(),
            )),
        };
        let group_keys = query.group_by.as_deref().unwrap_or(&[]);
        aggregate_rows(&filtered, &exprs, group_keys)?
    } else {
        filtered
    };

    // HAVING filter — apply after aggregation
    if let Some(ref having) = query.having {
        result.retain(|row| evaluate(having, row));
    }

    // Sort — resolve ORDER BY aliases against SELECT list
    if let Some(ref order_by) = query.order_by {
        let resolved = resolve_order_aliases(order_by, &query.columns);
        sort_rows(&mut result, &resolved);
    }

    // Limit
    if let Some(limit) = query.limit {
        result.truncate(limit as usize);
    }

    // Project — evaluate expressions and strip to requested columns
    if !matches!(query.columns, ColumnList::All) {
        let named_exprs = match &query.columns {
            ColumnList::Named(exprs) => exprs,
            _ => unreachable!(),
        };

        // Compute expression columns first, then retain only requested columns.
        // Skip if aggregation already computed them (re-evaluating would lose
        // columns that only existed in pre-aggregation rows, e.g. dict fields).
        let has_expr_cols = named_exprs.iter().any(|e| matches!(e, SelectExpr::Expr { .. }));
        let already_aggregated = has_aggregates || query.group_by.is_some();
        if has_expr_cols && !already_aggregated {
            for row in &mut result {
                for expr in named_exprs {
                    if let SelectExpr::Expr { expr: e, alias } = expr {
                        let name = alias.clone().unwrap_or_else(|| e.display_name());
                        let val = evaluate_expr(e, row);
                        row.insert(name, val);
                    }
                }
            }
        }

        let col_set: std::collections::HashSet<&str> =
            columns.iter().map(|s| s.as_str()).collect();
        for row in &mut result {
            row.retain(|k, _| col_set.contains(k.as_str()));
        }
    }

    Ok((result, columns))
}

fn aggregate_rows(
    rows: &[Row],
    exprs: &[SelectExpr],
    group_keys: &[String],
) -> crate::errors::Result<Vec<Row>> {
    // Group rows by group_keys
    let mut groups: Vec<(Vec<Value>, Vec<&Row>)> = Vec::new();
    let mut key_index: HashMap<Vec<String>, usize> = HashMap::new();

    if group_keys.is_empty() {
        // No GROUP BY — all rows are one group
        let all_refs: Vec<&Row> = rows.iter().collect();
        groups.push((vec![], all_refs));
    } else {
        for row in rows {
            let key: Vec<String> = group_keys
                .iter()
                .map(|k| {
                    row.get(k)
                        .map(|v| v.to_display_string())
                        .unwrap_or_default()
                })
                .collect();
            let key_vals: Vec<Value> = group_keys
                .iter()
                .map(|k| row.get(k).cloned().unwrap_or(Value::Null))
                .collect();
            if let Some(&idx) = key_index.get(&key) {
                groups[idx].1.push(row);
            } else {
                let idx = groups.len();
                key_index.insert(key, idx);
                groups.push((key_vals, vec![row]));
            }
        }
    }

    // Compute aggregates per group
    let mut result = Vec::new();
    for (key_vals, group_rows) in &groups {
        let mut out = Row::new();

        // Fill in group key values
        for (i, k) in group_keys.iter().enumerate() {
            out.insert(k.clone(), key_vals[i].clone());
        }

        // Compute each expression
        for expr in exprs {
            match expr {
                SelectExpr::Column(name) => {
                    // Already filled if it's a group key; otherwise take first row's value
                    if !out.contains_key(name) {
                        if let Some(first) = group_rows.first() {
                            out.insert(
                                name.clone(),
                                first.get(name).cloned().unwrap_or(Value::Null),
                            );
                        }
                    }
                }
                SelectExpr::Aggregate { func, arg, arg_expr, alias } => {
                    let out_name = alias
                        .clone()
                        .unwrap_or_else(|| expr.output_name());
                    let val = compute_aggregate(func, arg, arg_expr.as_ref(), group_rows);
                    out.insert(out_name, val);
                }
                SelectExpr::Expr { expr: e, alias } => {
                    let out_name = alias.clone().unwrap_or_else(|| e.display_name());
                    if e.contains_aggregate() {
                        let val = evaluate_agg_expr(e, group_rows);
                        out.insert(out_name, val);
                    } else if let Some(first) = group_rows.first() {
                        let val = evaluate_expr(e, first);
                        out.insert(out_name, val);
                    }
                }
            }
        }

        result.push(out);
    }

    Ok(result)
}

/// Resolve a per-row value for an aggregate argument.
/// If `arg_expr` is set, evaluate it; otherwise look up `arg` as a column name.
fn resolve_agg_value<'a>(arg: &str, arg_expr: Option<&Expr>, row: &'a Row) -> Value {
    if let Some(expr) = arg_expr {
        evaluate_expr(expr, row)
    } else {
        row.get(arg).cloned().unwrap_or(Value::Null)
    }
}

fn compute_aggregate(func: &AggFunc, arg: &str, arg_expr: Option<&Expr>, rows: &[&Row]) -> Value {
    match func {
        AggFunc::Count => {
            if arg == "*" && arg_expr.is_none() {
                Value::Int(rows.len() as i64)
            } else {
                let count = rows
                    .iter()
                    .filter(|r| {
                        let v = resolve_agg_value(arg, arg_expr, r);
                        !v.is_null()
                    })
                    .count();
                Value::Int(count as i64)
            }
        }
        AggFunc::Sum => {
            let mut total = 0.0f64;
            let mut has_any = false;
            for r in rows {
                let v = resolve_agg_value(arg, arg_expr, r);
                match v {
                    Value::Int(n) => { total += n as f64; has_any = true; }
                    Value::Float(f) => { total += f; has_any = true; }
                    _ => {}
                }
            }
            if has_any { Value::Float(total) } else { Value::Null }
        }
        AggFunc::Avg => {
            let mut total = 0.0f64;
            let mut count = 0usize;
            for r in rows {
                let v = resolve_agg_value(arg, arg_expr, r);
                match v {
                    Value::Int(n) => { total += n as f64; count += 1; }
                    Value::Float(f) => { total += f; count += 1; }
                    _ => {}
                }
            }
            if count > 0 { Value::Float(total / count as f64) } else { Value::Null }
        }
        AggFunc::Min => {
            let mut min_val: Option<Value> = None;
            for r in rows {
                let v = resolve_agg_value(arg, arg_expr, r);
                if v.is_null() { continue; }
                min_val = Some(match min_val {
                    None => v,
                    Some(ref current) => {
                        if v.partial_cmp(current) == Some(std::cmp::Ordering::Less) {
                            v
                        } else {
                            current.clone()
                        }
                    }
                });
            }
            min_val.unwrap_or(Value::Null)
        }
        AggFunc::Max => {
            let mut max_val: Option<Value> = None;
            for r in rows {
                let v = resolve_agg_value(arg, arg_expr, r);
                if v.is_null() { continue; }
                max_val = Some(match max_val {
                    None => v,
                    Some(ref current) => {
                        if v.partial_cmp(current) == Some(std::cmp::Ordering::Greater) {
                            v
                        } else {
                            current.clone()
                        }
                    }
                });
            }
            max_val.unwrap_or(Value::Null)
        }
    }
}

fn evaluate_with_fts(clause: &WhereClause, row: &Row, fts: &FtsResults) -> bool {
    match clause {
        WhereClause::BoolOp(bop) => {
            let left = evaluate_with_fts(&bop.left, row, fts);
            match bop.op {
                BoolOpKind::And => left && evaluate_with_fts(&bop.right, row, fts),
                BoolOpKind::Or => left || evaluate_with_fts(&bop.right, row, fts),
            }
        }
        WhereClause::Comparison(cmp) => {
            // Check if we have FTS results for this comparison
            if cmp.op == CmpOp::Like || cmp.op == CmpOp::NotLike {
                if let Some(SqlValue::String(pattern)) = &cmp.value {
                    let key = (cmp.column.clone(), pattern.clone());
                    if let Some(matching_paths) = fts.get(&key) {
                        let row_path = row.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let matched = matching_paths.contains(row_path);
                        return if cmp.op == CmpOp::Like { matched } else { !matched };
                    }
                }
            }
            evaluate_comparison(cmp, row)
        }
    }
}

pub use crate::query_join::execute_join_query;

pub(crate) fn execute_inner(
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
            match bop.op {
                BoolOpKind::And => left && evaluate(&bop.right, row),
                BoolOpKind::Or => left || evaluate(&bop.right, row),
            }
        }
        WhereClause::Comparison(cmp) => evaluate_comparison(cmp, row),
    }
}

/// Evaluate an Expr against a row, returning a Value.
pub fn evaluate_expr(expr: &Expr, row: &Row) -> Value {
    match expr {
        Expr::Literal(SqlValue::Int(n)) => Value::Int(*n),
        Expr::Literal(SqlValue::Float(f)) => Value::Float(*f),
        Expr::Literal(SqlValue::String(s)) => Value::String(s.clone()),
        Expr::Literal(SqlValue::Null) => Value::Null,
        Expr::Literal(SqlValue::List(_)) => Value::Null,
        Expr::Column(name) => {
            if let Some(val) = row.get(name) {
                return val.clone();
            }
            // Try all possible dot splits for dict access (e.g. "s.params.key")
            for (i, _) in name.match_indices('.') {
                let dict_col = &name[..i];
                let dict_key = &name[i + 1..];
                if let Some(Value::Dict(map)) = row.get(dict_col) {
                    return map.get(dict_key).cloned().unwrap_or(Value::Null);
                }
            }
            Value::Null
        }
        Expr::UnaryMinus(inner) => {
            match evaluate_expr(inner, row) {
                Value::Int(n) => Value::Int(-n),
                Value::Float(f) => Value::Float(-f),
                Value::Null => Value::Null,
                _ => Value::Null, // non-numeric → NULL
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let lv = evaluate_expr(left, row);
            let rv = evaluate_expr(right, row);

            // NULL propagation: any NULL operand → NULL
            if lv.is_null() || rv.is_null() {
                return Value::Null;
            }

            // Extract numeric values with int→float coercion
            match (&lv, &rv) {
                (Value::Int(a), Value::Int(b)) => {
                    match op {
                        ArithOp::Add => Value::Int(a.wrapping_add(*b)),
                        ArithOp::Sub => Value::Int(a.wrapping_sub(*b)),
                        ArithOp::Mul => Value::Int(a.wrapping_mul(*b)),
                        ArithOp::Div => {
                            if *b == 0 { Value::Null } else { Value::Int(a / b) }
                        }
                        ArithOp::Mod => {
                            if *b == 0 { Value::Null } else { Value::Int(a % b) }
                        }
                    }
                }
                _ => {
                    // Coerce to float
                    let a = match &lv {
                        Value::Int(n) => *n as f64,
                        Value::Float(f) => *f,
                        _ => return Value::Null,
                    };
                    let b = match &rv {
                        Value::Int(n) => *n as f64,
                        Value::Float(f) => *f,
                        _ => return Value::Null,
                    };
                    match op {
                        ArithOp::Add => Value::Float(a + b),
                        ArithOp::Sub => Value::Float(a - b),
                        ArithOp::Mul => Value::Float(a * b),
                        ArithOp::Div => {
                            if b == 0.0 { Value::Null } else { Value::Float(a / b) }
                        }
                        ArithOp::Mod => {
                            if b == 0.0 { Value::Null } else { Value::Float(a % b) }
                        }
                    }
                }
            }
        }
        Expr::Case { whens, else_expr } => {
            for (condition, result) in whens {
                if evaluate(condition, row) {
                    return evaluate_expr(result, row);
                }
            }
            match else_expr {
                Some(e) => evaluate_expr(e, row),
                None => Value::Null,
            }
        }
        Expr::CurrentDate => {
            Value::Date(chrono::Local::now().naive_local().date())
        }
        Expr::CurrentTimestamp => {
            Value::DateTime(chrono::Local::now().naive_local())
        }
        Expr::DateAdd { date, days } => {
            let date_val = evaluate_expr(date, row);
            let days_val = evaluate_expr(days, row);
            let n = match &days_val {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => return Value::Null,
            };
            let duration = chrono::Duration::days(n);
            match date_val {
                Value::Date(d) => {
                    match d.checked_add_signed(duration) {
                        Some(result) => Value::Date(result),
                        None => Value::Null,
                    }
                }
                Value::DateTime(dt) => {
                    match dt.checked_add_signed(duration) {
                        Some(result) => Value::DateTime(result),
                        None => Value::Null,
                    }
                }
                _ => Value::Null,
            }
        }
        Expr::DateDiff { left, right } => {
            let lv = evaluate_expr(left, row);
            let rv = evaluate_expr(right, row);
            let left_date = match &lv {
                Value::Date(d) => d.and_hms_opt(0, 0, 0).unwrap(),
                Value::DateTime(dt) => *dt,
                _ => return Value::Null,
            };
            let right_date = match &rv {
                Value::Date(d) => d.and_hms_opt(0, 0, 0).unwrap(),
                Value::DateTime(dt) => *dt,
                _ => return Value::Null,
            };
            Value::Int((left_date - right_date).num_days())
        }
        Expr::Aggregate { func, arg, .. } => {
            // Post-aggregation: look up the pre-computed column name
            let func_name = match func {
                AggFunc::Count => "COUNT",
                AggFunc::Sum => "SUM",
                AggFunc::Avg => "AVG",
                AggFunc::Min => "MIN",
                AggFunc::Max => "MAX",
            };
            let col = format!("{}({})", func_name, arg);
            row.get(&col).cloned().unwrap_or(Value::Null)
        }
    }
}

fn evaluate_agg_expr(expr: &Expr, group_rows: &[&Row]) -> Value {
    match expr {
        Expr::Aggregate { func, arg, arg_expr } => {
            compute_aggregate(func, arg, arg_expr.as_deref(), group_rows)
        }
        Expr::BinaryOp { left, op, right } => {
            let lv = evaluate_agg_expr(left, group_rows);
            let rv = evaluate_agg_expr(right, group_rows);
            apply_arith_op(op, &lv, &rv)
        }
        Expr::UnaryMinus(inner) => {
            match evaluate_agg_expr(inner, group_rows) {
                Value::Int(n) => Value::Int(-n),
                Value::Float(f) => Value::Float(-f),
                _ => Value::Null,
            }
        }
        other => {
            if let Some(first) = group_rows.first() {
                evaluate_expr(other, first)
            } else {
                Value::Null
            }
        }
    }
}

fn apply_arith_op(op: &ArithOp, lv: &Value, rv: &Value) -> Value {
    if lv.is_null() || rv.is_null() {
        return Value::Null;
    }
    match (lv, rv) {
        (Value::Int(a), Value::Int(b)) => match op {
            ArithOp::Add => Value::Int(a.wrapping_add(*b)),
            ArithOp::Sub => Value::Int(a.wrapping_sub(*b)),
            ArithOp::Mul => Value::Int(a.wrapping_mul(*b)),
            ArithOp::Div => if *b == 0 { Value::Null } else { Value::Int(a / b) },
            ArithOp::Mod => if *b == 0 { Value::Null } else { Value::Int(a % b) },
        },
        _ => {
            let a = match lv {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Value::Null,
            };
            let b = match rv {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => return Value::Null,
            };
            match op {
                ArithOp::Add => Value::Float(a + b),
                ArithOp::Sub => Value::Float(a - b),
                ArithOp::Mul => Value::Float(a * b),
                ArithOp::Div => if b == 0.0 { Value::Null } else { Value::Float(a / b) },
                ArithOp::Mod => if b == 0.0 { Value::Null } else { Value::Float(a % b) },
            }
        }
    }
}

fn evaluate_comparison(cmp: &Comparison, row: &Row) -> bool {
    // If we have expression-based comparison (new path), use it for standard ops
    if let (Some(left_expr), Some(right_expr)) = (&cmp.left_expr, &cmp.right_expr) {
        if matches!(cmp.op, CmpOp::Eq | CmpOp::Ne | CmpOp::Lt | CmpOp::Gt | CmpOp::Le | CmpOp::Ge) {
            let left_val = evaluate_expr(left_expr, row);
            let right_val = evaluate_expr(right_expr, row);

            // NULL comparison: always false (except IS NULL handled below)
            if left_val.is_null() || right_val.is_null() {
                return false;
            }

            // Coerce for comparison: if types differ, try int→float
            let ord = compare_model_values(&left_val, &right_val);

            return match cmp.op {
                CmpOp::Eq => ord == Some(Ordering::Equal),
                CmpOp::Ne => ord != Some(Ordering::Equal),
                CmpOp::Lt => ord == Some(Ordering::Less),
                CmpOp::Gt => ord == Some(Ordering::Greater),
                CmpOp::Le => matches!(ord, Some(Ordering::Less | Ordering::Equal)),
                CmpOp::Ge => matches!(ord, Some(Ordering::Greater | Ordering::Equal)),
                _ => false,
            };
        }
    }

    // Fall back to legacy column-based comparison for IS NULL, IN, LIKE, etc.
    let actual = row.get(&cmp.column);

    if cmp.op == CmpOp::IsNull {
        return actual.map_or(true, |v| v.is_null());
    }
    if cmp.op == CmpOp::IsNotNull {
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

    match cmp.op {
        CmpOp::Eq => eq_match(actual, expected),
        CmpOp::Ne => !eq_match(actual, expected),
        CmpOp::Lt => compare_values(actual, expected) == Some(Ordering::Less),
        CmpOp::Gt => compare_values(actual, expected) == Some(Ordering::Greater),
        CmpOp::Le => matches!(compare_values(actual, expected), Some(Ordering::Less | Ordering::Equal)),
        CmpOp::Ge => matches!(compare_values(actual, expected), Some(Ordering::Greater | Ordering::Equal)),
        CmpOp::Like => like_match(actual, expected),
        CmpOp::NotLike => !like_match(actual, expected),
        CmpOp::In => {
            if let SqlValue::List(items) = expected {
                items.iter().any(|v| eq_match(actual, v))
            } else {
                eq_match(actual, expected)
            }
        }
        CmpOp::IsNull | CmpOp::IsNotNull => unreachable!(),
    }
}

/// Compare two model::Value instances, with int↔float coercion.
fn compare_model_values(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)),
        _ => a.partial_cmp(b),
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
                Value::DateTime(_) => {
                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
                        .map(Value::DateTime)
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
    actual.partial_cmp(&coerced)
}

/// Convert a SqlValue to a Value for index lookups (without a target type for coercion).
fn sql_value_to_index_value(sv: &SqlValue) -> Value {
    match sv {
        SqlValue::String(s) => {
            // Try datetime first (more specific)
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                return Value::DateTime(dt);
            }
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                return Value::DateTime(dt);
            }
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
            match cmp.op {
                CmpOp::Eq => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_eq(&cmp.column, &val);
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                CmpOp::Lt => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    // exclusive upper bound: use range with max < val
                    // lookup_range is inclusive, so we get all <= val then remove exact matches
                    let range_paths = index.lookup_range(&cmp.column, None, Some(&val));
                    let eq_paths: std::collections::HashSet<&str> = index.lookup_eq(&cmp.column, &val).into_iter().collect();
                    Some(range_paths.into_iter().filter(|p| !eq_paths.contains(p)).map(|s| s.to_string()).collect())
                }
                CmpOp::Gt => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let range_paths = index.lookup_range(&cmp.column, Some(&val), None);
                    let eq_paths: std::collections::HashSet<&str> = index.lookup_eq(&cmp.column, &val).into_iter().collect();
                    Some(range_paths.into_iter().filter(|p| !eq_paths.contains(p)).map(|s| s.to_string()).collect())
                }
                CmpOp::Le => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_range(&cmp.column, None, Some(&val));
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                CmpOp::Ge => {
                    let val = sql_value_to_index_value(cmp.value.as_ref()?);
                    let paths = index.lookup_range(&cmp.column, Some(&val), None);
                    Some(paths.into_iter().map(|s| s.to_string()).collect())
                }
                CmpOp::In => {
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
            match bop.op {
                BoolOpKind::And => {
                    match (left, right) {
                        (Some(l), Some(r)) => Some(l.intersection(&r).cloned().collect()),
                        (Some(l), None) => Some(l), // narrow with left, scan-verify right
                        (None, Some(r)) => Some(r),
                        (None, None) => None,
                    }
                }
                BoolOpKind::Or => {
                    match (left, right) {
                        (Some(l), Some(r)) => Some(l.union(&r).cloned().collect()),
                        _ => None, // Can't use index if either side needs full scan
                    }
                }
            }
        }
    }
}

/// If an ORDER BY column matches a SELECT alias, replace its expr with the
/// aliased expression so sorting uses the computed value.
fn resolve_order_aliases(specs: &[OrderSpec], columns: &ColumnList) -> Vec<OrderSpec> {
    let named = match columns {
        ColumnList::Named(exprs) => exprs,
        _ => return specs.to_vec(),
    };

    // Build alias → expr map
    let alias_map: HashMap<String, &Expr> = named
        .iter()
        .filter_map(|se| match se {
            SelectExpr::Expr { expr, alias: Some(a) } => Some((a.clone(), expr)),
            _ => None,
        })
        .collect();

    specs
        .iter()
        .map(|spec| {
            // If the ORDER BY column name matches a SELECT alias, use that expression
            if let Some(expr) = alias_map.get(&spec.column) {
                OrderSpec {
                    column: spec.column.clone(),
                    expr: Some((*expr).clone()),
                    descending: spec.descending,
                }
            } else {
                spec.clone()
            }
        })
        .collect()
}

fn sort_rows(rows: &mut Vec<Row>, specs: &[OrderSpec]) {
    rows.sort_by(|a, b| {
        for spec in specs {
            let (va, vb) = if let Some(ref expr) = spec.expr {
                (evaluate_expr(expr, a), evaluate_expr(expr, b))
            } else {
                (
                    a.get(&spec.column).cloned().unwrap_or(Value::Null),
                    b.get(&spec.column).cloned().unwrap_or(Value::Null),
                )
            };

            // NULLs sort last
            let ordering = match (&va, &vb) {
                (Value::Null, Value::Null) => Ordering::Equal,
                (Value::Null, _) => Ordering::Greater,
                (_, Value::Null) => Ordering::Less,
                (a_val, b_val) => {
                    compare_model_values(a_val, b_val).unwrap_or(Ordering::Equal)
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
            subquery: None,
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        };
        let (rows, _cols) = execute_inner(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn test_where_gt() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            subquery: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "count".into(),
                op: CmpOp::Gt,
                value: Some(SqlValue::Int(5)),
                left_expr: Some(Expr::Column("count".into())),
                right_expr: Some(Expr::Literal(SqlValue::Int(5))),
            })),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_order_by_desc() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            subquery: None,
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: Some(vec![OrderSpec {
                column: "count".into(),
                expr: Some(Expr::Column("count".into())),
                descending: true,
            }]),
            limit: None,
        };
        let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
        assert_eq!(rows[0]["count"], Value::Int(20));
        assert_eq!(rows[2]["count"], Value::Int(5));
    }

    #[test]
    fn test_limit() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            subquery: None,
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: Some(2),
        };
        let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_like() {
        let q = SelectQuery {
            columns: ColumnList::All,
            table: "test".into(),
            table_alias: None,
            subquery: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "title".into(),
                op: CmpOp::Like,
                value: Some(SqlValue::String("%lph%".into())),
                left_expr: Some(Expr::Column("title".into())),
                right_expr: None,
            })),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        };
        let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
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
            subquery: None,
            joins: vec![],
            where_clause: Some(WhereClause::Comparison(Comparison {
                column: "optional".into(),
                op: CmpOp::IsNull,
                value: None,
                left_expr: Some(Expr::Column("optional".into())),
                right_expr: None,
            })),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
        };
        let (result, _) = execute_inner(&q, &rows, None).unwrap();
        // All rows where optional is NULL or missing
        assert_eq!(result.len(), 3);
    }

    // ── Expression evaluation tests ─────────────────────────��─────

    #[test]
    fn test_evaluate_expr_literal() {
        let row = Row::new();
        assert_eq!(evaluate_expr(&Expr::Literal(SqlValue::Int(42)), &row), Value::Int(42));
        assert_eq!(evaluate_expr(&Expr::Literal(SqlValue::Float(3.14)), &row), Value::Float(3.14));
        assert_eq!(evaluate_expr(&Expr::Literal(SqlValue::Null), &row), Value::Null);
    }

    #[test]
    fn test_evaluate_expr_column() {
        let row = Row::from([("x".into(), Value::Int(10))]);
        assert_eq!(evaluate_expr(&Expr::Column("x".into()), &row), Value::Int(10));
        assert_eq!(evaluate_expr(&Expr::Column("missing".into()), &row), Value::Null);
    }

    #[test]
    fn test_evaluate_expr_int_arithmetic() {
        let row = Row::from([("a".into(), Value::Int(10)), ("b".into(), Value::Int(3))]);
        let add = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Add,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&add, &row), Value::Int(13));

        let sub = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Sub,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&sub, &row), Value::Int(7));

        let mul = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Mul,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&mul, &row), Value::Int(30));

        let div = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Div,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&div, &row), Value::Int(3)); // integer division

        let modulo = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Mod,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&modulo, &row), Value::Int(1));
    }

    #[test]
    fn test_evaluate_expr_float_coercion() {
        let row = Row::from([("a".into(), Value::Int(10)), ("b".into(), Value::Float(3.0))]);
        let add = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Add,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&add, &row), Value::Float(13.0));
    }

    #[test]
    fn test_evaluate_expr_null_propagation() {
        let row = Row::from([("a".into(), Value::Int(10))]);
        let add = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Add,
            right: Box::new(Expr::Column("missing".into())),
        };
        assert_eq!(evaluate_expr(&add, &row), Value::Null);
    }

    #[test]
    fn test_evaluate_expr_div_by_zero() {
        let row = Row::from([("a".into(), Value::Int(10)), ("b".into(), Value::Int(0))]);
        let div = Expr::BinaryOp {
            left: Box::new(Expr::Column("a".into())),
            op: ArithOp::Div,
            right: Box::new(Expr::Column("b".into())),
        };
        assert_eq!(evaluate_expr(&div, &row), Value::Null);
    }

    #[test]
    fn test_evaluate_expr_unary_minus() {
        let row = Row::from([("x".into(), Value::Int(5))]);
        let neg = Expr::UnaryMinus(Box::new(Expr::Column("x".into())));
        assert_eq!(evaluate_expr(&neg, &row), Value::Int(-5));
    }

    #[test]
    fn test_select_with_expression() {
        // Integration test: SELECT count * 2 AS doubled FROM test
        let stmt = crate::query_parser::parse_query(
            "SELECT count * 2 AS doubled FROM test"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, cols) = execute_inner(&q, &make_rows(), None).unwrap();
            assert_eq!(cols, vec!["doubled"]);
            assert_eq!(rows.len(), 3);
            // Rows are: count=10, count=5, count=20
            let values: Vec<Value> = rows.iter().map(|r| r["doubled"].clone()).collect();
            assert!(values.contains(&Value::Int(20)));
            assert!(values.contains(&Value::Int(10)));
            assert!(values.contains(&Value::Int(40)));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_where_with_expression() {
        // SELECT * FROM test WHERE count * 2 > 15
        let stmt = crate::query_parser::parse_query(
            "SELECT * FROM test WHERE count * 2 > 15"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
            // count=10 → 20 > 15 ✓, count=5 → 10 > 15 ✗, count=20 → 40 > 15 ✓
            assert_eq!(rows.len(), 2);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_order_by_expression() {
        // SELECT * FROM test ORDER BY count * -1 ASC (effectively DESC by count)
        let stmt = crate::query_parser::parse_query(
            "SELECT title, count FROM test ORDER BY count * -1 ASC"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
            // count: 20 → -20, 10 → -10, 5 → -5, ASC means -20, -10, -5
            assert_eq!(rows[0]["count"], Value::Int(20));
            assert_eq!(rows[1]["count"], Value::Int(10));
            assert_eq!(rows[2]["count"], Value::Int(5));
        } else {
            panic!("Expected Select");
        }
    }

    // ── CASE WHEN evaluation tests ────────────────────────────────

    #[test]
    fn test_case_when_eval_basic() {
        let row = Row::from([("status".into(), Value::String("ACTIVE".into()))]);
        let expr = Expr::Case {
            whens: vec![(
                WhereClause::Comparison(Comparison {
                    column: "status".into(),
                    op: CmpOp::Eq,
                    value: Some(SqlValue::String("ACTIVE".into())),
                    left_expr: Some(Expr::Column("status".into())),
                    right_expr: Some(Expr::Literal(SqlValue::String("ACTIVE".into()))),
                }),
                Box::new(Expr::Literal(SqlValue::Int(1))),
            )],
            else_expr: Some(Box::new(Expr::Literal(SqlValue::Int(0)))),
        };
        assert_eq!(evaluate_expr(&expr, &row), Value::Int(1));
    }

    #[test]
    fn test_case_when_eval_else() {
        let row = Row::from([("status".into(), Value::String("KILLED".into()))]);
        let expr = Expr::Case {
            whens: vec![(
                WhereClause::Comparison(Comparison {
                    column: "status".into(),
                    op: CmpOp::Eq,
                    value: Some(SqlValue::String("ACTIVE".into())),
                    left_expr: Some(Expr::Column("status".into())),
                    right_expr: Some(Expr::Literal(SqlValue::String("ACTIVE".into()))),
                }),
                Box::new(Expr::Literal(SqlValue::Int(1))),
            )],
            else_expr: Some(Box::new(Expr::Literal(SqlValue::Int(0)))),
        };
        assert_eq!(evaluate_expr(&expr, &row), Value::Int(0));
    }

    #[test]
    fn test_case_when_eval_no_else_null() {
        let row = Row::from([("x".into(), Value::Int(99))]);
        let expr = Expr::Case {
            whens: vec![(
                WhereClause::Comparison(Comparison {
                    column: "x".into(),
                    op: CmpOp::Eq,
                    value: Some(SqlValue::Int(1)),
                    left_expr: Some(Expr::Column("x".into())),
                    right_expr: Some(Expr::Literal(SqlValue::Int(1))),
                }),
                Box::new(Expr::Literal(SqlValue::String("one".into()))),
            )],
            else_expr: None,
        };
        assert_eq!(evaluate_expr(&expr, &row), Value::Null);
    }

    #[test]
    fn test_case_when_in_aggregate_query() {
        // SUM(CASE WHEN count > 5 THEN count ELSE 0 END)
        // Rows: count=10, count=5, count=20 → should sum 10 + 0 + 20 = 30
        let stmt = crate::query_parser::parse_query(
            "SELECT SUM(CASE WHEN count > 5 THEN count ELSE 0 END) AS total FROM test"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, cols) = execute_inner(&q, &make_rows(), None).unwrap();
            assert_eq!(cols, vec!["total"]);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["total"], Value::Float(30.0));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_case_when_with_unary_minus_in_aggregate() {
        // SUM(CASE WHEN title = 'Alpha' THEN count ELSE -count END)
        // Alpha: 10, Beta: -5, Gamma: -20 → 10 - 5 - 20 = -15
        let stmt = crate::query_parser::parse_query(
            "SELECT SUM(CASE WHEN title = 'Alpha' THEN count ELSE -count END) AS net FROM test"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, _) = execute_inner(&q, &make_rows(), None).unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["net"], Value::Float(-15.0));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_dateadd_with_dict_in_group_by() {
        // Simulate a joined row with a dict field, then GROUP BY + DateAdd expr
        use indexmap::IndexMap;
        let mut params = IndexMap::new();
        params.insert("exit_days".to_string(), Value::Int(21));

        let rows = vec![
            Row::from([
                ("o.token".into(), Value::String("BTC".into())),
                ("o.event_date".into(), Value::Date(
                    chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
                )),
                ("o.size".into(), Value::Int(100)),
                ("s.params".into(), Value::Dict(params.clone())),
            ]),
            Row::from([
                ("o.token".into(), Value::String("BTC".into())),
                ("o.event_date".into(), Value::Date(
                    chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
                )),
                ("o.size".into(), Value::Int(50)),
                ("s.params".into(), Value::Dict(params.clone())),
            ]),
        ];

        let q = SelectQuery {
            columns: ColumnList::Named(vec![
                SelectExpr::Column("o.token".into()),
                SelectExpr::Column("o.event_date".into()),
                SelectExpr::Expr {
                    expr: Expr::DateAdd {
                        date: Box::new(Expr::Column("o.event_date".into())),
                        days: Box::new(Expr::Column("s.params.exit_days".into())),
                    },
                    alias: Some("exit_date".into()),
                },
                SelectExpr::Aggregate {
                    func: AggFunc::Sum,
                    arg: "o.size".into(),
                    arg_expr: Some(Expr::Column("o.size".into())),
                    alias: Some("total".into()),
                },
            ]),
            table: "orders".into(),
            table_alias: None,
            subquery: None,
            joins: vec![],
            where_clause: None,
            group_by: Some(vec!["o.token".into(), "o.event_date".into()]),
            having: None,
            order_by: None,
            limit: None,
        };

        let (rows, cols) = execute_inner(&q, &rows, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(cols.contains(&"exit_date".to_string()));
        assert_eq!(rows[0]["total"], Value::Float(150.0));
        // The key test: exit_date should be 2026-01-22, not Null
        assert_eq!(
            rows[0]["exit_date"],
            Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 1, 22).unwrap())
        );
    }

    #[test]
    fn test_aggregate_arithmetic() {
        // SUM(count) for all rows = 10 + 5 + 20 = 35
        // COUNT(*) = 3
        // SUM produces Float, COUNT produces Int → mixed → Float division
        let stmt = crate::query_parser::parse_query(
            "SELECT SUM(count) / COUNT(*) AS avg_count FROM test"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, cols) = execute_inner(&q, &make_rows(), None).unwrap();
            assert_eq!(cols, vec!["avg_count"]);
            assert_eq!(rows.len(), 1);
            match &rows[0]["avg_count"] {
                Value::Float(f) => assert!((f - 11.666666666666666).abs() < 0.001),
                other => panic!("Expected Float, got {:?}", other),
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_aggregate_subtraction_with_group_by() {
        let rows = vec![
            {
                let mut r = Row::new();
                r.insert("token".into(), Value::String("BTC".into()));
                r.insert("side".into(), Value::String("BUY".into()));
                r.insert("size".into(), Value::Float(100.0));
                r
            },
            {
                let mut r = Row::new();
                r.insert("token".into(), Value::String("BTC".into()));
                r.insert("side".into(), Value::String("SELL".into()));
                r.insert("size".into(), Value::Float(60.0));
                r
            },
        ];
        let stmt = crate::query_parser::parse_query(
            "SELECT token, SUM(CASE WHEN side = 'BUY' THEN size ELSE 0 END) - SUM(CASE WHEN side = 'SELL' THEN size ELSE 0 END) AS net FROM test GROUP BY token"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (result, _) = execute_inner(&q, &rows, None).unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0]["net"], Value::Float(40.0));
        } else {
            panic!("Expected Select");
        }
    }

    // ── Issue #42: Aggregate subtraction without GROUP BY ──

    #[test]
    fn test_aggregate_subtraction_no_group() {
        // SUM(count) = 10 + 5 + 20 = 35, COUNT(*) = 3, diff = 35 - 3 = 32
        let stmt = crate::query_parser::parse_query(
            "SELECT SUM(count) - COUNT(*) as diff FROM test"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (rows, cols) = execute_inner(&q, &make_rows(), None).unwrap();
            assert_eq!(cols, vec!["diff"]);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["diff"], Value::Float(32.0));
        } else {
            panic!("Expected Select");
        }
    }

    // ── Issue #42: Aggregate division with GROUP BY ──

    #[test]
    fn test_aggregate_division_with_group_by() {
        let rows = vec![
            {
                let mut r = Row::new();
                r.insert("category".into(), Value::String("A".into()));
                r.insert("count".into(), Value::Int(10));
                r
            },
            {
                let mut r = Row::new();
                r.insert("category".into(), Value::String("A".into()));
                r.insert("count".into(), Value::Int(20));
                r
            },
            {
                let mut r = Row::new();
                r.insert("category".into(), Value::String("B".into()));
                r.insert("count".into(), Value::Int(6));
                r
            },
        ];
        // Group A: SUM(count)=30, COUNT(*)=2, ratio=15.0
        // Group B: SUM(count)=6, COUNT(*)=1, ratio=6.0
        let stmt = crate::query_parser::parse_query(
            "SELECT category, SUM(count) / COUNT(*) as ratio FROM test GROUP BY category"
        ).unwrap();
        if let crate::query_parser::Statement::Select(q) = stmt {
            let (result, cols) = execute_inner(&q, &rows, None).unwrap();
            assert!(cols.contains(&"ratio".to_string()));
            assert_eq!(result.len(), 2);
            // Find group A and B by category value
            let group_a = result.iter().find(|r| r["category"] == Value::String("A".into())).unwrap();
            let group_b = result.iter().find(|r| r["category"] == Value::String("B".into())).unwrap();
            match &group_a["ratio"] {
                Value::Float(f) => assert!((f - 15.0).abs() < 0.001),
                other => panic!("Expected Float for group A ratio, got {:?}", other),
            }
            match &group_b["ratio"] {
                Value::Float(f) => assert!((f - 6.0).abs() < 0.001),
                other => panic!("Expected Float for group B ratio, got {:?}", other),
            }
        } else {
            panic!("Expected Select");
        }
    }
}
