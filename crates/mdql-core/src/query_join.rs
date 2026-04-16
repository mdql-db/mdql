//! JOIN query execution for multi-table queries.

use std::collections::HashMap;

use crate::errors::MdqlError;
use crate::model::{Row, Value};
use crate::query_ast::*;
use crate::schema::Schema;

pub fn execute_join_query(
    query: &SelectQuery,
    tables: &HashMap<String, (Schema, Vec<Row>)>,
) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
    if query.joins.is_empty() {
        return Err(MdqlError::QueryExecution("No JOIN clause in query".into()));
    }

    let left_name = &query.table;
    let left_alias = query.table_alias.as_deref().unwrap_or(left_name);

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

    for join in &query.joins {
        let right_name = &join.table;
        let right_alias = join.alias.as_deref().unwrap_or(right_name);

        let (_right_schema, right_rows) = tables.get(right_name.as_str()).ok_or_else(|| {
            MdqlError::QueryExecution(format!("Unknown table '{}'", right_name))
        })?;

        let (on_left_table, on_left_col) = resolve_dotted(&join.left_col, &aliases);
        let (on_right_table, on_right_col) = resolve_dotted(&join.right_col, &aliases);

        let (left_key, right_key) = if on_right_table == *right_name {
            let left_alias_for_col = reverse_alias(&on_left_table, &aliases, query, &query.joins);
            (format!("{}.{}", left_alias_for_col, on_left_col), on_right_col)
        } else {
            let right_alias_for_col = reverse_alias(&on_right_table, &aliases, query, &query.joins);
            (format!("{}.{}", right_alias_for_col, on_right_col), on_left_col)
        };

        let mut right_index: HashMap<String, Vec<&Row>> = HashMap::new();
        for r in right_rows {
            if let Some(key) = r.get(&right_key) {
                let key_str = key.to_display_string();
                right_index.entry(key_str).or_default().push(r);
            }
        }

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

    let (mut result, columns) = super::query_engine::execute_inner(query, &current_rows, None)?;

    if !result.is_empty() {
        let mut base_counts: HashMap<String, usize> = HashMap::new();
        for key in &columns {
            if let Some((_prefix, base)) = key.split_once('.') {
                *base_counts.entry(base.to_string()).or_default() += 1;
            }
        }
        let unique_bases: Vec<String> = base_counts
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .map(|(base, _)| base)
            .collect();

        if !unique_bases.is_empty() {
            let unique_set: std::collections::HashSet<&str> =
                unique_bases.iter().map(|s| s.as_str()).collect();
            for row in &mut result {
                let additions: Vec<(String, Value)> = row
                    .iter()
                    .filter_map(|(k, v)| {
                        k.split_once('.').and_then(|(_, base)| {
                            if unique_set.contains(base) {
                                Some((base.to_string(), v.clone()))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                for (k, v) in additions {
                    row.insert(k, v);
                }
            }
        }
    }

    Ok((result, columns))
}

fn reverse_alias(
    table_name: &str,
    aliases: &HashMap<String, String>,
    query: &SelectQuery,
    joins: &[JoinClause],
) -> String {
    if query.table == table_name {
        return query.table_alias.as_deref().unwrap_or(&query.table).to_string();
    }
    for j in joins {
        if j.table == table_name {
            return j.alias.as_deref().unwrap_or(&j.table).to_string();
        }
    }
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
