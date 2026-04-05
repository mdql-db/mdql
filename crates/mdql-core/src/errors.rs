use thiserror::Error;

#[derive(Error, Debug)]
pub enum MdqlError {
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    #[error("Schema invalid: {0}")]
    SchemaInvalid(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Query parse error: {0}")]
    QueryParse(String),

    #[error("Query execution error: {0}")]
    QueryExecution(String),

    #[error("Database config error: {0}")]
    DatabaseConfig(String),

    #[error("Journal recovery error: {0}")]
    JournalRecovery(String),

    #[error("{0}")]
    General(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MdqlError>;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub file_path: String,
    pub error_type: String,
    pub field: Option<String>,
    pub message: String,
    pub line_number: Option<usize>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line_number {
            write!(f, "{}:{}: {}", self.file_path, line, self.message)
        } else {
            write!(f, "{}: {}", self.file_path, self.message)
        }
    }
}
