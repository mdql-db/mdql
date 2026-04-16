//! MDQL browser UI — axum server with embedded SPA.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use mdql_core::executor::{self, QueryResult};
use mdql_core::loader;
use mdql_core::model::{Row, Value};
use mdql_core::projector::format_results;
use mdql_core::schema::Schema;

#[derive(Embed)]
#[folder = "static/"]
struct StaticFiles;

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    tables: Arc<Mutex<HashMap<String, (Schema, Vec<Row>)>>>,
    fk_errors: Arc<Mutex<Vec<mdql_core::errors::ValidationError>>>,
}

#[derive(Serialize)]
struct TableInfo {
    name: String,
    row_count: usize,
}

#[derive(Serialize)]
struct TablesResponse {
    tables: Vec<TableInfo>,
}

#[derive(Serialize)]
struct TableDetailResponse {
    table: String,
    primary_key: String,
    row_count: usize,
    frontmatter: HashMap<String, FieldInfo>,
    sections: HashMap<String, SectionInfo>,
}

#[derive(Serialize)]
struct FieldInfo {
    #[serde(rename = "type")]
    field_type: String,
    required: bool,
    enum_values: Option<Vec<String>>,
}

#[derive(Serialize)]
struct SectionInfo {
    content_type: String,
    required: bool,
}

#[derive(Deserialize)]
struct QueryRequest {
    sql: String,
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "table".into()
}

#[derive(Serialize)]
struct QueryResponse {
    columns: Option<Vec<String>>,
    rows: Option<Vec<HashMap<String, serde_json::Value>>>,
    output: Option<String>,
    error: Option<String>,
    row_count: Option<usize>,
}

pub async fn run_server(db_path: PathBuf, port: u16) {
    // Load the database
    let tables = match load_all_tables(&db_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load database: {}", e);
            std::process::exit(1);
        }
    };

    let fk_errors: Arc<Mutex<Vec<mdql_core::errors::ValidationError>>> =
        Arc::new(Mutex::new(Vec::new()));

    let state = AppState {
        db_path: db_path.clone(),
        tables: Arc::new(Mutex::new(tables)),
        fk_errors: fk_errors.clone(),
    };

    // Start filesystem watcher for FK validation
    {
        let tables_clone = state.tables.clone();
        let fk_errors_clone = fk_errors.clone();
        let db_path_clone = db_path.clone();
        tokio::task::spawn_blocking(move || {
            let watcher = match mdql_core::watcher::FkWatcher::start(db_path_clone.clone()) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Warning: could not start FK watcher: {}", e);
                    return;
                }
            };
            loop {
                if let Some(errors) = watcher.poll() {
                    *fk_errors_clone.lock().unwrap() = errors;
                    // Also reload tables on file change
                    if let Ok(new_tables) = load_all_tables(&db_path_clone) {
                        *tables_clone.lock().unwrap() = new_tables;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });
    }

    let app = Router::new()
        .route("/api/tables", get(list_tables))
        .route("/api/tables/{name}", get(table_detail))
        .route("/api/query", post(execute_query))
        .route("/api/reload", post(reload_tables))
        .route("/api/fk-errors", get(get_fk_errors))
        .fallback(static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind");

    println!("MDQL client running at http://localhost:{}", port);

    axum::serve(listener, app).await.expect("Server failed");
}

fn load_all_tables(db_path: &std::path::Path) -> Result<HashMap<String, (Schema, Vec<Row>)>, String> {
    // Try as database first
    if let Ok((_config, tables, _errors)) = loader::load_database(db_path) {
        return Ok(tables);
    }

    // Try as single table
    match loader::load_table(db_path) {
        Ok((schema, rows, _errors)) => {
            let mut map = HashMap::new();
            map.insert(schema.table.clone(), (schema, rows));
            Ok(map)
        }
        Err(e) => Err(format!("Failed to load: {}", e)),
    }
}

async fn list_tables(State(state): State<AppState>) -> Json<TablesResponse> {
    let tables = state.tables.lock().unwrap();
    let mut infos: Vec<TableInfo> = tables
        .iter()
        .map(|(name, (_schema, rows))| TableInfo {
            name: name.clone(),
            row_count: rows.len(),
        })
        .collect();
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Json(TablesResponse { tables: infos })
}

async fn table_detail(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<TableDetailResponse>, StatusCode> {
    let tables = state.tables.lock().unwrap();
    let (schema, rows) = tables.get(&name).ok_or(StatusCode::NOT_FOUND)?;

    let frontmatter: HashMap<String, FieldInfo> = schema
        .frontmatter
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                FieldInfo {
                    field_type: format!("{:?}", v.field_type),
                    required: v.required,
                    enum_values: v.enum_values.clone(),
                },
            )
        })
        .collect();

    let sections: HashMap<String, SectionInfo> = schema
        .sections
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                SectionInfo {
                    content_type: v.content_type.clone(),
                    required: v.required,
                },
            )
        })
        .collect();

    Ok(Json(TableDetailResponse {
        table: schema.table.clone(),
        primary_key: schema.primary_key.clone(),
        row_count: rows.len(),
        frontmatter,
        sections,
    }))
}

async fn execute_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let result = executor::execute(&state.db_path, &req.sql);

    match result {
        Ok((QueryResult::Rows { rows, columns }, _warnings)) => {
            if req.format == "json" || req.format == "csv" {
                let output = format_results(&rows, Some(&columns), &req.format, 80);
                Json(QueryResponse {
                    columns: None,
                    rows: None,
                    output: Some(output),
                    error: None,
                    row_count: Some(rows.len()),
                })
            } else {
                let json_rows: Vec<HashMap<String, serde_json::Value>> = rows
                    .iter()
                    .map(|row| {
                        columns
                            .iter()
                            .map(|col| {
                                let val = row.get(col).unwrap_or(&Value::Null);
                                (col.clone(), value_to_json(val))
                            })
                            .collect()
                    })
                    .collect();

                Json(QueryResponse {
                    columns: Some(columns),
                    rows: Some(json_rows.clone()),
                    output: None,
                    error: None,
                    row_count: Some(json_rows.len()),
                })
            }
        }
        Ok((QueryResult::Message(msg), _warnings)) => {
            // Reload tables after write
            if let Ok(new_tables) = load_all_tables(&state.db_path) {
                let mut tables = state.tables.lock().unwrap();
                *tables = new_tables;
            }
            Json(QueryResponse {
                columns: None,
                rows: None,
                output: Some(msg),
                error: None,
                row_count: None,
            })
        }
        Err(e) => Json(QueryResponse {
            columns: None,
            rows: None,
            output: None,
            error: Some(e.to_string()),
            row_count: None,
        }),
    }
}

async fn get_fk_errors(State(state): State<AppState>) -> Json<serde_json::Value> {
    let errors = state.fk_errors.lock().unwrap();
    let error_list: Vec<serde_json::Value> = errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "file": e.file_path,
                "field": e.field,
                "message": e.message,
            })
        })
        .collect();
    Json(serde_json::json!({ "errors": error_list }))
}

async fn reload_tables(State(state): State<AppState>) -> Json<serde_json::Value> {
    match load_all_tables(&state.db_path) {
        Ok(new_tables) => {
            let mut tables = state.tables.lock().unwrap();
            *tables = new_tables;
            Json(serde_json::json!({ "status": "ok" }))
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "message": e })),
    }
}

fn value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::Null => serde_json::Value::Null,
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Int(n) => serde_json::json!(n),
        Value::Float(f) => serde_json::json!(f),
        Value::Bool(b) => serde_json::json!(b),
        Value::Date(d) => serde_json::Value::String(d.format("%Y-%m-%d").to_string()),
        Value::DateTime(dt) => serde_json::Value::String(dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
        Value::List(items) => serde_json::json!(items),
        Value::Dict(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match StaticFiles::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for unknown routes
            match StaticFiles::get("index.html") {
                Some(content) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/html".to_string())],
                    content.data.into_owned(),
                )
                    .into_response(),
                None => (StatusCode::NOT_FOUND, "Not found").into_response(),
            }
        }
    }
}

