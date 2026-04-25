//! Hand-written recursive descent parser for the MDQL SQL subset.

use regex::Regex;
use std::sync::LazyLock;

use crate::errors::MdqlError;
pub use crate::query_ast::*;

// ── Tokenizer ──────────────────────────────────────────────────────────────

static KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "ORDER", "BY",
    "ASC", "DESC", "LIMIT", "LIKE", "IN", "IS", "NOT", "NULL",
    "JOIN", "ON", "AS", "GROUP", "HAVING",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
    "ALTER", "TABLE", "RENAME", "FIELD", "TO", "DROP", "MERGE", "FIELDS",
    "CASE", "WHEN", "THEN", "ELSE", "END",
    "INTERVAL", "DAY", "DAYS", "CURRENT_DATE", "CURRENT_TIMESTAMP", "DATEDIFF",
    "CREATE", "VIEW", "CASCADE", "RESTRICT",
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
            ("keyword", "SELECT") => {
                let q = self.parse_select()?;
                self.expect_end()?;
                Ok(Statement::Select(q))
            }
            ("keyword", "INSERT") => Ok(Statement::Insert(self.parse_insert()?)),
            ("keyword", "UPDATE") => Ok(Statement::Update(self.parse_update()?)),
            ("keyword", "DELETE") => Ok(Statement::Delete(self.parse_delete()?)),
            ("keyword", "ALTER") => self.parse_alter(),
            ("keyword", "CREATE") => self.parse_create_view(),
            ("keyword", "DROP") => self.parse_drop_view(),
            _ => Err(MdqlError::QueryParse(format!(
                "Expected SELECT, INSERT, UPDATE, DELETE, ALTER, CREATE, or DROP, got '{}'",
                t.raw
            ))),
        }
    }

    fn parse_select(&mut self) -> Result<SelectQuery, MdqlError> {
        self.expect("keyword", Some("SELECT"))?;
        let columns = self.parse_columns()?;
        self.expect("keyword", Some("FROM"))?;

        // Subquery: FROM (SELECT ...)
        let mut subquery = None;
        let (table, mut table_alias) = if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "(") {
            self.advance();
            let inner = self.parse_select()?;
            self.expect("op", Some(")"))?;
            subquery = Some(Box::new(inner));
            let alias = if let Some(t) = self.peek() {
                if t.token_type == "ident" && !self.is_clause_keyword(t) {
                    Some(self.advance().value)
                } else {
                    None
                }
            } else {
                None
            };
            ("_subquery".to_string(), alias)
        } else {
            let t = self.parse_ident()?;
            (t, None)
        };

        // Optional table alias (for non-subquery)
        if subquery.is_none() {
            if let Some(t) = self.peek() {
                if t.token_type == "ident" && !self.is_clause_keyword(t) {
                    table_alias = Some(self.advance().value);
                }
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

        let mut having = None;
        if self.match_keyword("HAVING") {
            having = Some(self.parse_or_expr()?);
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

        Ok(SelectQuery {
            columns,
            table,
            table_alias,
            subquery,
            joins,
            where_clause,
            group_by,
            having,
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

    fn parse_create_view(&mut self) -> Result<Statement, MdqlError> {
        self.expect("keyword", Some("CREATE"))?;
        self.expect("keyword", Some("VIEW"))?;
        let view_name = self.parse_ident()?;

        let columns = if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "(") {
            self.advance();
            let mut cols = vec![self.parse_ident()?];
            while self.peek().map_or(false, |t| t.token_type == "op" && t.value == ",") {
                self.advance();
                cols.push(self.parse_ident()?);
            }
            self.expect("op", Some(")"))?;
            Some(cols)
        } else {
            None
        };

        self.expect("keyword", Some("AS"))?;
        let query = Box::new(self.parse_select()?);
        self.expect_end()?;

        Ok(Statement::CreateView(CreateViewQuery {
            view_name,
            columns,
            query,
        }))
    }

    fn parse_drop_view(&mut self) -> Result<Statement, MdqlError> {
        self.expect("keyword", Some("DROP"))?;
        self.expect("keyword", Some("VIEW"))?;
        let view_name = self.parse_ident()?;
        self.expect_end()?;
        Ok(Statement::DropView(DropViewQuery { view_name }))
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

        let expr = self.parse_additive()?;

        let alias = if self.match_keyword("AS") {
            Some(self.parse_ident()?)
        } else if self.peek().map_or(false, |t| {
            t.token_type == "ident" && !self.is_clause_keyword(t)
        }) {
            Some(self.advance().value)
        } else {
            None
        };

        // Bare aggregate → SelectExpr::Aggregate for backward compat
        if let Expr::Aggregate { func, arg, arg_expr } = expr {
            return Ok(SelectExpr::Aggregate {
                func,
                arg,
                arg_expr: arg_expr.map(|e| *e),
                alias,
            });
        }

        if alias.is_none() {
            if let Expr::Column(name) = &expr {
                return Ok(SelectExpr::Column(name.clone()));
            }
        }

        Ok(SelectExpr::Expr { expr, alias })
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
            let is_sub = op_tok.value == "-";

            // Check for INTERVAL keyword: expr +/- INTERVAL n DAY
            if self.peek().map_or(false, |t| t.token_type == "keyword" && t.value == "INTERVAL") {
                self.advance(); // consume INTERVAL
                let days_expr = self.parse_multiplicative()?;
                // Expect DAY or DAYS
                if !self.match_keyword("DAY") && !self.match_keyword("DAYS") {
                    return Err(MdqlError::QueryParse("Expected DAY after INTERVAL value".into()));
                }
                let days = if is_sub {
                    Expr::UnaryMinus(Box::new(days_expr))
                } else {
                    days_expr
                };
                left = Expr::DateAdd {
                    date: Box::new(left),
                    days: Box::new(days),
                };
                continue;
            }

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
        if self.peek_is_agg_func() {
            return self.parse_agg_expr();
        }

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
            "keyword" if t.value == "CASE" => {
                self.parse_case_expr()
            }
            "keyword" if t.value == "CURRENT_DATE" => {
                self.advance();
                Ok(Expr::CurrentDate)
            }
            "keyword" if t.value == "CURRENT_TIMESTAMP" => {
                self.advance();
                Ok(Expr::CurrentTimestamp)
            }
            "keyword" if t.value == "DATEDIFF" => {
                self.advance();
                self.expect("op", Some("("))?;
                let left = self.parse_additive()?;
                self.expect("op", Some(","))?;
                let right = self.parse_additive()?;
                self.expect("op", Some(")"))?;
                Ok(Expr::DateDiff { left: Box::new(left), right: Box::new(right) })
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

    fn parse_case_expr(&mut self) -> Result<Expr, MdqlError> {
        self.expect("keyword", Some("CASE"))?;
        let mut whens = Vec::new();
        while self.match_keyword("WHEN") {
            let condition = self.parse_or_expr()?;
            self.expect("keyword", Some("THEN"))?;
            let result = self.parse_additive()?;
            whens.push((condition, Box::new(result)));
        }
        if whens.is_empty() {
            return Err(MdqlError::QueryParse("CASE requires at least one WHEN clause".into()));
        }
        let else_expr = if self.match_keyword("ELSE") {
            Some(Box::new(self.parse_additive()?))
        } else {
            None
        };
        self.expect("keyword", Some("END"))?;
        Ok(Expr::Case { whens, else_expr })
    }

    fn parse_agg_expr(&mut self) -> Result<Expr, MdqlError> {
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
        let (arg, arg_expr) = if self.peek().map_or(false, |t| t.token_type == "op" && t.value == "*") {
            self.advance();
            ("*".to_string(), None)
        } else {
            let expr = self.parse_additive()?;
            if let Expr::Column(name) = &expr {
                (name.clone(), None)
            } else {
                (expr.display_name(), Some(Box::new(expr)))
            }
        };
        self.expect("op", Some(")"))?;
        Ok(Expr::Aggregate { func, arg, arg_expr })
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
            | "CASE" | "WHEN" | "THEN" | "ELSE" | "END"
            | "HAVING" | "INTERVAL" | "DAY" | "DAYS"
            | "CURRENT_DATE" | "CURRENT_TIMESTAMP" | "DATEDIFF"
            | "CREATE" | "VIEW" | "CASCADE" | "RESTRICT"
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
                    ..
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

    // ── CASE WHEN tests ──────────────────────────────────────────

    #[test]
    fn test_case_when_basic() {
        let stmt = parse_query(
            "SELECT CASE WHEN status = 'ACTIVE' THEN 1 ELSE 0 END FROM test"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    expr: Expr::Case { .. },
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
    fn test_case_when_multiple_branches() {
        let stmt = parse_query(
            "SELECT CASE WHEN x > 10 THEN 'high' WHEN x > 5 THEN 'mid' ELSE 'low' END FROM test"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                if let SelectExpr::Expr { expr: Expr::Case { whens, else_expr }, .. } = &exprs[0] {
                    assert_eq!(whens.len(), 2);
                    assert!(else_expr.is_some());
                } else {
                    panic!("Expected Case expression");
                }
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_case_when_no_else() {
        let stmt = parse_query(
            "SELECT CASE WHEN x = 1 THEN 'one' END FROM test"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                if let SelectExpr::Expr { expr: Expr::Case { whens, else_expr }, .. } = &exprs[0] {
                    assert_eq!(whens.len(), 1);
                    assert!(else_expr.is_none());
                } else {
                    panic!("Expected Case expression");
                }
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_case_when_in_aggregate() {
        let stmt = parse_query(
            "SELECT SUM(CASE WHEN side = 'BUY' THEN size ELSE -size END) AS net FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(matches!(&exprs[0], SelectExpr::Aggregate {
                    func: AggFunc::Sum,
                    arg_expr: Some(Expr::Case { .. }),
                    alias: Some(a),
                    ..
                } if a == "net"));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_case_when_with_alias() {
        let stmt = parse_query(
            "SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END AS sign FROM test"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert!(matches!(&exprs[0], SelectExpr::Expr {
                    expr: Expr::Case { .. },
                    alias: Some(a),
                } if a == "sign"));
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_create_view() {
        let stmt = parse_query("CREATE VIEW live AS SELECT * FROM strategies WHERE status = 'LIVE'").unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "live");
            assert!(cv.columns.is_none());
            assert_eq!(cv.query.table, "strategies");
            assert!(cv.query.where_clause.is_some());
        } else {
            panic!("Expected CreateView, got {:?}", stmt);
        }
    }

    #[test]
    fn test_create_view_with_columns() {
        let stmt = parse_query("CREATE VIEW v1 (a, b) AS SELECT title, status FROM t").unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "v1");
            assert_eq!(cv.columns, Some(vec!["a".into(), "b".into()]));
        } else {
            panic!("Expected CreateView");
        }
    }

    #[test]
    fn test_drop_view() {
        let stmt = parse_query("DROP VIEW live").unwrap();
        if let Statement::DropView(dv) = stmt {
            assert_eq!(dv.view_name, "live");
        } else {
            panic!("Expected DropView, got {:?}", stmt);
        }
    }

    #[test]
    fn test_create_view_case_insensitive() {
        let stmt = parse_query("create view My_View as select * from t").unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "My_View");
        } else {
            panic!("Expected CreateView");
        }
    }

    // ── Issue #42: Arithmetic between aggregates in column expressions ──

    #[test]
    fn test_aggregate_division() {
        let stmt = parse_query(
            "SELECT token, SUM(sell) / SUM(buy) as ratio FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            assert_eq!(q.group_by, Some(vec!["token".into()]));
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 2);
                assert!(exprs[1].is_aggregate());
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_aggregate_subtraction() {
        let stmt = parse_query(
            "SELECT token, SUM(sell) - SUM(buy) as net FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs[1].output_name(), "net");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_create_view_with_arithmetic() {
        let stmt = parse_query(
            "CREATE VIEW positions AS SELECT token, SUM(sell) / SUM(buy) as ratio FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "positions");
        } else {
            panic!("Expected CreateView, got {:?}", stmt);
        }
    }

    // ── Issue #43: Subqueries in FROM ──

    #[test]
    fn test_subquery_in_from() {
        let stmt = parse_query(
            "SELECT token, sell_size FROM (SELECT token, SUM(size) as sell_size FROM orders GROUP BY token) LIMIT 5"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            assert!(q.subquery.is_some());
            assert_eq!(q.limit, Some(5));
            let sub = q.subquery.unwrap();
            assert_eq!(sub.table, "orders");
            assert!(sub.group_by.is_some());
        } else {
            panic!("Expected Select");
        }
    }

    // ── Issue #44: HAVING in CREATE VIEW ──

    #[test]
    fn test_create_view_with_having() {
        let stmt = parse_query(
            "CREATE VIEW positions AS SELECT token, SUM(sell) as sell_size, SUM(buy) as buy_size FROM orders GROUP BY token HAVING sell_size > buy_size"
        ).unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "positions");
            assert!(cv.query.having.is_some());
        } else {
            panic!("Expected CreateView, got {:?}", stmt);
        }
    }

    // ── Issue #42: Aggregate multiplication ──

    #[test]
    fn test_aggregate_multiplication() {
        let stmt = parse_query(
            "SELECT SUM(a) * 2 as doubled FROM test"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(exprs[0].is_aggregate());
                assert_eq!(exprs[0].output_name(), "doubled");
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_complex_aggregate_arithmetic() {
        let stmt = parse_query(
            "SELECT SUM(CASE WHEN side = 'SELL' THEN size ELSE 0 END) / SUM(CASE WHEN side = 'BUY' THEN size ELSE 0 END) as ratio FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert!(exprs[0].is_aggregate());
                assert_eq!(exprs[0].output_name(), "ratio");
            } else {
                panic!("Expected Named columns");
            }
            assert_eq!(q.group_by, Some(vec!["token".into()]));
        } else {
            panic!("Expected Select");
        }
    }

    // ── Issue #43: Subquery with alias and WHERE ──

    #[test]
    fn test_subquery_with_alias() {
        let stmt = parse_query(
            "SELECT x FROM (SELECT x FROM t) sub"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            assert!(q.subquery.is_some());
            let sub = q.subquery.unwrap();
            assert_eq!(sub.table, "t");
            if let ColumnList::Named(exprs) = &q.columns {
                assert_eq!(exprs.len(), 1);
                assert_eq!(exprs[0].output_name(), "x");
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_subquery_with_where() {
        let stmt = parse_query(
            "SELECT x FROM (SELECT x FROM t WHERE y > 0) LIMIT 5"
        ).unwrap();
        if let Statement::Select(q) = stmt {
            assert!(q.subquery.is_some());
            assert_eq!(q.limit, Some(5));
            let sub = q.subquery.unwrap();
            assert_eq!(sub.table, "t");
            assert!(sub.where_clause.is_some());
        } else {
            panic!("Expected Select");
        }
    }

    // ── Issue #42 + CREATE VIEW: aggregate subtraction in view ──

    #[test]
    fn test_create_view_aggregate_subtraction() {
        let stmt = parse_query(
            "CREATE VIEW v AS SELECT token, SUM(sell) - SUM(buy) as net FROM orders GROUP BY token"
        ).unwrap();
        if let Statement::CreateView(cv) = stmt {
            assert_eq!(cv.view_name, "v");
            assert_eq!(cv.query.group_by, Some(vec!["token".into()]));
            if let ColumnList::Named(exprs) = &cv.query.columns {
                assert_eq!(exprs.len(), 2);
                assert_eq!(exprs[1].output_name(), "net");
                assert!(exprs[1].is_aggregate());
            } else {
                panic!("Expected Named columns");
            }
        } else {
            panic!("Expected CreateView, got {:?}", stmt);
        }
    }
}
