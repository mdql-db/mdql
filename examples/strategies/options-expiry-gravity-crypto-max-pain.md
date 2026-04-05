---
title: "Options Expiry Gravity (Max Pain Pin)"
status: HYPOTHESIS
mechanism: 4
implementation: 7
safety: 6
frequency: 6
composite: 1008
categories:
  - options-derivatives
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

BTC and ETH spot prices exhibit a measurable gravitational pull toward the "max pain" strike price in the 48 hours before weekly and monthly Deribit options expiry. Market makers who are net short options delta-hedge their books in ways that create systematic price pressure toward the strike where total option value is minimized. If this effect is real and large enough to survive fees and funding, a mean-reversion trade — long perp if spot is below max pain, short perp if above — entered 48 hours before expiry and closed at expiry should generate positive expected value.

This is a hypothesis — needs backtest.

---

## Why it might be an edge

**Mechanism is structural, not sentiment-based.** Options market makers are forced hedgers. They don't choose to move price toward max pain — it emerges mechanically from delta hedging a net short options book. This is the same mechanism documented (controversially) in equity markets around monthly OPEX.

**Event is fully predictable.** Deribit options expire every Friday at 08:00 UTC. There is no information advantage needed — the calendar is public and fixed years in advance.

**Data is public and free.** Open interest by strike is available via Deribit's API with no subscription required.

**The effect should be largest on monthly expiries.** End-of-month BTC/ETH options carry substantially higher OI than weekly expirations — often $1B+ notional — meaning market maker hedging flows are larger and more likely to move price.

**Crypto options market structure may amplify the effect.** Retail in crypto predominantly buys options (calls especially), making professional desks systematically net short. This directional skew in who holds which side is the precondition for max pain gravity to exist.

**Why it might not be an edge:** The effect is contested even in equities. Crypto MMs may hedge differently or less continuously. A 5% BTC trend move in 48 hours — common — can overwhelm any pin effect. Backtest required before any conclusion.

---

## Backtest Methodology

### Data inputs

| Data | Source | Format | Notes |
|---|---|---|---|
| Historical options OI by strike | Deribit API (`/public/get_book_summary_by_currency`) | Strike, OI, expiry | Pull at each entry time: Wednesday 08:00 UTC |
| BTC/ETH hourly OHLCV | Binance API (`/api/v3/klines`) | OHLCV, 1h bars | Use to get entry and exit prices |
| Deribit expiry calendar | Computed: every Friday 08:00 UTC | Date list | Verify against Deribit historical records |
| Funding rates (Hyperliquid) | Hyperliquid API (`/info` → `fundingHistory`) | 8h funding rate | To cost each 48h hold accurately |

**Backtest window:** January 2023 – present (~100 weekly events, ~24 monthly events)

Start in 2023 to allow crypto options market to have matured sufficiently. Pre-2022 OI was lower and market structure was different.

### Max pain calculation

For each expiry event at time T (entry = T − 48h):

1. Pull all call and put OI by strike for that expiry
2. For each candidate expiry price P (iterate over every listed strike):
   - Total pain = Σ (max(0, strike − P) × call_OI) + Σ (max(0, P − strike) × put_OI)
   - This is the aggregate intrinsic value options holders would receive — which is what market makers pay
3. Max pain strike = P that minimizes total pain
4. Record: max pain price, spot price at T − 48h, distance = (spot − max pain) / max pain

### Trade classification

| Condition | Trade |
|---|---|
| spot < max pain × 0.95 (spot > 5% below max pain) | Long BTC or ETH perp |
| spot > max pain × 1.05 (spot > 5% above max pain) | Short BTC or ETH perp |
| \|spot − max pain\| / max pain ≤ 5% | No trade (insufficient dislocation) |

### Filtering conditions to test

The backtest should test both unfiltered and filtered versions:

- **OI concentration filter:** Only trade expiries where >30% of total OI (calls + puts combined) is within ±3% of current spot. Rationale: high ATM OI concentration means more hedging pressure near current price.
- **Monthly-only filter:** Only trade last-Friday-of-month expiries. Rationale: larger OI → larger hedging flows → stronger effect.
- **OI size filter:** Only trade expiries where total OI notional exceeds a threshold (e.g., $500M for BTC, $200M for ETH). Rationale: small expiries have insufficient hedging volume to move price.

Run each filter combination separately and compare. Do not data-mine — pre-register filter logic before seeing results.

### Simulation mechanics

For each qualifying event:

1. **Entry:** Wednesday 08:00 UTC at Binance hourly close price (no fill improvement assumed)
2. **Exit:** Friday 08:00 UTC at Binance hourly close price
3. **Fees:** 0.045% taker entry + 0.045% taker exit = 0.09% round-trip
4. **Funding:** Sum of actual 8h funding payments over the 48h hold (6 funding periods). Use sign correctly — longs pay when funding is positive, shorts pay when funding is negative.
5. **Stop loss:** If price moves 4% against entry at any hourly close, exit at that close. (Test with and without stop loss.)
6. **Slippage assumption:** 0.05% additional cost on each leg (conservative for BTC/ETH at 1–2x leverage on Hyperliquid)

### Metrics to compute

| Metric | Target to advance | Kill signal |
|---|---|---|
| Win rate | >50% | <40% |
| Avg return per trade (gross) | >1.5% | <0.5% |
| Avg return per trade (net of fees + funding) | >0.8% | <0% |
| Sharpe ratio (trade-by-trade) | >0.8 | <0.3 |
| Max drawdown (consecutive losses) | <4 consecutive losers | 5+ consecutive losses |
| % of expiries with dislocation >5% | Record this — sets expected trade frequency | — |
| Monthly vs. weekly split | Compare separately | If monthly strong but weekly weak, restrict to monthly |

### Baseline comparison

Before concluding the strategy has edge, compare against:

1. **Random 48h BTC/ETH long:** Pick a random Wednesday 08:00 UTC entry each week, long for 48h, same fees. This is the null — does the max pain signal add anything over just being long for 48h?
2. **Always-long rule:** Long every expiry week regardless of max pain dislocation. Does the 5% filter help or hurt?
3. **Always-short rule:** Same logic.

The strategy only advances if the max pain signal beats the random baseline by a meaningful margin — not just because BTC trends up.

---

## Entry Rules


**Data pipeline (runs Wednesday 04:00 UTC to allow time for computation):**
1. Pull Deribit OI for current week's Friday expiry
2. Calculate max pain strike
3. Pull BTC/ETH spot from Binance
4. Compute dislocation: (spot − max pain) / max pain
5. Apply filters (OI concentration, total OI threshold)
6. If qualifying: send order to Hyperliquid at 08:00 UTC Wednesday

**Entry:**
- Time: Wednesday 08:00 UTC
- Instrument: BTC-USD or ETH-USD perpetual on Hyperliquid
- Direction: Long if spot < max pain − 5%; Short if spot > max pain + 5%
- Order type: Limit order at mid (or market if unfilled within 5 minutes)
- Leverage: 1x initially (expand to 2x after forward validation)

## Exit Rules

**Exit:**
- Primary: Friday 08:00 UTC, market order
- Stop loss: 4% adverse move from entry price (checked hourly)
- No take profit — full 48h convergence window

**Position held for:** 48 hours exactly (6 funding periods)

---

## Position Sizing

**Paper trading phase:**
- $300 notional per trade (BTC and ETH separately, not both simultaneously)
- Fixed notional, not fixed leverage — keeps risk constant regardless of price level

**Live phase (post-validation):**
- Start at $500 notional per trade
- Scale to 1% of total deployed capital per trade
- BTC and ETH may be traded simultaneously if both show qualifying dislocations in the same week, but treat as separate positions

---

## Go-Live Criteria

All four conditions must be met before deploying real capital:

1. Backtest net return per trade (after fees and funding) is positive across the full sample, and positive in the filtered subsample
2. Backtest Sharpe ratio (trade-by-trade) > 0.8
3. Backtest results beat the random-48h-long baseline by >1% per trade on average
4. At least 5 paper trades closed with net P&L positive and no single trade losing >5% of notional

Paper trading uses exact Hyperliquid API calls (same code as live) with real prices and real funding — no simulation shortcuts.

---

## Kill Criteria

### Kill during backtesting

- Net return per trade after fees + funding ≤ 0 across full sample → strategy does not work, do not paper trade
- Backtest result does not beat random-48h-long baseline → signal adds no value, kill
- Win rate < 45% with Sharpe < 0.5 → too noisy to be useful

### Kill during paper trading

- After 10 paper trades: net P&L negative → kill or redesign
- After 10 paper trades: average trade return < 0.5% net → edge too thin for capital deployment
- 5 consecutive losing paper trades → pause and re-examine
- Any single paper trade loses > 6% of notional (stop loss failed or gap through stop) → review execution logic before continuing

### Kill during live trading

- 3-month rolling net P&L negative after all costs → kill
- Max pain dislocation >5% becomes rare (<1 qualifying trade per month) → market structure changed, reassess
- Funding rates consistently exceed 0.1% per 8h period → cost structure makes 48h holds uneconomical

---

## Risks

**Effect may not exist in crypto.** The max pain pin is documented in equities but academic evidence in crypto is sparse. Crypto market makers may hedge differently, run tighter delta-neutral books, or not hedge continuously. If the mechanism doesn't operate, there is no edge. The backtest will reveal this — but the backtest itself only has ~100 weekly events, which is a moderate sample.

**Trend overrides pin in 48 hours.** BTC can move 5–10% in 48 hours on news or macro. A strong directional trend will completely overwhelm any gravitational effect from options hedging. This is the primary risk — the strategy bets on mean reversion and can be steamrolled by momentum. The 4% stop loss is the primary mitigation.

**Crowding.** Max pain is publicly known and discussed in crypto communities. If enough traders fade the same dislocation, the signal is arbitraged away before it generates returns. Unlike token unlock schedules (which require tracking obscure data), max pain is one Google search away. Monitor whether the edge decays as the strategy runs.

**Funding rates are a real cost.** Holding a perp for 48h means 6 funding payments. In high-funding environments (BTC funding >0.05% per 8h), the funding cost alone erodes ~0.3% from a long position. In regimes where funding consistently runs against the strategy's direction, the edge may not survive costs.

**OI data quality.** Pulling OI at a single point 48 hours before expiry is noisy — OI shifts as traders roll or close positions. The max pain calculation at T-48h may not represent the actual hedging pressure in the final hours. A more robust version would update the max pain estimate at T-24h and T-4h, but this adds complexity.

**Execution risk at expiry.** Exiting at exactly Friday 08:00 UTC requires reliable automation. A missed exit turns a 48h trade into an open-ended position — particularly dangerous if a stop loss was not triggered and the position is offside.

**Liquidity on Hyperliquid.** BTC and ETH are among the most liquid instruments on Hyperliquid, so slippage at $300–$500 notional should be minimal. This risk increases only if position sizes scale significantly.

---

## Data Sources

| Data | Source | Endpoint / Method |
|---|---|---|
| Deribit options OI by strike | Deribit REST API | `GET /public/get_book_summary_by_currency?currency=BTC&kind=option` |
| Historical Deribit OI (for backtest) | Deribit API or CoinGlass | Deribit allows historical queries; CoinGlass has archived OI data |
| Expiry calendar | Computed from rule (every Friday 08:00 UTC) + verify via Deribit | `GET /public/get_instruments?currency=BTC&kind=option&expired=false` |
| BTC/ETH hourly prices | Binance REST API | `GET /api/v3/klines?symbol=BTCUSDT&interval=1h` |
| Funding rates (live) | Hyperliquid API | `POST /info` with `{"type": "fundingHistory", "coin": "BTC"}` |
| Funding rates (historical backtest) | Hyperliquid API or Coinalyze | Historical 8h funding by coin |

---

## Implementation notes

**Max pain calculation is simple arithmetic.** No external library needed. For each expiry, iterate over all listed strikes, compute total intrinsic value that options buyers would receive at each candidate price, find the minimum. Can be written in <50 lines of Python.

**Backtest script outline:**
1. Load expiry calendar (all Fridays 2023–present)
2. For each expiry: load OI snapshot at T-48h (Wednesday 08:00 UTC)
3. Calculate max pain
4. Load BTC/ETH hourly price at T-48h
5. Classify trade or no-trade
6. Load hourly prices from T-48h to T, apply stop loss logic
7. Record exit price (at T or at stop)
8. Apply fees and funding
9. Aggregate results, compute metrics, compare to baselines

**Suggested implementation priority:** BTC first (higher OI, more liquid, likely stronger effect), ETH second after BTC backtest is complete.

---

## Relationship to Strategy 001

This strategy is entirely independent of token unlock shorts (Strategy 001). It operates on different instruments (BTC/ETH vs. altcoins), different timeframes (48h vs. 24 days), and a different mechanism (options hedging vs. token recipient selling). The two strategies can run simultaneously without correlation concerns. If both are live, monitor portfolio-level funding exposure — both strategies may sometimes be on the same side of BTC/ETH perp markets.
