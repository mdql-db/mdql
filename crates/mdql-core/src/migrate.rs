//! Field migration operations on markdown files.
//!
//! Both frontmatter keys and H2 sections are "fields" in MDQL.

use std::path::Path;

use regex::Regex;
use std::sync::LazyLock;

use crate::errors::MdqlError;
use crate::parser::{normalize_heading};
use crate::txn::atomic_write;

static FENCE_OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(`{3,}|~{3,})").unwrap());
static H2_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^##\s+(.+)$").unwrap());

// ── Section span detection ────────────────────────────────────────────────

struct SectionSpan {
    #[allow(dead_code)]
    raw_heading: String,
    normalized_heading: String,
    heading_line_idx: usize,
    end_line_idx: usize, // exclusive
}

fn find_sections(lines: &[String], normalize: bool) -> Vec<SectionSpan> {
    let mut sections = Vec::new();
    let mut in_fence = false;
    let mut fence_char: Option<char> = None;
    let mut fence_width: usize = 0;

    // Skip frontmatter
    let mut start = 0;
    if !lines.is_empty() && lines[0].trim() == "---" {
        for i in 1..lines.len() {
            if lines[i].trim() == "---" {
                start = i + 1;
                break;
            }
        }
    }

    for i in start..lines.len() {
        let line = &lines[i];

        if let Some(caps) = FENCE_OPEN_RE.captures(line) {
            let marker = caps.get(1).unwrap().as_str();
            let char = marker.chars().next().unwrap();
            let width = marker.len();
            if !in_fence {
                in_fence = true;
                fence_char = Some(char);
                fence_width = width;
                continue;
            } else if Some(char) == fence_char && width >= fence_width && line.trim() == marker {
                in_fence = false;
                fence_char = None;
                fence_width = 0;
                continue;
            }
        }

        if in_fence {
            continue;
        }

        if let Some(caps) = H2_RE.captures(line) {
            let raw_h = caps.get(1).unwrap().as_str().trim().to_string();
            let norm_h = if normalize {
                normalize_heading(&raw_h)
            } else {
                raw_h.clone()
            };
            sections.push(SectionSpan {
                raw_heading: raw_h,
                normalized_heading: norm_h,
                heading_line_idx: i,
                end_line_idx: lines.len(),
            });
        }
    }

    // Fix up end indices
    for i in 0..sections.len().saturating_sub(1) {
        let next_start = sections[i + 1].heading_line_idx;
        sections[i].end_line_idx = next_start;
    }

    sections
}

// ── Frontmatter field operations ────────────────────────────────────────

fn find_frontmatter_bounds(lines: &[String]) -> Option<(usize, usize)> {
    if lines.is_empty() || lines[0].trim() != "---" {
        return None;
    }
    for i in 1..lines.len() {
        if lines[i].trim() == "---" {
            return Some((1, i));
        }
    }
    None
}

pub fn rename_frontmatter_key_in_file(
    path: &Path,
    old_key: &str,
    new_key: &str,
) -> crate::errors::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let bounds = match find_frontmatter_bounds(&lines) {
        Some(b) => b,
        None => return Ok(false),
    };

    let pattern = Regex::new(&format!(r"^{}(\s*:.*)$", regex::escape(old_key))).unwrap();

    let mut changed = false;
    for i in bounds.0..bounds.1 {
        if let Some(caps) = pattern.captures(&lines[i].clone()) {
            lines[i] = format!("{}{}", new_key, caps.get(1).unwrap().as_str());
            changed = true;
            break;
        }
    }

    if changed {
        atomic_write(path, &lines.join("\n"))?;
    }
    Ok(changed)
}

pub fn drop_frontmatter_key_in_file(
    path: &Path,
    key: &str,
) -> crate::errors::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let bounds = match find_frontmatter_bounds(&lines) {
        Some(b) => b,
        None => return Ok(false),
    };

    let pattern = Regex::new(&format!(r"^{}\s*:", regex::escape(key))).unwrap();

    let mut key_range = None;
    for i in bounds.0..bounds.1 {
        if pattern.is_match(&lines[i]) {
            let mut end = i + 1;
            while end < bounds.1
                && (lines[end].starts_with(' ') || lines[end].starts_with('\t'))
            {
                end += 1;
            }
            key_range = Some((i, end));
            break;
        }
    }

    match key_range {
        Some((start, end)) => {
            lines.drain(start..end);
            atomic_write(path, &lines.join("\n"))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

// ── Section operations ──────────────────────────────────────────────────

pub fn rename_section_in_file(
    path: &Path,
    old_name: &str,
    new_name: &str,
    normalize: bool,
) -> crate::errors::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let sections = find_sections(&lines, normalize);

    let mut changed = false;
    for sec in &sections {
        if sec.normalized_heading == old_name {
            lines[sec.heading_line_idx] = format!("## {}", new_name);
            changed = true;
        }
    }

    if changed {
        atomic_write(path, &lines.join("\n"))?;
    }
    Ok(changed)
}

pub fn drop_section_in_file(
    path: &Path,
    section_name: &str,
    normalize: bool,
) -> crate::errors::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let sections = find_sections(&lines, normalize);

    let to_remove: Vec<_> = sections
        .iter()
        .filter(|s| s.normalized_heading == section_name)
        .collect();

    if to_remove.is_empty() {
        return Ok(false);
    }

    // Remove from bottom up
    for sec in to_remove.iter().rev() {
        let mut start = sec.heading_line_idx;
        let end = sec.end_line_idx;
        if start > 0 && lines[start - 1].trim().is_empty() {
            start -= 1;
        }
        lines.drain(start..end);
    }

    atomic_write(path, &lines.join("\n"))?;
    Ok(true)
}

pub fn merge_sections_in_file(
    path: &Path,
    source_names: &[String],
    into: &str,
    normalize: bool,
) -> crate::errors::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let sections = find_sections(&lines, normalize);

    let all_names: std::collections::HashSet<&str> =
        source_names.iter().map(|s| s.as_str()).collect();
    let matching: Vec<_> = sections
        .iter()
        .filter(|s| all_names.contains(s.normalized_heading.as_str()))
        .collect();

    if matching.len() < 2 {
        return Ok(false);
    }

    // Collect bodies
    let mut bodies = Vec::new();
    for sec in &matching {
        let body_lines = &lines[sec.heading_line_idx + 1..sec.end_line_idx];
        let body = body_lines.join("\n").trim().to_string();
        if !body.is_empty() {
            bodies.push(body);
        }
    }

    let merged_body = bodies.join("\n\n");

    // Replace first, delete rest
    let target = matching[0];
    let to_delete: Vec<_> = matching[1..].iter().collect();

    let target_replacement = vec![
        format!("## {}", into),
        String::new(),
        merged_body,
        String::new(),
    ];

    let old_span = target.end_line_idx - target.heading_line_idx;
    let new_span = target_replacement.len();

    lines.splice(
        target.heading_line_idx..target.end_line_idx,
        target_replacement,
    );

    let mut shift = new_span as i64 - old_span as i64;

    for sec in to_delete.iter().rev() {
        let mut adj_start = (sec.heading_line_idx as i64 + shift) as usize;
        let adj_end = (sec.end_line_idx as i64 + shift) as usize;
        if adj_start > 0 && lines[adj_start - 1].trim().is_empty() {
            adj_start -= 1;
        }
        let removed = adj_end - adj_start;
        lines.drain(adj_start..adj_end);
        shift -= removed as i64;
    }

    atomic_write(path, &lines.join("\n"))?;
    Ok(true)
}

// ── Schema update ─────────────────────────────────────────────────────────

pub fn update_schema(
    schema_path: &Path,
    rename_frontmatter: Option<(&str, &str)>,
    drop_frontmatter: Option<&str>,
    rename_section: Option<(&str, &str)>,
    drop_section: Option<&str>,
    merge_sections: Option<(&[String], &str)>,
) -> crate::errors::Result<()> {
    let text = std::fs::read_to_string(schema_path)?;
    let file_lines: Vec<&str> = text.split('\n').collect();

    if file_lines.is_empty() || file_lines[0].trim() != "---" {
        return Ok(());
    }

    let mut end_idx = None;
    for i in 1..file_lines.len() {
        if file_lines[i].trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => return Ok(()),
    };

    let fm_text = file_lines[1..end_idx].join("\n");
    let mut fm: serde_yaml::Value =
        serde_yaml::from_str(&fm_text).unwrap_or(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

    let fm_map = match fm.as_mapping_mut() {
        Some(m) => m,
        None => return Err(MdqlError::General("schema frontmatter is not a YAML mapping".into())),
    };

    // Frontmatter field operations
    let fm_key = serde_yaml::Value::String("frontmatter".into());
    let fm_fields = fm_map
        .entry(fm_key.clone())
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

    if let Some(fields_map) = fm_fields.as_mapping_mut() {
        if let Some((old, new)) = rename_frontmatter {
            let old_key = serde_yaml::Value::String(old.to_string());
            let new_key = serde_yaml::Value::String(new.to_string());
            if let Some(val) = fields_map.remove(&old_key) {
                fields_map.insert(new_key, val);
            }
        }
        if let Some(key) = drop_frontmatter {
            fields_map.remove(&serde_yaml::Value::String(key.to_string()));
        }
    }

    // Section operations
    let sec_key = serde_yaml::Value::String("sections".into());
    let sections = fm_map
        .entry(sec_key.clone())
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

    if let Some(sections_map) = sections.as_mapping_mut() {
        if let Some((old, new)) = rename_section {
            let old_key = serde_yaml::Value::String(old.to_string());
            let new_key = serde_yaml::Value::String(new.to_string());
            if let Some(val) = sections_map.remove(&old_key) {
                sections_map.insert(new_key, val);
            }
        }
        if let Some(key) = drop_section {
            sections_map.remove(&serde_yaml::Value::String(key.to_string()));
        }
        if let Some((sources, target)) = merge_sections {
            let mut target_config = None;
            for s in sources {
                let k = serde_yaml::Value::String(s.clone());
                if target_config.is_none() {
                    target_config = sections_map.get(&k).cloned();
                }
            }
            let target_config = target_config.unwrap_or_else(|| {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("type".into()),
                    serde_yaml::Value::String("markdown".into()),
                );
                m.insert(
                    serde_yaml::Value::String("required".into()),
                    serde_yaml::Value::Bool(false),
                );
                serde_yaml::Value::Mapping(m)
            });
            for s in sources {
                sections_map.remove(&serde_yaml::Value::String(s.clone()));
            }
            sections_map.insert(
                serde_yaml::Value::String(target.to_string()),
                target_config,
            );
        }
    }

    // Re-serialize
    let new_fm = serde_yaml::to_string(&fm).unwrap_or_default();
    let new_fm = new_fm.trim_end();

    let mut new_lines = vec!["---".to_string(), new_fm.to_string(), "---".to_string()];
    for line in &file_lines[end_idx + 1..] {
        new_lines.push(line.to_string());
    }

    atomic_write(schema_path, &new_lines.join("\n"))?;
    Ok(())
}
