//! B-tree indexes on frontmatter fields for fast filtered queries.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{Row, Value};
use crate::schema::Schema;

/// Index for a single table's frontmatter fields.
/// Each indexed field maps values → set of file paths.
#[derive(Debug)]
pub struct TableIndex {
    indexes: HashMap<String, BTreeMap<IndexKey, Vec<String>>>,
}

/// Wrapper around Value that implements Ord for use as BTreeMap key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexKey {
    String(String),
    Int(i64),
    Bool(bool),
    Date(chrono::NaiveDate),
}

impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (IndexKey::Int(a), IndexKey::Int(b)) => a.cmp(b),
            (IndexKey::String(a), IndexKey::String(b)) => a.cmp(b),
            (IndexKey::Bool(a), IndexKey::Bool(b)) => a.cmp(b),
            (IndexKey::Date(a), IndexKey::Date(b)) => a.cmp(b),
            // Cross-type: order by variant index
            _ => self.variant_order().cmp(&other.variant_order()),
        }
    }
}

impl IndexKey {
    fn variant_order(&self) -> u8 {
        match self {
            IndexKey::Bool(_) => 0,
            IndexKey::Int(_) => 1,
            IndexKey::String(_) => 2,
            IndexKey::Date(_) => 3,
        }
    }
}

fn value_to_key(v: &Value) -> Option<IndexKey> {
    match v {
        Value::String(s) => Some(IndexKey::String(s.clone())),
        Value::Int(n) => Some(IndexKey::Int(*n)),
        Value::Bool(b) => Some(IndexKey::Bool(*b)),
        Value::Date(d) => Some(IndexKey::Date(*d)),
        Value::Float(f) => {
            // Store floats as their bit representation for ordering
            Some(IndexKey::Int(f.to_bits() as i64))
        }
        Value::Null | Value::List(_) => None,
    }
}

impl TableIndex {
    /// Build indexes for all frontmatter fields.
    pub fn build(rows: &[Row], schema: &Schema) -> Self {
        let mut indexes: HashMap<String, BTreeMap<IndexKey, Vec<String>>> = HashMap::new();

        // Create an index for each frontmatter field
        for field_name in schema.frontmatter.keys() {
            indexes.insert(field_name.clone(), BTreeMap::new());
        }

        for row in rows {
            let path = match row.get("path") {
                Some(Value::String(p)) => p.clone(),
                _ => continue,
            };

            for field_name in schema.frontmatter.keys() {
                if let Some(val) = row.get(field_name) {
                    if let Some(key) = value_to_key(val) {
                        indexes
                            .entry(field_name.clone())
                            .or_default()
                            .entry(key)
                            .or_default()
                            .push(path.clone());
                    }
                }
            }
        }

        TableIndex { indexes }
    }

    /// Lookup rows with exact value match on a field.
    pub fn lookup_eq(&self, field: &str, value: &Value) -> Vec<&str> {
        let key = match value_to_key(value) {
            Some(k) => k,
            None => return Vec::new(),
        };

        self.indexes
            .get(field)
            .and_then(|btree| btree.get(&key))
            .map(|paths| paths.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Lookup rows within a range on a field (inclusive bounds).
    pub fn lookup_range(
        &self,
        field: &str,
        min: Option<&Value>,
        max: Option<&Value>,
    ) -> Vec<&str> {
        let btree = match self.indexes.get(field) {
            Some(bt) => bt,
            None => return Vec::new(),
        };

        let min_key = min.and_then(value_to_key);
        let max_key = max.and_then(value_to_key);

        let mut result = Vec::new();

        use std::ops::Bound;
        let range_start = match &min_key {
            Some(k) => Bound::Included(k),
            None => Bound::Unbounded,
        };
        let range_end = match &max_key {
            Some(k) => Bound::Included(k),
            None => Bound::Unbounded,
        };

        for (_key, paths) in btree.range((range_start, range_end)) {
            for p in paths {
                result.push(p.as_str());
            }
        }

        result
    }

    /// Lookup rows matching any value in a set (for IN clauses).
    pub fn lookup_in(&self, field: &str, values: &[Value]) -> Vec<&str> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for val in values {
            for path in self.lookup_eq(field, val) {
                if seen.insert(path) {
                    result.push(path);
                }
            }
        }

        result
    }

    /// Remove all entries for a given path (used on UPDATE/DELETE).
    pub fn invalidate(&mut self, path: &str) {
        for btree in self.indexes.values_mut() {
            for paths in btree.values_mut() {
                paths.retain(|p| p != path);
            }
        }
    }

    /// Add entries for a new/updated row.
    pub fn update(&mut self, path: &str, row: &Row, schema: &Schema) {
        // First remove old entries
        self.invalidate(path);

        // Then add new entries
        for field_name in schema.frontmatter.keys() {
            if let Some(val) = row.get(field_name) {
                if let Some(key) = value_to_key(val) {
                    self.indexes
                        .entry(field_name.clone())
                        .or_default()
                        .entry(key)
                        .or_default()
                        .push(path.to_string());
                }
            }
        }
    }

    /// Check if a field is indexed.
    pub fn has_index(&self, field: &str) -> bool {
        self.indexes.contains_key(field)
    }

    /// Get the set of indexed fields.
    pub fn indexed_fields(&self) -> Vec<&str> {
        self.indexes.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;
    use indexmap::IndexMap;

    fn test_schema() -> Schema {
        let mut frontmatter = IndexMap::new();
        frontmatter.insert(
            "status".to_string(),
            FieldDef {
                field_type: FieldType::String,
                required: true,
                enum_values: Some(vec!["ACTIVE".into(), "KILLED".into()]),
            },
        );
        frontmatter.insert(
            "score".to_string(),
            FieldDef {
                field_type: FieldType::Int,
                required: false,
                enum_values: None,
            },
        );
        Schema {
            table: "test".into(),
            primary_key: "path".into(),
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

    fn test_rows() -> Vec<Row> {
        vec![
            Row::from([
                ("path".into(), Value::String("a.md".into())),
                ("status".into(), Value::String("ACTIVE".into())),
                ("score".into(), Value::Int(10)),
            ]),
            Row::from([
                ("path".into(), Value::String("b.md".into())),
                ("status".into(), Value::String("KILLED".into())),
                ("score".into(), Value::Int(5)),
            ]),
            Row::from([
                ("path".into(), Value::String("c.md".into())),
                ("status".into(), Value::String("ACTIVE".into())),
                ("score".into(), Value::Int(20)),
            ]),
        ]
    }

    #[test]
    fn test_build_and_lookup_eq() {
        let idx = TableIndex::build(&test_rows(), &test_schema());
        let active = idx.lookup_eq("status", &Value::String("ACTIVE".into()));
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"a.md"));
        assert!(active.contains(&"c.md"));

        let killed = idx.lookup_eq("status", &Value::String("KILLED".into()));
        assert_eq!(killed.len(), 1);
        assert_eq!(killed[0], "b.md");
    }

    #[test]
    fn test_lookup_range() {
        let idx = TableIndex::build(&test_rows(), &test_schema());
        let range = idx.lookup_range(
            "score",
            Some(&Value::Int(5)),
            Some(&Value::Int(10)),
        );
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_lookup_in() {
        let idx = TableIndex::build(&test_rows(), &test_schema());
        let result = idx.lookup_in(
            "score",
            &[Value::Int(5), Value::Int(20)],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_invalidate_and_update() {
        let schema = test_schema();
        let mut idx = TableIndex::build(&test_rows(), &schema);

        // Invalidate b.md
        idx.invalidate("b.md");
        let killed = idx.lookup_eq("status", &Value::String("KILLED".into()));
        assert_eq!(killed.len(), 0);

        // Re-add with updated status
        let new_row = Row::from([
            ("path".into(), Value::String("b.md".into())),
            ("status".into(), Value::String("ACTIVE".into())),
            ("score".into(), Value::Int(15)),
        ]);
        idx.update("b.md", &new_row, &schema);
        let active = idx.lookup_eq("status", &Value::String("ACTIVE".into()));
        assert_eq!(active.len(), 3);
    }
}
