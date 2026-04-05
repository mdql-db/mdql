---
title: "Polymarket Binary Resolution Basis Arb — Perp Token Mispricing at Settlement"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 5
frequency: 2
composite: 240
categories:
  - basis-trade
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

In the final 2–4 hours before a Polymarket binary market resolves on a crypto price outcome (e.g., "Will BTC close above $100,000 on Dec 31?"), the YES/NO share price encodes informed flow that has not yet been fully reflected in the BTC/ETH perpetual futures price on Hyperliquid or Binance.

**Specific causal chain:**

1. Polymarket YES shares converge mathematically to $1.00 or $0.00 at resolution — this is a hard contractual guarantee enforced by the UMA oracle or Polymarket's resolution rules.
2. Sophisticated participants who have formed a strong view on the outcome (e.g., they can see the BTC price trajectory with 2 hours left) buy YES/NO shares aggressively, pushing the probability toward 95%+.
3. The spot/perp market has not yet moved proportionally — either because the move is gradual and not yet "obvious" to perp traders, or because perp liquidity is deeper and slower to reprice on soft information.
4. The gap between implied probability (Polymarket) and implied probability (spot price relative to strike) represents a tradeable lag.
5. As resolution approaches, the spot price must converge to either above or below the strike — the binary outcome forces a directional resolution in the underlying asset too.

**The edge is not that Polymarket is always right. The edge is that when Polymarket is at 95%+ with 2 hours left, the conditional distribution of outcomes is extremely skewed, and the perp market has not yet priced that skew.**

---

## Structural Mechanism

### Why this *might* happen (causal, not historical)

**Mechanism 1 — Information aggregation speed differential:**
Polymarket is a dedicated information market. Participants are incentivised to price probability correctly because they profit from accuracy. Perp markets are dominated by directional traders, momentum players, and hedgers — not probability estimators. When a binary event is 2 hours from resolution, a Polymarket participant with a strong view will express it in Polymarket (small, targeted market) before it propagates to the much larger perp market.

**Mechanism 2 — Liquidity asymmetry creates lag:**
Polymarket markets for crypto price events have $1M–$10M in liquidity. BTC perp markets have $500M+ daily volume. A $100K informed bet moves Polymarket YES shares by several percentage points. The same $100K in BTC perps moves the price by ~0.02%. The signal is visible in Polymarket before it's large enough to move perps.

**Mechanism 3 — Forced convergence of the underlying:**
Unlike a prediction market on a political event, a "BTC above $X by date Y" market has a direct mechanical link to the underlying. If YES is at 97% with 1 hour left, BTC is almost certainly already near or above $X. The spot price *will* resolve above or below $X — this is not a soft signal, it's a near-certain directional statement about where BTC will be in 60 minutes.

### Why this is NOT a guaranteed edge (honest assessment)

- Polymarket does not *force* the perp price to move. The convergence is probabilistic, not contractual.
- The 95% Polymarket signal could be wrong (black swan in final hour).
- Arb bots may already be connecting these markets, compressing the lag.
- Resolution disputes (UMA oracle challenges) can delay settlement and leave the position open past the intended exit.

**Score justification: 5/10** — The mechanism is real and causal, but the outcome is probabilistic. This is a signal strategy, not a convergence arb. Treat it as such.

---

## Entry Rules


### Universe
Only Polymarket markets of the form:
- "Will [BTC/ETH] [exceed/close above/close below] $[X] by [date/time]?"
- Resolution date must be known and fixed (not "by end of year" with ambiguous close time)
- Market must have >$500K total liquidity (YES + NO sides combined) to filter noise
- Underlying asset must have a liquid perp on Hyperliquid (BTC, ETH, SOL)

### Entry Conditions (ALL must be true simultaneously)

| Condition | Threshold | Rationale |
|---|---|---|
| YES share price | ≥ 0.92 OR ≤ 0.08 | High-conviction signal; below 92% the signal is too noisy |
| Time to resolution | 1–4 hours remaining | Too early = too much can change; too late = perp already moved |
| Spot/perp move in implied direction (last 60 min) | < 1.5% | Confirms lag exists; if perp already moved 3%, the arb is closed |
| Polymarket YES share price momentum | Moving toward 1.0 (or 0.0) — not stalling | Stalling at 92% may indicate uncertainty, not informed flow |
| Polymarket 24h volume on this market | > $50K | Filters thin markets where 92% may be a single large bet |

**Entry action:**
- YES ≥ 0.92 → Long BTC/ETH perp on Hyperliquid
- NO ≥ 0.92 (YES ≤ 0.08) → Short BTC/ETH perp on Hyperliquid
- Use market order (speed matters; this is a short-duration trade)
- Enter at 1x leverage only during backtesting phase

## Exit Rules

### Exit Conditions (first trigger wins)

| Exit trigger | Action |
|---|---|
| Spot/perp moves ≥ 2.5% in entry direction | Close position — gap closed, take profit |
| Hard stop: adverse move ≥ 1.5% on perp | Close position — signal was wrong |
| Time to resolution < 15 minutes | Close position regardless of P&L — resolution risk too high |
| Polymarket YES share reverses below 85% (from ≥92%) | Close position — signal invalidated |
| Resolution confirmed | Close position immediately post-resolution |

### Do NOT hold through resolution
The UMA oracle resolution process can take hours to days. Do not hold the perp position waiting for Polymarket to settle. Exit the perp trade before resolution or when the gap closes.

---

## Position Sizing

**Base position size:** 1% of portfolio per trade during backtesting and paper trading.

**Rationale:** This is an unproven signal strategy. 1% limits drawdown to manageable levels while generating enough trades to produce statistically meaningful results.

**Scaling rules (post-validation only):**
- If backtest shows Sharpe > 1.5 and win rate > 55% on ≥ 50 trades: scale to 2% per trade
- Never exceed 3% per trade regardless of signal strength
- Maximum concurrent positions: 2 (BTC and ETH simultaneously if both trigger)

**Leverage:** 1x during paper trading. Maximum 2x post-validation. This is not a high-leverage strategy — the edge is in signal quality, not leverage amplification.

---

## Backtest Methodology

### Data Sources

**Polymarket historical data:**
- Polymarket API: `https://gamma-api.polymarket.com/markets` — returns market metadata, resolution dates, outcomes
- Polymarket CLOB historical trades: `https://clob.polymarket.com/trades` — tick-level trade data
- The Graph (Polymarket subgraph): `https://thegraph.com/explorer/subgraphs/81Dm16JjuFSrqz813HysXoUPvzTwE7fsfPk2RTf66nyC` — on-chain resolution data
- Polymarket data dumps: Available via `https://data.polymarket.com` (bulk CSV exports)

**Perp price data:**
- Hyperliquid: `https://api.hyperliquid.xyz/info` — historical candles endpoint, 1-minute OHLCV
- Binance: `https://api.binance.com/api/v3/klines` — 1-minute BTC/ETH perp candles (BTCUSDT_PERP)
- Use Binance as primary for backtesting (longer history); Hyperliquid for live execution

### Backtest Period
- Target: All Polymarket crypto price markets that resolved between **January 2022 and December 2024**
- Expected universe: ~50–150 qualifying markets (BTC/ETH price threshold markets with >$500K liquidity)
- Note: Polymarket volume was thin pre-2023; weight 2023–2024 data more heavily in analysis

### Event Identification
1. Pull all resolved Polymarket markets with "BTC", "ETH", "SOL" in title and binary YES/NO structure
2. Filter: resolution_date known, total_volume > $500K, underlying has Hyperliquid/Binance perp
3. For each qualifying market, extract the YES share price time series in the 4 hours before resolution
4. Identify entry signals: first timestamp where YES ≥ 0.92 (or ≤ 0.08) with ≥ 1 hour remaining
5. Cross-reference with perp price: check if perp moved < 1.5% in the prior 60 minutes

### Metrics to Compute

| Metric | Target | Kill threshold |
|---|---|---|
| Win rate | > 55% | < 45% |
| Average win / average loss ratio | > 1.5 | < 1.0 |
| Sharpe ratio (annualised) | > 1.2 | < 0.5 |
| Max drawdown | < 15% | > 25% |
| Number of qualifying trades | ≥ 30 | < 15 (insufficient data) |
| Average holding time | 30–120 minutes | — |
| Signal decay test | Win rate in 2022 vs 2024 | If win rate drops >15pp, signal is decaying |

### Baseline Comparison
Compare against:
1. **Random directional trade** at same time windows (same entry time, random long/short) — tests whether the Polymarket signal adds value vs. noise
2. **Momentum baseline** — enter in direction of last 60-minute perp move at same time windows — tests whether Polymarket signal beats simple momentum
3. **Always-long BTC baseline** — tests whether this is just a bull market artifact

### Backtest Implementation Notes
- Use **1-minute candles** for perp price — do not use hourly data, it will miss intra-hour moves
- Simulate **market order slippage**: assume 0.05% slippage on entry and exit (Hyperliquid taker fee is 0.035%)
- Do NOT look ahead: entry signal must be computed using only data available at that timestamp
- Flag any market where resolution was disputed or delayed — exclude from primary results, analyse separately

---

## Go-Live Criteria

The following must ALL be satisfied before moving to paper trading:

1. **≥ 30 qualifying historical trades** identified and analysed
2. **Win rate ≥ 55%** on the full sample
3. **Win/loss ratio ≥ 1.3** (average winner at least 1.3x average loser)
4. **Sharpe ratio ≥ 1.0** (annualised, using 1% position sizing)
5. **Signal beats random baseline** by ≥ 10 percentage points in win rate
6. **No single trade accounts for > 30% of total P&L** (checks for outlier dependency)
7. **Signal is present in 2024 data** (not just 2022–2023) — confirms it hasn't fully decayed

If all 6 criteria are met: move to **paper trading for 60 days** with real-time signal monitoring before any live capital deployment.

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

### During backtesting
- Fewer than 15 qualifying trades found in the historical data (universe too small to be viable)
- Win rate < 45% on historical sample
- Signal shows no statistically significant difference from random baseline (p > 0.10)
- Win rate in 2024 is more than 15 percentage points below 2022–2023 win rate (signal decayed)

### During paper trading
- 10 consecutive paper trade losses
- Paper trade win rate < 45% after ≥ 20 trades
- Average holding time consistently exceeds 4 hours (means exit conditions aren't triggering cleanly)
- Polymarket introduces changes to resolution mechanics or oracle that alter the convergence guarantee

### Ongoing monitoring
- Polymarket restricts API access or goes offline (execution becomes impossible)
- Regulatory action against Polymarket that creates resolution uncertainty
- A competing arb bot is demonstrably front-running entries (detectable if Polymarket YES share moves and perp moves simultaneously within seconds — the lag window has closed)

---

## Risks

### Risk 1: Signal is noise, not informed flow (HIGH probability)
A Polymarket YES share at 92% with 2 hours left may simply reflect that BTC is already near the strike price — not that informed traders know something the perp market doesn't. The "lag" may be an illusion created by the different scaling of the two markets. **Mitigation:** The baseline comparison in the backtest directly tests this. If the signal doesn't beat random, kill it.

### Risk 2: Resolution disputes (MEDIUM probability, HIGH impact)
UMA oracle challenges can delay Polymarket resolution by days. If the perp position is held through this period, it's exposed to unrelated market moves. **Mitigation:** Hard rule — exit perp position at T-15 minutes regardless of Polymarket settlement status.

### Risk 3: Thin Polymarket liquidity creates false signals (MEDIUM probability)
A single $200K bet can move YES shares from 80% to 95% on a thin market. This is one whale, not informed consensus. **Mitigation:** Require >$50K in 24h volume AND >$500K total market liquidity. Also check whether the YES share move was a single large trade or distributed flow.

### Risk 4: Arb bots have already closed the gap (UNKNOWN probability)
As of 2024, several on-chain arb bots monitor Polymarket and connected spot markets. The lag window may already be sub-minute, making this strategy non-executable without HFT infrastructure. **Mitigation:** The signal decay test in the backtest (2022 vs 2024 win rates) will reveal this. If 2024 win rate is materially lower, the strategy is dead.

### Risk 5: Regulatory risk on Polymarket (LOW probability, HIGH impact)
Polymarket has faced regulatory scrutiny (CFTC). A shutdown or US-person restriction would eliminate the data source and execution venue. **Mitigation:** Monitor regulatory developments. This is a tail risk, not a trading risk.

### Risk 6: Correlation with broader market moves (MEDIUM probability)
If BTC drops 5% in the 2 hours before resolution, a YES share at 92% will collapse regardless of the "informed flow" thesis. The strategy is exposed to large macro moves during the holding period. **Mitigation:** 1.5% hard stop on the perp position limits this exposure. Do not widen the stop.

---

## Data Sources

| Data | Source | URL / Endpoint |
|---|---|---|
| Polymarket market metadata | Polymarket Gamma API | `https://gamma-api.polymarket.com/markets?closed=true&limit=500` |
| Polymarket CLOB trade history | Polymarket CLOB API | `https://clob.polymarket.com/trades?market={condition_id}` |
| Polymarket on-chain resolution | The Graph subgraph | `https://thegraph.com/explorer/subgraphs/81Dm16JjuFSrqz813HysXoUPvzTwE7fsfPk2RTf66nyC` |
| Polymarket bulk data dumps | Polymarket Data | `https://data.polymarket.com` |
| BTC/ETH perp 1-min candles (backtest) | Binance API | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1m` |
| BTC/ETH perp 1-min candles (live) | Hyperliquid API | `https://api.hyperliquid.xyz/info` (POST, type: `candleSnapshot`) |
| Polymarket YES share price series | Polymarket CLOB | `https://clob.polymarket.com/prices-history?market={condition_id}&interval=1m` |
| UMA oracle resolution records | UMA Protocol | `https://oracle.uma.xyz` — dispute history and resolution timestamps |

### Data Pipeline Notes
- Polymarket condition IDs must be mapped to market titles manually or via the Gamma API `question` field — filter for "BTC", "ETH", "SOL", "price", "above", "below"
- Align Polymarket timestamps (UTC) with Binance candle timestamps (UTC) — both are UTC, no conversion needed
- Polymarket CLOB API rate limits: 10 requests/second — implement backoff
- The Graph subgraph may have indexing delays — cross-reference with Polymarket API for resolution confirmation

---

*This specification is sufficient to build a backtest. The hypothesis is plausible but unproven. Do not allocate live capital until go-live criteria are met.*
