---
title: "crvUSD Soft Liquidation Band Entry — Forced AMM Flow Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 7
composite: 1260
categories:
  - defi-protocol
  - liquidation
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When ETH price approaches a dense cluster of crvUSD LLAMMA soft-liquidation bands, the LLAMMA smart contract **must** execute a calculable volume of collateral conversion — ETH→crvUSD on the way down, crvUSD→ETH on the way up. This flow is not discretionary: it is a smart contract invariant triggered on every oracle update. By reading open position band data on-chain, we can compute the cumulative forced conversion volume per price tick *before* it executes. Positioning ahead of this flow in ETH perpetual futures should capture a predictable, mechanically-sourced price impulse.

This is not "ETH tends to sell off near these levels." It is: "X million dollars of ETH **will be sold** by a smart contract when price crosses $Y, and we can calculate X and Y right now."

---

## Structural Mechanism

### How LLAMMA Works

crvUSD uses a novel AMM called LLAMMA (Lending-Liquidating AMM Algorithm) as its liquidation engine. Each borrower's collateral is deposited into a price band defined by two prices: `p_cd` (collateral-to-debt conversion start) and `p_cu` (collateral-to-debt conversion end).

- **Price falling through band:** LLAMMA continuously sells ETH and buys crvUSD, proportional to how far price has moved through the band. At band bottom, 100% of collateral is crvUSD.
- **Price rising through band:** LLAMMA continuously buys ETH and sells crvUSD. At band top, 100% of collateral is ETH.
- **Trigger:** Every oracle price update (approximately every few minutes on Ethereum mainnet). The AMM recalculates the required collateral composition and executes swaps internally.

### Why This Creates Tradeable Flow

1. **Band positions are fully public.** Every borrower's band range, collateral size, and current composition is readable from the LLAMMA contract (`get_band_assets`, `user_state`, `bands_x`, `bands_y`).
2. **Aggregate flow is computable.** Sum all positions with bands overlapping a given $50 price tick → get total ETH that *must* be sold (or bought) as price crosses that tick.
3. **Flow is not yet priced into perps.** The LLAMMA operates on Ethereum mainnet; its swaps affect crvUSD/ETH spot liquidity on Curve pools, not ETH perpetual futures directly. The perp market must reprice via arbitrage, creating a lag window.
4. **The mechanism is causal, not correlational.** The contract code is the causal agent. There is no "tends to" — only "will, conditional on price reaching the band."

### Flow Diagram

```
ETH spot price falls
        ↓
LLAMMA oracle update triggers
        ↓
Contract sells ETH → buys crvUSD (internal swap)
        ↓
Curve pool ETH/crvUSD spot price depresses
        ↓
Arb bots sell ETH on CEX/perp to capture spread
        ↓
ETH perp price falls
        ↑
We are already short from 1–2% above the band cluster
```

---

## Market Context

- **Applicable asset:** ETH (primary crvUSD collateral). BTC and other collateral types if/when added.
- **Venue:** ETH-USDC perpetual futures on Hyperliquid (execution). On-chain data from Ethereum mainnet.
- **Relevant scale:** As of early 2025, crvUSD had ~$200–400M in outstanding loans, with ETH as dominant collateral. Individual band clusters can represent $5–50M in forced conversion volume at specific price ticks. This is meaningful relative to typical ETH perp order book depth but not dominant.
- **Mechanism frequency:** Oracle updates occur every ~3–5 minutes. Soft liquidation is continuous, not a single event.

---

## Entry Rules

### Signal Construction

**Step 1 — Band Map (updated every 15 minutes)**
Query LLAMMA contract for all active positions. For each position, extract:
- `band_lower` and `band_upper` (price boundaries)
- `collateral_amount` (ETH at risk of conversion)
- `current_composition` (% already converted to crvUSD)

Aggregate into a "band heat map": for each $50 ETH price tick, compute:
- `forced_sell_volume_ETH`: ETH that will be sold if price falls through this tick
- `forced_buy_volume_ETH`: ETH that will be bought if price rises through this tick

**Step 2 — Cluster Identification**
A "dense cluster" is defined as: cumulative forced conversion volume ≥ **$10M ETH equivalent** within a **$200 price range** (4 consecutive $50 ticks).

**Step 3 — Entry Trigger**

| Direction | Condition |
|-----------|-----------|
| **Short ETH perp** | Spot ETH price is within **1.5%–3% above** the top of a dense downward-band cluster AND price is trending downward (last 30-min candle close < open) |
| **Long ETH perp** | Spot ETH price is within **1.5%–3% below** the bottom of a dense upward-band cluster AND price is trending upward (last 30-min candle close > open) |

The 1.5%–3% buffer exists to enter *before* the forced flow begins, not after. The directional filter (30-min trend) reduces false entries when price is oscillating near a band without commitment.

**Step 4 — Confirmation (optional, reduces false positives)**
Check that the ETH/crvUSD Curve pool has shown net ETH outflow in the last oracle cycle (on-chain mempool or event log monitoring). This confirms LLAMMA has begun executing.

---

## Exit Rules

| Condition | Action |
|-----------|--------|
| Price exits the far side of the band cluster (full conversion complete) | Close position — forced flow is exhausted |
| Price reverses back through entry level (band cluster not reached) | Stop loss — thesis invalidated |
| Position held > 8 hours without band cluster being reached | Time-based exit — opportunity cost |
| Funding rate turns strongly adverse (> 0.05% per 8h against position) | Close — carry cost erodes edge |

**Target:** Capture the price move through the band cluster. Expected move = price impact of $10M+ forced sell/buy into ETH spot liquidity. Rough estimate: 0.3%–1.5% depending on market depth at time of entry.

**Stop loss:** 1% against entry (tight, because if price reverses before reaching the band, the thesis is wrong).

---

## Position Sizing

```
Base position size = min(
    0.5% of portfolio NAV,
    $cluster_forced_volume_USD × 0.02
)
```

**Rationale for the 2% multiplier:** We expect to capture roughly 2% of the forced flow as price impact on the perp. This is conservative. If the cluster is $20M, we size to $400K notional maximum.

**Leverage:** 3x–5x on Hyperliquid. The edge is small per trade; leverage is necessary but kept moderate given stop-loss tightness.

**Maximum concurrent positions:** 2 (one long, one short if bands exist at different price levels simultaneously).

**Scale-in rule:** Enter 50% at 2.5% above/below cluster, add remaining 50% when price is 1% above/below cluster (closer confirmation).

---

## Backtest Methodology

### Data Requirements

| Dataset | Source | Notes |
|---------|--------|-------|
| LLAMMA position state (historical) | Ethereum archive node (e.g., Alchemy, Infura) | Query `bands_x`, `bands_y`, `user_state` at each block |
| ETH/USD oracle price feed | Chainlink ETH/USD on-chain logs | Same timestamps as LLAMMA updates |
| ETH perp OHLCV | Hyperliquid historical data API | 1-minute candles |
| Curve pool swap events | Ethereum event logs (`TokenExchange`) | Validate actual flow execution |

### Backtest Period
- **Primary:** January 2023 – December 2024 (covers crvUSD launch and multiple ETH volatility regimes)
- **Stress test:** August 2023 ETH drawdown, March 2024 rally, August 2024 crypto selloff

### Methodology Steps

1. **Reconstruct band map** at each 15-minute interval using archive node snapshots. This is the most technically demanding step — requires replaying contract state.
2. **Identify all cluster events** where $10M+ forced conversion was queued within a $200 range.
3. **Simulate entries** at 2.5% and 1% above/below cluster using ETH perp 1-minute OHLCV.
4. **Simulate exits** using band-exit rules above.
5. **Calculate P&L** net of:
   - Hyperliquid taker fees (0.035%)
   - Estimated slippage (0.05% for $400K notional)
   - Funding rate costs (use actual historical funding)
6. **Measure:** Win rate, average P&L per trade, Sharpe ratio, max drawdown, number of qualifying signals per month.

### Key Backtest Questions
- How many $10M+ cluster events occurred per month? (Hypothesis: 5–20)
- What was the average price move through a cluster vs. outside clusters?
- Did the perp lead, lag, or coincide with spot LLAMMA execution?
- Were there false signals where price approached but reversed before entering the band?

### Minimum Viable Signal
- ≥ 30 qualifying cluster events in backtest period
- Win rate ≥ 55%
- Average P&L per trade ≥ 0.2% (net of costs)
- Sharpe ≥ 1.0 on trade-level returns

---

## Go-Live Criteria

- [ ] Backtest passes minimum viable signal thresholds above
- [ ] Real-time band map pipeline operational (15-min refresh, < 30s latency)
- [ ] Oracle update event listener live (detects LLAMMA execution in real time)
- [ ] Manual review of first 10 live signals before automation
- [ ] Paper trading for 30 days with ≥ 10 qualifying signals observed
- [ ] Paper trading Sharpe ≥ 0.8 (lower threshold than backtest due to sample size)

---

## Kill Criteria

| Trigger | Action |
|---------|--------|
| 10 consecutive losing trades in live trading | Suspend, full review |
| Live Sharpe < 0.3 over 60-day rolling window | Suspend |
| crvUSD TVL drops below $50M (insufficient flow) | Suspend — edge disappears |
| Curve protocol upgrade changes LLAMMA mechanics | Immediate suspension, re-validate mechanism |
| Average cluster size drops below $5M (insufficient price impact) | Reduce size or suspend |
| Competing bots demonstrably front-running our entries | Re-evaluate entry timing, consider suspension |

---

## Risks

### Structural Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Band flow too small to move ETH perp** | High | Backtest will reveal; $10M threshold is a minimum filter |
| **Arb bots already price in LLAMMA flow** | Medium | Measure perp vs. spot lead/lag in backtest; if perp leads, edge is gone |
| **LLAMMA oracle manipulation** | Low | Curve uses Chainlink + TWAP; manipulation is expensive |
| **crvUSD protocol risk (smart contract bug)** | Low | We're trading perps, not holding crvUSD |
| **Band data staleness** | Medium | 15-min refresh may miss rapid position changes; add event-driven updates |

### Execution Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Entry too early — price reverses before band** | High | Tight 1% stop loss; scale-in approach |
| **Funding rate adversity on extended holds** | Medium | 8-hour time-based exit cap |
| **Hyperliquid liquidity insufficient for size** | Low | $400K notional is well within ETH perp depth |
| **Gas costs for on-chain data queries** | Negligible | Read-only calls, no gas cost |

### Competitive Risks

- **This edge is visible to anyone reading the Curve docs.** The question is whether the flow is large enough to be worth systematizing. If it is, other quant shops will find it.
- **Curve's own interface** (curve.fi/crvusd) shows health maps — sophisticated users already monitor this. The edge is in *aggregating and acting on* the data systematically, not in exclusive access.

---

## Data Sources

| Source | URL / Method | Update Frequency |
|--------|-------------|-----------------|
| LLAMMA contract (ETH collateral) | `0x7adcc491f0B7f9BC12837B8F5Edf0E580d176F1f` on Ethereum | Per block |
| Curve crvUSD health dashboard | curve.fi/crvusd | Real-time |
| Curve subgraph (band aggregates) | thegraph.com/hosted-service/subgraph/curvefi/crvusd | ~5 min lag |
| Chainlink ETH/USD feed | On-chain event logs | Per block |
| Hyperliquid perp data | api.hyperliquid.xyz | Real-time |
| Ethereum archive node | Alchemy / Infura (paid tier) | Historical replay |

---

## Implementation Notes

### Minimum Engineering Requirements
1. **Archive node access** for historical backtest (Alchemy Growth plan ~$200/month or self-hosted)
2. **Python script** to decode LLAMMA contract state: `bands_x[i]` (crvUSD in band i), `bands_y[i]` (ETH in band i), `active_band`, `A` (amplification factor)
3. **Band price calculator:** `p_band_up(n) = p0 / (A/(A-1))^n`, `p_band_down(n) = p0 / (A/(A-1))^(n+1)` — standard Curve LLAMMA math
4. **Hyperliquid Python SDK** for order execution
5. **Alert system** (Telegram/Discord) for manual review during paper trading phase

### Prototype Timeline
- Week 1–2: Archive node data pipeline, band map reconstruction
- Week 3: Backtest engine, signal identification
- Week 4: Backtest execution, results analysis
- Week 5–8: Paper trading with live pipeline
- Week 9+: Go/no-go decision

---

## Related Strategies to Consider

- **crvUSD hard liquidation front-run:** When a position's health drops below 0, a hard liquidation occurs — a single large market sell. More violent than soft liquidation but less predictable timing.
- **Curve gauge weight rebalancing:** Weekly gauge votes force predictable CRV emissions shifts, creating flow into/out of specific pools.
- **LST unbonding queue arb:** Similar "dam" mechanic — withdrawal queues create predictable NAV convergence (higher structural score, ~8/10).

---

*This document represents a hypothesis requiring empirical validation. No backtest results exist at time of writing. Do not allocate capital until go-live criteria are met.*
