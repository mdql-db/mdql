---
type: schema
table: strategies
primary_key: path

frontmatter:
  title:
    type: string
    required: true
  status:
    type: string
    required: true
    enum: [HYPOTHESIS, BACKTESTING, PAPER_TRADING, LIVE, ARCHIVED, KILLED, PAUSED]
  mechanism:
    type: int
    required: true
  implementation:
    type: int
    required: true
  safety:
    type: int
    required: true
  frequency:
    type: int
    required: true
  composite:
    type: int
    required: true
  categories:
    type: string[]
    required: true
  created:
    type: date
    required: true
  pipeline_stage:
    type: string
    required: true
  killed:
    type: date
    required: false
  kill_reason:
    type: string
    required: false

h1:
  required: false

sections: {}

rules:
  reject_unknown_frontmatter: true
  reject_unknown_sections: false
  reject_duplicate_sections: true
  normalize_numbered_headings: true
---

# strategies

Trading strategy specifications in the Zunid research pipeline.

## Field Notes

### composite
Product of mechanism x implementation x safety x frequency.

### status
Lifecycle: HYPOTHESIS -> BACKTESTING -> PAPER_TRADING -> LIVE -> ARCHIVED/KILLED.

### categories
Flexible tags from: defi-protocol, lending, liquidation, lst-staking,
basis-trade, airdrop, token-supply, calendar-seasonal, exchange-structure.
