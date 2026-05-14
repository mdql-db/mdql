//! CASCADE and RESTRICT delete: FK-aware deletion with BFS graph walk.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::database::{DatabaseConfig, ForeignKey};
use crate::model::{Row, Value};
use crate::schema::Schema;
use crate::txn::{TableLock, TableTransaction, atomic_write};

#[derive(Debug, Clone)]
pub enum CascadeAction {
    Delete { table: String, filename: String },
    PruneList { table: String, filename: String, column: String, value_to_remove: String },
}

#[derive(Debug, Clone)]
pub struct CascadePlan {
    pub primary_deletes: Vec<(String, String)>,
    pub cascade_actions: Vec<CascadeAction>,
    pub restrict_violations: Vec<String>,
}

impl CascadePlan {
    pub fn total_deletes(&self) -> usize {
        self.primary_deletes.len()
            + self.cascade_actions.iter()
                .filter(|a| matches!(a, CascadeAction::Delete { .. }))
                .count()
    }
}

pub fn build_cascade_plan(
    target_table: &str,
    matched_filenames: &[String],
    config: &DatabaseConfig,
    tables_data: &HashMap<String, (Schema, Vec<Row>)>,
) -> CascadePlan {
    let mut plan = CascadePlan {
        primary_deletes: matched_filenames.iter()
            .map(|f| (target_table.to_string(), f.clone()))
            .collect(),
        cascade_actions: Vec::new(),
        restrict_violations: Vec::new(),
    };

    let mut queue: VecDeque<(String, String)> = VecDeque::new();
    let mut visited: HashSet<(String, String)> = HashSet::new();

    for f in matched_filenames {
        let key = (target_table.to_string(), f.clone());
        visited.insert(key);
        queue.push_back((target_table.to_string(), f.clone()));
    }

    while let Some((deleted_table, deleted_filename)) = queue.pop_front() {
        let referencing_fks: Vec<&ForeignKey> = config
            .foreign_keys
            .iter()
            .filter(|fk| fk.to_table == deleted_table)
            .collect();

        for fk in referencing_fks {
            let from_rows = match tables_data.get(&fk.from_table) {
                Some(d) => &d.1,
                None => continue,
            };

            let match_value = resolve_fk_value(
                &deleted_table, &deleted_filename, &fk.to_column, tables_data,
            );
            let match_value = match match_value {
                Some(v) => v,
                None => continue,
            };

            for row in from_rows {
                let filename = match row.get("path").and_then(|v| v.as_str()) {
                    Some(f) => f.to_string(),
                    None => continue,
                };

                let fk_value = match row.get(&fk.from_column) {
                    Some(v) if !v.is_null() => v,
                    _ => continue,
                };

                match fk_value {
                    Value::List(items) => {
                        if items.iter().any(|item| item == &match_value) {
                            plan.cascade_actions.push(CascadeAction::PruneList {
                                table: fk.from_table.clone(),
                                filename: filename.clone(),
                                column: fk.from_column.clone(),
                                value_to_remove: match_value.clone(),
                            });
                        }
                    }
                    _ => {
                        if fk_value.to_display_string() == match_value {
                            let key = (fk.from_table.clone(), filename.clone());
                            if visited.insert(key) {
                                plan.cascade_actions.push(CascadeAction::Delete {
                                    table: fk.from_table.clone(),
                                    filename: filename.clone(),
                                });
                                queue.push_back((fk.from_table.clone(), filename.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    plan
}

pub fn build_restrict_plan(
    target_table: &str,
    matched_filenames: &[String],
    config: &DatabaseConfig,
    tables_data: &HashMap<String, (Schema, Vec<Row>)>,
) -> CascadePlan {
    let mut plan = CascadePlan {
        primary_deletes: matched_filenames.iter()
            .map(|f| (target_table.to_string(), f.clone()))
            .collect(),
        cascade_actions: Vec::new(),
        restrict_violations: Vec::new(),
    };

    for filename in matched_filenames {
        let referencing_fks: Vec<&ForeignKey> = config
            .foreign_keys
            .iter()
            .filter(|fk| fk.to_table == target_table)
            .collect();

        for fk in &referencing_fks {
            let from_rows = match tables_data.get(&fk.from_table) {
                Some(d) => &d.1,
                None => continue,
            };

            let match_value = resolve_fk_value(
                target_table, filename, &fk.to_column, tables_data,
            );
            let match_value = match match_value {
                Some(v) => v,
                None => continue,
            };

            for row in from_rows {
                let ref_filename = row.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let fk_value = match row.get(&fk.from_column) {
                    Some(v) if !v.is_null() => v,
                    _ => continue,
                };

                let matches = match fk_value {
                    Value::List(items) => items.iter().any(|i| i == &match_value),
                    _ => fk_value.to_display_string() == match_value,
                };

                if matches {
                    plan.restrict_violations.push(format!(
                        "{}/{} references {}/{} via {}.{}",
                        fk.from_table, ref_filename,
                        target_table, filename,
                        fk.from_table, fk.from_column,
                    ));
                }
            }
        }
    }

    plan
}

fn resolve_fk_value(
    table: &str,
    filename: &str,
    to_column: &str,
    tables_data: &HashMap<String, (Schema, Vec<Row>)>,
) -> Option<String> {
    if to_column == "path" {
        return Some(filename.to_string());
    }
    let rows = &tables_data.get(table)?.1;
    let row = rows.iter().find(|r| {
        r.get("path").and_then(|v| v.as_str()).map_or(false, |p| p == filename)
    })?;
    row.get(to_column).map(|v| v.to_display_string())
}

pub fn execute_cascade_plan(
    plan: &CascadePlan,
    db_path: &Path,
) -> crate::errors::Result<String> {
    if plan.primary_deletes.is_empty() {
        return Ok("DELETE 0".to_string());
    }

    let mut affected_tables: HashSet<String> = HashSet::new();
    for (table, _) in &plan.primary_deletes {
        affected_tables.insert(table.clone());
    }
    for action in &plan.cascade_actions {
        match action {
            CascadeAction::Delete { table, .. } | CascadeAction::PruneList { table, .. } => {
                affected_tables.insert(table.clone());
            }
        }
    }

    let mut table_names: Vec<String> = affected_tables.into_iter().collect();
    table_names.sort();

    let _locks: Vec<TableLock> = table_names
        .iter()
        .map(|name| TableLock::acquire(&db_path.join(name)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut txns: HashMap<String, TableTransaction> = HashMap::new();
    for name in &table_names {
        txns.insert(name.clone(), TableTransaction::new(&db_path.join(name), "CASCADE DELETE")?);
    }

    let result = (|| -> crate::errors::Result<(usize, usize, usize)> {
        let mut delete_count = 0;
        let mut cascade_delete_count = 0;
        let mut prune_count = 0;

        for (table, filename) in &plan.primary_deletes {
            let filepath = db_path.join(table).join(filename);
            if filepath.exists() {
                let content = std::fs::read_to_string(&filepath)?;
                txns.get_mut(table).unwrap().record_delete(&filepath, &content)?;
                std::fs::remove_file(&filepath)?;
                crate::checksums::remove_checksum(&db_path.join(table), filename)?;
                delete_count += 1;
            }
        }

        for action in &plan.cascade_actions {
            match action {
                CascadeAction::Delete { table, filename } => {
                    let filepath = db_path.join(table).join(filename);
                    if filepath.exists() {
                        let content = std::fs::read_to_string(&filepath)?;
                        txns.get_mut(table).unwrap().record_delete(&filepath, &content)?;
                        std::fs::remove_file(&filepath)?;
                        crate::checksums::remove_checksum(&db_path.join(table), filename)?;
                        cascade_delete_count += 1;
                    }
                }
                CascadeAction::PruneList { table, filename, column, value_to_remove } => {
                    let filepath = db_path.join(table).join(filename);
                    if filepath.exists() {
                        let content = std::fs::read_to_string(&filepath)?;
                        txns.get_mut(table).unwrap().record_delete(&filepath, &content)?;
                        let updated = prune_list_value(&content, column, value_to_remove);
                        atomic_write(&filepath, &updated)?;
                        prune_count += 1;
                    }
                }
            }
        }

        Ok((delete_count, cascade_delete_count, prune_count))
    })();

    match result {
        Ok((delete_count, cascade_delete_count, prune_count)) => {
            for (_, txn) in txns {
                txn.commit()?;
            }
            let mut msg = format!("DELETE {}", delete_count);
            if cascade_delete_count > 0 || prune_count > 0 {
                msg.push_str(" (cascade:");
                if cascade_delete_count > 0 {
                    msg.push_str(&format!(" {} deleted", cascade_delete_count));
                }
                if prune_count > 0 {
                    if cascade_delete_count > 0 { msg.push(','); }
                    msg.push_str(&format!(" {} list refs pruned", prune_count));
                }
                msg.push(')');
            }
            Ok(msg)
        }
        Err(e) => {
            for (_, txn) in txns {
                let _ = txn.rollback();
            }
            Err(e)
        }
    }
}

fn prune_list_value(content: &str, column: &str, value_to_remove: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut in_target_list = false;
    let mut found_column = false;

    for line in &lines {
        if in_target_list {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                let item = trimmed.strip_prefix("- ").unwrap().trim();
                let item = item.trim_matches('"').trim_matches('\'');
                if item == value_to_remove {
                    continue;
                }
                result.push(*line);
            } else {
                in_target_list = false;
                result.push(*line);
            }
        } else if !found_column && line.trim_start().starts_with(&format!("{}:", column)) {
            found_column = true;
            let after_colon = line.trim_start()
                .strip_prefix(&format!("{}:", column))
                .unwrap_or("")
                .trim();
            if after_colon.is_empty() {
                in_target_list = true;
            }
            result.push(*line);
        } else {
            result.push(*line);
        }
    }

    let mut out = result.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Value;
    use crate::schema::Schema;

    fn test_schema(table: &str) -> Schema {
        Schema {
            table: table.to_string(),
            primary_key: "path".to_string(),
            frontmatter: indexmap::IndexMap::new(),
            sections: indexmap::IndexMap::new(),
            h1_required: false,
            rules: crate::schema::Rules {
                reject_unknown_frontmatter: false,
                reject_unknown_sections: false,
                reject_duplicate_sections: false,
                normalize_numbered_headings: false,
            },
        }
    }

    fn make_row(path: &str, fields: &[(&str, Value)]) -> Row {
        let mut row = Row::new();
        row.insert("path".to_string(), Value::String(path.to_string()));
        for (k, v) in fields {
            row.insert(k.to_string(), v.clone());
        }
        row
    }

    #[test]
    fn test_cascade_no_dependents() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[("title", Value::String("Alpha".into()))]),
        ]));

        let plan = build_cascade_plan("strats", &["alpha.md".into()], &config, &tables);
        assert_eq!(plan.primary_deletes.len(), 1);
        assert!(plan.cascade_actions.is_empty());
    }

    #[test]
    fn test_cascade_single_level() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strats".into(),
                to_column: "path".into(),
            }],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[]),
        ]));
        tables.insert("backtests".into(), (test_schema("backtests"), vec![
            make_row("bt-alpha.md", &[("strategy", Value::String("alpha.md".into()))]),
            make_row("bt-beta.md", &[("strategy", Value::String("beta.md".into()))]),
        ]));

        let plan = build_cascade_plan("strats", &["alpha.md".into()], &config, &tables);
        assert_eq!(plan.primary_deletes.len(), 1);
        assert_eq!(plan.cascade_actions.len(), 1);
        assert!(matches!(&plan.cascade_actions[0], CascadeAction::Delete { table, filename }
            if table == "backtests" && filename == "bt-alpha.md"));
    }

    #[test]
    fn test_cascade_multi_level() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![
                ForeignKey {
                    from_table: "backtests".into(),
                    from_column: "strategy".into(),
                    to_table: "strats".into(),
                    to_column: "path".into(),
                },
                ForeignKey {
                    from_table: "events".into(),
                    from_column: "backtest".into(),
                    to_table: "backtests".into(),
                    to_column: "path".into(),
                },
            ],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[]),
        ]));
        tables.insert("backtests".into(), (test_schema("backtests"), vec![
            make_row("bt-alpha.md", &[("strategy", Value::String("alpha.md".into()))]),
        ]));
        tables.insert("events".into(), (test_schema("events"), vec![
            make_row("ev-1.md", &[("backtest", Value::String("bt-alpha.md".into()))]),
        ]));

        let plan = build_cascade_plan("strats", &["alpha.md".into()], &config, &tables);
        assert_eq!(plan.primary_deletes.len(), 1);
        assert_eq!(plan.cascade_actions.len(), 2);
    }

    #[test]
    fn test_cascade_list_prune() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "strats".into(),
                from_column: "ancestry".into(),
                to_table: "strats".into(),
                to_column: "path".into(),
            }],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[]),
            make_row("beta.md", &[("ancestry", Value::List(vec![
                "alpha.md".to_string(), "gamma.md".to_string(),
            ]))]),
        ]));

        let plan = build_cascade_plan("strats", &["alpha.md".into()], &config, &tables);
        assert_eq!(plan.primary_deletes.len(), 1);
        assert_eq!(plan.cascade_actions.len(), 1);
        assert!(matches!(&plan.cascade_actions[0], CascadeAction::PruneList { column, value_to_remove, .. }
            if column == "ancestry" && value_to_remove == "alpha.md"));
    }

    #[test]
    fn test_cascade_self_referential_no_loop() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "strats".into(),
                from_column: "parent".into(),
                to_table: "strats".into(),
                to_column: "path".into(),
            }],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[("parent", Value::String("beta.md".into()))]),
            make_row("beta.md", &[("parent", Value::String("alpha.md".into()))]),
        ]));

        let plan = build_cascade_plan("strats", &["alpha.md".into()], &config, &tables);
        assert_eq!(plan.primary_deletes.len(), 1);
        assert_eq!(plan.cascade_actions.len(), 1);
        assert!(matches!(&plan.cascade_actions[0], CascadeAction::Delete { filename, .. }
            if filename == "beta.md"));
    }

    #[test]
    fn test_restrict_blocks() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strats".into(),
                to_column: "path".into(),
            }],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[]),
        ]));
        tables.insert("backtests".into(), (test_schema("backtests"), vec![
            make_row("bt-alpha.md", &[("strategy", Value::String("alpha.md".into()))]),
        ]));

        let plan = build_restrict_plan("strats", &["alpha.md".into()], &config, &tables);
        assert!(!plan.restrict_violations.is_empty());
    }

    #[test]
    fn test_restrict_allows() {
        let config = DatabaseConfig {
            name: "test".into(),
            foreign_keys: vec![ForeignKey {
                from_table: "backtests".into(),
                from_column: "strategy".into(),
                to_table: "strats".into(),
                to_column: "path".into(),
            }],
            views: vec![],
            sync: None,
        };
        let mut tables: HashMap<String, (Schema, Vec<Row>)> = HashMap::new();
        tables.insert("strats".into(), (test_schema("strats"), vec![
            make_row("alpha.md", &[]),
        ]));
        tables.insert("backtests".into(), (test_schema("backtests"), vec![
            make_row("bt-beta.md", &[("strategy", Value::String("beta.md".into()))]),
        ]));

        let plan = build_restrict_plan("strats", &["alpha.md".into()], &config, &tables);
        assert!(plan.restrict_violations.is_empty());
    }

    #[test]
    fn test_prune_list_value() {
        let content = "---\ntitle: Test\nancestry:\n  - alpha.md\n  - gamma.md\n---\n\n# Test\n";
        let result = prune_list_value(content, "ancestry", "alpha.md");
        assert!(!result.contains("alpha.md"));
        assert!(result.contains("gamma.md"));
        assert!(result.contains("title: Test"));
    }
}
