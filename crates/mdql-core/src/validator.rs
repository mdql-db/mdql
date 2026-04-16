//! Validate parsed markdown files against a schema.

use std::collections::{HashMap, HashSet};

use crate::database::DatabaseConfig;
use crate::errors::ValidationError;
use crate::model::{Row, Value};
use crate::parser::ParsedFile;
use crate::schema::Schema;
use crate::stamp::TIMESTAMP_FIELDS;

pub fn validate_file(parsed: &ParsedFile, schema: &Schema) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let fp = &parsed.path;

    // Parse-level errors
    for msg in &parsed.parse_errors {
        errors.push(ValidationError {
            file_path: fp.clone(),
            error_type: "parse_error".to_string(),
            field: None,
            message: msg.clone(),
            line_number: None,
        });
    }

    if errors.iter().any(|e| e.error_type == "parse_error") {
        return errors;
    }

    let fm = &parsed.raw_frontmatter;
    let fm_map = match fm.as_mapping() {
        Some(m) => m,
        None => return errors,
    };

    // --- Frontmatter field checks ---
    for (name, field_def) in &schema.frontmatter {
        let key = serde_yaml::Value::String(name.clone());
        match fm_map.get(&key) {
            None => {
                if field_def.required {
                    errors.push(ValidationError {
                        file_path: fp.clone(),
                        error_type: "missing_field".to_string(),
                        field: Some(name.clone()),
                        message: format!("Missing required frontmatter field '{}'", name),
                        line_number: None,
                    });
                }
            }
            Some(value) => {
                if let Some(type_err) = check_type(value, &field_def.field_type, name) {
                    errors.push(ValidationError {
                        file_path: fp.clone(),
                        error_type: "type_mismatch".to_string(),
                        field: Some(name.clone()),
                        message: type_err,
                        line_number: None,
                    });
                }

                if let Some(ref enum_vals) = field_def.enum_values {
                    if !value.is_null() {
                        let str_val = yaml_value_to_string(value);
                        if !enum_vals.contains(&str_val) {
                            errors.push(ValidationError {
                                file_path: fp.clone(),
                                error_type: "enum_violation".to_string(),
                                field: Some(name.clone()),
                                message: format!(
                                    "Field '{}' value '{}' not in allowed values: {:?}",
                                    name, str_val, enum_vals
                                ),
                                line_number: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Validate timestamp fields as datetime (ISO 8601)
    for ts_field in TIMESTAMP_FIELDS {
        let key = serde_yaml::Value::String(ts_field.to_string());
        if let Some(value) = fm_map.get(&key) {
            if let Some(type_err) = check_type(
                value,
                &crate::schema::FieldType::DateTime,
                ts_field,
            ) {
                errors.push(ValidationError {
                    file_path: fp.clone(),
                    error_type: "type_mismatch".to_string(),
                    field: Some(ts_field.to_string()),
                    message: type_err,
                    line_number: None,
                });
            }
        }
    }

    // Unknown frontmatter
    if schema.rules.reject_unknown_frontmatter {
        for (key_val, _) in fm_map {
            if let Some(key) = key_val.as_str() {
                if !schema.frontmatter.contains_key(key)
                    && !TIMESTAMP_FIELDS.contains(&key)
                {
                    errors.push(ValidationError {
                        file_path: fp.clone(),
                        error_type: "unknown_field".to_string(),
                        field: Some(key.to_string()),
                        message: format!(
                            "Unknown frontmatter field '{}' (not in schema)",
                            key
                        ),
                        line_number: None,
                    });
                }
            }
        }
    }

    // --- H1 checks ---
    if schema.h1_required && parsed.h1.is_none() {
        errors.push(ValidationError {
            file_path: fp.clone(),
            error_type: "missing_h1".to_string(),
            field: None,
            message: "Missing required H1 heading".to_string(),
            line_number: None,
        });
    }

    if let Some(ref h1_field) = schema.h1_must_equal_frontmatter {
        if let Some(ref h1) = parsed.h1 {
            let key = serde_yaml::Value::String(h1_field.clone());
            if let Some(expected_val) = fm_map.get(&key) {
                let expected = yaml_value_to_string(expected_val);
                if h1 != &expected {
                    errors.push(ValidationError {
                        file_path: fp.clone(),
                        error_type: "h1_mismatch".to_string(),
                        field: None,
                        message: format!(
                            "H1 '{}' does not match frontmatter '{}' (expected '{}')",
                            h1, h1_field, expected
                        ),
                        line_number: parsed.h1_line_number,
                    });
                }
            }
        }
    }

    // --- Section checks ---
    let section_names: Vec<&str> = parsed
        .sections
        .iter()
        .map(|s| s.normalized_heading.as_str())
        .collect();

    // Count occurrences
    let mut section_counter: HashMap<&str, usize> = HashMap::new();
    for name in &section_names {
        *section_counter.entry(name).or_insert(0) += 1;
    }

    // Duplicate sections
    if schema.rules.reject_duplicate_sections {
        for (name, count) in &section_counter {
            if *count > 1 {
                errors.push(ValidationError {
                    file_path: fp.clone(),
                    error_type: "duplicate_section".to_string(),
                    field: Some(name.to_string()),
                    message: format!(
                        "Duplicate section '{}' (appears {} times)",
                        name, count
                    ),
                    line_number: None,
                });
            }
        }
    }

    // Required sections
    for (name, section_def) in &schema.sections {
        if section_def.required && !section_names.contains(&name.as_str()) {
            errors.push(ValidationError {
                file_path: fp.clone(),
                error_type: "missing_section".to_string(),
                field: Some(name.clone()),
                message: format!("Missing required section '{}'", name),
                line_number: None,
            });
        }
    }

    // Unknown sections
    if schema.rules.reject_unknown_sections {
        for section in &parsed.sections {
            if !schema.sections.contains_key(&section.normalized_heading) {
                errors.push(ValidationError {
                    file_path: fp.clone(),
                    error_type: "unknown_section".to_string(),
                    field: Some(section.normalized_heading.clone()),
                    message: format!(
                        "Unknown section '{}' (not in schema)",
                        section.normalized_heading
                    ),
                    line_number: Some(section.line_number),
                });
            }
        }
    }

    errors
}

fn check_type(
    value: &serde_yaml::Value,
    expected: &crate::schema::FieldType,
    field_name: &str,
) -> Option<String> {
    use crate::schema::FieldType;

    if value.is_null() {
        return None;
    }

    match expected {
        FieldType::String => {
            if !value.is_string() {
                return Some(format!(
                    "Field '{}' expected string, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::Int => {
            if value.is_bool() {
                return Some(format!(
                    "Field '{}' expected int, got bool",
                    field_name
                ));
            }
            // serde_yaml may parse integers as i64 or u64
            if !value.is_i64() && !value.is_u64() {
                return Some(format!(
                    "Field '{}' expected int, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::Float => {
            if value.is_bool() {
                return Some(format!(
                    "Field '{}' expected float, got bool",
                    field_name
                ));
            }
            if !value.is_f64() && !value.is_i64() && !value.is_u64() {
                return Some(format!(
                    "Field '{}' expected float, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::Bool => {
            if !value.is_bool() {
                return Some(format!(
                    "Field '{}' expected bool, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::Date => {
            if let Some(s) = value.as_str() {
                if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
                    return Some(format!(
                        "Field '{}' expected date (YYYY-MM-DD), got string '{}'",
                        field_name, s
                    ));
                }
                return None;
            }
            if !value.is_string() {
                return Some(format!(
                    "Field '{}' expected date, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::DateTime => {
            if let Some(s) = value.as_str() {
                let ok = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
                    || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").is_ok();
                if !ok {
                    return Some(format!(
                        "Field '{}' expected datetime (ISO 8601), got string '{}'",
                        field_name, s
                    ));
                }
                return None;
            }
            if !value.is_string() {
                return Some(format!(
                    "Field '{}' expected datetime, got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
        }
        FieldType::StringArray => {
            match value.as_sequence() {
                None => {
                    return Some(format!(
                        "Field '{}' expected string[], got {}",
                        field_name,
                        yaml_type_name(value)
                    ));
                }
                Some(seq) => {
                    for (i, item) in seq.iter().enumerate() {
                        if !item.is_string() {
                            return Some(format!(
                                "Field '{}[{}]' expected string, got {}",
                                field_name,
                                i,
                                yaml_type_name(item)
                            ));
                        }
                    }
                }
            }
        }
        FieldType::Dict => {
            if !value.is_mapping() {
                return Some(format!(
                    "Field '{}' expected dict (mapping), got {}",
                    field_name,
                    yaml_type_name(value)
                ));
            }
            if let Some(mapping) = value.as_mapping() {
                for (k, v) in mapping {
                    if v.is_mapping() || v.is_sequence() {
                        return Some(format!(
                            "Field '{}' dict value for key '{}' must be a scalar, got {}",
                            field_name,
                            k.as_str().unwrap_or("?"),
                            yaml_type_name(v)
                        ));
                    }
                }
            }
        }
    }

    None
}

fn yaml_type_name(value: &serde_yaml::Value) -> &'static str {
    match value {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "bool",
        serde_yaml::Value::Number(_) => {
            if value.is_f64() && !value.is_i64() && !value.is_u64() {
                "float"
            } else {
                "int"
            }
        }
        serde_yaml::Value::String(_) => "str",
        serde_yaml::Value::Sequence(_) => "list",
        serde_yaml::Value::Mapping(_) => "mapping",
        _ => "unknown",
    }
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => format!("{:?}", value),
    }
}

/// Validate all foreign key constraints across a loaded database.
pub fn validate_foreign_keys(
    db_config: &DatabaseConfig,
    tables: &HashMap<String, (Schema, Vec<Row>)>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for fk in &db_config.foreign_keys {
        let to_table = match tables.get(&fk.to_table) {
            Some(t) => t,
            None => {
                errors.push(ValidationError {
                    file_path: format!("_mdql.md"),
                    error_type: "fk_missing_table".to_string(),
                    field: None,
                    message: format!(
                        "Foreign key references unknown table '{}'",
                        fk.to_table
                    ),
                    line_number: None,
                });
                continue;
            }
        };

        let from_table = match tables.get(&fk.from_table) {
            Some(t) => t,
            None => {
                errors.push(ValidationError {
                    file_path: format!("_mdql.md"),
                    error_type: "fk_missing_table".to_string(),
                    field: None,
                    message: format!(
                        "Foreign key references unknown table '{}'",
                        fk.from_table
                    ),
                    line_number: None,
                });
                continue;
            }
        };

        // Build set of valid target values
        let valid_values: HashSet<String> = to_table
            .1
            .iter()
            .filter_map(|row| {
                row.get(&fk.to_column).and_then(|v| match v {
                    Value::Null => None,
                    _ => Some(v.to_display_string()),
                })
            })
            .collect();

        // Check each row in the referencing table
        for row in &from_table.1 {
            let value = match row.get(&fk.from_column) {
                Some(Value::Null) | None => continue,
                Some(v) => v,
            };

            let file_path = row
                .get("path")
                .map(|v| format!("{}/{}", fk.from_table, v.to_display_string()))
                .unwrap_or_else(|| fk.from_table.clone());

            let values_to_check: Vec<String> = match value {
                Value::List(items) => items.iter().map(|s| s.clone()).collect(),
                _ => vec![value.to_display_string()],
            };

            for value_str in &values_to_check {
                if !valid_values.contains(value_str) {
                    errors.push(ValidationError {
                        file_path: file_path.clone(),
                        error_type: "fk_violation".to_string(),
                        field: Some(fk.from_column.clone()),
                        message: format!(
                            "{} = '{}' not found in {}.{}",
                            fk.from_column, value_str, fk.to_table, fk.to_column
                        ),
                        line_number: None,
                    });
                }
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_text;
    use crate::schema::*;
    use indexmap::IndexMap;

    fn make_schema() -> Schema {
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
        frontmatter.insert("status".to_string(), FieldDef {
            field_type: FieldType::String,
            required: false,
            enum_values: Some(vec!["ACTIVE".into(), "ARCHIVED".into()]),
        });

        let mut sections = IndexMap::new();
        sections.insert("Summary".to_string(), SectionDef {
            content_type: "markdown".to_string(),
            required: true,
        });

        Schema {
            table: "test".to_string(),
            primary_key: "path".to_string(),
            frontmatter,
            h1_required: false,
            h1_must_equal_frontmatter: None,
            sections,
            rules: Rules {
                reject_unknown_frontmatter: true,
                reject_unknown_sections: false,
                reject_duplicate_sections: true,
                normalize_numbered_headings: false,
            },
        }
    }

    #[test]
    fn test_valid_file() {
        let text = "---\ntitle: \"Hello\"\ncount: 5\n---\n\n## Summary\n\nA summary.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_missing_required_field() {
        let text = "---\ntitle: \"Hello\"\n---\n\n## Summary\n\nText.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "missing_field" && e.field.as_deref() == Some("count")));
    }

    #[test]
    fn test_type_mismatch() {
        let text = "---\ntitle: \"Hello\"\ncount: \"not a number\"\n---\n\n## Summary\n\nText.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "type_mismatch" && e.field.as_deref() == Some("count")));
    }

    #[test]
    fn test_enum_violation() {
        let text = "---\ntitle: \"Hello\"\ncount: 5\nstatus: INVALID\n---\n\n## Summary\n\nText.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "enum_violation"));
    }

    #[test]
    fn test_unknown_frontmatter() {
        let text = "---\ntitle: \"Hello\"\ncount: 5\nextra: bad\n---\n\n## Summary\n\nText.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "unknown_field" && e.field.as_deref() == Some("extra")));
    }

    #[test]
    fn test_missing_required_section() {
        let text = "---\ntitle: \"Hello\"\ncount: 5\n---\n\n## Other\n\nText.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "missing_section"));
    }

    #[test]
    fn test_duplicate_section() {
        let text = "---\ntitle: \"Hello\"\ncount: 5\n---\n\n## Summary\n\nFirst.\n\n## Summary\n\nSecond.\n";
        let parsed = parse_text(text, "test.md", false);
        let errors = validate_file(&parsed, &make_schema());
        assert!(errors.iter().any(|e| e.error_type == "duplicate_section"));
    }

    // --- Foreign key validation tests ---

    use crate::database::{DatabaseConfig, ForeignKey};

    fn make_fk_tables() -> HashMap<String, (Schema, Vec<Row>)> {
        let strategy_schema = Schema {
            table: "strategies".to_string(),
            primary_key: "path".to_string(),
            frontmatter: IndexMap::new(),
            h1_required: false,
            h1_must_equal_frontmatter: None,
            sections: IndexMap::new(),
            rules: Rules {
                reject_unknown_frontmatter: false,
                reject_unknown_sections: false,
                reject_duplicate_sections: false,
                normalize_numbered_headings: false,
            },
        };

        let backtest_schema = Schema {
            table: "backtests".to_string(),
            primary_key: "path".to_string(),
            frontmatter: IndexMap::new(),
            h1_required: false,
            h1_must_equal_frontmatter: None,
            sections: IndexMap::new(),
            rules: Rules {
                reject_unknown_frontmatter: false,
                reject_unknown_sections: false,
                reject_duplicate_sections: false,
                normalize_numbered_headings: false,
            },
        };

        let mut s1 = Row::new();
        s1.insert("path".into(), Value::String("alpha.md".into()));
        let mut s2 = Row::new();
        s2.insert("path".into(), Value::String("beta.md".into()));

        let mut b1 = Row::new();
        b1.insert("path".into(), Value::String("bt-alpha.md".into()));
        b1.insert("strategy".into(), Value::String("alpha.md".into()));
        let mut b2 = Row::new();
        b2.insert("path".into(), Value::String("bt-beta.md".into()));
        b2.insert("strategy".into(), Value::String("beta.md".into()));

        let mut tables = HashMap::new();
        tables.insert("strategies".into(), (strategy_schema, vec![s1, s2]));
        tables.insert("backtests".into(), (backtest_schema, vec![b1, b2]));
        tables
    }

    fn make_fk_config() -> DatabaseConfig {
        DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strategies".into(),
                to_column: "path".into(),
            }],
        }
    }

    #[test]
    fn test_fk_valid() {
        let tables = make_fk_tables();
        let config = make_fk_config();
        let errors = validate_foreign_keys(&config, &tables);
        assert!(errors.is_empty(), "Expected no FK errors, got: {:?}", errors);
    }

    #[test]
    fn test_fk_violation() {
        let mut tables = make_fk_tables();
        // Add a backtest referencing a nonexistent strategy
        let mut broken = Row::new();
        broken.insert("path".into(), Value::String("bt-broken.md".into()));
        broken.insert("strategy".into(), Value::String("nonexistent.md".into()));
        tables.get_mut("backtests").unwrap().1.push(broken);

        let config = make_fk_config();
        let errors = validate_foreign_keys(&config, &tables);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "fk_violation");
        assert!(errors[0].message.contains("nonexistent.md"));
    }

    #[test]
    fn test_fk_null_not_violation() {
        let mut tables = make_fk_tables();
        // Add a backtest with null strategy — should not be a violation
        let mut nullref = Row::new();
        nullref.insert("path".into(), Value::String("bt-null.md".into()));
        nullref.insert("strategy".into(), Value::Null);
        tables.get_mut("backtests").unwrap().1.push(nullref);

        let config = make_fk_config();
        let errors = validate_foreign_keys(&config, &tables);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_fk_missing_table() {
        let tables = make_fk_tables();
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "nonexistent_table".into(),
                to_column: "path".into(),
            }],
        };
        let errors = validate_foreign_keys(&config, &tables);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, "fk_missing_table");
    }

    #[test]
    fn test_fk_string_array_valid() {
        let mut tables = make_fk_tables();
        // Add a row with a string[] FK where both values exist
        let array_row = Row::from([
            ("path".into(), Value::String("bt-multi.md".into())),
            ("strategy".into(), Value::List(vec![
                "alpha.md".into(),
                "beta.md".into(),
            ])),
        ]);
        tables.get_mut("backtests").unwrap().1.push(array_row);

        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strategies".into(),
                to_column: "path".into(),
            }],
        };
        let errors = validate_foreign_keys(&config, &tables);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_fk_string_array_one_invalid() {
        let mut tables = make_fk_tables();
        let array_row = Row::from([
            ("path".into(), Value::String("bt-multi.md".into())),
            ("strategy".into(), Value::List(vec![
                "alpha.md".into(),
                "nonexistent.md".into(),
            ])),
        ]);
        tables.get_mut("backtests").unwrap().1.push(array_row);

        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strategies".into(),
                to_column: "path".into(),
            }],
        };
        let errors = validate_foreign_keys(&config, &tables);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nonexistent.md"));
    }
}
