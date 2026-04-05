//! Full-text search on section content using Tantivy.

use std::collections::HashMap;
use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema as TantivySchema, Value as TantivyValue, STORED, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};

use crate::errors::{MdqlError, Result};
use crate::model::{Row, Value};

/// Full-text search engine for a single table's section content.
pub struct TableSearcher {
    index: Index,
    schema: TantivySchema,
    path_field: tantivy::schema::Field,
    section_fields: HashMap<String, tantivy::schema::Field>,
}

impl TableSearcher {
    /// Build a Tantivy index from rows.
    /// `section_names` are the section column names to index.
    pub fn build(rows: &[Row], section_names: &[String]) -> Result<Self> {
        let mut schema_builder = TantivySchema::builder();

        let path_field = schema_builder.add_text_field("_path", STORED);
        let mut section_fields = HashMap::new();
        for name in section_names {
            let field = schema_builder.add_text_field(name, TEXT | STORED);
            section_fields.insert(name.clone(), field);
        }
        // A combined "all sections" field for unqualified searches
        let all_field = schema_builder.add_text_field("_all", TEXT);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        let mut writer: IndexWriter = index
            .writer(50_000_000)
            .map_err(|e| MdqlError::General(format!("Tantivy writer error: {}", e)))?;

        for row in rows {
            let path = match row.get("path") {
                Some(Value::String(p)) => p.clone(),
                _ => continue,
            };

            let mut document = doc!(path_field => path);
            let mut all_text = String::new();

            for (name, &field) in &section_fields {
                if let Some(Value::String(content)) = row.get(name) {
                    document.add_text(field, content);
                    all_text.push_str(content);
                    all_text.push('\n');
                }
            }

            document.add_text(all_field, &all_text);
            writer
                .add_document(document)
                .map_err(|e| MdqlError::General(format!("Tantivy add error: {}", e)))?;
        }

        writer
            .commit()
            .map_err(|e| MdqlError::General(format!("Tantivy commit error: {}", e)))?;

        Ok(TableSearcher {
            index,
            schema,
            path_field,
            section_fields,
        })
    }

    /// Build from a table directory (stores index on disk for persistence).
    pub fn build_on_disk(
        rows: &[Row],
        section_names: &[String],
        index_dir: &Path,
    ) -> Result<Self> {
        std::fs::create_dir_all(index_dir)?;

        let mut schema_builder = TantivySchema::builder();
        let path_field = schema_builder.add_text_field("_path", STORED);
        let mut section_fields = HashMap::new();
        for name in section_names {
            let field = schema_builder.add_text_field(name, TEXT | STORED);
            section_fields.insert(name.clone(), field);
        }
        let all_field = schema_builder.add_text_field("_all", TEXT);

        let schema = schema_builder.build();

        // If an existing index exists, remove it and rebuild
        let index = if Index::open_in_dir(index_dir).is_ok() {
            // Remove old index
            std::fs::remove_dir_all(index_dir)?;
            std::fs::create_dir_all(index_dir)?;
            Index::create_in_dir(index_dir, schema.clone())
                .map_err(|e| MdqlError::General(format!("Tantivy create error: {}", e)))?
        } else {
            Index::create_in_dir(index_dir, schema.clone())
                .map_err(|e| MdqlError::General(format!("Tantivy create error: {}", e)))?
        };

        let mut writer: IndexWriter = index
            .writer(50_000_000)
            .map_err(|e| MdqlError::General(format!("Tantivy writer error: {}", e)))?;

        for row in rows {
            let path = match row.get("path") {
                Some(Value::String(p)) => p.clone(),
                _ => continue,
            };

            let mut document = doc!(path_field => path);
            let mut all_text = String::new();

            for (name, &field) in &section_fields {
                if let Some(Value::String(content)) = row.get(name) {
                    document.add_text(field, content);
                    all_text.push_str(content);
                    all_text.push('\n');
                }
            }

            document.add_text(all_field, &all_text);
            writer
                .add_document(document)
                .map_err(|e| MdqlError::General(format!("Tantivy add error: {}", e)))?;
        }

        writer
            .commit()
            .map_err(|e| MdqlError::General(format!("Tantivy commit error: {}", e)))?;

        Ok(TableSearcher {
            index,
            schema,
            path_field,
            section_fields,
        })
    }

    /// Search for a term across all sections (or a specific section).
    /// Returns matching file paths.
    pub fn search(&self, query_str: &str, field: Option<&str>) -> Result<Vec<String>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| MdqlError::General(format!("Tantivy reader error: {}", e)))?;

        let searcher = reader.searcher();

        // Determine which fields to search
        let search_fields: Vec<tantivy::schema::Field> = if let Some(field_name) = field {
            if let Some(&f) = self.section_fields.get(field_name) {
                vec![f]
            } else {
                return Ok(Vec::new());
            }
        } else {
            // Search the combined _all field
            let all_field = self.schema.get_field("_all")
                .map_err(|e| MdqlError::General(format!("Missing _all field: {}", e)))?;
            vec![all_field]
        };

        let parser = QueryParser::for_index(&self.index, search_fields);
        let query = parser
            .parse_query(query_str)
            .map_err(|e| MdqlError::General(format!("Tantivy parse error: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(10000))
            .map_err(|e| MdqlError::General(format!("Tantivy search error: {}", e)))?;

        let mut paths = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)
                .map_err(|e| MdqlError::General(format!("Tantivy doc error: {}", e)))?;
            if let Some(path_value) = doc.get_first(self.path_field) {
                if let Some(text) = path_value.as_str() {
                    paths.push(text.to_string());
                }
            }
        }

        Ok(paths)
    }

    /// Rebuild the index from fresh rows.
    pub fn rebuild(&mut self, rows: &[Row]) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .map_err(|e| MdqlError::General(format!("Tantivy writer error: {}", e)))?;

        writer
            .delete_all_documents()
            .map_err(|e| MdqlError::General(format!("Tantivy delete error: {}", e)))?;

        let all_field = self.schema.get_field("_all")
            .map_err(|e| MdqlError::General(format!("Missing _all field: {}", e)))?;

        for row in rows {
            let path = match row.get("path") {
                Some(Value::String(p)) => p.clone(),
                _ => continue,
            };

            let mut document = doc!(self.path_field => path);
            let mut all_text = String::new();

            for (name, &field) in &self.section_fields {
                if let Some(Value::String(content)) = row.get(name) {
                    document.add_text(field, content);
                    all_text.push_str(content);
                    all_text.push('\n');
                }
            }

            document.add_text(all_field, &all_text);
            writer
                .add_document(document)
                .map_err(|e| MdqlError::General(format!("Tantivy add error: {}", e)))?;
        }

        writer
            .commit()
            .map_err(|e| MdqlError::General(format!("Tantivy commit error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Row, Value};

    fn test_rows() -> Vec<Row> {
        vec![
            Row::from([
                ("path".into(), Value::String("a.md".into())),
                (
                    "Summary".into(),
                    Value::String("This is about machine learning and neural networks".into()),
                ),
                (
                    "Details".into(),
                    Value::String("Deep dive into backpropagation algorithms".into()),
                ),
            ]),
            Row::from([
                ("path".into(), Value::String("b.md".into())),
                (
                    "Summary".into(),
                    Value::String("A guide to database optimization".into()),
                ),
                (
                    "Details".into(),
                    Value::String("Index tuning and query planning for PostgreSQL".into()),
                ),
            ]),
            Row::from([
                ("path".into(), Value::String("c.md".into())),
                (
                    "Summary".into(),
                    Value::String("Introduction to neural network architectures".into()),
                ),
            ]),
        ]
    }

    #[test]
    fn test_search_all_sections() {
        let sections = vec!["Summary".into(), "Details".into()];
        let searcher = TableSearcher::build(&test_rows(), &sections).unwrap();

        let results = searcher.search("neural", None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"a.md".to_string()));
        assert!(results.contains(&"c.md".to_string()));
    }

    #[test]
    fn test_search_specific_section() {
        let sections = vec!["Summary".into(), "Details".into()];
        let searcher = TableSearcher::build(&test_rows(), &sections).unwrap();

        let results = searcher.search("backpropagation", Some("Details")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "a.md");
    }

    #[test]
    fn test_search_no_results() {
        let sections = vec!["Summary".into(), "Details".into()];
        let searcher = TableSearcher::build(&test_rows(), &sections).unwrap();

        let results = searcher.search("quantum", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rebuild() {
        let sections = vec!["Summary".into()];
        let mut searcher = TableSearcher::build(&test_rows(), &sections).unwrap();

        // Rebuild with different data
        let new_rows = vec![Row::from([
            ("path".into(), Value::String("d.md".into())),
            (
                "Summary".into(),
                Value::String("Quantum computing basics".into()),
            ),
        ])];

        searcher.rebuild(&new_rows).unwrap();
        let results = searcher.search("quantum", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "d.md");

        // Old data should be gone
        let old_results = searcher.search("neural", None).unwrap();
        assert!(old_results.is_empty());
    }
}
