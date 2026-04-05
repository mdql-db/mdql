//! Parse markdown files into structured representations.
//!
//! Handles frontmatter extraction, H1/H2 detection, code fence tracking,
//! and numbered heading normalization.

use std::path::Path;

use regex::Regex;
use std::sync::LazyLock;

use crate::errors::MdqlError;

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub raw_heading: String,
    pub normalized_heading: String,
    pub body: String,
    pub line_number: usize,
}

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: String,
    pub raw_frontmatter: serde_yaml::Value,
    pub h1: Option<String>,
    pub h1_line_number: Option<usize>,
    pub sections: Vec<Section>,
    pub parse_errors: Vec<String>,
}

static NUMBERED_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+\.\s+").unwrap());
static FENCE_OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(`{3,}|~{3,})").unwrap());
static H1_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#\s+(.+)$").unwrap());
static H2_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^##\s+(.+)$").unwrap());

pub fn normalize_heading(raw: &str) -> String {
    NUMBERED_HEADING_RE.replace(raw, "").trim().to_string()
}

pub fn parse_file(
    path: &Path,
    relative_to: Option<&Path>,
    normalize_numbered: bool,
) -> crate::errors::Result<ParsedFile> {
    let rel_path = if let Some(base) = relative_to {
        path.strip_prefix(base)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    } else {
        path.to_string_lossy().to_string()
    };

    let text = std::fs::read_to_string(path).map_err(|e| {
        MdqlError::Parse(format!("Cannot read {}: {}", rel_path, e))
    })?;

    Ok(parse_text(&text, &rel_path, normalize_numbered))
}

/// Parse markdown text directly (useful for testing and when content is already in memory).
pub fn parse_text(text: &str, rel_path: &str, normalize_numbered: bool) -> ParsedFile {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut raw_frontmatter = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    let mut body_start: usize = 0;
    let mut parse_errors: Vec<String> = Vec::new();

    // --- Parse frontmatter ---
    if !lines.is_empty() && lines[0].trim() == "---" {
        let mut closing = None;
        for i in 1..lines.len() {
            if lines[i].trim() == "---" {
                closing = Some(i);
                break;
            }
        }

        if let Some(close_idx) = closing {
            let fm_text: String = lines[1..close_idx].join("\n");
            match serde_yaml::from_str::<serde_yaml::Value>(&fm_text) {
                Ok(serde_yaml::Value::Null) => {
                    // Empty frontmatter
                }
                Ok(val @ serde_yaml::Value::Mapping(_)) => {
                    raw_frontmatter = val;
                }
                Ok(val) => {
                    let type_name = match &val {
                        serde_yaml::Value::Bool(_) => "bool",
                        serde_yaml::Value::Number(_) => "number",
                        serde_yaml::Value::String(_) => "str",
                        serde_yaml::Value::Sequence(_) => "list",
                        _ => "unknown",
                    };
                    parse_errors.push(format!(
                        "Frontmatter is not a mapping (got {})",
                        type_name
                    ));
                }
                Err(e) => {
                    parse_errors.push(format!("Malformed YAML in frontmatter: {}", e));
                }
            }
            body_start = close_idx + 1;
        } else {
            parse_errors.push("Unclosed frontmatter (no closing '---')".to_string());
            body_start = 1;
        }
    } else {
        parse_errors.push("No frontmatter found (file must start with '---')".to_string());
    }

    // --- Parse body: H1, H2 sections ---
    let mut h1: Option<String> = None;
    let mut h1_line_number: Option<usize> = None;
    let mut sections: Vec<Section> = Vec::new();

    let mut in_fence = false;
    let mut fence_char: Option<char> = None;
    let mut fence_width: usize = 0;

    let mut current_heading: Option<String> = None;
    let mut current_heading_normalized: Option<String> = None;
    let mut current_heading_line: Option<usize> = None;
    let mut current_body_lines: Vec<&str> = Vec::new();

    let finalize_section = |heading: &mut Option<String>,
                                heading_norm: &mut Option<String>,
                                heading_line: &mut Option<usize>,
                                body_lines: &mut Vec<&str>,
                                sections: &mut Vec<Section>| {
        if let Some(raw_h) = heading.take() {
            let norm_h = heading_norm.take().unwrap_or_else(|| raw_h.clone());
            let body = body_lines.join("\n").trim().to_string();
            sections.push(Section {
                raw_heading: raw_h,
                normalized_heading: norm_h,
                body,
                line_number: heading_line.take().unwrap_or(0),
            });
            body_lines.clear();
        }
    };

    for i in body_start..lines.len() {
        let line = lines[i];
        let line_num = i + 1; // 1-indexed

        // --- Code fence tracking ---
        if let Some(caps) = FENCE_OPEN_RE.captures(line) {
            let marker = caps.get(1).unwrap().as_str();
            let char = marker.chars().next().unwrap();
            let width = marker.len();

            if !in_fence {
                in_fence = true;
                fence_char = Some(char);
                fence_width = width;
                if current_heading.is_some() {
                    current_body_lines.push(line);
                }
                continue;
            } else if Some(char) == fence_char
                && width >= fence_width
                && line.trim() == marker
            {
                // Closing fence
                in_fence = false;
                fence_char = None;
                fence_width = 0;
                if current_heading.is_some() {
                    current_body_lines.push(line);
                }
                continue;
            }
        }

        if in_fence {
            if current_heading.is_some() {
                current_body_lines.push(line);
            }
            continue;
        }

        // --- H1 detection ---
        if let Some(caps) = H1_RE.captures(line) {
            if h1.is_none() {
                h1 = Some(caps.get(1).unwrap().as_str().trim().to_string());
                h1_line_number = Some(line_num);
            } else {
                parse_errors.push(format!(
                    "Duplicate H1 at line {} (first was at line {})",
                    line_num,
                    h1_line_number.unwrap_or(0)
                ));
            }
            continue;
        }

        // --- H2 detection ---
        if let Some(caps) = H2_RE.captures(line) {
            finalize_section(
                &mut current_heading,
                &mut current_heading_normalized,
                &mut current_heading_line,
                &mut current_body_lines,
                &mut sections,
            );
            let raw_h = caps.get(1).unwrap().as_str().trim().to_string();
            let norm_h = if normalize_numbered {
                normalize_heading(&raw_h)
            } else {
                raw_h.clone()
            };
            current_heading = Some(raw_h);
            current_heading_normalized = Some(norm_h);
            current_heading_line = Some(line_num);
            current_body_lines.clear();
            continue;
        }

        // --- Regular content ---
        if current_heading.is_some() {
            current_body_lines.push(line);
        }
    }

    finalize_section(
        &mut current_heading,
        &mut current_heading_normalized,
        &mut current_heading_line,
        &mut current_body_lines,
        &mut sections,
    );

    ParsedFile {
        path: rel_path.to_string(),
        raw_frontmatter,
        h1,
        h1_line_number,
        sections,
        parse_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parse() {
        let text = "---\ntitle: \"Hello\"\nstatus: \"active\"\n---\n\n## Summary\n\nA summary.\n\n## Details\n\nSome details.\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.is_empty());
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].normalized_heading, "Summary");
        assert_eq!(parsed.sections[0].body, "A summary.");
        assert_eq!(parsed.sections[1].normalized_heading, "Details");
        assert_eq!(parsed.sections[1].body, "Some details.");
    }

    #[test]
    fn test_frontmatter_extraction() {
        let text = "---\ntitle: \"Test\"\ncount: 42\n---\n\nBody text.\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.is_empty());
        let fm = parsed.raw_frontmatter.as_mapping().unwrap();
        assert_eq!(
            fm.get(&serde_yaml::Value::String("title".into()))
                .unwrap()
                .as_str()
                .unwrap(),
            "Test"
        );
        assert_eq!(
            fm.get(&serde_yaml::Value::String("count".into()))
                .unwrap()
                .as_u64()
                .unwrap(),
            42
        );
    }

    #[test]
    fn test_no_frontmatter() {
        let text = "Just some text.\n";
        let parsed = parse_text(text, "test.md", false);
        assert_eq!(parsed.parse_errors.len(), 1);
        assert!(parsed.parse_errors[0].contains("No frontmatter"));
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let text = "---\ntitle: Test\nNo closing delimiter.\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.iter().any(|e| e.contains("Unclosed")));
    }

    #[test]
    fn test_h1_detection() {
        let text = "---\ntitle: \"Test\"\n---\n\n# My Title\n\n## Section\n\nBody.\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.is_empty());
        assert_eq!(parsed.h1.as_deref(), Some("My Title"));
        assert_eq!(parsed.h1_line_number, Some(5));
    }

    #[test]
    fn test_duplicate_h1() {
        let text = "---\ntitle: \"Test\"\n---\n\n# First\n\n# Second\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.iter().any(|e| e.contains("Duplicate H1")));
    }

    #[test]
    fn test_code_fence_ignores_headings() {
        let text = "---\ntitle: \"Test\"\n---\n\n## Section\n\n```\n# Not a heading\n## Also not\n```\n\nAfter fence.\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.is_empty());
        assert!(parsed.h1.is_none());
        assert_eq!(parsed.sections.len(), 1);
        assert!(parsed.sections[0].body.contains("# Not a heading"));
    }

    #[test]
    fn test_numbered_heading_normalization() {
        let text = "---\ntitle: \"Test\"\n---\n\n## 1. Hypothesis\n\nContent.\n\n## 2. Method\n\nMore.\n";
        let parsed = parse_text(text, "test.md", true);
        assert!(parsed.parse_errors.is_empty());
        assert_eq!(parsed.sections[0].raw_heading, "1. Hypothesis");
        assert_eq!(parsed.sections[0].normalized_heading, "Hypothesis");
        assert_eq!(parsed.sections[1].normalized_heading, "Method");
    }

    #[test]
    fn test_numbered_heading_no_normalization() {
        let text = "---\ntitle: \"Test\"\n---\n\n## 1. Hypothesis\n\nContent.\n";
        let parsed = parse_text(text, "test.md", false);
        assert_eq!(parsed.sections[0].normalized_heading, "1. Hypothesis");
    }

    #[test]
    fn test_tilde_fence() {
        let text = "---\ntitle: \"Test\"\n---\n\n## Section\n\n~~~\n## fake heading\n~~~\n\nReal content.\n";
        let parsed = parse_text(text, "test.md", false);
        assert_eq!(parsed.sections.len(), 1);
        assert!(parsed.sections[0].body.contains("## fake heading"));
    }

    #[test]
    fn test_section_line_numbers() {
        let text = "---\ntitle: \"Test\"\n---\n\n## First\n\nBody 1.\n\n## Second\n\nBody 2.\n";
        let parsed = parse_text(text, "test.md", false);
        assert_eq!(parsed.sections[0].line_number, 5);
        assert_eq!(parsed.sections[1].line_number, 9);
    }

    #[test]
    fn test_empty_sections() {
        let text = "---\ntitle: \"Test\"\n---\n\n## Empty\n\n## Also Empty\n";
        let parsed = parse_text(text, "test.md", false);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].body, "");
        assert_eq!(parsed.sections[1].body, "");
    }

    #[test]
    fn test_malformed_yaml() {
        let text = "---\n: [invalid yaml\n---\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.iter().any(|e| e.contains("Malformed YAML")));
    }

    #[test]
    fn test_non_mapping_frontmatter() {
        let text = "---\n- a list\n- not a mapping\n---\n";
        let parsed = parse_text(text, "test.md", false);
        assert!(parsed.parse_errors.iter().any(|e| e.contains("not a mapping")));
    }
}
