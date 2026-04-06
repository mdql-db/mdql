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

    Ok((db_config, tables, all_errors))
}
