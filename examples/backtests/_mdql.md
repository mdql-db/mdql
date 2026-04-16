---
type: schema
table: backtests
primary_key: path

frontmatter:
  title:
    type: string
    required: true
  strategy:
    type: string
    required: true
  exchange:
    type: string
    required: true
  period_start:
    type: date
    required: true
  period_end:
    type: date
    required: true
  total_trades:
    type: int
    required: true
  win_rate:
    type: float
    required: true
  sharpe:
    type: float
    required: true
  max_drawdown:
    type: float
    required: true
  status:
    type: string
    required: true
    enum: [pass, fail, inconclusive]
  created:
    type: datetime
    required: true

h1:
  required: false

sections:
  Summary:
    type: markdown
    required: true
  Configuration:
    type: markdown
    required: false
  Results:
    type: markdown
    required: true

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: false
  reject_duplicate_sections: true
---

# backtests

Backtest results linked to strategy specifications.
Each backtest references a strategy file by path.
