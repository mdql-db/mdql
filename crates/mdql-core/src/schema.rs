//! Load and validate table-level _mdql.md files (type: schema).

use std::path::Path;

use indexmap::IndexMap;

use crate::errors::MdqlError;
use crate::parser::parse_file;

pub const MDQL_FILENAME: &str = "_mdql.md";

pub const VALID_FIELD_TYPES: &[&str] = &["string", "int", "float", "bool", "date", "datetime", "string[]", "dict"];

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Date,
    DateTime,
    StringArray,
    Dict,
}

impl FieldType {
    pub fn from_str(s: &str) -> Option<FieldType> {
        match s {
            "string" => Some(FieldType::String),
            "int" => Some(FieldType::Int),
            "float" => Some(FieldType::Float),
            "bool" => Some(FieldType::Bool),
            "date" => Some(FieldType::Date),
            "datetime" => Some(FieldType::DateTime),
            "string[]" => Some(FieldType::StringArray),
            "dict" => Some(FieldType::Dict),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Int => "int",
            FieldType::Float => "float",
            FieldType::Bool => "bool",
            FieldType::Date => "date",
            FieldType::DateTime => "datetime",
            FieldType::StringArray => "string[]",
            FieldType::Dict => "dict",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub field_type: FieldType,
    pub required: bool,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SectionDef {
    pub content_type: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub reject_unknown_frontmatter: bool,
    pub reject_unknown_sections: bool,
    pub reject_duplicate_sections: bool,
    pub normalize_numbered_headings: bool,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub table: String,
    pub primary_key: String,
    pub frontmatter: IndexMap<String, FieldDef>,
    pub h1_required: bool,
    pub h1_must_equal_frontmatter: Option<String>,
    pub sections: IndexMap<String, SectionDef>,
    pub rules: Rules,
}

impl Schema {
    /// All non-section keys that appear in rows.
    pub fn metadata_keys(&self) -> std::collections::HashSet<String> {
        let mut keys: std::collections::HashSet<String> = self
            .frontmatter
            .keys()
            .cloned()
            .collect();
        keys.insert("path".to_string());
        keys.insert("h1".to_string());
        keys.insert("created".to_string());
        keys.insert("modified".to_string());
        keys
    }
}

fn yaml_to_str(val: &serde_yaml::Value) -> Option<&str> {
    val.as_str()
}

fn yaml_to_bool(val: &serde_yaml::Value) -> Option<bool> {
    val.as_bool()
}

fn yaml_to_mapping(val: &serde_yaml::Value) -> Option<&serde_yaml::Mapping> {
    val.as_mapping()
}

pub fn load_schema(folder: &Path) -> crate::errors::Result<Schema> {
    let schema_path = folder.join(MDQL_FILENAME);
    if !schema_path.exists() {
        return Err(MdqlError::SchemaNotFound(format!(
            "No {} in {}",
            MDQL_FILENAME,
            folder.display()
        )));
    }

    let parsed = parse_file(&schema_path, Some(folder), false)?;

    if !parsed.parse_errors.is_empty() {
        return Err(MdqlError::SchemaInvalid(format!(
            "Cannot parse {}: {}",
            MDQL_FILENAME,
            parsed.parse_errors.join("; ")
        )));
    }

    let fm = &parsed.raw_frontmatter;
    validate_meta_schema(fm, &schema_path)?;

    let fm_map = fm.as_mapping().ok_or_else(|| {
        MdqlError::SchemaInvalid(format!(
            "{}: frontmatter must be a YAML mapping",
            MDQL_FILENAME
        ))
    })?;

    // Build field definitions
    let mut frontmatter_defs: IndexMap<String, FieldDef> = IndexMap::new();
    let fm_key = serde_yaml::Value::String("frontmatter".into());
    if let Some(fm_fields) = fm_map.get(&fm_key) {
        if let Some(fields_map) = yaml_to_mapping(fm_fields) {
            for (name_val, spec_val) in fields_map {
                let name = name_val.as_str().unwrap_or("").to_string();
                let spec = spec_val.as_mapping().ok_or_else(|| {
                    MdqlError::SchemaInvalid(format!(
                        "{}: frontmatter.{} must be a mapping",
                        MDQL_FILENAME, name
                    ))
                })?;

                let ftype_str = spec
                    .get(&serde_yaml::Value::String("type".into()))
                    .and_then(yaml_to_str)
                    .unwrap_or("string");

                let field_type = FieldType::from_str(ftype_str).ok_or_else(|| {
                    MdqlError::SchemaInvalid(format!(
                        "{}: frontmatter.{} has invalid type '{}'. Valid types: {}",
                        MDQL_FILENAME,
                        name,
                        ftype_str,
                        VALID_FIELD_TYPES.join(", ")
                    ))
                })?;

                let required = spec
                    .get(&serde_yaml::Value::String("required".into()))
                    .and_then(yaml_to_bool)
                    .unwrap_or(false);

                let enum_values = spec
                    .get(&serde_yaml::Value::String("enum".into()))
                    .and_then(|v| v.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .map(|v| match v {
                                serde_yaml::Value::String(s) => s.clone(),
                                other => format!("{:?}", other),
                            })
                            .collect()
                    });

                frontmatter_defs.insert(name, FieldDef {
                    field_type,
                    required,
                    enum_values,
                });
            }
        }
    }

    // Build section definitions
    let mut section_defs: IndexMap<String, SectionDef> = IndexMap::new();
    let sections_key = serde_yaml::Value::String("sections".into());
    if let Some(sections_val) = fm_map.get(&sections_key) {
        if let Some(sections_map) = yaml_to_mapping(sections_val) {
            for (name_val, spec_val) in sections_map {
                let name = name_val.as_str().unwrap_or("").to_string();
                let spec = spec_val.as_mapping().ok_or_else(|| {
                    MdqlError::SchemaInvalid(format!(
                        "{}: sections.{} must be a mapping",
                        MDQL_FILENAME, name
                    ))
                })?;

                let content_type = spec
                    .get(&serde_yaml::Value::String("type".into()))
                    .and_then(yaml_to_str)
                    .unwrap_or("markdown")
                    .to_string();

                let required = spec
                    .get(&serde_yaml::Value::String("required".into()))
                    .and_then(yaml_to_bool)
                    .unwrap_or(false);

                section_defs.insert(name, SectionDef {
                    content_type,
                    required,
                });
            }
        }
    }

    // H1 config
    let h1_key = serde_yaml::Value::String("h1".into());
    let h1_config = fm_map.get(&h1_key);
    let h1_required = h1_config
        .and_then(yaml_to_mapping)
        .and_then(|m| m.get(&serde_yaml::Value::String("required".into())))
        .and_then(yaml_to_bool)
        .unwrap_or(true);
    let h1_must_equal = h1_config
        .and_then(yaml_to_mapping)
        .and_then(|m| m.get(&serde_yaml::Value::String("must_equal_frontmatter".into())))
        .and_then(yaml_to_str)
        .map(|s| s.to_string());

    // Rules
    let rules_key = serde_yaml::Value::String("rules".into());
    let rules_map = fm_map.get(&rules_key).and_then(yaml_to_mapping);

    let get_rule_bool = |key: &str, default: bool| -> bool {
        rules_map
            .and_then(|m| m.get(&serde_yaml::Value::String(key.into())))
            .and_then(yaml_to_bool)
            .unwrap_or(default)
    };

    let rules = Rules {
        reject_unknown_frontmatter: get_rule_bool("reject_unknown_frontmatter", true),
        reject_unknown_sections: get_rule_bool("reject_unknown_sections", true),
        reject_duplicate_sections: get_rule_bool("reject_duplicate_sections", true),
        normalize_numbered_headings: get_rule_bool("normalize_numbered_headings", false),
    };

    // Table name
    let table = fm_map
        .get(&serde_yaml::Value::String("table".into()))
        .and_then(yaml_to_str)
        .unwrap_or("")
        .to_string();

    let primary_key = fm_map
        .get(&serde_yaml::Value::String("primary_key".into()))
        .and_then(yaml_to_str)
        .unwrap_or("path")
        .to_string();

    Ok(Schema {
        table,
        primary_key,
        frontmatter: frontmatter_defs,
        h1_required,
        h1_must_equal_frontmatter: h1_must_equal,
        sections: section_defs,
        rules,
    })
}

fn validate_meta_schema(fm: &serde_yaml::Value, path: &Path) -> crate::errors::Result<()> {
    let map = fm.as_mapping().ok_or_else(|| {
        MdqlError::SchemaInvalid(format!("{}: frontmatter must be a mapping", path.display()))
    })?;

    // type: schema
    let type_val = map.get(&serde_yaml::Value::String("type".into()));
    if type_val.and_then(yaml_to_str) != Some("schema") {
        return Err(MdqlError::SchemaInvalid(format!(
            "{}: frontmatter must have 'type: schema'",
            path.display()
        )));
    }

    // table must be a string
    let table_val = map.get(&serde_yaml::Value::String("table".into()));
    if table_val.and_then(yaml_to_str).is_none() {
        return Err(MdqlError::SchemaInvalid(format!(
            "{}: frontmatter must have 'table' as a string",
            path.display()
        )));
    }

    // frontmatter must be a mapping if present
    let fm_val = map.get(&serde_yaml::Value::String("frontmatter".into()));
    if let Some(v) = fm_val {
        if !v.is_mapping() && !v.is_null() {
            return Err(MdqlError::SchemaInvalid(format!(
                "{}: 'frontmatter' must be a mapping",
                path.display()
            )));
        }
    }

    // sections must be a mapping if present
    let sec_val = map.get(&serde_yaml::Value::String("sections".into()));
    if let Some(v) = sec_val {
        if !v.is_mapping() && !v.is_null() {
            return Err(MdqlError::SchemaInvalid(format!(
                "{}: 'sections' must be a mapping",
                path.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_schema_file(dir: &Path, content: &str) {
        fs::write(dir.join(MDQL_FILENAME), content).unwrap();
    }

    #[test]
    fn test_load_basic_schema() {
        let dir = tempfile::tempdir().unwrap();
        make_schema_file(
            dir.path(),
            "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\nh1:\n  required: false\nsections: {}\nrules:\n  reject_unknown_frontmatter: true\n  reject_unknown_sections: false\n  reject_duplicate_sections: true\n---\n",
        );
        let schema = load_schema(dir.path()).unwrap();
        assert_eq!(schema.table, "test");
        assert_eq!(schema.primary_key, "path");
        assert!(schema.frontmatter.contains_key("title"));
        assert!(schema.frontmatter["title"].required);
        assert_eq!(schema.frontmatter["title"].field_type, FieldType::String);
        assert!(!schema.h1_required);
        assert!(!schema.rules.reject_unknown_sections);
    }

    #[test]
    fn test_missing_schema() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_schema(dir.path());
        assert!(matches!(result, Err(MdqlError::SchemaNotFound(_))));
    }

    #[test]
    fn test_wrong_type() {
        let dir = tempfile::tempdir().unwrap();
        make_schema_file(dir.path(), "---\ntype: database\nname: test\n---\n");
        let result = load_schema(dir.path());
        assert!(matches!(result, Err(MdqlError::SchemaInvalid(_))));
    }

    #[test]
    fn test_enum_values() {
        let dir = tempfile::tempdir().unwrap();
        make_schema_file(
            dir.path(),
            "---\ntype: schema\ntable: test\nfrontmatter:\n  status:\n    type: string\n    required: true\n    enum: [ACTIVE, ARCHIVED]\n---\n",
        );
        let schema = load_schema(dir.path()).unwrap();
        assert_eq!(
            schema.frontmatter["status"].enum_values,
            Some(vec!["ACTIVE".to_string(), "ARCHIVED".to_string()])
        );
    }
}
