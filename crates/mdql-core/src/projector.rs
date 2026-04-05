//! Format query/inspect results for output.

use crate::model::{Row, Value};

pub fn format_results(
    rows: &[Row],
    columns: Option<&[String]>,
    output_format: &str,
    truncate: usize,
) -> String {
    if rows.is_empty() {
        return "No results.".to_string();
    }

    let cols: Vec<String> = match columns {
        Some(c) => c.to_vec(),
        None => {
            let mut seen: Vec<String> = Vec::new();
            let mut set = std::collections::HashSet::new();
            for r in rows {
                for k in r.keys() {
                    if set.insert(k.clone()) {
                        seen.push(k.clone());
                    }
                }
            }
            seen
        }
    };

    match output_format {
        "json" => format_json(rows, &cols),
        "csv" => format_csv(rows, &cols),
        _ => format_table(rows, &cols, truncate),
    }
}

fn format_json(rows: &[Row], columns: &[String]) -> String {
    let projected: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let mut obj = serde_json::Map::new();
            for c in columns {
                let val = r.get(c).unwrap_or(&Value::Null);
                obj.insert(c.clone(), value_to_json(val));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    serde_json::to_string_pretty(&projected).unwrap_or_default()
}

fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Null => serde_json::Value::Null,
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Int(n) => serde_json::json!(*n),
        Value::Float(f) => serde_json::json!(*f),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Date(d) => serde_json::Value::String(d.format("%Y-%m-%d").to_string()),
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(|s| serde_json::Value::String(s.clone())).collect())
        }
    }
}

fn format_csv(rows: &[Row], columns: &[String]) -> String {
    let mut out = String::new();
    // Header
    out.push_str(&columns.join(","));
    out.push('\n');
    // Rows
    for r in rows {
        let vals: Vec<String> = columns
            .iter()
            .map(|c| {
                let val = r.get(c).unwrap_or(&Value::Null);
                csv_value(val)
            })
            .collect();
        out.push_str(&vals.join(","));
        out.push('\n');
    }
    out
}

fn csv_value(val: &Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::List(items) => items.join(";"),
        Value::Date(d) => d.format("%Y-%m-%d").to_string(),
        other => other.to_display_string(),
    }
}

fn format_table(rows: &[Row], columns: &[String], truncate: usize) -> String {
    // Calculate column widths
    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();

    let cell_data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let val = r.get(c).unwrap_or(&Value::Null);
                    let s = truncate_str(&val.to_display_string(), truncate);
                    if s.len() > widths[i] {
                        widths[i] = s.len();
                    }
                    s
                })
                .collect()
        })
        .collect();

    let mut out = String::new();

    // Header
    let header: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<width$}", c, width = widths[i]))
        .collect();
    out.push_str(&header.join("  "));
    out.push('\n');

    // Separator
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push_str(&sep.join("  "));
    out.push('\n');

    // Data rows
    for cells in &cell_data {
        let line: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                // Right-align numbers
                if rows
                    .first()
                    .and_then(|r| r.get(&columns[i]))
                    .map_or(false, |v| matches!(v, Value::Int(_) | Value::Float(_)))
                {
                    format!("{:>width$}", c, width = widths[i])
                } else {
                    format!("{:<width$}", c, width = widths[i])
                }
            })
            .collect();
        out.push_str(&line.join("  "));
        out.push('\n');
    }

    // Remove trailing newline to match Python tabulate behavior
    out.trim_end_matches('\n').to_string()
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ");
    let s = s.trim();
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
