//! AST types for the MDQL SQL subset.

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
    Case { whens: Vec<(WhereClause, Box<Expr>)>, else_expr: Option<Box<Expr>> },
    DateAdd { date: Box<Expr>, days: Box<Expr> },
    DateDiff { left: Box<Expr>, right: Box<Expr> },
    CurrentDate,
    CurrentTimestamp,
    Aggregate { func: AggFunc, arg: String, arg_expr: Option<Box<Expr>> },
}

impl Expr {
    pub fn as_column(&self) -> Option<&str> {
        if let Expr::Column(name) = self { Some(name) } else { None }
    }

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
            Expr::Case { .. } => "CASE".to_string(),
            Expr::DateAdd { date, days } => format!("DATE_ADD({}, {})", date.display_name(), days.display_name()),
            Expr::DateDiff { left, right } => format!("DATEDIFF({}, {})", left.display_name(), right.display_name()),
            Expr::CurrentDate => "CURRENT_DATE".to_string(),
            Expr::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            Expr::Aggregate { func, arg, .. } => {
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
    }

    pub fn contains_aggregate(&self) -> bool {
        match self {
            Expr::Aggregate { .. } => true,
            Expr::BinaryOp { left, right, .. } => {
                left.contains_aggregate() || right.contains_aggregate()
            }
            Expr::UnaryMinus(inner) => inner.contains_aggregate(),
            Expr::Case { whens, else_expr } => {
                whens.iter().any(|(_, e)| e.contains_aggregate())
                    || else_expr.as_ref().map_or(false, |e| e.contains_aggregate())
            }
            _ => false,
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
    pub op: String,
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
    Aggregate { func: AggFunc, arg: String, arg_expr: Option<Expr>, alias: Option<String> },
    Expr { expr: Expr, alias: Option<String> },
}

impl SelectExpr {
    pub fn output_name(&self) -> String {
        match self {
            SelectExpr::Column(name) => name.clone(),
            SelectExpr::Aggregate { func, arg, alias, .. } => {
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
        match self {
            SelectExpr::Aggregate { .. } => true,
            SelectExpr::Expr { expr, .. } => expr.contains_aggregate(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    pub columns: ColumnList,
    pub table: String,
    pub table_alias: Option<String>,
    pub subquery: Option<Box<SelectQuery>>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<WhereClause>,
    pub group_by: Option<Vec<String>>,
    pub having: Option<WhereClause>,
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
pub struct CreateViewQuery {
    pub view_name: String,
    pub columns: Option<Vec<String>>,
    pub query: Box<SelectQuery>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropViewQuery {
    pub view_name: String,
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
    CreateView(CreateViewQuery),
    DropView(DropViewQuery),
}

impl Statement {
    pub fn table_name(&self) -> &str {
        match self {
            Statement::Select(q) => &q.table,
            Statement::Insert(q) => &q.table,
            Statement::Update(q) => &q.table,
            Statement::Delete(q) => &q.table,
            Statement::AlterRename(q) => &q.table,
            Statement::AlterDrop(q) => &q.table,
            Statement::AlterMerge(q) => &q.table,
            Statement::CreateView(q) => &q.view_name,
            Statement::DropView(q) => &q.view_name,
        }
    }
}
