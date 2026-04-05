---
title: "Hyperliquid Insurance Fund Liquidation Overshoot"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 5
composite: 900
categories:
  - liquidation
  - funding-rates
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's liquidation engine closes a large position, it market-sells (or market-buys) into the live order book without regard for price optimization. The engine's sole objective is position closure before bankruptcy price is breached. This mechanical indifference to price creates a temporary, measurable deviation between the mark price and the index price. Because the mark price is anchored to the index via a contractually enforced funding mechanism, the deviation must close — either through price reversion or through funding rate pressure. The trade is: buy the mechanical overshoot immediately after the liquidation engine finishes, exit when the mark-index gap closes.

The edge is **not** that liquidations cause reversals in general. The edge is that the mark-index spread is a contractually bounded quantity on Hyperliquid, and a large liquidation-induced deviation from that bound creates a measurable, time-limited arbitrage against a known gravitational force.

---

## Structural Mechanism

### Why the overshoot exists

1. **Liquidation engine priority:** Hyperliquid's engine liquidates positions at or before the bankruptcy price. It does not use TWAP, iceberg orders, or any price-minimizing execution. It hits the book with market orders until the position is flat.
2. **Order book thinness:** During volatile periods — exactly when large liquidations occur — the order book is thinner than normal. Market participants pull liquidity during uncertainty. A $500K+ market order into a thin book moves price disproportionately.
3. **No price optimization mandate:** The engine is not penalized for moving price; it is penalized only for losses to the insurance fund. Aggressive execution is the rational behavior of the engine, not a bug.

### Why the overshoot reverts

1. **Mark-index anchoring:** Hyperliquid's mark price is a function of the index price (a weighted median of external CEX prices). The index does not move because HL's liquidation engine hit its own book. The spread between mark and index is therefore a temporary artifact.
2. **Funding rate pressure:** If mark trades below index, the funding rate turns negative (longs receive funding). This creates a mechanical incentive for market participants to go long and close the gap. The funding mechanism is contractually enforced by the protocol.
3. **Arbitrageurs:** Basis traders and market makers observe the mark-index spread and trade against it. Their activity is the proximate cause of reversion; the funding mechanism is the structural guarantee that makes their trade profitable.

### Why this is not already fully arbitraged away

- The reversion window is short (minutes), requiring live monitoring infrastructure that most participants do not maintain.
- Liquidation events are clustered and unpredictable in timing, making it operationally inconvenient to staff.
- The edge per event is small in percentage terms (~0.3–0.8%), making it unattractive to large funds but viable for a bot with low overhead.
- The signal (liquidation feed + mark-index spread) is public but requires real-time API integration to act on.

---

## Market Universe

**Primary markets:** BTC-PERP, ETH-PERP, SOL-PERP on Hyperliquid.
**Secondary markets (if data supports):** Any HL perpetual with >$10M average daily volume.
**Excluded:** Low-liquidity HL markets where the liquidation itself may be the dominant price-setting event and reversion is not guaranteed.

---

## Entry Rules

All conditions must be satisfied simultaneously before entry is triggered.

| # | Condition | Value | Rationale |
|---|-----------|-------|-----------|
| 1 | Liquidation notional | ≥ $500K | Below this, book impact is insufficient to create measurable overshoot |
| 2 | Mark-index deviation | Mark < Index by ≥ 0.3% | Minimum spread to cover fees and slippage with margin |
| 3 | Liquidation status | Engine has finished (no active liquidation in feed for ≥ 5 seconds) | Do not enter during active liquidation; cascade risk |
| 4 | Time since liquidation | ≤ 30 seconds | After 30 seconds, arbitrageurs have likely already closed most of the gap |
| 5 | Cascade check | No additional liquidations in same asset in prior 10 seconds | Cascade in progress; structural reversion is not reliable |
| 6 | Funding rate direction | Current funding rate is not already deeply negative (< -0.05% per 8h) | Pre-existing negative funding suggests structural bearishness, not mechanical overshoot |

**Entry instrument:** Long perpetual on Hyperliquid in the liquidated asset.
**Entry execution:** Market order. Limit orders risk missing the reversion entirely. Accept the spread cost.

---

## Exit Rules

Exit on the **first** of the following conditions:

| Priority | Condition | Action |
|----------|-----------|--------|
| 1 | Mark price returns to within 0.05% of index price | Close long at market — target achieved |
| 2 | 15-minute timeout from entry | Close long at market — reversion did not occur in expected window |
| 3 | Mark price falls >0.5% below entry price | Close long at market — cascade or new information, abort |
| 4 | New liquidation event >$500K in same asset | Close long at market — cascade risk, re-evaluate |

**Exit execution:** Market order on all exits. Speed of exit matters more than price precision.

---

## Position Sizing

**Base position size:** 0.5% of total portfolio per trade.
**Maximum concurrent positions:** 2 (across different assets only — never two positions in the same asset).
**Maximum portfolio exposure at any time:** 1% of total portfolio.
**Rationale:** This is a high-frequency, small-edge strategy. Individual trade P&L is small. Sizing must be conservative because cascade risk (the primary failure mode) can produce rapid, large losses. The strategy's value is in aggregate edge across many events, not in individual trade size.

**Leverage:** Maximum 3x. Higher leverage amplifies cascade losses beyond acceptable risk parameters. The reversion target is 0.3–0.8%, which does not justify high leverage.

**Do not size up during high-volatility periods.** Liquidation frequency increases during volatility, but so does cascade risk. Maintain flat sizing regardless of opportunity frequency.

---

## Backtest Methodology

### Data requirements

| Dataset | Source | Notes |
|---------|--------|-------|
| Historical liquidation events | Hyperliquid public API (`/info` endpoint, `liquidations` field) | Available in real-time; historical reconstruction requires log archiving from a specific start date |
| Mark price (tick-level) | Hyperliquid public WebSocket feed | Must be archived independently; not available retroactively beyond recent history |
| Index price (tick-level) | Hyperliquid public API (`/info`, `oraclePrice`) | Same archiving requirement |
| Order book snapshots | Hyperliquid public WebSocket | Required to estimate realistic entry/exit slippage |
| Funding rate history | Hyperliquid public API | Available historically |

**Critical data gap:** Hyperliquid does not provide a public historical archive of tick-level mark prices and liquidation events beyond a rolling window. **Begin archiving immediately upon strategy approval.** Minimum 90 days of data required before backtest is meaningful. This is the primary reason the strategy is at pre-backtest stage.

### Backtest procedure

1. **Reconstruct liquidation events** from archived feed. Record: timestamp, asset, notional size, direction (long or short liquidated).
2. **Compute mark-index spread** at the moment each liquidation closes (defined as 5 seconds after last liquidation event in a cluster).
3. **Apply entry filter:** Keep only events where spread ≥ 0.3% and all other entry conditions are met.
4. **Simulate entry:** Use mark price at T+0 (liquidation close) plus estimated slippage of 0.05% (conservative for BTC/ETH, 0.10% for smaller assets).
5. **Simulate exit:** Scan forward tick-by-tick for first exit condition. Apply 0.05% slippage on exit.
6. **Compute per-trade P&L:** Net of entry slippage, exit slippage, and Hyperliquid taker fees (currently 0.035% per side = 0.07% round trip).
7. **Aggregate metrics:** Win rate, average P&L per trade, Sharpe ratio, maximum drawdown, maximum consecutive losses.

### Minimum viable backtest results to proceed

| Metric | Threshold |
|--------|-----------|
| Win rate | ≥ 60% |
| Average net P&L per trade | ≥ 0.10% (after all costs) |
| Maximum drawdown | ≤ 3% of portfolio |
| Sample size | ≥ 200 qualifying events |
| Sharpe ratio (annualized) | ≥ 1.0 |

### Slippage sensitivity test

Run backtest at 3x estimated slippage. If strategy remains profitable, execution risk is manageable. If strategy breaks even or loses at 3x slippage, the edge is too thin and the strategy is killed.

---

## Go-Live Criteria

All of the following must be satisfied before live capital is deployed:

1. **Backtest passes** all minimum viable thresholds above on ≥ 90 days of archived data.
2. **Paper trading passes:** 30-day paper trading period with ≥ 20 qualifying events, achieving win rate ≥ 55% and positive net P&L.
3. **Infrastructure validated:** Bot correctly identifies liquidation events, computes mark-index spread, and executes market orders within 15 seconds of liquidation close in live conditions. Latency must be measured and documented.
4. **Cascade detection validated:** Bot correctly identifies cascade conditions and aborts in at least 3 observed cascade events during paper trading.
5. **Fee structure confirmed:** Confirm current Hyperliquid taker fee schedule. If fees increase above 0.05% per side, re-run break-even analysis before go-live.

---

## Kill Criteria

The strategy is suspended immediately if any of the following occur:

| Trigger | Action |
|---------|--------|
| 10 consecutive losing trades | Suspend, audit entry/exit logic, do not resume without review |
| Portfolio drawdown ≥ 2% attributable to this strategy | Suspend, full review required |
| Win rate falls below 45% over any rolling 30-trade window | Suspend, re-evaluate structural assumptions |
| Hyperliquid changes liquidation engine behavior (documented in protocol updates) | Suspend immediately, re-validate mechanism before resuming |
| Mark-index spread mechanism changes (e.g., new oracle methodology) | Suspend immediately, re-validate mechanism before resuming |
| Average time-to-reversion exceeds 10 minutes over 20-trade rolling window | Mechanism may be weakening; suspend and investigate |

---

## Risks

### Primary risks

**1. Cascade liquidations (highest severity)**
A large liquidation triggers further liquidations as price falls through additional bankruptcy prices. The structural reversion assumption breaks down entirely during cascades. The stop-loss rule (exit if mark falls >0.5% from entry) is the primary defense, but in a fast cascade, slippage on the stop may be severe. *Mitigation: Never size above 0.5% of portfolio per trade. Accept that cascade events will produce the strategy's worst losses.*

**2. Execution latency**
The reversion window is measured in minutes. A 30-second delay is acceptable; a 5-minute delay is not. If the bot's infrastructure introduces latency beyond 30 seconds from liquidation close to order submission, the edge is likely gone before entry. *Mitigation: Measure and document latency continuously. Kill the strategy if median latency exceeds 20 seconds.*

**3. Thin edge after costs**
The gross edge (0.3–0.8% spread) is partially consumed by taker fees (0.07% round trip) and slippage (0.10–0.20% round trip). Net edge per trade may be 0.05–0.50%. A fee increase or wider spreads during volatile periods can eliminate the edge entirely. *Mitigation: Run break-even analysis at current fee levels before each quarter. Monitor average realized slippage.*

**4. Protocol risk**
Hyperliquid is a relatively new protocol. Smart contract bugs, oracle failures, or protocol upgrades could affect the mark-index mechanism or liquidation engine behavior without warning. *Mitigation: Never allocate more than 5% of total portfolio to strategies that depend on Hyperliquid-specific mechanics.*

**5. Adverse selection**
Large liquidations sometimes occur because informed participants are exiting ahead of bad news. In these cases, the index price itself may move down after the liquidation, making the mark-index spread a false signal. *Mitigation: The 15-minute timeout and 0.5% stop-loss bound the loss in this scenario. Monitor whether losses cluster around specific news events.*

### Secondary risks

- **API reliability:** Hyperliquid's public API may experience downtime or rate limiting. Bot must handle API failures gracefully and not enter positions on stale data.
- **Regulatory risk:** Perpetual futures trading may face regulatory restrictions in certain jurisdictions. Confirm legal status before deployment.
- **Liquidity risk:** During extreme market stress, even BTC and ETH perps on HL may have insufficient liquidity to exit at acceptable prices. The 3x leverage cap partially mitigates this.

---

## Data Sources

| Data | Source | Access method | Latency |
|------|--------|---------------|---------|
| Live liquidation feed | Hyperliquid WebSocket API | `wss://api.hyperliquid.xyz/ws`, subscribe to `liquidations` | Real-time |
| Mark price | Hyperliquid WebSocket API | Subscribe to `markPrices` channel | Real-time |
| Index (oracle) price | Hyperliquid REST API | `POST /info`, type `"oraclePrices"` | ~1 second polling |
| Order book | Hyperliquid WebSocket API | Subscribe to `l2Book` for target asset | Real-time |
| Funding rate | Hyperliquid REST API | `POST /info`, type `"fundingHistory"` | Historical available |
| Trade execution | Hyperliquid REST API | `POST /exchange` with signed order | ~100–500ms round trip |

**Archive requirement:** Begin logging mark prices, index prices, and liquidation events to a local database immediately. Use a persistent process (not a cron job) to avoid gaps. Timestamp all records with server-received time AND local time to measure latency.

---

## Open Questions for Backtest Phase

1. What is the empirical distribution of time-to-reversion for qualifying events? Is 15 minutes the right timeout, or should it be 5 minutes or 30 minutes?
2. Does the minimum liquidation size threshold of $500K correctly filter for events with measurable book impact? Should this be asset-specific (e.g., $200K for smaller assets, $1M for BTC)?
3. Is the 0.3% minimum spread threshold correctly calibrated, or does it exclude too many profitable events or include too many marginal ones?
4. Do cascade events have identifiable precursors (e.g., rapid sequence of smaller liquidations before the large one) that could improve the cascade filter?
5. Does the edge vary systematically by time of day (e.g., weaker during Asian hours when arbitrageurs are more active on HL)?
6. Is there a correlation between funding rate level at time of liquidation and subsequent reversion speed?

---

## Next Steps

| Step | Owner | Deadline |
|------|-------|----------|
| Deploy data archiving bot for mark price, index price, liquidation feed | Engineering | Immediate |
| Confirm Hyperliquid API rate limits and WebSocket stability | Engineering | Week 1 |
| Define cascade detection algorithm precisely | Research | Week 2 |
| Build backtest engine against archived data | Engineering | Day 91 (after 90-day archive) |
| Run backtest and slippage sensitivity analysis | Research | Day 95 |
| Decision: proceed to paper trading or kill | Research + Risk | Day 96 |
