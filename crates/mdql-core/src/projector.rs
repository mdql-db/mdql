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
        Value::DateTime(dt) => serde_json::Value::String(dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
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
        Value::DateTime(dt) => dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
        other => other.to_display_string(),
    }
}

fn format_table(rows: &[Row], columns: &[String], truncate: usize) -> String {
    let ncols = columns.len();
    if ncols == 0 {
        return "No results.".to_string();
    }

    // First pass: collect raw (newline-flattened, trimmed) cell strings and natural widths
    let mut natural_widths: Vec<usize> = columns.iter().map(|c| c.chars().count()).collect();

    let raw_cells: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            columns
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let val = r.get(c).unwrap_or(&Value::Null);
                    let s = val.to_display_string().replace('\n', " ");
                    let s = s.trim().to_string();
                    let char_len = s.chars().count();
                    if char_len > natural_widths[i] {
                        natural_widths[i] = char_len;
                    }
                    s
                })
                .collect()
        })
        .collect();

    // Determine effective max width per column
    let gap = 2; // spaces between columns
    let total_gap = gap * (ncols.saturating_sub(1));

    let effective_widths = if truncate > 0 {
        // Explicit truncate: cap each column at truncate
        natural_widths.iter().map(|&w| w.min(truncate)).collect::<Vec<_>>()
    } else {
        // Auto-fit to terminal width
        let term_width = terminal_width().unwrap_or(120);
        fit_columns_to_width(&natural_widths, term_width.saturating_sub(total_gap))
    };

    // Second pass: truncate cells AND headers to effective widths
    let cell_data: Vec<Vec<String>> = raw_cells
        .iter()
        .map(|row_cells| {
            row_cells
                .iter()
                .enumerate()
                .map(|(i, s)| truncate_str(s, effective_widths[i]))
                .collect()
        })
        .collect();

    let truncated_headers: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| truncate_str(c, effective_widths[i]))
        .collect();

    // Display widths = max of (truncated header, truncated cells) — guaranteed <= effective_widths
    let mut display_widths: Vec<usize> = truncated_headers.iter().map(|h| h.chars().count()).collect();
    for row_cells in &cell_data {
        for (i, s) in row_cells.iter().enumerate() {
            let w = s.chars().count();
            if w > display_widths[i] {
                display_widths[i] = w;
            }
        }
    }

    let mut out = String::new();

    // Header
    let header: Vec<String> = truncated_headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<width$}", h, width = display_widths[i]))
        .collect();
    out.push_str(&header.join("  "));
    out.push('\n');

    // Separator
    let sep: Vec<String> = display_widths.iter().map(|w| "-".repeat(*w)).collect();
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
                    format!("{:>width$}", c, width = display_widths[i])
                } else {
                    format!("{:<width$}", c, width = display_widths[i])
                }
            })
            .collect();
        out.push_str(&line.join("  "));
        out.push('\n');
    }

    // Remove trailing newline to match Python tabulate behavior
    out.trim_end_matches('\n').to_string()
}

/// Get terminal width, if available.
fn terminal_width() -> Option<usize> {
    // Try COLUMNS env var first (set by many shells)
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(w) = cols.parse::<usize>() {
            if w > 0 {
                return Some(w);
            }
        }
    }

    // Try ioctl on stderr (fd 2), then stdout (fd 1), then stdin (fd 0)
    // stderr is often still connected to the terminal even when stdout is piped
    #[cfg(unix)]
    {
        use std::mem::zeroed;
        for fd in [2, 1, 0] {
            unsafe {
                let mut ws: libc::winsize = zeroed();
                if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
                    return Some(ws.ws_col as usize);
                }
            }
        }
    }

    None
}

/// Distribute available width across columns to fit within budget.
fn fit_columns_to_width(natural: &[usize], available: usize) -> Vec<usize> {
    let total_natural: usize = natural.iter().sum();
    if total_natural <= available {
        return natural.to_vec();
    }

    // Simple proportional: each column gets (natural / total_natural) * available
    // Floor at 4 chars minimum
    let min_col = 4;
    let mut widths: Vec<usize> = natural
        .iter()
        .map(|&w| {
            let share = ((w as f64 / total_natural as f64) * available as f64) as usize;
            share.max(min_col)
        })
        .collect();

    // If rounding pushed us over, trim from the widest
    let mut total: usize = widths.iter().sum();
    while total > available {
        if let Some(i) = widths.iter().enumerate()
            .filter(|(_, &w)| w > min_col)
            .max_by_key(|(_, &w)| w)
            .map(|(i, _)| i)
        {
            widths[i] -= 1;
            total -= 1;
        } else {
            break;
        }
    }

    widths
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ");
    let s = s.trim();
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
