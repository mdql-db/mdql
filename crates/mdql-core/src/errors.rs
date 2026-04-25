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
pub enum ValidationErrorKind {
    ParseError,
    MissingField,
    TypeMismatch,
    EnumViolation,
    UnknownField,
    MissingH1,
    H1Mismatch,
    DuplicateSection,
    MissingSection,
    UnknownSection,
    FkMissingTable,
    FkViolation,
    ViewError,
}

impl ValidationErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParseError => "parse_error",
            Self::MissingField => "missing_field",
            Self::TypeMismatch => "type_mismatch",
            Self::EnumViolation => "enum_violation",
            Self::UnknownField => "unknown_field",
            Self::MissingH1 => "missing_h1",
            Self::H1Mismatch => "h1_mismatch",
            Self::DuplicateSection => "duplicate_section",
            Self::MissingSection => "missing_section",
            Self::UnknownSection => "unknown_section",
            Self::FkMissingTable => "fk_missing_table",
            Self::FkViolation => "fk_violation",
            Self::ViewError => "view_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub file_path: String,
    pub error_type: ValidationErrorKind,
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
