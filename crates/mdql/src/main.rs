use std::collections::HashMap;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use mdql_core::api::{Database, Table, coerce_cli_value};
use mdql_core::errors::MdqlError;
use mdql_core::loader::load_table;
use mdql_core::model::Value;
use mdql_core::projector::format_results;
use mdql_core::query_engine::{execute_join_query, execute_query};
use mdql_core::query_parser::{Statement, parse_query};
use mdql_core::schema::{MDQL_FILENAME, load_schema};

#[derive(Parser)]
#[command(name = "mdql", about = "A strict Markdown database with SQL-like queries")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate all markdown files in a table folder
    Validate {
        /// Path to table folder
        folder: PathBuf,
    },
    /// Run a SQL statement against a table or database
    Query {
        /// Path to table or database folder
        folder: PathBuf,
        /// SQL-like query string
        sql: String,
        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
        /// Max chars per cell in table mode
        #[arg(short, long, default_value = "80")]
        truncate: usize,
    },
    /// Create a new entry in a table
    Create {
        /// Path to table folder
        folder: PathBuf,
        /// Field values as key=value (repeatable)
        #[arg(short = 's', long = "set", num_args = 1)]
        set_fields: Vec<String>,
        /// Override auto-generated filename
        #[arg(long)]
        filename: Option<String>,
    },
    /// Inspect normalized rows from a table folder
    Inspect {
        /// Path to table folder
        folder: PathBuf,
        /// Inspect a single file
        #[arg(short, long)]
        file: Option<String>,
        /// Output format: table, json, csv
        #[arg(long, default_value = "table")]
        format: String,
        /// Max chars per cell in table mode
        #[arg(short, long, default_value = "80")]
        truncate: usize,
    },
    /// Print the effective schema
    Schema {
        /// Path to table or database folder
        folder: PathBuf,
    },
    /// Add or update created/modified timestamps
    Stamp {
        /// Path to table folder
        folder: PathBuf,
    },
    /// Open interactive REPL
    Repl {
        /// Path to table or database folder (optional, auto-discovers)
        folder: Option<PathBuf>,
    },
    /// Open browser UI for running queries
    Client {
        /// Path to table or database folder
        folder: Option<PathBuf>,
        /// Port to serve on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

fn is_database_dir(folder: &std::path::Path) -> bool {
    let mdql_file = folder.join(MDQL_FILENAME);
    if !mdql_file.exists() {
        return false;
    }
    if let Ok(text) = std::fs::read_to_string(&mdql_file) {
        let lines: Vec<&str> = text.split('\n').collect();
        if !lines.is_empty() && lines[0].trim() == "---" {
            for i in 1..lines.len() {
                if lines[i].trim() == "---" {
                    let fm_text = lines[1..i].join("\n");
                    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&fm_text) {
                        if let Some(m) = val.as_mapping() {
                            return m
                                .get(&serde_yaml::Value::String("type".into()))
                                .and_then(|v| v.as_str())
                                == Some("database");
                        }
                    }
                    break;
                }
            }
        }
    }
    false
}

fn discover_db(start: Option<&std::path::Path>) -> Option<PathBuf> {
    let mut folder = start
        .unwrap_or(&std::env::current_dir().unwrap_or_default())
        .to_path_buf();
    if !folder.is_absolute() {
        folder = std::env::current_dir().unwrap_or_default().join(folder);
    }
    loop {
        if folder.join(MDQL_FILENAME).exists() {
            return Some(folder);
        }
        if !folder.pop() {
            return None;
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Validate { folder }) => cmd_validate(&folder),
        Some(Commands::Query {
            folder,
            sql,
            format,
            truncate,
        }) => cmd_query(&folder, &sql, &format, truncate),
        Some(Commands::Create {
            folder,
            set_fields,
            filename,
        }) => cmd_create(&folder, &set_fields, filename.as_deref()),
        Some(Commands::Inspect {
            folder,
            file,
            format,
            truncate,
        }) => cmd_inspect(&folder, file.as_deref(), &format, truncate),
        Some(Commands::Schema { folder }) => cmd_schema(&folder),
        Some(Commands::Stamp { folder }) => cmd_stamp(&folder),
        Some(Commands::Repl { folder }) => {
            let db_path = folder
                .as_ref()
                .and_then(|f| discover_db(Some(f)))
                .or_else(|| discover_db(None));
            match db_path {
                Some(p) => cmd_repl(&p),
                None => {
                    eprintln!("No _mdql.md found in current directory or any parent.");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Client { folder, port }) => {
            let db_path = folder
                .as_ref()
                .and_then(|f| discover_db(Some(f)))
                .or_else(|| discover_db(None));
            match db_path {
                Some(p) => {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(mdql_web::run_server(p, port));
                    Ok(())
                }
                None => {
                    eprintln!("No _mdql.md found in current directory or any parent.");
                    std::process::exit(1);
                }
            }
        }
        None => {
            // No subcommand — start REPL if we can discover a db
            match discover_db(None) {
                Some(p) => cmd_repl(&p),
                None => {
                    eprintln!("No _mdql.md found in current directory or any parent.");
                    std::process::exit(1);
                }
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_validate(folder: &std::path::Path) -> Result<(), MdqlError> {
    let (schema, rows, errors) = load_table(folder)?;

    if errors.is_empty() {
        println!("All {} files valid in table '{}'", rows.len(), schema.table);
    } else {
        for err in &errors {
            eprintln!("{}", err);
        }
        let error_files: std::collections::HashSet<_> =
            errors.iter().map(|e| &e.file_path).collect();
        eprintln!(
            "\n{} valid, {} invalid",
            rows.len(),
            error_files.len()
        );
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_query(
    folder: &std::path::Path,
    sql: &str,
    format: &str,
    truncate: usize,
) -> Result<(), MdqlError> {
    let stmt = parse_query(sql)?;
    let is_db = is_database_dir(folder);

    match stmt {
        Statement::Select(ref q) => {
            if !q.joins.is_empty() {
                let (_db_config, tables, _errors) =
                    mdql_core::loader::load_database(folder)?;
                let (result_rows, result_columns) = execute_join_query(q, &tables)?;
                println!(
                    "{}",
                    format_results(&result_rows, Some(&result_columns), format, truncate)
                );
            } else if is_db {
                let (_db_config, tables, _errors) =
                    mdql_core::loader::load_database(folder)?;
                let (schema, rows) = tables
                    .get(&q.table)
                    .ok_or_else(|| {
                        MdqlError::QueryExecution(format!(
                            "table '{}' not found in database",
                            q.table
                        ))
                    })?;
                let (result_rows, result_columns) = execute_query(q, rows, schema)?;
                println!(
                    "{}",
                    format_results(&result_rows, Some(&result_columns), format, truncate)
                );
            } else {
                let (schema, rows, _errors) = load_table(folder)?;
                let (result_rows, result_columns) = execute_query(q, &rows, &schema)?;
                println!(
                    "{}",
                    format_results(&result_rows, Some(&result_columns), format, truncate)
                );
            }
        }
        _ => {
            // Write operations go through Table API
            let mut table = if is_db {
                let mut db = Database::new(folder)?;
                let table_name = match &stmt {
                    Statement::Insert(q) => q.table.clone(),
                    Statement::Update(q) => q.table.clone(),
                    Statement::Delete(q) => q.table.clone(),
                    Statement::AlterRename(q) => q.table.clone(),
                    Statement::AlterDrop(q) => q.table.clone(),
                    Statement::AlterMerge(q) => q.table.clone(),
                    _ => unreachable!(),
                };
                // We need to get the table path, then create a standalone Table
                let t = db.table(&table_name)?;
                Table::new(&t.path)?
            } else {
                Table::new(folder)?
            };
            let result = table.execute_sql(sql)?;
            println!("{}", result);
        }
    }

    Ok(())
}

fn cmd_create(
    folder: &std::path::Path,
    set_fields: &[String],
    filename: Option<&str>,
) -> Result<(), MdqlError> {
    let table = Table::new(folder)?;
    let mut data: HashMap<String, Value> = HashMap::new();

    for pair in set_fields {
        let (key, raw_value) = pair.split_once('=').ok_or_else(|| {
            MdqlError::General(format!(
                "Invalid --set format '{}' (expected key=value)",
                pair
            ))
        })?;
        let key = key.trim();
        let raw_value = raw_value.trim();

        if let Some(field_def) = table.schema().frontmatter.get(key) {
            data.insert(
                key.to_string(),
                coerce_cli_value(raw_value, &field_def.field_type)?,
            );
        } else {
            data.insert(key.to_string(), Value::String(raw_value.to_string()));
        }
    }

    let filepath = table.insert(&data, None, filename, false)?;
    println!(
        "Created {}",
        filepath
            .strip_prefix(folder)
            .unwrap_or(&filepath)
            .display()
    );
    Ok(())
}

fn cmd_inspect(
    folder: &std::path::Path,
    file: Option<&str>,
    format: &str,
    truncate: usize,
) -> Result<(), MdqlError> {
    let (_schema, mut rows, _errors) = load_table(folder)?;

    if let Some(f) = file {
        rows.retain(|r| {
            r.get("path")
                .and_then(|v| v.as_str())
                .map_or(false, |p| p == f)
        });
        if rows.is_empty() {
            return Err(MdqlError::General(format!(
                "File '{}' not found or invalid",
                f
            )));
        }
    }

    println!("{}", format_results(&rows, None, format, truncate));
    Ok(())
}

fn cmd_schema(folder: &std::path::Path) -> Result<(), MdqlError> {
    let is_db = is_database_dir(folder);

    if is_db {
        let db_config = mdql_core::database::load_database_config(folder)?;
        println!("Database: {}", db_config.name);
        println!();

        let mut table_dirs: Vec<_> = std::fs::read_dir(folder)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.join(MDQL_FILENAME).exists())
            .collect();
        table_dirs.sort();

        for td in &table_dirs {
            match load_schema(td) {
                Ok(s) => {
                    print_table_schema(&s);
                    println!();
                }
                Err(e) => eprintln!("Error loading {}: {}", td.display(), e),
            }
        }

        if !db_config.foreign_keys.is_empty() {
            println!("Foreign keys:");
            for fk in &db_config.foreign_keys {
                println!(
                    "  {}.{} -> {}.{}",
                    fk.from_table, fk.from_column, fk.to_table, fk.to_column
                );
            }
        }
    } else {
        let s = load_schema(folder)?;
        print_table_schema(&s);
    }

    Ok(())
}

fn print_table_schema(s: &mdql_core::schema::Schema) {
    println!("Table: {}", s.table);
    println!("  Primary key: {}", s.primary_key);
    println!("  H1 required: {}", s.h1_required);

    println!("  Frontmatter:");
    for (name, fd) in &s.frontmatter {
        let req = if fd.required { "required" } else { "optional" };
        let enum_str = fd
            .enum_values
            .as_ref()
            .map(|e| format!(" enum={:?}", e))
            .unwrap_or_default();
        println!("    {}: {} ({}){}", name, fd.field_type.as_str(), req, enum_str);
    }

    if !s.sections.is_empty() {
        println!("  Sections:");
        for (name, sd) in &s.sections {
            let req = if sd.required { "required" } else { "optional" };
            println!("    {}: {} ({})", name, sd.content_type, req);
        }
    }

    println!("  Rules:");
    println!(
        "    reject_unknown_frontmatter: {}",
        s.rules.reject_unknown_frontmatter
    );
    println!(
        "    reject_unknown_sections: {}",
        s.rules.reject_unknown_sections
    );
    println!(
        "    reject_duplicate_sections: {}",
        s.rules.reject_duplicate_sections
    );
    println!(
        "    normalize_numbered_headings: {}",
        s.rules.normalize_numbered_headings
    );
}

fn cmd_stamp(folder: &std::path::Path) -> Result<(), MdqlError> {
    let mut results = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if name.ends_with(".md") && name != MDQL_FILENAME {
            let result = mdql_core::stamp::stamp_file(&path, None)?;
            results.push((name, result));
        }
    }

    let created_count = results.iter().filter(|(_, r)| r.created_set).count();
    let modified_count = results.iter().filter(|(_, r)| r.modified_updated).count();

    println!(
        "Stamped {} files: {} created set, {} modified updated",
        results.len(),
        created_count,
        modified_count
    );

    Ok(())
}

// --- REPL autocomplete helper ---

use rustyline::hint::HistoryHinter;

struct MdqlHelper {
    completer: MdqlCompleter,
    hinter: HistoryHinter,
}

impl rustyline::completion::Completer for MdqlHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl rustyline::hint::Hinter for MdqlHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl rustyline::highlight::Highlighter for MdqlHelper {}
impl rustyline::validate::Validator for MdqlHelper {}
impl rustyline::Helper for MdqlHelper {}

#[derive(Clone)]
struct MdqlCompleter {
    keywords: Vec<String>,
    table_names: Vec<String>,
    column_names: Vec<String>,
    commands: Vec<String>,
}

impl MdqlCompleter {
    fn new(table_names: Vec<String>, column_names: Vec<String>) -> Self {
        let keywords = [
            "SELECT", "FROM", "WHERE", "ORDER BY", "ASC", "DESC", "LIMIT",
            "AND", "OR", "IN", "LIKE", "IS NULL", "IS NOT NULL",
            "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM",
            "ALTER TABLE", "RENAME FIELD", "DROP FIELD", "MERGE FIELDS", "INTO",
            "JOIN", "ON", "GROUP BY", "HAVING", "DISTINCT",
            "COUNT", "SUM", "AVG", "MIN", "MAX",
            "AS", "*",
        ].iter().map(|s| s.to_string()).collect();
        let commands = vec![
            "\\d".to_string(), "\\q".to_string(), "\\?".to_string(),
            "quit".to_string(), "exit".to_string(), "help".to_string(),
        ];
        Self { keywords, table_names, column_names, commands }
    }
}

impl rustyline::completion::Completer for MdqlCompleter {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        let line_to_pos = &line[..pos];

        // Find the start of the current word (. breaks for alias prefixes like c.column)
        let start = line_to_pos.rfind(|c: char| c.is_whitespace() || c == ',' || c == '.')
            .map(|i| i + 1)
            .unwrap_or(0);
        let partial = &line_to_pos[start..];

        if partial.is_empty() {
            return Ok((start, vec![]));
        }

        let partial_upper = partial.to_uppercase();
        let partial_lower = partial.to_lowercase();

        let mut candidates: Vec<String> = Vec::new();

        // Commands (at start of line)
        if start == 0 {
            for cmd in &self.commands {
                if cmd.starts_with(partial) || cmd.starts_with(&partial_lower) {
                    candidates.push(cmd.clone());
                }
            }
        }

        // SQL keywords (match uppercase)
        for kw in &self.keywords {
            if kw.starts_with(&partial_upper) {
                candidates.push(kw.clone());
            }
        }

        // Table names (case-insensitive)
        for t in &self.table_names {
            if t.to_lowercase().starts_with(&partial_lower) {
                candidates.push(t.clone());
            }
        }

        // Column names (case-insensitive)
        for c in &self.column_names {
            if c.to_lowercase().starts_with(&partial_lower) {
                if c.contains(' ') {
                    candidates.push(format!("`{}`", c));
                } else {
                    candidates.push(c.clone());
                }
            }
        }

        candidates.sort();
        candidates.dedup();
        Ok((start, candidates))
    }
}

fn collect_schema_info(db_path: &std::path::Path, is_db: bool) -> (Vec<String>, Vec<String>) {
    let mut table_names = Vec::new();
    let mut column_names = vec!["path".to_string(), "h1".to_string(), "created".to_string(), "modified".to_string()];

    if is_db {
        if let Ok(entries) = std::fs::read_dir(db_path) {
            let mut dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir() && p.join(MDQL_FILENAME).exists())
                .collect();
            dirs.sort();
            for td in dirs {
                if let Ok(s) = load_schema(&td) {
                    table_names.push(s.table.clone());
                    for name in s.frontmatter.keys() {
                        if !column_names.contains(name) {
                            column_names.push(name.clone());
                        }
                    }
                    for name in s.sections.keys() {
                        if !column_names.contains(name) {
                            column_names.push(name.clone());
                        }
                    }
                }
            }
        }
    } else if let Ok(s) = load_schema(db_path) {
        table_names.push(s.table.clone());
        for name in s.frontmatter.keys() {
            if !column_names.contains(name) {
                column_names.push(name.clone());
            }
        }
        for name in s.sections.keys() {
            if !column_names.contains(name) {
                column_names.push(name.clone());
            }
        }
    }

    (table_names, column_names)
}

fn cmd_repl(db_path: &std::path::Path) -> Result<(), MdqlError> {
    use rustyline::error::ReadlineError;

    let is_db = is_database_dir(db_path);

    if is_db {
        let db_config = mdql_core::database::load_database_config(db_path)?;
        println!("Connected to database '{}' at {}", db_config.name, db_path.display());
    } else {
        let s = load_schema(db_path)?;
        println!("Connected to table '{}' at {}", s.table, db_path.display());
    }

    println!("Type SQL queries, or \\q to quit. Tab to autocomplete.\n");

    let history_path = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("mdql_history.txt");

    let (table_names, column_names) = collect_schema_info(db_path, is_db);
    let helper = MdqlHelper {
        completer: MdqlCompleter::new(table_names, column_names),
        hinter: HistoryHinter::new(),
    };

    let config = rustyline::Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .build();

    let mut rl = rustyline::Editor::with_config(config)
        .map_err(|e| MdqlError::General(e.to_string()))?;
    rl.set_helper(Some(helper));
    let _ = rl.load_history(&history_path);

    loop {
        match rl.readline("mdql> ") {
            Ok(line) => {
                let sql = line.trim();
                if sql.is_empty() {
                    continue;
                }
                rl.add_history_entry(sql).ok();

                if sql == "\\q" || sql == "quit" || sql == "exit" {
                    break;
                }
                if sql == "\\d" {
                    describe_all(db_path, is_db);
                    continue;
                }
                if sql.starts_with("\\d ") {
                    describe_table(db_path, sql[3..].trim(), is_db);
                    continue;
                }
                if sql == "\\?" || sql == "help" {
                    println!("  \\d          list tables (or show fields if single table)");
                    println!("  \\d <table>  describe a table's fields");
                    println!("  \\q          quit");
                    continue;
                }

                match exec_repl_query(db_path, sql, is_db) {
                    Ok(()) => {}
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    Ok(())
}

fn exec_repl_query(folder: &std::path::Path, sql: &str, is_db: bool) -> Result<(), MdqlError> {
    let stmt = parse_query(sql)?;

    match stmt {
        Statement::Select(ref q) => {
            if !q.joins.is_empty() {
                let (_, tables, _) = mdql_core::loader::load_database(folder)?;
                let (rows, cols) = execute_join_query(q, &tables)?;
                println!("{}", format_results(&rows, Some(&cols), "table", 0));
            } else if is_db {
                let (_, tables, _) = mdql_core::loader::load_database(folder)?;
                let (schema, rows) = tables
                    .get(&q.table)
                    .ok_or_else(|| MdqlError::QueryExecution(format!("table '{}' not found", q.table)))?;
                let (result_rows, cols) = execute_query(q, rows, schema)?;
                println!("{}", format_results(&result_rows, Some(&cols), "table", 0));
            } else {
                let (schema, rows, _) = load_table(folder)?;
                let (result_rows, cols) = execute_query(q, &rows, &schema)?;
                println!("{}", format_results(&result_rows, Some(&cols), "table", 0));
            }
        }
        _ => {
            let mut table = if is_db {
                let mut db = Database::new(folder)?;
                let table_name = match &stmt {
                    Statement::Insert(q) => q.table.clone(),
                    Statement::Update(q) => q.table.clone(),
                    Statement::Delete(q) => q.table.clone(),
                    Statement::AlterRename(q) => q.table.clone(),
                    Statement::AlterDrop(q) => q.table.clone(),
                    Statement::AlterMerge(q) => q.table.clone(),
                    _ => unreachable!(),
                };
                let t = db.table(&table_name)?;
                Table::new(&t.path)?
            } else {
                Table::new(folder)?
            };
            let result = table.execute_sql(sql)?;
            println!("{}", result);
        }
    }

    Ok(())
}

fn describe_all(db_path: &std::path::Path, is_db: bool) {
    if is_db {
        let mut table_dirs: Vec<_> = std::fs::read_dir(db_path)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.join(MDQL_FILENAME).exists())
            .collect();
        table_dirs.sort();
        println!("Tables:");
        for td in table_dirs {
            match load_schema(&td) {
                Ok(s) => println!("  {}", s.table),
                Err(_) => println!("  {} (error loading schema)", td.display()),
            }
        }
    } else {
        match load_schema(db_path) {
            Ok(s) => print_fields(&s),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

fn describe_table(db_path: &std::path::Path, table_name: &str, is_db: bool) {
    let table_dir = if is_db {
        db_path.join(table_name)
    } else {
        db_path.to_path_buf()
    };

    match load_schema(&table_dir) {
        Ok(s) => print_fields(&s),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn print_fields(s: &mdql_core::schema::Schema) {
    println!("Table: {}", s.table);
    println!("  path  (primary key)");
    for (name, fd) in &s.frontmatter {
        let req = if fd.required { "required" } else { "optional" };
        let enum_str = fd.enum_values.as_ref().map(|e| format!("  enum={:?}", e)).unwrap_or_default();
        println!("  {}  {}, {}{}", name, fd.field_type.as_str(), req, enum_str);
    }
    for (name, sd) in &s.sections {
        let req = if sd.required { "required" } else { "optional" };
        println!("  {}  {}, {}", name, sd.content_type, req);
    }
}
