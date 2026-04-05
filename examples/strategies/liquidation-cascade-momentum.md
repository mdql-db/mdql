---
title: "Strategy Specification: Liquidation Cascade Momentum"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 3
frequency: 7
composite: 525
categories:
  - liquidation
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Executive Summary

Monitor perpetual futures markets for dense clusters of estimated liquidation levels. When price approaches such a cluster, enter a position in the direction that would trigger the cascade, riding the self-reinforcing liquidation waterfall. Exit quickly once momentum exhausts.

**Honest upfront assessment: This strategy has a real theoretical edge but faces serious practical obstacles around data availability, execution speed, and market crowding. Read the concerns section before building anything.**

---

## 1. Hypothesis

### Core Claim

Perpetual futures exchanges carry large pools of leveraged positions. These positions have computable (or estimable) liquidation prices. When spot/mark price enters a zone with high notional value of liquidation orders clustered within a tight price band, the mechanical process of exchange-forced liquidations generates cascading market sell (or buy) orders that:

1. Push price further into the cluster
2. Trigger additional liquidations
3. Create a momentum spike disproportionate to the initiating price move

The edge is **mechanical predictability**: the cascade is not driven by information or sentiment — it is driven by margin math. If you can identify the cluster before it is triggered, you have a directional bet with an asymmetric payoff: small move to entry point, potentially large cascade if the cluster is real.

### Why This Might Be an Edge

| Factor | Assessment |
|--------|-----------|
| Mechanism is deterministic | Liquidation at exchange level is rule-based, not discretionary |
| 24/7 markets | Human traders miss setups; automation does not |
| Cluster data is partially observable | OI concentration, funding extremes, and heatmaps give signal |
| Speed advantage | Cascade typically unfolds in seconds to minutes; fast entry matters |
| Market structure knowledge | Most retail traders do not systematically model this |

### Why This Might NOT Be an Edge

| Factor | Assessment |
|--------|-----------|
| Crowded trade | Every major quant desk, HFT, and "liquidation hunting" bot targets this |
| Data quality | Free liquidation level data is estimated, not ground truth |
| Cascade frequency | True cascades are rare; most clusters absorb price without cascading |
| Slippage | Entry during cascade means you are competing with the cascade itself |
| Adverse selection | By the time a signal is clear, the move may be 80% done |

---

## 2. Data Sources — Honest Audit

This section is critical. If the data is not available, the strategy is not buildable.

### 2.1 Hyperliquid API

**What is available (free, programmatic):**
- Mark price, index price (real-time)
- Open interest by asset (real-time)
- Funding rates (current and historical)
- Individual trade fills (public)
- Order book depth
- Account-level position data (authenticated)

**What is NOT available:**
- Individual user liquidation prices (private account data — not accessible)
- Aggregate liquidation price distribution across all accounts (not exposed)
- Raw liquidation order flow in a structured feed (some liquidation events appear in trade history as tagged fills, but not with cluster-level statistics)

**Verdict:** Useful for OI, funding, and mark price. Cannot directly observe liquidation clusters. ⚠️

### 2.2 CoinGlass

**What is available (paid tiers):**
- Liquidation heatmaps (estimated, model-based)
- Historical liquidation data (aggregated, delayed)
- Open interest charts
- Long/short ratio

**What is NOT available free:**
- Real-time granular liquidation level API access (requires paid subscription; Pro tier ~$30–100/month)
- Heatmap data via API (web scraping is fragile and against ToS)
- Individual exchange liquidation price books

**Critical caveat:** CoinGlass liquidation heatmaps are **modeled estimates**, not actual exchange data. The model assumes typical leverage distributions (e.g., 10x, 20x, 50x) applied to historical price entry points based on OI buildup. The actual liquidation prices depend on maintenance margin, individual account leverage, and partial liquidations — none of which are publicly observable. The heatmap is a reasonable proxy, not ground truth.

**Verdict:** Useful for visual reference and backtesting proxies. API access requires payment. Data is estimated, not real. ⚠️⚠️

### 2.3 Binance Futures

**What is available (free):**
- Open interest (aggregate, per asset, real-time via REST/WebSocket)
- Long/short account ratio (top traders and all accounts)
- Top trader long/short position ratio
- Funding rate (current and history)
- Liquidation order stream (WebSocket: real-time liquidation fills as they occur, including side, quantity, price)
- Mark price
- Kline/OHLCV data

**What is NOT available:**
- Future liquidation price distribution (prospective clusters)
- Individual account leverage or liquidation prices

**Verdict: The Binance liquidation order WebSocket stream is the most actionable free data source for this strategy.** It tells you liquidations are happening NOW — useful for cascade detection and momentum entry, but reactive rather than predictive. ✅⚠️

### 2.4 Data Source Summary

| Source | Predictive Value | Reactive Value | Cost | API Quality |
|--------|-----------------|----------------|------|-------------|
| Hyperliquid OI/Funding | Medium | Low | Free | Good |
| CoinGlass Heatmap | Medium (estimated) | Low | ~$30-100/mo | Fragile |
| Binance Liquidation Stream | Low | High | Free | Excellent |
| Binance OI/Ratios | Medium | Medium | Free | Excellent |
| Coinalyze (alternative) | Medium | Low | Freemium | Moderate |

**Practical conclusion:** A fully predictive version of this strategy (enter before cascade starts) requires paid estimated data. A reactive version (enter as cascade begins, using live liquidation stream) is buildable with free data but sacrifices entry quality.

---

## 3. Strategy Variants

Given the data constraints, define two sub-variants:

### Variant A: Predictive Cascade Entry (requires paid data)
Use CoinGlass heatmap API or equivalent to identify dense estimated liquidation zones. Enter when price is within X% of the zone. Higher upside, worse data quality.

### Variant B: Reactive Cascade Momentum (free data)
Use Binance liquidation WebSocket stream. When liquidation volume in a rolling window exceeds threshold, enter in the direction of liquidations. Lower upside (entering mid-cascade), better data reliability.

**This spec covers both variants. Backtest methodology must be adapted per variant.**

---

## 4. Definitions and Parameters

### 4.1 Liquidation Cluster (Variant A — Estimated)

A **liquidation cluster** is defined as:
- A price band of width `W` (default: 0.5% of current price)
- Containing estimated notional liquidation value ≥ `L_threshold` (default: $50M for BTC, $20M for ETH, scaled by asset ADV)
- Derived from CoinGlass heatmap or equivalent model

### 4.2 Cascade Trigger (Variant B — Reactive)

A **cascade trigger** occurs when:
- Liquidation notional in rolling `T_window` (default: 60 seconds) exceeds `C_threshold` (default: $10M for BTC)
- Price is moving in the direction of liquidations (not reverting)
- OI is dropping (confirming forced closes, not voluntary)

### 4.3 Supporting Conditions (Both Variants)

| Condition | Rationale |
|-----------|-----------|
| Funding rate extreme (> +0.1% or < -0.1% per 8h) | High leverage in one direction = more fuel |
| OI at or near 30-day high | More leveraged positions outstanding |
| Long/short ratio skewed > 60/40 | Crowded positioning = cascade potential |
| Time filter: avoid 08:00 UTC funding settlement ±5min | Erratic price action |

---

## 5. Entry Rules

### 5.1 Variant A — Predictive Entry

**Pre-conditions (all must be true):**
1. CoinGlass (or model) identifies a liquidation cluster of notional value ≥ `L_threshold` within `D_proximity` (default: 1.5%) of current mark price
2. Price is moving toward the cluster (not away from it) — confirm with 5-minute price momentum
3. OI ≥ 70th percentile of trailing 30-day OI for this asset
4. Funding rate magnitude ≥ 0.05% per 8h in the direction that would be hurt by the cascade
5. No major macro event in next 2 hours (FOMC, CPI, etc.) — manual override flag

**Entry:**
- Enter market order (or aggressive limit at best bid/ask) in the direction of the anticipated cascade
- Direction: if cluster is below price (long liquidations), go SHORT; if cluster is above price (short liquidations), go LONG
- Entry size: per position sizing rules (Section 7)
- Record entry price, OI at entry, funding at entry

### 5.2 Variant B — Reactive Entry

**Pre-conditions:**
1. Liquidation notional in last 60 seconds ≥ `C_threshold`
2. Liquidations are predominantly one-sided (> 70% long or > 70% short by notional)
3. OI delta in last 60 seconds is negative (positions closing, not opening)
4. Price has moved ≥ 0.3% in the direction of liquidations in last 60 seconds
5. Bid-ask spread ≤ 2x normal spread (not in a liquidity vacuum)

**Entry:**
- Enter market order immediately upon condition trigger
- Direction: same as liquidation direction (longs being liquidated = go short, vice versa)
- Entry size: per position sizing rules (Section 7)

---

## 6. Exit Rules

### 6.1 Take Profit — Tiered

| Tier | Target | Size to Close |
|------|--------|---------------|
| TP1 | Entry + 0.8% | 40% of position |
| TP2 | Entry + 1.8% | 35% of position |
| TP3 | Entry + 3.5% | 25% of position |

Note: TP3 is rarely hit. Cascades are fast and exhaust. Do not be greedy.

### 6.2 Stop Loss

- **Hard stop:** Entry − 0.5% (fixed, placed immediately on entry)
- **Time stop:** Exit 100% of position if no movement toward TP1 within 5 minutes of entry
- **OI reversal stop:** If OI begins increasing while price moves against you (new positions opening against your direction), exit immediately — the cascade is being absorbed

### 6.3 Trailing Stop (if TP1 hit)

Once TP1 is hit, move stop to breakeven on remaining position. After TP2, trail at 0.5% below highest close (or above lowest close for shorts) on 1-minute bars.

### 6.4 Momentum Exhaustion Exit

Exit remaining position if:
- Liquidation stream volume drops to < 20% of peak rate during cascade
- Price consolidates for 3 consecutive 1-minute bars without new lows/highs
- Order book bid/ask imbalance normalizes (bid/ask depth ratio returns to 1.0 ± 0.2)

---

## 7. Position Sizing

### 7.1 Base Rules

- **Risk per trade:** 1% of account equity
- **Stop distance:** 0.5% (from entry rules above)
- **Position size calculation:**

```
Position Size (USD) = (Account Equity × 0.01) / Stop Distance
Position Size (USD) = (Account Equity × 0.01) / 0.005
Position Size (USD) = Account Equity × 2
```

Example: $10,000 account → $20,000 notional position (2x leverage on account). With 10x available on exchange, this is well within margin requirements.

### 7.2 Leverage Cap

- Maximum leverage: **5x account equity** regardless of above formula
- Maximum single position notional: **$25,000** for accounts under $10,000 (slippage concern — see risks)

### 7.3 Scaling Rules

- Do not increase position size mid-cascade (chasing)
- If two setups trigger simultaneously on different assets, reduce each to 0.6% risk (not 1% each)
- After 3 consecutive losses, reduce position size to 0.5% risk per trade until 5 winners restore confidence

### 7.4 Small Account Reality Check

For a $5,000–$10,000 account:
- Position notional: $10,000–$20,000
- This is genuine noise-level size on BTC or ETH perpetuals
- Slippage on market entry during a cascade will be significant — model 0.1–0.2% slippage in backtests
- The strategy is **marginally viable** at this size but edges will be compressed by transaction costs
- More realistic target account: $50,000+

---

## 8. Backtest Methodology

### 8.1 Data Requirements

| Dataset | Source | Frequency | Period |
|---------|--------|-----------|--------|
| BTCUSDT perp OHLCV | Binance | 1-minute | Jan 2021 – present |
| ETHUSDT perp OHLCV | Binance | 1-minute | Jan 2021 – present |
| Aggregate liquidation data (historical) | CoinGlass CSV export or Coinalyze | 1-hour buckets | Jan 2021 – present |
| Open interest | Binance/Coinalyze | 1-hour | Jan 2021 – present |
| Funding rate | Binance | 8-hour | Jan 2021 – present |
| Long/short ratio | Binance | 1-hour | Jan 2021 – present |

**Note on liquidation data:** Binance provides historical aggregate liquidation data but not the tick-level stream historically. CoinGlass provides historical liquidation charts. Neither provides the exact liquidation cluster heatmap historically. **Variant B (reactive) is more backtestable than Variant A.**

### 8.2 Proxy for Liquidation Clusters (Variant A Backtest)

Since ground-truth cluster data is not historically available, construct a proxy:

1. Use OI buildup periods: identify 24-hour windows where OI increased by ≥ 15% and funding rate moved ≥ 0.08% per 8h in one direction
2. Estimate liquidation concentration by binning historical entry prices weighted by OI delta into 0.5% price bands
3. Flag bands where estimated cumulative notional exceeds `L_threshold`
4. This is a noisy proxy — acknowledge in results

### 8.3 Variant B Backtest Procedure

1. **Load data:** 1-minute OHLCV + hourly aggregated liquidation volume (by side) + OI + funding
2. **Simulate cascade detection:** At each 1-minute bar, check if liquidation volume in trailing window exceeds `C_threshold`
3. **Apply entry conditions:** All pre-conditions from Section 5.2 (adapted to hourly liquidation data — note this degrades signal quality vs. real-time)
4. **Simulate entry:** Open position at next bar open + 0.15% slippage (market impact estimate)
5. **Apply exit rules:** Per Section 6, evaluated bar-by-bar
6. **Record:** Entry/exit price, slippage, P&L, trade duration, winning/losing, cascade size

### 8.4 Backtest Configuration

```
Assets:          BTC, ETH (primary); SOL, DOGE (secondary validation)
Start date:      January 1, 2021
End date:        December 31, 2024
Initial capital: $10,000
Commission:      0.05% per side (Binance taker fee)
Slippage:        0.15% per side (conservative cascade entry estimate)
Funding costs:   Apply 8-hour funding accrual to held positions
Leverage:        2x account notional (per position sizing)
Max positions:   2 simultaneous
```

### 8.5 Key Backtest Metrics to Report

| Metric | Minimum Acceptable |
|--------|--------------------|
| Total trades | ≥ 100 (statistical significance) |
| Win rate | ≥ 45% |
| Average R:R (actual) | ≥ 1.5
