//! Orchestrate loading a table folder into validated rows.

use std::collections::HashMap;
use std::path::Path;

use rayon::prelude::*;

use crate::database::{DatabaseConfig, load_database_config};
use crate::errors::ValidationError;
use crate::model::{Row, to_row};
use crate::parser::parse_file;
use crate::schema::{MDQL_FILENAME, Schema, load_schema};
use crate::validator::validate_file;

/// Load all markdown files in a folder, validate, and return rows.
pub fn load_table(
    folder: &Path,
) -> crate::errors::Result<(Schema, Vec<Row>, Vec<ValidationError>)> {
    let schema = load_schema(folder)?;
    let (rows, errors) = load_md_files(folder, &schema, None)?;
    Ok((schema, rows, errors))
}

/// Load with an optional mtime-based cache. Unchanged files are served from cache.
pub fn load_table_cached(
    folder: &Path,
    cache: &mut crate::cache::TableCache,
) -> crate::errors::Result<(Schema, Vec<Row>, Vec<ValidationError>)> {
    let schema = load_schema(folder)?;
    let (rows, errors) = load_md_files(folder, &schema, Some(cache))?;
    Ok((schema, rows, errors))
}

fn load_md_files(
    folder: &Path,
    schema: &Schema,
    mut cache: Option<&mut crate::cache::TableCache>,
) -> crate::errors::Result<(Vec<Row>, Vec<ValidationError>)> {
    let mut md_files: Vec<_> = std::fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            name_str.ends_with(".md") && name_str != MDQL_FILENAME
        })
        .map(|e| e.path())
        .collect();
    md_files.sort();

    // If cache is fresh (dir mtime unchanged), try to serve all from cache
    if let Some(ref cache) = cache {
        if !cache.is_stale(folder) {
            let mut rows = Vec::new();
            let mut all_cached = true;
            for md_file in &md_files {
                let rel = md_file.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Some(mtime) = crate::cache::file_mtime(md_file) {
                    if let Some(row) = cache.get(&rel, mtime) {
                        rows.push(row.clone());
                        continue;
                    }
                }
                all_cached = false;
                break;
            }
            if all_cached {
                return Ok((rows, Vec::new()));
            }
        }
    }

    // Parse (possibly in parallel)
    let results: Vec<_> = md_files
        .par_iter()
        .map(|md_file| {
            let rel = md_file.file_name().unwrap_or_default().to_string_lossy().to_string();
            let parsed = parse_file(
                md_file,
                Some(folder),
                schema.rules.normalize_numbered_headings,
            );
            match parsed {
                Ok(p) => {
                    let errors = validate_file(&p, schema);
                    if errors.is_empty() {
                        let row = to_row(&p, schema);
                        let mtime = crate::cache::file_mtime(md_file);
                        (Some((rel, row, mtime)), errors)
                    } else {
                        (None, errors)
                    }
                }
                Err(e) => {
                    let ve = ValidationError {
                        file_path: md_file.to_string_lossy().to_string(),
                        error_type: "parse_error".to_string(),
                        field: None,
                        message: e.to_string(),
                        line_number: None,
                    };
                    (None, vec![ve])
                }
            }
        })
        .collect();

    let mut rows = Vec::new();
    let mut all_errors = Vec::new();
    for (row_opt, errors) in results {
        all_errors.extend(errors);
        if let Some((rel, row, mtime)) = row_opt {
            if let Some(ref mut c) = cache {
                if let Some(mt) = mtime {
                    c.put(rel, mt, row.clone());
                }
            }
            rows.push(row);
        }
    }

    if let Some(c) = cache {
        c.set_table_mtime(folder);
    }

    Ok((rows, all_errors))
}

/// Load a multi-table database directory.
pub fn load_database(
    db_dir: &Path,
) -> crate::errors::Result<(
    DatabaseConfig,
    HashMap<String, (Schema, Vec<Row>)>,
    Vec<ValidationError>,
)> {
    let db_config = load_database_config(db_dir)?;

    let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
    let mut all_errors = Vec::new();

    let mut children: Vec<_> = std::fs::read_dir(db_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(MDQL_FILENAME).exists())
        .collect();
    children.sort();

    for child in children {
        let (schema, rows, errors) = load_table(&child)?;
        tables.insert(schema.table.clone(), (schema, rows));
        all_errors.extend(errors);
    }

    // Validate foreign key constraints across all tables
    let fk_errors = crate::validator::validate_foreign_keys(&db_config, &tables);
    all_errors.extend(fk_errors);

    // Materialize views
    for view_def in &db_config.views {
        if tables.contains_key(&view_def.name) {
            all_errors.push(ValidationError {
                file_path: MDQL_FILENAME.to_string(),
                error_type: "view_error".to_string(),
                field: Some(view_def.name.clone()),
                message: format!(
                    "View '{}' conflicts with existing table name",
                    view_def.name
                ),
                line_number: None,
            });
            continue;
        }

        match materialize_view(view_def, &tables) {
            Ok((schema, rows)) => {
                tables.insert(view_def.name.clone(), (schema, rows));
            }
            Err(e) => {
                all_errors.push(ValidationError {
                    file_path: MDQL_FILENAME.to_string(),
                    error_type: "view_error".to_string(),
                    field: Some(view_def.name.clone()),
                    message: format!("View '{}': {}", view_def.name, e),
                    line_number: None,
                });
            }
        }
    }

    Ok((db_config, tables, all_errors))
}

pub fn materialize_view(
    view_def: &crate::database::ViewDef,
    tables: &HashMap<String, (Schema, Vec<crate::model::Row>)>,
) -> crate::errors::Result<(Schema, Vec<crate::model::Row>)> {
    use crate::query_parser::{Statement, parse_query};

    let stmt = parse_query(&view_def.query)?;
    let select = match stmt {
        Statement::Select(q) => q,
        _ => {
            return Err(crate::errors::MdqlError::QueryExecution(
                "View query must be a SELECT statement".into(),
            ))
        }
    };

    let (rows, columns) = if !select.joins.is_empty() {
        crate::query_engine::execute_join_query(&select, tables)?
    } else {
        let (schema, table_rows) = tables.get(&select.table).ok_or_else(|| {
            crate::errors::MdqlError::QueryExecution(format!(
                "table '{}' not found in database",
                select.table
            ))
        })?;
        crate::query_engine::execute_query(&select, table_rows, schema)?
    };

    let schema = build_view_schema(&view_def.name, &columns, &rows);
    Ok((schema, rows))
}

fn build_view_schema(
    name: &str,
    columns: &[String],
    rows: &[crate::model::Row],
) -> Schema {
    use crate::schema::*;
    use indexmap::IndexMap;

    let mut frontmatter = IndexMap::new();
    for col in columns {
        if col == "path" || col == "h1" || col == "created" || col == "modified" {
            continue;
        }
        let field_type = rows
            .iter()
            .find_map(|r| r.get(col))
            .map(|v| match v {
                crate::model::Value::Int(_) => FieldType::Int,
                crate::model::Value::Float(_) => FieldType::Float,
                crate::model::Value::Bool(_) => FieldType::Bool,
                crate::model::Value::Date(_) => FieldType::Date,
                crate::model::Value::DateTime(_) => FieldType::DateTime,
                crate::model::Value::List(_) => FieldType::StringArray,
                crate::model::Value::Dict(_) => FieldType::Dict,
                _ => FieldType::String,
            })
            .unwrap_or(FieldType::String);

        frontmatter.insert(
            col.clone(),
            FieldDef {
                field_type,
                required: false,
                enum_values: None,
            },
        );
    }

    Schema {
        table: name.to_string(),
        primary_key: "path".to_string(),
        frontmatter,
        h1_required: false,
        h1_must_equal_frontmatter: None,
        sections: IndexMap::new(),
        rules: Rules {
            reject_unknown_frontmatter: false,
            reject_unknown_sections: false,
            reject_duplicate_sections: false,
            normalize_numbered_headings: false,
        },
    }
}
