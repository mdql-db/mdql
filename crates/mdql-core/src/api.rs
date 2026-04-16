//! Object-oriented API for MDQL databases and tables.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use std::sync::LazyLock;

use crate::database::{DatabaseConfig, load_database_config};
use crate::errors::{MdqlError, ValidationError};
use crate::migrate;
use crate::model::{Row, Value};
use crate::parser::parse_file;
use crate::query_engine::{evaluate, sql_value_to_value};
use crate::query_parser::*;
use crate::schema::{FieldType, Schema, MDQL_FILENAME, load_schema};
use crate::stamp::TIMESTAMP_FIELDS;
use crate::txn::{TableLock, TableTransaction, atomic_write, recover_journal, with_multi_file_txn};
use crate::validator::validate_file;

static SLUGIFY_NON_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\w\s-]").unwrap());
static SLUGIFY_WHITESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\s_]+").unwrap());
static SLUGIFY_MULTI_DASH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-+").unwrap());

pub fn slugify(text: &str, max_length: usize) -> String {
    let slug = text.to_lowercase();
    let slug = slug.trim();
    let slug = SLUGIFY_NON_WORD.replace_all(&slug, "");
    let slug = SLUGIFY_WHITESPACE.replace_all(&slug, "-");
    let slug = SLUGIFY_MULTI_DASH.replace_all(&slug, "-");
    let slug = slug.trim_matches('-').to_string();
    if slug.len() > max_length {
        slug[..max_length].trim_end_matches('-').to_string()
    } else {
        slug
    }
}

fn write_and_validate(
    filepath: &Path,
    content: &str,
    old_content: Option<&str>,
    schema: &Schema,
    table_path: &Path,
) -> crate::errors::Result<()> {
    atomic_write(filepath, content)?;

    let parsed = parse_file(filepath, Some(table_path), schema.rules.normalize_numbered_headings)?;
    let errors = validate_file(&parsed, schema);

    if !errors.is_empty() {
        if let Some(old) = old_content {
            atomic_write(filepath, old)?;
        } else {
            let _ = std::fs::remove_file(filepath);
        }
        let msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        return Err(MdqlError::General(format!("Validation failed: {}", msgs.join("; "))));
    }

    Ok(())
}

fn format_yaml_value(value: &Value, field_type: &FieldType) -> String {
    match (value, field_type) {
        (Value::String(s), FieldType::String) => format!("\"{}\"", s),
        (Value::String(s), FieldType::Date) => format!("\"{}\"", s),
        (Value::String(s), FieldType::DateTime) => format!("\"{}\"", s),
        (Value::Date(d), _) => format!("\"{}\"", d.format("%Y-%m-%d")),
        (Value::DateTime(dt), _) => format!("\"{}\"", dt.format("%Y-%m-%dT%H:%M:%S")),
        (Value::Int(n), _) => n.to_string(),
        (Value::Float(f), _) => format!("{}", f),
        (Value::Bool(b), _) => if *b { "true" } else { "false" }.to_string(),
        (Value::List(items), _) => {
            if items.is_empty() {
                "[]".to_string()
            } else {
                let list: Vec<String> = items.iter().map(|i| format!("  - {}", i)).collect();
                format!("\n{}", list.join("\n"))
            }
        }
        (Value::Dict(map), _) => {
            if map.is_empty() {
                "{}".to_string()
            } else {
                let lines: Vec<String> = map.iter()
                    .map(|(k, v)| {
                        let val_str = match v {
                            Value::String(s) => format!("\"{}\"", s),
                            Value::Int(n) => n.to_string(),
                            Value::Float(f) => format!("{}", f),
                            Value::Bool(b) => b.to_string(),
                            _ => v.to_display_string(),
                        };
                        format!("  {}: {}", k, val_str)
                    })
                    .collect();
                format!("\n{}", lines.join("\n"))
            }
        }
        (Value::Null, _) => "null".to_string(),
        _ => value.to_display_string(),
    }
}

fn serialize_frontmatter(
    data: &HashMap<String, Value>,
    schema: &Schema,
    preserve_created: Option<&str>,
) -> String {
    let now = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut fm_lines: Vec<String> = Vec::new();

    for (name, field_def) in &schema.frontmatter {
        if TIMESTAMP_FIELDS.contains(&name.as_str()) {
            continue;
        }
        if let Some(val) = data.get(name) {
            let formatted = format_yaml_value(val, &field_def.field_type);
            let is_multiline_list = matches!(field_def.field_type, FieldType::StringArray) && !matches!(val, Value::List(items) if items.is_empty());
            let is_multiline_dict = matches!(field_def.field_type, FieldType::Dict) && !matches!(val, Value::Dict(m) if m.is_empty());
            if is_multiline_list || is_multiline_dict {
                fm_lines.push(format!("{}:{}", name, formatted));
            } else {
                fm_lines.push(format!("{}: {}", name, formatted));
            }
        }
    }

    // Also write any unknown frontmatter keys that were in data
    for (name, val) in data {
        if !schema.frontmatter.contains_key(name)
            && !TIMESTAMP_FIELDS.contains(&name.as_str())
            && !schema.sections.contains_key(name)
            && name != "path"
            && name != "h1"
        {
            fm_lines.push(format!("{}: \"{}\"", name, val.to_display_string()));
        }
    }

    let created = preserve_created
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            data.get("created")
                .map(|v| v.to_display_string())
                .unwrap_or_else(|| now.clone())
        });
    fm_lines.push(format!("created: \"{}\"", created));
    fm_lines.push(format!("modified: \"{}\"", now));

    format!("---\n{}\n---\n", fm_lines.join("\n"))
}

fn serialize_body(data: &HashMap<String, Value>, schema: &Schema) -> String {
    let mut body = String::new();

    if schema.h1_required {
        let h1_text = if let Some(ref field) = schema.h1_must_equal_frontmatter {
            data.get(field)
                .map(|v| v.to_display_string())
                .unwrap_or_default()
        } else {
            data.get("h1")
                .or_else(|| data.get("title"))
                .map(|v| v.to_display_string())
                .unwrap_or_default()
        };
        body.push_str(&format!("\n# {}\n", h1_text));
    }

    for (name, section_def) in &schema.sections {
        let section_body = data
            .get(name)
            .map(|v| v.to_display_string())
            .unwrap_or_default();
        if section_def.required || !section_body.is_empty() {
            body.push_str(&format!("\n## {}\n\n{}\n", name, section_body));
        }
    }

    body
}

fn read_existing(filepath: &Path) -> crate::errors::Result<(HashMap<String, String>, String)> {
    let text = std::fs::read_to_string(filepath)?;
    let lines: Vec<&str> = text.split('\n').collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return Err(MdqlError::General(format!(
            "No frontmatter in {}",
            filepath.file_name().unwrap_or_default().to_string_lossy()
        )));
    }

    let mut end_idx = None;
    for i in 1..lines.len() {
        if lines[i].trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = end_idx.ok_or_else(|| {
        MdqlError::General(format!(
            "Unclosed frontmatter in {}",
            filepath.file_name().unwrap_or_default().to_string_lossy()
        ))
    })?;

    let fm_text = lines[1..end_idx].join("\n");
    let fm: serde_yaml::Value = serde_yaml::from_str(&fm_text).unwrap_or(serde_yaml::Value::Null);
    let mut fm_map = HashMap::new();
    if let Some(mapping) = fm.as_mapping() {
        for (k, v) in mapping {
            if let Some(key) = k.as_str() {
                let val = match v {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => format!("{:?}", v),
                };
                fm_map.insert(key.to_string(), val);
            }
        }
    }

    let raw_body = lines[end_idx + 1..].join("\n");
    Ok((fm_map, raw_body))
}

pub fn coerce_cli_value(raw: &str, field_type: &FieldType) -> crate::errors::Result<Value> {
    match field_type {
        FieldType::Int => raw
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|e| MdqlError::General(e.to_string())),
        FieldType::Float => raw
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|e| MdqlError::General(e.to_string())),
        FieldType::Bool => Ok(Value::Bool(
            matches!(raw.to_lowercase().as_str(), "true" | "1" | "yes"),
        )),
        FieldType::StringArray => Ok(Value::List(
            raw.split(',').map(|s| s.trim().to_string()).collect(),
        )),
        FieldType::Date => {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
                Ok(Value::Date(d))
            } else {
                Ok(Value::String(raw.to_string()))
            }
        }
        FieldType::DateTime => {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
                Ok(Value::DateTime(dt))
            } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f") {
                Ok(Value::DateTime(dt))
            } else {
                Ok(Value::String(raw.to_string()))
            }
        }
        FieldType::String => Ok(Value::String(raw.to_string())),
        FieldType::Dict => {
            match serde_yaml::from_str::<serde_yaml::Value>(raw) {
                Ok(serde_yaml::Value::Mapping(m)) => {
                    let mut dict = indexmap::IndexMap::new();
                    for (k, v) in m {
                        if let Some(key) = k.as_str() {
                            let val = crate::model::yaml_to_value_pub(&v, None);
                            dict.insert(key.to_string(), val);
                        }
                    }
                    Ok(Value::Dict(dict))
                }
                _ => Err(MdqlError::General(format!(
                    "Expected YAML mapping for dict field, got: {}", raw
                ))),
            }
        }
    }
}

// ── Table ─────────────────────────────────────────────────────────────────

pub struct Table {
    pub path: PathBuf,
    schema: Schema,
    cache: std::sync::Mutex<crate::cache::TableCache>,
}

impl Table {
    pub fn new(path: impl Into<PathBuf>) -> crate::errors::Result<Self> {
        let path = path.into();
        recover_journal(&path)?;
        let schema = load_schema(&path)?;
        Ok(Table {
            path,
            schema,
            cache: std::sync::Mutex::new(crate::cache::TableCache::new()),
        })
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn name(&self) -> &str {
        &self.schema.table
    }

    pub fn insert(
        &self,
        data: &HashMap<String, Value>,
        body: Option<&str>,
        filename: Option<&str>,
        replace: bool,
    ) -> crate::errors::Result<PathBuf> {
        let fname = match filename {
            Some(f) => f.to_string(),
            None => {
                let title = data
                    .get("title")
                    .ok_or_else(|| {
                        MdqlError::General(
                            "Cannot derive filename: provide 'title' in data or pass filename"
                                .into(),
                        )
                    })?
                    .to_display_string();
                slugify(&title, 80)
            }
        };

        let fname = if fname.ends_with(".md") {
            fname
        } else {
            format!("{}.md", fname)
        };

        let filepath = self.path.join(&fname);

        let _lock = TableLock::acquire(&self.path)?;

        let mut preserve_created: Option<String> = None;
        let mut old_content: Option<String> = None;

        if filepath.exists() {
            if !replace {
                return Err(MdqlError::General(format!(
                    "File already exists: {}",
                    fname
                )));
            }
            let (old_fm, _) = read_existing(&filepath)?;
            if let Some(c) = old_fm.get("created") {
                preserve_created = Some(c.clone());
            }
            old_content = Some(std::fs::read_to_string(&filepath)?);
        }

        let mut content = serialize_frontmatter(
            data,
            &self.schema,
            preserve_created.as_deref(),
        );

        if let Some(b) = body {
            if !b.starts_with('\n') {
                content.push('\n');
            }
            content.push_str(b);
            if !b.ends_with('\n') {
                content.push('\n');
            }
        } else {
            content.push_str(&serialize_body(data, &self.schema));
        }

        write_and_validate(&filepath, &content, old_content.as_deref(), &self.schema, &self.path)?;

        self.cache.lock().unwrap().invalidate_all();
        Ok(filepath)
    }

    pub fn update(
        &self,
        filename: &str,
        data: &HashMap<String, Value>,
        body: Option<&str>,
    ) -> crate::errors::Result<PathBuf> {
        let _lock = TableLock::acquire(&self.path)?;
        self.update_no_lock(filename, data, body)
    }

    fn update_no_lock(
        &self,
        filename: &str,
        data: &HashMap<String, Value>,
        body: Option<&str>,
    ) -> crate::errors::Result<PathBuf> {
        let fname = if filename.ends_with(".md") {
            filename.to_string()
        } else {
            format!("{}.md", filename)
        };

        let filepath = self.path.join(&fname);
        if !filepath.exists() {
            return Err(MdqlError::General(format!("File not found: {}", fname)));
        }

        let old_content = std::fs::read_to_string(&filepath)?;
        let (old_fm_raw, old_body) = read_existing(&filepath)?;

        // Merge: read existing frontmatter as Value, overlay with data
        let parsed = parse_file(
            &filepath,
            Some(&self.path),
            self.schema.rules.normalize_numbered_headings,
        )?;
        let existing_row = crate::model::to_row(&parsed, &self.schema);

        // Collect section headings so we can exclude them from frontmatter
        let section_keys: std::collections::HashSet<&str> = parsed
            .sections
            .iter()
            .map(|s| s.normalized_heading.as_str())
            .collect();

        let mut merged = existing_row;
        for (k, v) in data {
            merged.insert(k.clone(), v.clone());
        }

        // Remove section keys — they belong in the body, not frontmatter
        merged.retain(|k, _| !section_keys.contains(k.as_str()));

        let preserve_created = old_fm_raw.get("created").map(|s| s.as_str());

        let mut content = serialize_frontmatter(&merged, &self.schema, preserve_created);

        if let Some(b) = body {
            if !b.starts_with('\n') {
                content.push('\n');
            }
            content.push_str(b);
            if !b.ends_with('\n') {
                content.push('\n');
            }
        } else {
            content.push_str(&old_body);
        }

        write_and_validate(&filepath, &content, Some(&old_content), &self.schema, &self.path)?;

        self.cache.lock().unwrap().invalidate_all();
        Ok(filepath)
    }

    pub fn delete(&self, filename: &str) -> crate::errors::Result<PathBuf> {
        let _lock = TableLock::acquire(&self.path)?;
        self.delete_no_lock(filename)
    }

    fn delete_no_lock(&self, filename: &str) -> crate::errors::Result<PathBuf> {
        let fname = if filename.ends_with(".md") {
            filename.to_string()
        } else {
            format!("{}.md", filename)
        };

        let filepath = self.path.join(&fname);
        if !filepath.exists() {
            return Err(MdqlError::General(format!("File not found: {}", fname)));
        }

        std::fs::remove_file(&filepath)?;
        self.cache.lock().unwrap().invalidate_all();
        Ok(filepath)
    }

    pub fn execute_sql(&mut self, sql: &str) -> crate::errors::Result<String> {
        let stmt = parse_query(sql)?;

        match stmt {
            Statement::Select(q) => self.exec_select(&q),
            Statement::Insert(q) => self.exec_insert(&q),
            Statement::Update(q) => self.exec_update(&q),
            Statement::Delete(q) => self.exec_delete(&q),
            Statement::AlterRename(q) => {
                let count = self.rename_field(&q.old_name, &q.new_name)?;
                Ok(format!(
                    "ALTER TABLE — renamed '{}' to '{}' in {} files",
                    q.old_name, q.new_name, count
                ))
            }
            Statement::AlterDrop(q) => {
                let count = self.drop_field(&q.field_name)?;
                Ok(format!(
                    "ALTER TABLE — dropped '{}' from {} files",
                    q.field_name, count
                ))
            }
            Statement::AlterMerge(q) => {
                let count = self.merge_fields(&q.sources, &q.into)?;
                let names: Vec<String> = q.sources.iter().map(|s| format!("'{}'", s)).collect();
                Ok(format!(
                    "ALTER TABLE — merged {} into '{}' in {} files",
                    names.join(", "),
                    q.into,
                    count
                ))
            }
        }
    }

    /// Execute a SELECT query and return structured results.
    pub fn query_sql(&mut self, sql: &str) -> crate::errors::Result<(Vec<Row>, Vec<String>)> {
        let stmt = parse_query(sql)?;
        let select = match stmt {
            Statement::Select(q) => q,
            _ => return Err(MdqlError::QueryParse("Only SELECT queries supported".into())),
        };
        let (_, rows, _) = crate::loader::load_table_cached(&self.path, &mut self.cache.lock().unwrap())?;
        crate::query_engine::execute_query(&select, &rows, &self.schema)
    }

    fn exec_select(&self, query: &SelectQuery) -> crate::errors::Result<String> {
        let (_, rows, _) = crate::loader::load_table_cached(&self.path, &mut self.cache.lock().unwrap())?;
        let (result_rows, result_columns) = crate::query_engine::execute_query(query, &rows, &self.schema)?;
        Ok(crate::projector::format_results(
            &result_rows,
            Some(&result_columns),
            "table",
            0,
        ))
    }

    fn exec_insert(&self, query: &InsertQuery) -> crate::errors::Result<String> {
        let mut data: HashMap<String, Value> = HashMap::new();
        for (col, val) in query.columns.iter().zip(query.values.iter()) {
            let field_def = self.schema.frontmatter.get(col);
            if let Some(fd) = field_def {
                if matches!(fd.field_type, FieldType::StringArray) {
                    if let SqlValue::String(s) = val {
                        data.insert(
                            col.clone(),
                            Value::List(s.split(',').map(|v| v.trim().to_string()).collect()),
                        );
                        continue;
                    }
                }
            }
            data.insert(col.clone(), sql_value_to_value(val));
        }
        let filepath = self.insert(&data, None, None, false)?;
        Ok(format!(
            "INSERT 1 ({})",
            filepath.file_name().unwrap_or_default().to_string_lossy()
        ))
    }

    fn exec_update(&self, query: &UpdateQuery) -> crate::errors::Result<String> {
        let (_, rows, _) = crate::loader::load_table_cached(&self.path, &mut self.cache.lock().unwrap())?;

        let matching: Vec<&Row> = if let Some(ref wc) = query.where_clause {
            rows.iter().filter(|r| evaluate(wc, r)).collect()
        } else {
            rows.iter().collect()
        };

        if matching.is_empty() {
            return Ok("UPDATE 0".to_string());
        }

        let mut data: HashMap<String, Value> = HashMap::new();
        for (col, val) in &query.assignments {
            let field_def = self.schema.frontmatter.get(col);
            if let Some(fd) = field_def {
                if matches!(fd.field_type, FieldType::StringArray) {
                    if let SqlValue::String(s) = val {
                        data.insert(
                            col.clone(),
                            Value::List(s.split(',').map(|v| v.trim().to_string()).collect()),
                        );
                        continue;
                    }
                }
            }
            data.insert(col.clone(), sql_value_to_value(val));
        }

        let paths: Vec<String> = matching
            .iter()
            .filter_map(|r| r.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        let _lock = TableLock::acquire(&self.path)?;
        let count;
        {
            let mut txn = TableTransaction::new(&self.path, "UPDATE")?;
            let mut c = 0;
            for path_str in &paths {
                let filepath = self.path.join(path_str);
                txn.backup(&filepath)?;
                self.update_no_lock(path_str, &data, None)?;
                c += 1;
            }
            count = c;
            txn.commit()?;
        }

        Ok(format!("UPDATE {}", count))
    }

    fn exec_delete(&self, query: &DeleteQuery) -> crate::errors::Result<String> {
        let (_, rows, _) = crate::loader::load_table_cached(&self.path, &mut self.cache.lock().unwrap())?;

        let matching: Vec<&Row> = if let Some(ref wc) = query.where_clause {
            rows.iter().filter(|r| evaluate(wc, r)).collect()
        } else {
            rows.iter().collect()
        };

        if matching.is_empty() {
            return Ok("DELETE 0".to_string());
        }

        let paths: Vec<String> = matching
            .iter()
            .filter_map(|r| r.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        let _lock = TableLock::acquire(&self.path)?;
        let count;
        {
            let mut txn = TableTransaction::new(&self.path, "DELETE")?;
            let mut c = 0;
            for path_str in &paths {
                let filepath = self.path.join(path_str);
                let content = std::fs::read_to_string(&filepath)?;
                txn.record_delete(&filepath, &content)?;
                self.delete_no_lock(path_str)?;
                c += 1;
            }
            count = c;
            txn.commit()?;
        }

        Ok(format!("DELETE {}", count))
    }

    fn data_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.path)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |e| e == "md")
                    && p.file_name()
                        .map_or(false, |n| n.to_string_lossy() != MDQL_FILENAME)
            })
            .collect();
        files.sort();
        files
    }

    fn field_kind(&self, name: &str) -> crate::errors::Result<&str> {
        if self.schema.frontmatter.contains_key(name) {
            return Ok("frontmatter");
        }
        if self.schema.sections.contains_key(name) {
            return Ok("section");
        }
        Err(MdqlError::General(format!(
            "Field '{}' not found in schema (not a frontmatter field or section)",
            name
        )))
    }

    pub fn rename_field(&mut self, old_name: &str, new_name: &str) -> crate::errors::Result<usize> {
        let kind = self.field_kind(old_name)?.to_string();
        let normalize = self.schema.rules.normalize_numbered_headings;

        let _lock = TableLock::acquire(&self.path)?;
        let mut count = 0;

        with_multi_file_txn(
            &self.path,
            &format!("RENAME FIELD {} -> {}", old_name, new_name),
            |txn| {
                for md_file in self.data_files() {
                    txn.backup(&md_file)?;
                    if kind == "frontmatter" {
                        if migrate::rename_frontmatter_key_in_file(&md_file, old_name, new_name)? {
                            count += 1;
                        }
                    } else {
                        if migrate::rename_section_in_file(&md_file, old_name, new_name, normalize)? {
                            count += 1;
                        }
                    }
                }

                let schema_path = self.path.join(MDQL_FILENAME);
                txn.backup(&schema_path)?;
                if kind == "frontmatter" {
                    migrate::update_schema(&schema_path, Some((old_name, new_name)), None, None, None, None)?;
                } else {
                    migrate::update_schema(&schema_path, None, None, Some((old_name, new_name)), None, None)?;
                }
                Ok(())
            },
        )?;

        self.schema = load_schema(&self.path)?;
        Ok(count)
    }

    pub fn drop_field(&mut self, field_name: &str) -> crate::errors::Result<usize> {
        let kind = self.field_kind(field_name)?.to_string();
        let normalize = self.schema.rules.normalize_numbered_headings;

        let _lock = TableLock::acquire(&self.path)?;
        let mut count = 0;

        with_multi_file_txn(
            &self.path,
            &format!("DROP FIELD {}", field_name),
            |txn| {
                for md_file in self.data_files() {
                    txn.backup(&md_file)?;
                    if kind == "frontmatter" {
                        if migrate::drop_frontmatter_key_in_file(&md_file, field_name)? {
                            count += 1;
                        }
                    } else {
                        if migrate::drop_section_in_file(&md_file, field_name, normalize)? {
                            count += 1;
                        }
                    }
                }

                let schema_path = self.path.join(MDQL_FILENAME);
                txn.backup(&schema_path)?;
                if kind == "frontmatter" {
                    migrate::update_schema(&schema_path, None, Some(field_name), None, None, None)?;
                } else {
                    migrate::update_schema(&schema_path, None, None, None, Some(field_name), None)?;
                }
                Ok(())
            },
        )?;

        self.schema = load_schema(&self.path)?;
        Ok(count)
    }

    pub fn merge_fields(&mut self, sources: &[String], into: &str) -> crate::errors::Result<usize> {
        for name in sources {
            let kind = self.field_kind(name)?;
            if kind != "section" {
                return Err(MdqlError::General(format!(
                    "Cannot merge frontmatter field '{}' — merge is only supported for section fields",
                    name
                )));
            }
        }

        let normalize = self.schema.rules.normalize_numbered_headings;
        let _lock = TableLock::acquire(&self.path)?;
        let mut count = 0;

        let sources_owned: Vec<String> = sources.to_vec();

        with_multi_file_txn(
            &self.path,
            &format!("MERGE FIELDS -> {}", into),
            |txn| {
                for md_file in self.data_files() {
                    txn.backup(&md_file)?;
                    if migrate::merge_sections_in_file(&md_file, &sources_owned, into, normalize)? {
                        count += 1;
                    }
                }

                let schema_path = self.path.join(MDQL_FILENAME);
                txn.backup(&schema_path)?;
                migrate::update_schema(
                    &schema_path,
                    None, None, None, None,
                    Some((&sources_owned, into)),
                )?;
                Ok(())
            },
        )?;

        self.schema = load_schema(&self.path)?;
        Ok(count)
    }

    pub fn load(&self) -> crate::errors::Result<(Vec<Row>, Vec<ValidationError>)> {
        let (_, rows, errors) = crate::loader::load_table_cached(
            &self.path,
            &mut self.cache.lock().unwrap(),
        )?;
        Ok((rows, errors))
    }

    pub fn validate(&self) -> crate::errors::Result<Vec<ValidationError>> {
        let (_, _, errors) = crate::loader::load_table_cached(
            &self.path,
            &mut self.cache.lock().unwrap(),
        )?;
        Ok(errors)
    }
}

// ── Database ──────────────────────────────────────────────────────────────

pub struct Database {
    pub path: PathBuf,
    config: DatabaseConfig,
    tables: HashMap<String, Table>,
}

impl Database {
    pub fn new(path: impl Into<PathBuf>) -> crate::errors::Result<Self> {
        let path = path.into();
        let config = load_database_config(&path)?;
        let mut tables = HashMap::new();

        let mut children: Vec<_> = std::fs::read_dir(&path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.join(MDQL_FILENAME).exists())
            .collect();
        children.sort();

        for child in children {
            let t = Table::new(&child)?;
            tables.insert(t.name().to_string(), t);
        }

        Ok(Database {
            path,
            config,
            tables,
        })
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    pub fn table_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tables.keys().cloned().collect();
        names.sort();
        names
    }

    /// Rename a file and update all foreign key references across the database.
    pub fn rename(
        &self,
        table_name: &str,
        old_filename: &str,
        new_filename: &str,
    ) -> crate::errors::Result<String> {
        let old_name = if old_filename.ends_with(".md") {
            old_filename.to_string()
        } else {
            format!("{}.md", old_filename)
        };
        let new_name = if new_filename.ends_with(".md") {
            new_filename.to_string()
        } else {
            format!("{}.md", new_filename)
        };

        let table = self.tables.get(table_name).ok_or_else(|| {
            MdqlError::General(format!("Table '{}' not found", table_name))
        })?;

        let old_path = table.path.join(&old_name);
        if !old_path.exists() {
            return Err(MdqlError::General(format!(
                "File not found: {}/{}",
                table_name, old_name
            )));
        }

        let new_path = table.path.join(&new_name);
        if new_path.exists() {
            return Err(MdqlError::General(format!(
                "Target already exists: {}/{}",
                table_name, new_name
            )));
        }

        // Find all foreign keys that reference this table
        let referencing_fks: Vec<_> = self
            .config
            .foreign_keys
            .iter()
            .filter(|fk| fk.to_table == table_name && fk.to_column == "path")
            .collect();

        // Collect files that need updating
        let mut updates: Vec<(PathBuf, String, String)> = Vec::new(); // (file_path, column, old_value)

        for fk in &referencing_fks {
            let ref_table = self.tables.get(&fk.from_table).ok_or_else(|| {
                MdqlError::General(format!(
                    "Referencing table '{}' not found",
                    fk.from_table
                ))
            })?;

            // Scan files in the referencing table
            let entries: Vec<_> = std::fs::read_dir(&ref_table.path)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()) == Some("md")
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .map_or(true, |n| n != MDQL_FILENAME)
                })
                .collect();

            for entry in entries {
                if let Ok((fm, _body)) = read_existing(&entry) {
                    if let Some(val) = fm.get(&fk.from_column) {
                        if val == &old_name {
                            updates.push((
                                entry,
                                fk.from_column.clone(),
                                val.clone(),
                            ));
                        }
                    }
                }
            }
        }

        // Perform all changes: update references first, then rename
        let mut ref_count = 0;
        for (filepath, column, _old_val) in &updates {
            let text = std::fs::read_to_string(filepath)?;
            // Replace the frontmatter value: "column: old_name" → "column: new_name"
            let old_pattern = format!("{}: {}", column, old_name);
            let new_pattern = format!("{}: {}", column, new_name);
            let updated = text.replacen(&old_pattern, &new_pattern, 1);
            // Also handle quoted form
            let old_quoted = format!("{}: \"{}\"", column, old_name);
            let new_quoted = format!("{}: \"{}\"", column, new_name);
            let updated = updated.replacen(&old_quoted, &new_quoted, 1);
            atomic_write(filepath, &updated)?;
            ref_count += 1;
        }

        // Rename the file itself
        std::fs::rename(&old_path, &new_path)?;

        let mut msg = format!("RENAME {}/{} → {}", table_name, old_name, new_name);
        if ref_count > 0 {
            msg.push_str(&format!(
                " — updated {} reference{}",
                ref_count,
                if ref_count == 1 { "" } else { "s" }
            ));
        }
        Ok(msg)
    }

    pub fn table(&mut self, name: &str) -> crate::errors::Result<&mut Table> {
        if self.tables.contains_key(name) {
            Ok(self.tables.get_mut(name).expect("key verified above"))
        } else {
            let available: Vec<String> = self.tables.keys().cloned().collect();
            Err(MdqlError::General(format!(
                "Table '{}' not found in database '{}'. Available: {}",
                name,
                self.config.name,
                if available.is_empty() {
                    "(none)".to_string()
                } else {
                    available.join(", ")
                }
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World", 80), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("My Strategy: Alpha & Beta!", 80), "my-strategy-alpha-beta");
    }

    #[test]
    fn test_slugify_max_length() {
        let result = slugify("a very long title that exceeds the limit", 10);
        assert!(result.len() <= 10);
        assert!(!result.ends_with('-'));
    }

    #[test]
    fn test_slugify_whitespace() {
        assert_eq!(slugify("  hello   world  ", 80), "hello-world");
    }

    #[test]
    fn test_coerce_int() {
        let v = coerce_cli_value("42", &FieldType::Int).unwrap();
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn test_coerce_int_error() {
        assert!(coerce_cli_value("abc", &FieldType::Int).is_err());
    }

    #[test]
    fn test_coerce_float() {
        let v = coerce_cli_value("3.14", &FieldType::Float).unwrap();
        assert_eq!(v, Value::Float(3.14));
    }

    #[test]
    fn test_coerce_bool_true() {
        assert_eq!(coerce_cli_value("true", &FieldType::Bool).unwrap(), Value::Bool(true));
        assert_eq!(coerce_cli_value("yes", &FieldType::Bool).unwrap(), Value::Bool(true));
        assert_eq!(coerce_cli_value("1", &FieldType::Bool).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_coerce_bool_false() {
        assert_eq!(coerce_cli_value("false", &FieldType::Bool).unwrap(), Value::Bool(false));
        assert_eq!(coerce_cli_value("no", &FieldType::Bool).unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_coerce_string_array() {
        let v = coerce_cli_value("a, b, c", &FieldType::StringArray).unwrap();
        assert_eq!(v, Value::List(vec!["a".into(), "b".into(), "c".into()]));
    }

    #[test]
    fn test_coerce_date() {
        let v = coerce_cli_value("2026-04-16", &FieldType::Date).unwrap();
        assert_eq!(v, Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 4, 16).unwrap()));
    }

    #[test]
    fn test_coerce_datetime() {
        let v = coerce_cli_value("2026-04-16T10:30:00", &FieldType::DateTime).unwrap();
        match v {
            Value::DateTime(dt) => {
                assert_eq!(dt.date(), chrono::NaiveDate::from_ymd_opt(2026, 4, 16).unwrap());
            }
            _ => panic!("expected DateTime"),
        }
    }

    #[test]
    fn test_coerce_string() {
        let v = coerce_cli_value("hello", &FieldType::String).unwrap();
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn test_table_new_missing_schema() {
        let dir = tempfile::tempdir().unwrap();
        let result = Table::new(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_table_insert_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let schema_content = "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\n---\n";
        std::fs::write(dir.path().join("_mdql.md"), schema_content).unwrap();

        let table = Table::new(dir.path()).unwrap();
        let mut data = HashMap::new();
        data.insert("title".into(), Value::String("Hello".into()));

        let path = table.insert(&data, None, Some("hello"), false).unwrap();
        assert!(path.exists());

        let (rows, errors) = table.load().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("title"), Some(&Value::String("Hello".into())));
        assert!(errors.is_empty());
    }

    #[test]
    fn test_table_insert_duplicate_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let schema_content = "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\n---\n";
        std::fs::write(dir.path().join("_mdql.md"), schema_content).unwrap();

        let table = Table::new(dir.path()).unwrap();
        let mut data = HashMap::new();
        data.insert("title".into(), Value::String("Hello".into()));

        table.insert(&data, None, Some("hello"), false).unwrap();
        let result = table.insert(&data, None, Some("hello"), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_table_update() {
        let dir = tempfile::tempdir().unwrap();
        let schema_content = "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\n  score:\n    type: int\n---\n";
        std::fs::write(dir.path().join("_mdql.md"), schema_content).unwrap();

        let table = Table::new(dir.path()).unwrap();
        let mut data = HashMap::new();
        data.insert("title".into(), Value::String("Test".into()));
        data.insert("score".into(), Value::Int(10));
        table.insert(&data, None, Some("test"), false).unwrap();

        let mut update = HashMap::new();
        update.insert("score".into(), Value::Int(20));
        table.update("test", &update, None).unwrap();

        let (rows, _) = table.load().unwrap();
        assert_eq!(rows[0].get("score"), Some(&Value::Int(20)));
    }

    #[test]
    fn test_table_delete() {
        let dir = tempfile::tempdir().unwrap();
        let schema_content = "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\n---\n";
        std::fs::write(dir.path().join("_mdql.md"), schema_content).unwrap();

        let table = Table::new(dir.path()).unwrap();
        let mut data = HashMap::new();
        data.insert("title".into(), Value::String("Doomed".into()));
        table.insert(&data, None, Some("doomed"), false).unwrap();

        table.delete("doomed").unwrap();
        let (rows, _) = table.load().unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_table_validate() {
        let dir = tempfile::tempdir().unwrap();
        let schema_content = "---\ntype: schema\ntable: test\nprimary_key: path\nfrontmatter:\n  title:\n    type: string\n    required: true\n---\n";
        std::fs::write(dir.path().join("_mdql.md"), schema_content).unwrap();
        std::fs::write(dir.path().join("bad.md"), "---\n---\nNo title field\n").unwrap();

        let table = Table::new(dir.path()).unwrap();
        let errors = table.validate().unwrap();
        assert!(!errors.is_empty());
    }
}
