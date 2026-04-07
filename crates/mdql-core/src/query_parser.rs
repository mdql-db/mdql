//! Hand-written recursive descent parser for the MDQL SQL subset.

use regex::Regex;
use std::sync::LazyLock;

use crate::errors::MdqlError;

// ── AST nodes ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct OrderSpec {
    pub column: String,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub column: String,
    pub op: String,
    pub value: Option<SqlValue>,
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
pub struct SelectQuery {
    pub columns: ColumnList,
    pub table: String,
    pub table_alias: Option<String>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<Vec<OrderSpec>>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnList {
    All,
    Named(Vec<String>),
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
    "JOIN", "ON",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
    "ALTER", "TABLE", "RENAME", "FIELD", "TO", "DROP", "MERGE", "FIELDS",
];

static TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        \s*(?:
            (?P<backtick>`[^`]+`)
            | (?P<string>'(?:[^'\\]|\\.)*')
            | (?P<number>-?\d+(?:\.\d+)?)
            | (?P<op><=|>=|!=|[=<>,*()])
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

        let mut cols = vec![self.parse_ident()?];
        while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
            self.advance();
            cols.push(self.parse_ident()?);
        }
        Ok(ColumnList::Named(cols))
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
        // Handle parenthesized expressions
        if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "(") {
            self.advance();
            let expr = self.parse_or_expr()?;
            self.expect("op", Some(")"))?;
            return Ok(expr);
        }

        let col = self.parse_ident()?;

        // IS NULL / IS NOT NULL
        if self.match_keyword("IS") {
            if self.match_keyword("NOT") {
                self.expect("keyword", Some("NULL"))?;
                return Ok(WhereClause::Comparison(Comparison {
                    column: col,
                    op: "IS NOT NULL".into(),
                    value: None,
                }));
            }
            self.expect("keyword", Some("NULL"))?;
            return Ok(WhereClause::Comparison(Comparison {
                column: col,
                op: "IS NULL".into(),
                value: None,
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
            }));
        }

        // LIKE
        if self.match_keyword("LIKE") {
            let val = self.parse_value()?;
            return Ok(WhereClause::Comparison(Comparison {
                column: col,
                op: "LIKE".into(),
                value: Some(val),
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
                }));
            }
            return Err(MdqlError::QueryParse("Expected LIKE after NOT".into()));
        }

        // Standard operators
        if let Some(t) = self.peek() {
            if t.token_type == "op" && ["=", "!=", "<", ">", "<=", ">="].contains(&t.value.as_str())
            {
                let op = self.advance().value;
                let val = self.parse_value()?;
                return Ok(WhereClause::Comparison(Comparison {
                    column: col,
                    op,
                    value: Some(val),
                }));
            }
        }

        let got = self.peek().map_or("end".to_string(), |t| t.raw.clone());
        Err(MdqlError::QueryParse(format!(
            "Expected operator after '{}', got '{}'",
            col, got
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
        let col = self.parse_ident()?;
        let descending = if self.match_keyword("DESC") {
            true
        } else {
            self.match_keyword("ASC");
            false
        };
        Ok(OrderSpec {
            column: col,
            descending,
        })
    }

    fn is_clause_keyword(&self, t: &Token) -> bool {
        t.token_type == "keyword"
            && ["WHERE", "ORDER", "LIMIT", "JOIN", "ON"].contains(&t.value.as_str())
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
            assert_eq!(q.columns, ColumnList::Named(vec!["title".into(), "status".into()]));
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
                ColumnList::Named(vec!["Structural Mechanism".into()])
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
}
