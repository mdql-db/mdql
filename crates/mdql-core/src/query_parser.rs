//! Hand-written recursive descent parser for the MDQL SQL subset.

use regex::Regex;
use std::sync::LazyLock;

use crate::errors::MdqlError;

// ── AST nodes ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(SqlValue),
    Column(String),
    BinaryOp { left: Box<Expr>, op: ArithOp, right: Box<Expr> },
    UnaryMinus(Box<Expr>),
}

impl Expr {
    /// If the expression is a simple column reference, return the name.
    pub fn as_column(&self) -> Option<&str> {
        match self {
            Expr::Column(name) => Some(name),
            _ => None,
        }
    }

    /// A display name for this expression (used as output column name).
    pub fn display_name(&self) -> String {
        match self {
            Expr::Literal(SqlValue::Int(n)) => n.to_string(),
            Expr::Literal(SqlValue::Float(f)) => f.to_string(),
            Expr::Literal(SqlValue::String(s)) => format!("'{}'", s),
            Expr::Literal(SqlValue::Null) => "NULL".to_string(),
            Expr::Literal(SqlValue::List(_)) => "list".to_string(),
            Expr::Column(name) => name.clone(),
            Expr::BinaryOp { left, op, right } => {
                let op_str = match op {
                    ArithOp::Add => "+",
                    ArithOp::Sub => "-",
                    ArithOp::Mul => "*",
                    ArithOp::Div => "/",
                    ArithOp::Mod => "%",
                };
                format!("{} {} {}", left.display_name(), op_str, right.display_name())
            }
            Expr::UnaryMinus(inner) => format!("-{}", inner.display_name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderSpec {
    pub column: String,
    pub expr: Option<Expr>,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub column: String,
    pub op: String,
    pub value: Option<SqlValue>,
    pub left_expr: Option<Expr>,
    pub right_expr: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoolOp {
    pub op: String, // "AND" or "OR"
    pub left: Box<WhereClause>,
    pub right: Box<WhereClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    Comparison(Comparison),
    BoolOp(BoolOp),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    String(String),
    Int(i64),
    Float(f64),
    Null,
    List(Vec<SqlValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinClause {
    pub table: String,
    pub alias: Option<String>,
    pub left_col: String,
    pub right_col: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectExpr {
    Column(String),
    Aggregate { func: AggFunc, arg: String, alias: Option<String> },
    Expr { expr: Expr, alias: Option<String> },
}

impl SelectExpr {
    pub fn output_name(&self) -> String {
        match self {
            SelectExpr::Column(name) => name.clone(),
            SelectExpr::Aggregate { func, arg, alias } => {
                if let Some(a) = alias {
                    a.clone()
                } else {
                    let func_name = match func {
                        AggFunc::Count => "COUNT",
                        AggFunc::Sum => "SUM",
                        AggFunc::Avg => "AVG",
                        AggFunc::Min => "MIN",
                        AggFunc::Max => "MAX",
                    };
                    format!("{}({})", func_name, arg)
                }
            }
            SelectExpr::Expr { expr, alias } => {
                alias.clone().unwrap_or_else(|| expr.display_name())
            }
        }
    }

    pub fn is_aggregate(&self) -> bool {
        matches!(self, SelectExpr::Aggregate { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    pub columns: ColumnList,
    pub table: String,
    pub table_alias: Option<String>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<WhereClause>,
    pub group_by: Option<Vec<String>>,
    pub order_by: Option<Vec<OrderSpec>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnList {
    All,
    Named(Vec<SelectExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertQuery {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<SqlValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateQuery {
    pub table: String,
    pub assignments: Vec<(String, SqlValue)>,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteQuery {
    pub table: String,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlterRenameFieldQuery {
    pub table: String,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlterDropFieldQuery {
    pub table: String,
    pub field_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlterMergeFieldsQuery {
    pub table: String,
    pub sources: Vec<String>,
    pub into: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select(SelectQuery),
    Insert(InsertQuery),
    Update(UpdateQuery),
    Delete(DeleteQuery),
    AlterRename(AlterRenameFieldQuery),
    AlterDrop(AlterDropFieldQuery),
    AlterMerge(AlterMergeFieldsQuery),
}

// ── Tokenizer ──────────────────────────────────────────────────────────────

static KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "ORDER", "BY",
    "ASC", "DESC", "LIMIT", "LIKE", "IN", "IS", "NOT", "NULL",
    "JOIN", "ON", "AS", "GROUP",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
    "ALTER", "TABLE", "RENAME", "FIELD", "TO", "DROP", "MERGE", "FIELDS",
];

static AGG_FUNCS: &[&str] = &["COUNT", "SUM", "AVG", "MIN", "MAX"];

static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        \s*(?:
            (?P<backtick>`[^`]+`)
            | (?P<string>'(?:[^'\\]|\\.)*')
            | (?P<number>\d+(?:\.\d+)?)
            | (?P<op><=|>=|!=|[=<>,*()+\-/%])
            | (?P<word>[A-Za-z_][A-Za-z0-9_./-]*)
        )"#,
    )
    .unwrap()
});

#[derive(Debug, Clone)]
struct Token {
    token_type: String,
    value: String,
    raw: String,
}

fn tokenize(sql: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    for caps in TOKEN_RE.captures_iter(sql) {
        if let Some(m) = caps.name("backtick") {
            let raw = m.as_str();
            tokens.push(Token {
                token_type: "ident".into(),
                value: raw[1..raw.len() - 1].into(),
                raw: raw.into(),
            });
        } else if let Some(m) = caps.name("string") {
            let raw = m.as_str();
            tokens.push(Token {
                token_type: "string".into(),
                value: raw[1..raw.len() - 1].into(),
                raw: raw.into(),
            });
        } else if let Some(m) = caps.name("number") {
            let raw = m.as_str();
            tokens.push(Token {
                token_type: "number".into(),
                value: raw.into(),
                raw: raw.into(),
            });
        } else if let Some(m) = caps.name("op") {
            let raw = m.as_str();
            tokens.push(Token {
                token_type: "op".into(),
                value: raw.into(),
                raw: raw.into(),
            });
        } else if let Some(m) = caps.name("word") {
            let raw = m.as_str();
            if KEYWORDS.contains(&raw.to_uppercase().as_str()) {
                tokens.push(Token {
                    token_type: "keyword".into(),
                    value: raw.to_uppercase(),
                    raw: raw.into(),
                });
            } else {
                tokens.push(Token {
                    token_type: "ident".into(),
                    value: raw.into(),
                    raw: raw.into(),
                });
            }
        }
    }
    tokens
}

// ── Parser ─────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn expect(&mut self, type_: &str, value: Option<&str>) -> Result<Token, MdqlError> {
        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse(format!(
                "Unexpected end of query, expected {}",
                value.unwrap_or(type_)
            ))
        })?;
        let matches_type = t.token_type == type_;
        let matches_value = value.map_or(true, |v| t.value == v);
        if !matches_type || !matches_value {
            return Err(MdqlError::QueryParse(format!(
                "Expected {}, got '{}' at position {}",
                value.unwrap_or(type_),
                t.raw,
                self.pos
            )));
        }
        Ok(self.advance())
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        if let Some(t) = self.peek() {
            if t.token_type == "keyword" && t.value == kw {
                self.advance();
                return true;
            }
        }
        false
    }

    fn parse_statement(&mut self) -> Result<Statement, MdqlError> {
        let t = self.peek().ok_or_else(|| MdqlError::QueryParse("Empty query".into()))?;
        match (t.token_type.as_str(), t.value.as_str()) {
            ("keyword", "SELECT") => Ok(Statement::Select(self.parse_select()?)),
            ("keyword", "INSERT") => Ok(Statement::Insert(self.parse_insert()?)),
            ("keyword", "UPDATE") => Ok(Statement::Update(self.parse_update()?)),
            ("keyword", "DELETE") => Ok(Statement::Delete(self.parse_delete()?)),
            ("keyword", "ALTER") => self.parse_alter(),
            _ => Err(MdqlError::QueryParse(format!(
                "Expected SELECT, INSERT, UPDATE, DELETE, or ALTER, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_select(&mut self) -> Result<SelectQuery, MdqlError> {
        self.expect("keyword", Some("SELECT"))?;
        let columns = self.parse_columns()?;
        self.expect("keyword", Some("FROM"))?;
        let table = self.parse_ident()?;

        // Optional table alias
        let mut table_alias = None;
        if let Some(t) = self.peek() {
            if t.token_type == "ident" && !self.is_clause_keyword(t) {
                table_alias = Some(self.advance().value);
            }
        }

        // Optional JOIN(s)
        let mut joins = Vec::new();
        while self.match_keyword("JOIN") {
            let join_table = self.parse_ident()?;
            let mut join_alias = None;
            if let Some(t) = self.peek() {
                if t.token_type == "ident" && !self.is_clause_keyword(t) {
                    join_alias = Some(self.advance().value);
                }
            }
            self.expect("keyword", Some("ON"))?;
            let left_col = self.parse_ident()?;
            self.expect("op", Some("="))?;
            let right_col = self.parse_ident()?;
            joins.push(JoinClause {
                table: join_table,
                alias: join_alias,
                left_col,
                right_col,
            });
        }

        let mut where_clause = None;
        if self.match_keyword("WHERE") {
            where_clause = Some(self.parse_or_expr()?);
        }

        let mut group_by = None;
        if self.match_keyword("GROUP") {
            self.expect("keyword", Some("BY"))?;
            let mut cols = vec![self.parse_ident()?];
            while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
                self.advance();
                cols.push(self.parse_ident()?);
            }
            group_by = Some(cols);
        }

        let mut order_by = None;
        if self.match_keyword("ORDER") {
            self.expect("keyword", Some("BY"))?;
            order_by = Some(self.parse_order_by()?);
        }

        let mut limit = None;
        if self.match_keyword("LIMIT") {
            let t = self.expect("number", None)?;
            limit = Some(t.value.parse::<i64>().map_err(|_| {
                MdqlError::QueryParse(format!("Invalid LIMIT value: {}", t.value))
            })?);
        }

        self.expect_end()?;

        Ok(SelectQuery {
            columns,
            table,
            table_alias,
            joins,
            where_clause,
            group_by,
            order_by,
            limit,
        })
    }

    fn parse_insert(&mut self) -> Result<InsertQuery, MdqlError> {
        self.expect("keyword", Some("INSERT"))?;
        self.expect("keyword", Some("INTO"))?;
        let table = self.parse_ident()?;

        self.expect("op", Some("("))?;
        let mut columns = vec![self.parse_ident()?];
        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            columns.push(self.parse_ident()?);
        }
        self.expect("op", Some(")"))?;

        self.expect("keyword", Some("VALUES"))?;

        self.expect("op", Some("("))?;
        let mut values = vec![self.parse_value()?];
        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            values.push(self.parse_value()?);
        }
        self.expect("op", Some(")"))?;

        if columns.len() != values.len() {
            return Err(MdqlError::QueryParse(format!(
                "Column count ({}) does not match value count ({})",
                columns.len(),
                values.len()
            )));
        }

        self.expect_end()?;
        Ok(InsertQuery {
            table,
            columns,
            values,
        })
    }

    fn parse_update(&mut self) -> Result<UpdateQuery, MdqlError> {
        self.expect("keyword", Some("UPDATE"))?;
        let table = self.parse_ident()?;
        self.expect("keyword", Some("SET"))?;

        let mut assignments = Vec::new();
        let col = self.parse_ident()?;
        self.expect("op", Some("="))?;
        let val = self.parse_value()?;
        assignments.push((col, val));

        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            let col = self.parse_ident()?;
            self.expect("op", Some("="))?;
            let val = self.parse_value()?;
            assignments.push((col, val));
        }

        let mut where_clause = None;
        if self.match_keyword("WHERE") {
            where_clause = Some(self.parse_or_expr()?);
        }

        self.expect_end()?;
        Ok(UpdateQuery {
            table,
            assignments,
            where_clause,
        })
    }

    fn parse_delete(&mut self) -> Result<DeleteQuery, MdqlError> {
        self.expect("keyword", Some("DELETE"))?;
        self.expect("keyword", Some("FROM"))?;
        let table = self.parse_ident()?;

        let mut where_clause = None;
        if self.match_keyword("WHERE") {
            where_clause = Some(self.parse_or_expr()?);
        }

        self.expect_end()?;
        Ok(DeleteQuery {
            table,
            where_clause,
        })
    }

    fn parse_alter(&mut self) -> Result<Statement, MdqlError> {
        self.expect("keyword", Some("ALTER"))?;
        self.expect("keyword", Some("TABLE"))?;
        let table = self.parse_ident()?;

        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected RENAME, DROP, or MERGE after table name".into())
        })?;

        match (t.token_type.as_str(), t.value.as_str()) {
            ("keyword", "RENAME") => {
                self.advance();
                self.expect("keyword", Some("FIELD"))?;
                let old_name = self.parse_string_or_ident()?;
                self.expect("keyword", Some("TO"))?;
                let new_name = self.parse_string_or_ident()?;
                self.expect_end()?;
                Ok(Statement::AlterRename(AlterRenameFieldQuery {
                    table,
                    old_name,
                    new_name,
                }))
            }
            ("keyword", "DROP") => {
                self.advance();
                self.expect("keyword", Some("FIELD"))?;
                let field_name = self.parse_string_or_ident()?;
                self.expect_end()?;
                Ok(Statement::AlterDrop(AlterDropFieldQuery {
                    table,
                    field_name,
                }))
            }
            ("keyword", "MERGE") => {
                self.advance();
                self.expect("keyword", Some("FIELDS"))?;
                let mut sources = vec![self.parse_string_or_ident()?];
                while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
                    self.advance();
                    sources.push(self.parse_string_or_ident()?);
                }
                self.expect("keyword", Some("INTO"))?;
                let target = self.parse_string_or_ident()?;
                self.expect_end()?;
                Ok(Statement::AlterMerge(AlterMergeFieldsQuery {
                    table,
                    sources,
                    into: target,
                }))
            }
            _ => Err(MdqlError::QueryParse(format!(
                "Expected RENAME, DROP, or MERGE, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_string_or_ident(&mut self) -> Result<String, MdqlError> {
        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected field name, got end of query".into())
        })?;
        match t.token_type.as_str() {
            "string" => {
                let v = self.advance().value;
                Ok(v)
            }
            "ident" | "keyword" => {
                let v = self.advance().value;
                Ok(v)
            }
            _ => Err(MdqlError::QueryParse(format!(
                "Expected field name, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_columns(&mut self) -> Result<ColumnList, MdqlError> {
        if let Some(t) = self.peek() {
            if t.token_type == "op" && t.value == "*" {
                self.advance();
                return Ok(ColumnList::All);
            }
        }

        let mut exprs = vec![self.parse_select_expr()?];
        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            exprs.push(self.parse_select_expr()?);
        }
        Ok(ColumnList::Named(exprs))
    }

    fn peek_is_agg_func(&self) -> bool {
        let t = match self.peek() {
            Some(t) => t,
            None => return false,
        };
        let name_upper = t.value.to_uppercase();
        if !AGG_FUNCS.contains(&name_upper.as_str()) {
            return false;
        }
        // Only treat as aggregate if followed by (
        self.tokens
            .get(self.pos + 1)
            .map_or(false, |next| next.token_type == "op" && next.value == "(")
    }

    fn parse_select_expr(&mut self) -> Result<SelectExpr, MdqlError> {
        let _t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected column or aggregate, got end of query".into())
        })?;

        if self.peek_is_agg_func() {
            let func_name = self.advance().value.to_uppercase();
            let func = match func_name.as_str() {
                "COUNT" => AggFunc::Count,
                "SUM" => AggFunc::Sum,
                "AVG" => AggFunc::Avg,
                "MIN" => AggFunc::Min,
                "MAX" => AggFunc::Max,
                _ => unreachable!(),
            };
            self.expect("op", Some("("))?;
            let arg = if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "*") {
                self.advance();
                "*".to_string()
            } else {
                self.parse_ident()?
            };
            self.expect("op", Some(")"))?;

            let alias = if self.match_keyword("AS") {
                Some(self.parse_ident()?)
            } else {
                None
            };

            Ok(SelectExpr::Aggregate { func, arg, alias })
        } else {
            // Parse a general expression (could be a column, literal, or arithmetic)
            let expr = self.parse_additive()?;

            // Optional alias: explicit (AS alias) or implicit (just an ident)
            let alias = if self.match_keyword("AS") {
                Some(self.parse_ident()?)
            } else if self.peek().map_or(false, |t| {
                t.token_type == "ident" && !self.is_clause_keyword(t)
            }) {
                Some(self.advance().value)
            } else {
                None
            };

            // If it's a simple column reference with no alias, return Column variant
            // for backward compatibility
            if alias.is_none() {
                if let Expr::Column(name) = &expr {
                    return Ok(SelectExpr::Column(name.clone()));
                }
            }

            Ok(SelectExpr::Expr { expr, alias })
        }
    }

    // ── Expression parser (precedence climbing) ───────────────────────

    fn peek_is_additive_op(&self) -> bool {
        self.peek().map_or(false, |t| {
            t.token_type == "op" && (t.value == "+" || t.value == "-")
        })
    }

    fn peek_is_multiplicative_op(&self) -> bool {
        self.peek().map_or(false, |t| {
            t.token_type == "op" && (t.value == "*" || t.value == "/" || t.value == "%")
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, MdqlError> {
        let mut left = self.parse_multiplicative()?;
        while self.peek_is_additive_op() {
            let op_tok = self.advance();
            let op = match op_tok.value.as_str() {
                "+" => ArithOp::Add,
                "-" => ArithOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, MdqlError> {
        let mut left = self.parse_unary()?;
        while self.peek_is_multiplicative_op() {
            let op_tok = self.advance();
            let op = match op_tok.value.as_str() {
                "*" => ArithOp::Mul,
                "/" => ArithOp::Div,
                "%" => ArithOp::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, MdqlError> {
        if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "-") {
            self.advance();
            let inner = self.parse_atom()?;
            // Fold unary minus on literals
            match inner {
                Expr::Literal(SqlValue::Int(n)) => Ok(Expr::Literal(SqlValue::Int(-n))),
                Expr::Literal(SqlValue::Float(f)) => Ok(Expr::Literal(SqlValue::Float(-f))),
                _ => Ok(Expr::UnaryMinus(Box::new(inner))),
            }
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, MdqlError> {
        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected expression, got end of query".into())
        })?;

        match t.token_type.as_str() {
            "number" => {
                let v = self.advance().value;
                if v.contains('.') {
                    let f: f64 = v.parse().map_err(|_| {
                        MdqlError::QueryParse(format!("Invalid float: {}", v))
                    })?;
                    Ok(Expr::Literal(SqlValue::Float(f)))
                } else {
                    let n: i64 = v.parse().map_err(|_| {
                        MdqlError::QueryParse(format!("Invalid int: {}", v))
                    })?;
                    Ok(Expr::Literal(SqlValue::Int(n)))
                }
            }
            "string" => {
                let v = self.advance().value;
                Ok(Expr::Literal(SqlValue::String(v)))
            }
            "keyword" if t.value == "NULL" => {
                self.advance();
                Ok(Expr::Literal(SqlValue::Null))
            }
            "op" if t.value == "(" => {
                self.advance();
                let expr = self.parse_additive()?;
                self.expect("op", Some(")"))?;
                Ok(expr)
            }
            "ident" => {
                let name = self.advance().value;
                Ok(Expr::Column(name))
            }
            "keyword" if !Self::is_reserved_keyword(&t.value) => {
                let name = self.advance().value;
                Ok(Expr::Column(name))
            }
            _ => Err(MdqlError::QueryParse(format!(
                "Expected expression, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_ident(&mut self) -> Result<String, MdqlError> {
        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected identifier, got end of query".into())
        })?;
        match t.token_type.as_str() {
            "ident" | "keyword" => {
                let v = self.advance().value;
                Ok(v)
            }
            _ => Err(MdqlError::QueryParse(format!(
                "Expected identifier, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_or_expr(&mut self) -> Result<WhereClause, MdqlError> {
        let mut left = self.parse_and_expr()?;
        while self.match_keyword("OR") {
            let right = self.parse_and_expr()?;
            left = WhereClause::BoolOp(BoolOp {
                op: "OR".into(),
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<WhereClause, MdqlError> {
        let mut left = self.parse_comparison()?;
        while self.match_keyword("AND") {
            let right = self.parse_comparison()?;
            left = WhereClause::BoolOp(BoolOp {
                op: "AND".into(),
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<WhereClause, MdqlError> {
        // Handle parenthesized boolean expressions
        if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "(") {
            // Save position — might be arithmetic parens, not boolean
            let saved_pos = self.pos;
            self.advance();
            // Try parsing as boolean (OR/AND) expression
            let result = self.parse_or_expr();
            if result.is_ok() && self.peek().map_or(false, |t| t.token_type == "op" && t.value == ")") {
                self.advance();
                return result;
            }
            // Not a boolean paren — rewind and parse as arithmetic expression
            self.pos = saved_pos;
        }

        // Parse the left side as a full expression (column, literal, or arithmetic)
        let left_expr = self.parse_additive()?;

        // Extract column name for backward compat (simple column on left side)
        let col = left_expr.as_column().unwrap_or("").to_string();

        // IS NULL / IS NOT NULL (only valid with simple column)
        if self.match_keyword("IS") {
            if self.match_keyword("NOT") {
                self.expect("keyword", Some("NULL"))?;
                return Ok(WhereClause::Comparison(Comparison {
                    column: col,
                    op: "IS NOT NULL".into(),
                    value: None,
                    left_expr: Some(left_expr),
                    right_expr: None,
                }));
            }
            self.expect("keyword", Some("NULL"))?;
            return Ok(WhereClause::Comparison(Comparison {
                column: col,
                op: "IS NULL".into(),
                value: None,
                left_expr: Some(left_expr),
                right_expr: None,
            }));
        }

        // IN (val, val, ...)
        if self.match_keyword("IN") {
            self.expect("op", Some("("))?;
            let mut values = vec![self.parse_value()?];
            while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
                self.advance();
                values.push(self.parse_value()?);
            }
            self.expect("op", Some(")"))?;
            return Ok(WhereClause::Comparison(Comparison {
                column: col,
                op: "IN".into(),
                value: Some(SqlValue::List(values)),
                left_expr: Some(left_expr),
                right_expr: None,
            }));
        }

        // LIKE
        if self.match_keyword("LIKE") {
            let val = self.parse_value()?;
            return Ok(WhereClause::Comparison(Comparison {
                column: col,
                op: "LIKE".into(),
                value: Some(val),
                left_expr: Some(left_expr),
                right_expr: None,
            }));
        }

        // NOT LIKE
        if self.match_keyword("NOT") {
            if self.match_keyword("LIKE") {
                let val = self.parse_value()?;
                return Ok(WhereClause::Comparison(Comparison {
                    column: col,
                    op: "NOT LIKE".into(),
                    value: Some(val),
                    left_expr: Some(left_expr),
                    right_expr: None,
                }));
            }
            return Err(MdqlError::QueryParse("Expected LIKE after NOT".into()));
        }

        // Standard comparison operators
        if let Some(t) = self.peek() {
            if t.token_type == "op" && ["=", "!=", "<", ">", "<=", ">="].contains(&t.value.as_str())
            {
                let op = self.advance().value;
                // Parse right side as expression
                let right_expr = self.parse_additive()?;
                // Extract SqlValue for backward compat (simple literal on right side)
                let value = match &right_expr {
                    Expr::Literal(v) => Some(v.clone()),
                    _ => None,
                };
                return Ok(WhereClause::Comparison(Comparison {
                    column: col,
                    op,
                    value,
                    left_expr: Some(left_expr),
                    right_expr: Some(right_expr),
                }));
            }
        }

        let got = self.peek().map_or("end".to_string(), |t| t.raw.clone());
        Err(MdqlError::QueryParse(format!(
            "Expected operator after '{}', got '{}'",
            left_expr.display_name(), got
        )))
    }

    fn parse_value(&mut self) -> Result<SqlValue, MdqlError> {
        let t = self.peek().ok_or_else(|| {
            MdqlError::QueryParse("Expected value, got end of query".into())
        })?;
        match t.token_type.as_str() {
            "string" => {
                let v = self.advance().value;
                Ok(SqlValue::String(v))
            }
            "number" => {
                let v = self.advance().value;
                if v.contains('.') {
                    Ok(SqlValue::Float(v.parse().map_err(|_| {
                        MdqlError::QueryParse(format!("Invalid float: {}", v))
                    })?))
                } else {
                    Ok(SqlValue::Int(v.parse().map_err(|_| {
                        MdqlError::QueryParse(format!("Invalid int: {}", v))
                    })?))
                }
            }
            "keyword" if t.value == "NULL" => {
                self.advance();
                Ok(SqlValue::Null)
            }
            _ => Err(MdqlError::QueryParse(format!(
                "Expected value, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_order_by(&mut self) -> Result<Vec<OrderSpec>, MdqlError> {
        let mut specs = vec![self.parse_order_spec()?];
        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            specs.push(self.parse_order_spec()?);
        }
        Ok(specs)
    }

    fn parse_order_spec(&mut self) -> Result<OrderSpec, MdqlError> {
        let expr = self.parse_additive()?;
        let col = expr.as_column().unwrap_or("").to_string();
        let descending = if self.match_keyword("DESC") {
            true
        } else {
            self.match_keyword("ASC");
            false
        };
        Ok(OrderSpec {
            column: col,
            expr: Some(expr),
            descending,
        })
    }

    fn is_clause_keyword(&self, t: &Token) -> bool {
        t.token_type == "keyword"
            && ["WHERE", "ORDER", "LIMIT", "JOIN", "ON", "GROUP"].contains(&t.value.as_str())
    }

    /// Keywords that should never be consumed as column names inside expressions.
    fn is_reserved_keyword(kw: &str) -> bool {
        matches!(kw,
            "AS" | "FROM" | "WHERE" | "AND" | "OR" | "ORDER" | "BY"
            | "ASC" | "DESC" | "LIMIT" | "JOIN" | "ON" | "GROUP"
            | "SELECT" | "INSERT" | "INTO" | "VALUES" | "UPDATE" | "SET"
            | "DELETE" | "ALTER" | "TABLE" | "IS" | "NOT" | "IN" | "LIKE"
            | "RENAME" | "FIELD" | "TO" | "DROP" | "MERGE" | "FIELDS"
        )
    }

    fn expect_end(&self) -> Result<(), MdqlError> {
        if let Some(t) = self.peek() {
            return Err(MdqlError::QueryParse(format!(
                "Unexpected token '{}' at position {}",
                t.raw, self.pos
            )));
        }
        Ok(())
    }
}

pub fn parse_query(sql: &str) -> crate::errors::Result<Statement> {
    let tokens = tokenize(sql);
    if tokens.is_empty() {
        return Err(MdqlError::QueryParse("Empty query".into()));
    }
    let mut parser = Parser::new(tokens);
    parser.parse_statement()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let stmt = parse_query("SELECT title, status FROM strategies").unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.columns, ColumnList::Named(vec![SelectExpr::Column("title".into()), SelectExpr::Column("status".into())]));
            assert_eq!(q.table, "strategies");
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_star() {
        let stmt = parse_query("SELECT * FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.columns, ColumnList::All);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_where_clause() {
        let stmt = parse_query("SELECT title FROM test WHERE count > 5").unwrap();
        if let Statement::Select(q) = stmt {
            assert!(q.where_clause.is_some());
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_order_by() {
        let stmt =
            parse_query("SELECT title FROM test ORDER BY composite DESC, title ASC").unwrap();
        if let Statement::Select(q) = stmt {
            let ob = q.order_by.unwrap();
            assert_eq!(ob.len(), 2);
            assert!(ob[0].descending);
            assert!(!ob[1].descending);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_limit() {
        let stmt = parse_query("SELECT * FROM test LIMIT 10").unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.limit, Some(10));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_insert() {
        let stmt = parse_query(
            "INSERT INTO test (title, count) VALUES ('Hello', 42)",
        )
        .unwrap();
        if let Statement::Insert(q) = stmt {
            assert_eq!(q.table, "test");
            assert_eq!(q.columns, vec!["title", "count"]);
            assert_eq!(q.values[0], SqlValue::String("Hello".into()));
            assert_eq!(q.values[1], SqlValue::Int(42));
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_update() {
        let stmt = parse_query("UPDATE test SET status = 'KILLED' WHERE path = 'a.md'").unwrap();
        if let Statement::Update(q) = stmt {
            assert_eq!(q.table, "test");
            assert_eq!(q.assignments.len(), 1);
            assert!(q.where_clause.is_some());
        } else {
            panic!("Expected Update");
        }
    }

    #[test]
    fn test_delete() {
        let stmt = parse_query("DELETE FROM test WHERE status = 'draft'").unwrap();
        if let Statement::Delete(q) = stmt {
            assert_eq!(q.table, "test");
            assert!(q.where_clause.is_some());
        } else {
            panic!("Expected Delete");
        }
    }

    #[test]
    fn test_alter_rename() {
        let stmt =
            parse_query("ALTER TABLE test RENAME FIELD 'Summary' TO 'Overview'").unwrap();
        if let Statement::AlterRename(q) = stmt {
            assert_eq!(q.old_name, "Summary");
            assert_eq!(q.new_name, "Overview");
        } else {
            panic!("Expected AlterRename");
        }
    }

    #[test]
    fn test_alter_drop() {
        let stmt = parse_query("ALTER TABLE test DROP FIELD 'Details'").unwrap();
        if let Statement::AlterDrop(q) = stmt {
            assert_eq!(q.field_name, "Details");
        } else {
            panic!("Expected AlterDrop");
        }
    }

    #[test]
    fn test_alter_merge() {
        let stmt = parse_query(
            "ALTER TABLE test MERGE FIELDS 'Entry Rules', 'Exit Rules' INTO 'Trading Rules'",
        )
        .unwrap();
        if let Statement::AlterMerge(q) = stmt {
            assert_eq!(q.sources, vec!["Entry Rules", "Exit Rules"]);
            assert_eq!(q.into, "Trading Rules");
        } else {
            panic!("Expected AlterMerge");
        }
    }

    #[test]
    fn test_backtick_ident() {
        let stmt = parse_query("SELECT `Structural Mechanism` FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(
                q.columns,
                ColumnList::Named(vec![SelectExpr::Column("Structural Mechanism".into())])
            );
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_like_operator() {
        let stmt = parse_query("SELECT title FROM test WHERE categories LIKE '%defi%'").unwrap();
        if let Statement::Select(q) = stmt {
            if let Some(WhereClause::Comparison(c)) = q.where_clause {
                assert_eq!(c.op, "LIKE");
                assert_eq!(c.value, Some(SqlValue::String("%defi%".into())));
            } else {
                panic!("Expected LIKE comparison");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_in_operator() {
        let stmt =
            parse_query("SELECT * FROM test WHERE status IN ('ACTIVE', 'LIVE')").unwrap();
        if let Statement::Select(q) = stmt {
            if let Some(WhereClause::Comparison(c)) = q.where_clause {
                assert_eq!(c.op, "IN");
            } else {
                panic!("Expected IN comparison");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_is_null() {
        let stmt = parse_query("SELECT * FROM test WHERE title IS NULL").unwrap();
        if let Statement::Select(q) = stmt {
            if let Some(WhereClause::Comparison(c)) = q.where_clause {
                assert_eq!(c.op, "IS NULL");
            } else {
                panic!("Expected IS NULL comparison");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_and_or() {
        let stmt = parse_query(
            "SELECT * FROM test WHERE status = 'ACTIVE' AND count > 5 OR title LIKE '%test%'",
        )
        .unwrap();
        if let Statement::Select(q) = stmt {
            assert!(q.where_clause.is_some());
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_join() {
        let stmt = parse_query(
            "SELECT s.title, b.sharpe FROM strategies s JOIN backtests b ON b.strategy = s.path",
        )
        .unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.table, "strategies");
            assert_eq!(q.table_alias, Some("s".into()));
            assert_eq!(q.joins.len(), 1);
            let join = &q.joins[0];
            assert_eq!(join.table, "backtests");
            assert_eq!(join.alias, Some("b".into()));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_multi_join() {
        let stmt = parse_query(
            "SELECT s.title, b.sharpe, c.verdict FROM strategies s JOIN backtests b ON b.strategy = s.path JOIN critiques c ON c.strategy = s.path",
        )
        .unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.table, "strategies");
            assert_eq!(q.table_alias, Some("s".into()));
            assert_eq!(q.joins.len(), 2);
            assert_eq!(q.joins[0].table, "backtests");
            assert_eq!(q.joins[0].alias, Some("b".into()));
            assert_eq!(q.joins[0].left_col, "b.strategy");
            assert_eq!(q.joins[0].right_col, "s.path");
            assert_eq!(q.joins[1].table, "critiques");
            assert_eq!(q.joins[1].alias, Some("c".into()));
            assert_eq!(q.joins[1].left_col, "c.strategy");
            assert_eq!(q.joins[1].right_col, "s.path");
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_empty_query() {
        assert!(parse_query("").is_err());
    }

    #[test]
    fn test_count_star() {
        let stmt = parse_query("SELECT status, COUNT(*) AS cnt FROM strategies GROUP BY status").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 2);
                assert_eq!(exprs[0], SelectExpr::Column("status".into()));
                assert!(matches!(&exprs[1], SelectExpr::Aggregate {
                    func: AggFunc::Count,
                    arg,
                    alias: Some(a),
                } if arg == "*" && a == "cnt"));
            } else {
                panic!("Expected Named columns");
            }
            assert_eq!(q.group_by, Some(vec!["status".into()]));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_count_column_as_ident() {
        // "count" as a column name should NOT be parsed as the COUNT aggregate
        let stmt = parse_query("INSERT INTO test (title, count) VALUES ('Hello', 42)").unwrap();
        if let Statement::Insert(q) = stmt {
            assert_eq!(q.columns, vec!["title", "count"]);
        } else {
            panic!("Expected Insert");
        }
    }

    #[test]
    fn test_multiple_aggregates() {
        let stmt = parse_query("SELECT MIN(composite), MAX(composite), AVG(composite) FROM strategies").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 3);
                assert!(matches!(&exprs[0], SelectExpr::Aggregate { func: AggFunc::Min, .. }));
                assert!(matches!(&exprs[1], SelectExpr::Aggregate { func: AggFunc::Max, .. }));
                assert!(matches!(&exprs[2], SelectExpr::Aggregate { func: AggFunc::Avg, .. }));
            } else {
                panic!("Expected Named columns");
            }
            assert_eq!(q.group_by, None);
        } else {
            panic!("Expected Select");
        }
    }

    // ── Expression tests ──────────────────────────────────────────

    #[test]
    fn test_select_arithmetic_expr() {
        let stmt = parse_query("SELECT a + b FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    expr: Expr::BinaryOp { op: ArithOp::Add, .. },
                    alias: None,
                }));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_arithmetic_with_alias() {
        let stmt = parse_query("SELECT a + b AS total FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    alias: Some(a),
                    ..
                } if a == "total"));
                assert_eq!(exprs[0].output_name(), "total");
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_precedence() {
        // a + b * c should parse as a + (b * c)
        let stmt = parse_query("SELECT a + b * c FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                if let SelectExpr::Expr { expr, .. } = &exprs[0] {
                    if let Expr::BinaryOp { left, op, right } = expr {
                        assert_eq!(*op, ArithOp::Add);
                        assert!(matches!(left.as_ref(), Expr::Column(n) if n == "a"));
                        assert!(matches!(right.as_ref(), Expr::BinaryOp { op: ArithOp::Mul, .. }));
                    } else {
                        panic!("Expected BinaryOp");
                    }
                } else {
                    panic!("Expected Expr variant");
                }
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_parenthesized_expr() {
        // (a + b) * c should override default precedence
        let stmt = parse_query("SELECT (a + b) * c FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                if let SelectExpr::Expr { expr, .. } = &exprs[0] {
                    if let Expr::BinaryOp { left, op, .. } = expr {
                        assert_eq!(*op, ArithOp::Mul);
                        assert!(matches!(left.as_ref(), Expr::BinaryOp { op: ArithOp::Add, .. }));
                    } else {
                        panic!("Expected BinaryOp");
                    }
                } else {
                    panic!("Expected Expr variant");
                }
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_unary_minus() {
        let stmt = parse_query("SELECT -count FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    expr: Expr::UnaryMinus(_),
                    ..
                }));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_negative_literal() {
        let stmt = parse_query("SELECT -42 FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                // Unary minus folds into the literal
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    expr: Expr::Literal(SqlValue::Int(-42)),
                    ..
                }));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_where_arithmetic_expr() {
        let stmt = parse_query("SELECT * FROM test WHERE a + b > 10").unwrap();
        if let Statement::Select(q) = stmt {
            if let Some(WhereClause::Comparison(c)) = q.where_clause {
                assert_eq!(c.op, ">");
                assert!(matches!(&c.left_expr, Some(Expr::BinaryOp { op: ArithOp::Add, .. })));
                assert!(matches!(&c.right_expr, Some(Expr::Literal(SqlValue::Int(10)))));
            } else {
                panic!("Expected comparison");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_where_both_sides_expr() {
        let stmt = parse_query("SELECT * FROM test WHERE a * 2 > b + 1").unwrap();
        if let Statement::Select(q) = stmt {
            if let Some(WhereClause::Comparison(c)) = q.where_clause {
                assert_eq!(c.op, ">");
                assert!(matches!(&c.left_expr, Some(Expr::BinaryOp { op: ArithOp::Mul, .. })));
                assert!(matches!(&c.right_expr, Some(Expr::BinaryOp { op: ArithOp::Add, .. })));
            } else {
                panic!("Expected comparison");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_order_by_expr() {
        let stmt = parse_query("SELECT * FROM test ORDER BY a + b DESC").unwrap();
        if let Statement::Select(q) = stmt {
            let ob = q.order_by.unwrap();
            assert_eq!(ob.len(), 1);
            assert!(ob[0].descending);
            assert!(matches!(&ob[0].expr, Some(Expr::BinaryOp { op: ArithOp::Add, .. })));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_all_arithmetic_ops() {
        let stmt = parse_query("SELECT a + b, a - b, a * b, a / b, a % b FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 5);
                assert!(matches!(&exprs[0], SelectExpr::Expr { expr: Expr::BinaryOp { op: ArithOp::Add, .. }, .. }));
                assert!(matches!(&exprs[1], SelectExpr::Expr { expr: Expr::BinaryOp { op: ArithOp::Sub, .. }, .. }));
                assert!(matches!(&exprs[2], SelectExpr::Expr { expr: Expr::BinaryOp { op: ArithOp::Mul, .. }, .. }));
                assert!(matches!(&exprs[3], SelectExpr::Expr { expr: Expr::BinaryOp { op: ArithOp::Div, .. }, .. }));
                assert!(matches!(&exprs[4], SelectExpr::Expr { expr: Expr::BinaryOp { op: ArithOp::Mod, .. }, .. }));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_column_with_literal_arithmetic() {
        let stmt = parse_query("SELECT count * 2 + 1 FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                // Should be (count * 2) + 1
                if let SelectExpr::Expr { expr, .. } = &exprs[0] {
                    if let Expr::BinaryOp { left, op, right } = expr {
                        assert_eq!(*op, ArithOp::Add);
                        assert!(matches!(right.as_ref(), Expr::Literal(SqlValue::Int(1))));
                        assert!(matches!(left.as_ref(), Expr::BinaryOp { op: ArithOp::Mul, .. }));
                    } else {
                        panic!("Expected BinaryOp");
                    }
                } else {
                    panic!("Expected Expr");
                }
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mixed_columns_and_exprs() {
        let stmt = parse_query("SELECT title, a + b AS sum, count FROM test").unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0], SelectExpr::Column("title".into()));
                assert!(matches!(&exprs[1], SelectExpr::Expr { alias: Some(a), .. } if a == "sum"));
                assert_eq!(exprs[2], SelectExpr::Column("count".into()));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }
}
