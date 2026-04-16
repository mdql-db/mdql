//! Auto-manage created/modified timestamps in frontmatter.

use std::path::Path;

use chrono::NaiveDateTime;

use crate::txn::atomic_write;

pub const TIMESTAMP_FIELDS: &[&str] = &["created", "modified"];

pub fn stamp_file(
    path: &Path,
    now: Option<NaiveDateTime>,
) -> crate::errors::Result<StampResult> {
    let now_str = now
        .unwrap_or_else(|| chrono::Local::now().naive_local())
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let text = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = text.split('\n').collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return Ok(StampResult {
            created_set: false,
            modified_updated: false,
        });
    }

    let mut end_idx = None;
    for i in 1..lines.len() {
        if lines[i].trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            return Ok(StampResult {
                created_set: false,
                modified_updated: false,
            });
        }
    };

    let mut fm_lines: Vec<String> = lines[1..end_idx]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut created_idx = None;
    let mut modified_idx = None;
    for (i, line) in fm_lines.iter().enumerate() {
        let stripped = line.trim_start();
        if stripped.starts_with("created:") || stripped.starts_with("created :") {
            created_idx = Some(i);
        } else if stripped.starts_with("modified:") || stripped.starts_with("modified :") {
            modified_idx = Some(i);
        }
    }

    let mut created_set = false;
    if created_idx.is_none() {
        fm_lines.push(format!("created: \"{}\"", now_str));
        created_set = true;
    }

    if let Some(idx) = modified_idx {
        fm_lines[idx] = format!("modified: \"{}\"", now_str);
    } else {
        fm_lines.push(format!("modified: \"{}\"", now_str));
    }

    let mut new_lines: Vec<String> = vec!["---".to_string()];
    new_lines.extend(fm_lines);
    new_lines.push("---".to_string());
    for line in &lines[end_idx + 1..] {
        new_lines.push(line.to_string());
    }

    atomic_write(path, &new_lines.join("\n"))?;

    Ok(StampResult {
        created_set,
        modified_updated: true,
    })
}

#[derive(Debug)]
pub struct StampResult {
    pub created_set: bool,
    pub modified_updated: bool,
}
