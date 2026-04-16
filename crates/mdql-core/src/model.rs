//! Convert parsed files to normalized row dicts.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::parser::ParsedFile;
use crate::schema::{FieldType, Schema};

/// A row is a flat map from column name to value.
pub type Row = HashMap<String, Value>;

/// Dynamic value type for row entries.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Date(chrono::NaiveDate),
    DateTime(chrono::NaiveDateTime),
    List(Vec<String>),
    Dict(IndexMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format!("{}", f),
            Value::Bool(b) => b.to_string(),
            Value::Date(d) => d.format("%Y-%m-%d").to_string(),
            Value::DateTime(dt) => dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            Value::List(items) => items.join(", "),
            Value::Dict(map) => {
                let pairs: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::Date(a), Value::Date(b)) => a.partial_cmp(b),
            (Value::DateTime(a), Value::DateTime(b)) => a.partial_cmp(b),
            (Value::Date(a), Value::DateTime(b)) => a.and_hms_opt(0, 0, 0).unwrap().partial_cmp(b),
            (Value::DateTime(a), Value::Date(b)) => a.partial_cmp(&b.and_hms_opt(0, 0, 0).unwrap()),
            // Fallback: compare as strings
            _ => self.to_display_string().partial_cmp(&other.to_display_string()),
        }
    }
}

/// Public wrapper for yaml_to_value, used by api.rs for CLI coercion.
pub fn yaml_to_value_pub(val: &serde_yaml::Value, field_type: Option<&FieldType>) -> Value {
    yaml_to_value(val, field_type)
}

fn yaml_to_value(val: &serde_yaml::Value, field_type: Option<&FieldType>) -> Value {
    match val {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(FieldType::Float) = field_type {
                Value::Float(n.as_f64().unwrap_or(0.0))
            } else if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Int(u as i64)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => {
            // DateTime coercion
            if let Some(FieldType::DateTime) = field_type {
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                    return Value::DateTime(dt);
                }
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                    return Value::DateTime(dt);
                }
            }
            // Date coercion
            if let Some(FieldType::Date) = field_type {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    return Value::Date(date);
                }
            }
            Value::String(s.clone())
        }
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<String> = seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Value::List(items)
        }
        serde_yaml::Value::Mapping(mapping) => {
            if let Some(FieldType::Dict) = field_type {
                let mut dict = IndexMap::new();
                for (k, v) in mapping {
                    if let Some(key) = k.as_str() {
                        dict.insert(key.to_string(), yaml_to_value(v, None));
                    }
                }
                Value::Dict(dict)
            } else {
                Value::String(format!("{:?}", val))
            }
        }
        _ => Value::String(format!("{:?}", val)),
    }
}

/// Convert a validated ParsedFile into a flat row dict.
pub fn to_row(parsed: &ParsedFile, schema: &Schema) -> Row {
    let mut row = Row::new();
    row.insert("path".to_string(), Value::String(parsed.path.clone()));

    // Frontmatter fields — coerce types
    if let Some(fm_map) = parsed.raw_frontmatter.as_mapping() {
        for (key_val, value) in fm_map {
            if let Some(key) = key_val.as_str() {
                let field_type = schema.frontmatter.get(key).map(|fd| &fd.field_type)
                    .or_else(|| {
                        if crate::stamp::TIMESTAMP_FIELDS.contains(&key) {
                            Some(&FieldType::DateTime)
                        } else {
                            None
                        }
                    });
                row.insert(key.to_string(), yaml_to_value(value, field_type));
            }
        }
    }

    // H1
    if let Some(ref h1) = parsed.h1 {
        row.insert("h1".to_string(), Value::String(h1.clone()));
    }

    // Sections
    for section in &parsed.sections {
        row.insert(
            section.normalized_heading.clone(),
            Value::String(section.body.clone()),
        );
    }

    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_text;
    use crate::schema::*;
    use indexmap::IndexMap;

    fn test_schema() -> Schema {
        let mut frontmatter = IndexMap::new();
        frontmatter.insert("title".to_string(), FieldDef {
            field_type: FieldType::String,
            required: true,
            enum_values: None,
        });
        frontmatter.insert("count".to_string(), FieldDef {
            field_type: FieldType::Int,
            required: true,
            enum_values: None,
        });

        Schema {
            table: "test".to_string(),
            primary_key: "path".to_string(),
            frontmatter,
            h1_required: false,
            h1_must_equal_frontmatter: None,
            sections: IndexMap::new(),
            rules: Rules {
                reject_unknown_frontmatter: false,
                reject_unknown_sections: false,
                reject_duplicate_sections: true,
                normalize_numbered_headings: false,
            },
        }
    }

    #[test]
    fn test_to_row_basic() {
        let text = "---\ntitle: \"Hello\"\ncount: 42\n---\n\n## Summary\n\nA summary.\n";
        let parsed = parse_text(text, "test.md", false);
        let row = to_row(&parsed, &test_schema());
        assert_eq!(row["path"], Value::String("test.md".into()));
        assert_eq!(row["title"], Value::String("Hello".into()));
        assert_eq!(row["count"], Value::Int(42));
        assert_eq!(row["Summary"], Value::String("A summary.".into()));
    }

    #[test]
    fn test_to_row_with_h1() {
        let text = "---\ntitle: \"Test\"\ncount: 1\n---\n\n# My Title\n\n## Section\n\nBody.\n";
        let parsed = parse_text(text, "test.md", false);
        let row = to_row(&parsed, &test_schema());
        assert_eq!(row["h1"], Value::String("My Title".into()));
    }

    #[test]
    fn test_date_coercion() {
        let mut frontmatter = IndexMap::new();
        frontmatter.insert("created".to_string(), FieldDef {
            field_type: FieldType::Date,
            required: true,
            enum_values: None,
        });

        let schema = Schema {
            table: "test".to_string(),
            primary_key: "path".to_string(),
            frontmatter,
            h1_required: false,
            h1_must_equal_frontmatter: None,
            sections: IndexMap::new(),
            rules: Rules {
                reject_unknown_frontmatter: false,
                reject_unknown_sections: false,
                reject_duplicate_sections: true,
                normalize_numbered_headings: false,
            },
        };

        let text = "---\ncreated: \"2026-04-04\"\n---\n";
        let parsed = parse_text(text, "test.md", false);
        let row = to_row(&parsed, &schema);
        assert_eq!(
            row["created"],
            Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 4, 4).unwrap())
        );
    }
}
