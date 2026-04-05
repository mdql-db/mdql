"""Hand-written recursive descent parser for the MDQL SQL subset.

Supported grammar:
    SELECT columns FROM table
    [WHERE predicates]
    [ORDER BY col [ASC|DESC] [, ...]]
    [LIMIT n]

Predicates: =, !=, <, >, <=, >=, LIKE, IN (...), IS NULL, IS NOT NULL
Boolean: AND, OR (OR binds looser than AND)
Column names with spaces: use backtick quoting (`Structural Mechanism`)
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, Literal, Union

from mdql.errors import QueryParseError

# ── AST nodes ──────────────────────────────────────────────────────────────

@dataclass
class OrderSpec:
    column: str
    descending: bool = False


@dataclass
class Comparison:
    column: str
    op: str  # "=", "!=", "<", ">", "<=", ">=", "LIKE", "IN", "IS NULL", "IS NOT NULL"
    value: Any = None  # str, int, float, list[str], None


@dataclass
class BoolOp:
    op: str  # "AND" or "OR"
    left: WhereClause
    right: WhereClause


WhereClause = Union[Comparison, BoolOp]


@dataclass
class JoinClause:
    table: str
    alias: str | None
    left_col: str  # e.g. "b.strategy"
    right_col: str  # e.g. "s.path"


@dataclass
class Query:
    columns: list[str] | Literal["*"]
    table: str
    table_alias: str | None = None
    join: JoinClause | None = None
    where: WhereClause | None = None
    order_by: list[OrderSpec] | None = None
    limit: int | None = None


# ── Tokenizer ──────────────────────────────────────────────────────────────

KEYWORDS = {
    "SELECT", "FROM", "WHERE", "AND", "OR", "ORDER", "BY",
    "ASC", "DESC", "LIMIT", "LIKE", "IN", "IS", "NOT", "NULL",
    "JOIN", "ON",
}

_TOKEN_RE = re.compile(
    r"""
    \s*(?:
        (?P<backtick>`[^`]+`)               # backtick-quoted identifier
        | (?P<string>'(?:[^'\\]|\\.)*')      # single-quoted string
        | (?P<number>-?\d+(?:\.\d+)?)        # number
        | (?P<op><=|>=|!=|[=<>,*()])         # operators and punctuation
        | (?P<word>[A-Za-z_][A-Za-z0-9_./-]*)  # keyword or identifier
    )
    """,
    re.VERBOSE,
)


@dataclass
class Token:
    type: str  # "keyword", "ident", "string", "number", "op"
    value: str
    raw: str = ""


def _tokenize(sql: str) -> list[Token]:
    tokens: list[Token] = []
    pos = 0
    for m in _TOKEN_RE.finditer(sql):
        if m.group("backtick"):
            raw = m.group("backtick")
            tokens.append(Token("ident", raw[1:-1], raw))
        elif m.group("string"):
            raw = m.group("string")
            tokens.append(Token("string", raw[1:-1], raw))
        elif m.group("number"):
            raw = m.group("number")
            tokens.append(Token("number", raw, raw))
        elif m.group("op"):
            raw = m.group("op")
            tokens.append(Token("op", raw, raw))
        elif m.group("word"):
            raw = m.group("word")
            if raw.upper() in KEYWORDS:
                tokens.append(Token("keyword", raw.upper(), raw))
            else:
                tokens.append(Token("ident", raw, raw))
        pos = m.end()
    return tokens


# ── Parser ─────────────────────────────────────────────────────────────────

class _Parser:
    def __init__(self, tokens: list[Token]) -> None:
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Token | None:
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return None

    def advance(self) -> Token:
        t = self.tokens[self.pos]
        self.pos += 1
        return t

    def expect(self, type_: str, value: str | None = None) -> Token:
        t = self.peek()
        if t is None:
            raise QueryParseError(f"Unexpected end of query, expected {value or type_}")
        if t.type != type_ or (value is not None and t.value != value):
            raise QueryParseError(
                f"Expected {value or type_}, got '{t.raw}' at position {self.pos}"
            )
        return self.advance()

    def match_keyword(self, kw: str) -> bool:
        t = self.peek()
        if t and t.type == "keyword" and t.value == kw:
            self.advance()
            return True
        return False

    def parse_query(self) -> Query:
        self.expect("keyword", "SELECT")
        columns = self._parse_columns()

        self.expect("keyword", "FROM")
        table = self._parse_ident()

        # Optional table alias (FROM strategies s)
        table_alias: str | None = None
        t = self.peek()
        if t and t.type == "ident" and not self._is_clause_keyword(t):
            table_alias = self.advance().value

        # Optional JOIN
        join: JoinClause | None = None
        if self.match_keyword("JOIN"):
            join_table = self._parse_ident()
            join_alias: str | None = None
            t = self.peek()
            if t and t.type == "ident" and not self._is_clause_keyword(t):
                join_alias = self.advance().value
            self.expect("keyword", "ON")
            left_col = self._parse_ident()
            self.expect("op", "=")
            right_col = self._parse_ident()
            join = JoinClause(join_table, join_alias, left_col, right_col)

        where: WhereClause | None = None
        if self.match_keyword("WHERE"):
            where = self._parse_or_expr()

        order_by: list[OrderSpec] | None = None
        if self.match_keyword("ORDER"):
            self.expect("keyword", "BY")
            order_by = self._parse_order_by()

        limit: int | None = None
        if self.match_keyword("LIMIT"):
            limit_token = self.expect("number")
            limit = int(limit_token.value)

        if self.peek() is not None:
            raise QueryParseError(
                f"Unexpected token '{self.peek().raw}' at position {self.pos}"
            )

        return Query(
            columns=columns, table=table, table_alias=table_alias,
            join=join, where=where, order_by=order_by, limit=limit,
        )

    @staticmethod
    def _is_clause_keyword(t: Token) -> bool:
        """Check if a token is a keyword that starts a clause (not an alias)."""
        return t.type == "keyword" and t.value in {
            "WHERE", "ORDER", "LIMIT", "JOIN", "ON",
        }

    def _parse_columns(self) -> list[str] | Literal["*"]:
        t = self.peek()
        if t and t.type == "op" and t.value == "*":
            self.advance()
            return "*"

        cols = [self._parse_ident()]
        while self.peek() and self.peek().type == "op" and self.peek().value == ",":
            self.advance()
            cols.append(self._parse_ident())
        return cols

    def _parse_ident(self) -> str:
        t = self.peek()
        if t is None:
            raise QueryParseError("Expected identifier, got end of query")
        if t.type == "ident":
            self.advance()
            return t.value
        if t.type == "keyword":
            # Allow keywords as identifiers in column/table context
            self.advance()
            return t.value
        raise QueryParseError(f"Expected identifier, got '{t.raw}'")

    def _parse_or_expr(self) -> WhereClause:
        left = self._parse_and_expr()
        while self.match_keyword("OR"):
            right = self._parse_and_expr()
            left = BoolOp("OR", left, right)
        return left

    def _parse_and_expr(self) -> WhereClause:
        left = self._parse_comparison()
        while self.match_keyword("AND"):
            right = self._parse_comparison()
            left = BoolOp("AND", left, right)
        return left

    def _parse_comparison(self) -> Comparison:
        # Handle parenthesized expressions
        if self.peek() and self.peek().type == "op" and self.peek().value == "(":
            self.advance()
            expr = self._parse_or_expr()
            self.expect("op", ")")
            return expr

        col = self._parse_ident()

        # IS NULL / IS NOT NULL
        if self.match_keyword("IS"):
            if self.match_keyword("NOT"):
                self.expect("keyword", "NULL")
                return Comparison(col, "IS NOT NULL")
            self.expect("keyword", "NULL")
            return Comparison(col, "IS NULL")

        # IN (val, val, ...)
        if self.match_keyword("IN"):
            self.expect("op", "(")
            values = [self._parse_value()]
            while self.peek() and self.peek().type == "op" and self.peek().value == ",":
                self.advance()
                values.append(self._parse_value())
            self.expect("op", ")")
            return Comparison(col, "IN", values)

        # LIKE
        if self.match_keyword("LIKE"):
            val = self._parse_value()
            return Comparison(col, "LIKE", val)

        # NOT LIKE
        if self.match_keyword("NOT"):
            if self.match_keyword("LIKE"):
                val = self._parse_value()
                return Comparison(col, "NOT LIKE", val)
            raise QueryParseError("Expected LIKE after NOT")

        # Standard operators
        t = self.peek()
        if t and t.type == "op" and t.value in ("=", "!=", "<", ">", "<=", ">="):
            self.advance()
            val = self._parse_value()
            return Comparison(col, t.value, val)

        raise QueryParseError(f"Expected operator after '{col}', got '{t.raw if t else 'end'}'")

    def _parse_value(self) -> Any:
        t = self.peek()
        if t is None:
            raise QueryParseError("Expected value, got end of query")
        if t.type == "string":
            self.advance()
            return t.value
        if t.type == "number":
            self.advance()
            if "." in t.value:
                return float(t.value)
            return int(t.value)
        if t.type == "keyword" and t.value == "NULL":
            self.advance()
            return None
        raise QueryParseError(f"Expected value, got '{t.raw}'")

    def _parse_order_by(self) -> list[OrderSpec]:
        specs = [self._parse_order_spec()]
        while self.peek() and self.peek().type == "op" and self.peek().value == ",":
            self.advance()
            specs.append(self._parse_order_spec())
        return specs

    def _parse_order_spec(self) -> OrderSpec:
        col = self._parse_ident()
        desc = False
        if self.match_keyword("DESC"):
            desc = True
        elif self.match_keyword("ASC"):
            desc = False
        return OrderSpec(col, desc)


def parse_query(sql: str) -> Query:
    """Parse a SQL-like query string into a Query AST."""
    tokens = _tokenize(sql)
    if not tokens:
        raise QueryParseError("Empty query")
    parser = _Parser(tokens)
    return parser.parse_query()
