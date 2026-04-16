//! Load and validate database-level _mdql.md files (type: database).

use std::path::Path;

use crate::errors::MdqlError;
use crate::parser::parse_file;
use crate::schema::MDQL_FILENAME;

#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub name: String,
    pub foreign_keys: Vec<ForeignKey>,
}

pub fn is_database_dir(folder: &Path) -> bool {
    let mdql_file = folder.join(MDQL_FILENAME);
    if !mdql_file.exists() {
        return false;
    }
    if let Ok(text) = std::fs::read_to_string(&mdql_file) {
        if let Some(fm_text) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---").map(|(fm, _)| fm))
        {
            if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(fm_text) {
                if let Some(m) = val.as_mapping() {
                    return m
                        .get(&serde_yaml::Value::String("type".into()))
                        .and_then(|v| v.as_str())
                        == Some("database");
                }
            }
        }
    }
    false
}

pub fn load_database_config(db_dir: &Path) -> crate::errors::Result<DatabaseConfig> {
    let db_path = db_dir.join(MDQL_FILENAME);
    if !db_path.exists() {
        return Err(MdqlError::DatabaseConfig(format!(
            "No {} in {}",
            MDQL_FILENAME,
            db_dir.display()
        )));
    }

    let parsed = parse_file(&db_path, Some(db_dir), false)?;

    if !parsed.parse_errors.is_empty() {
        return Err(MdqlError::DatabaseConfig(format!(
            "Cannot parse {}: {}",
            MDQL_FILENAME,
            parsed.parse_errors.join("; ")
        )));
    }

    let fm = &parsed.raw_frontmatter;
    let fm_map = fm.as_mapping().ok_or_else(|| {
        MdqlError::DatabaseConfig(format!(
            "{}: frontmatter must be a mapping",
            MDQL_FILENAME
        ))
    })?;

    let type_val = fm_map.get(&serde_yaml::Value::String("type".into()));
    if type_val.and_then(|v| v.as_str()) != Some("database") {
        return Err(MdqlError::DatabaseConfig(format!(
            "{}: frontmatter must have 'type: database'",
            MDQL_FILENAME
        )));
    }

    let name = fm_map
        .get(&serde_yaml::Value::String("name".into()))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            MdqlError::DatabaseConfig(format!(
                "{}: frontmatter must have 'name' as a string",
                MDQL_FILENAME
            ))
        })?
        .to_string();

    let mut fks = Vec::new();
    if let Some(fk_list) = fm_map.get(&serde_yaml::Value::String("foreign_keys".into())) {
        if let Some(seq) = fk_list.as_sequence() {
            for fk_def in seq {
                let fk_map = fk_def.as_mapping().ok_or_else(|| {
                    MdqlError::DatabaseConfig(format!(
                        "{}: each foreign_key must be a mapping",
                        MDQL_FILENAME
                    ))
                })?;

                let from_spec = fk_map
                    .get(&serde_yaml::Value::String("from".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let to_spec = fk_map
                    .get(&serde_yaml::Value::String("to".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !from_spec.contains('.') || !to_spec.contains('.') {
                    return Err(MdqlError::DatabaseConfig(format!(
                        "{}: foreign_key 'from' and 'to' must be 'table.column' format",
                        MDQL_FILENAME
                    )));
                }

                let (from_table, from_col) = from_spec.split_once('.').unwrap();
                let (to_table, to_col) = to_spec.split_once('.').unwrap();

                fks.push(ForeignKey {
                    from_table: from_table.to_string(),
                    from_column: from_col.to_string(),
                    to_table: to_table.to_string(),
                    to_column: to_col.to_string(),
                });
            }
        }
    }

    Ok(DatabaseConfig {
        name,
        foreign_keys: fks,
    })
}
