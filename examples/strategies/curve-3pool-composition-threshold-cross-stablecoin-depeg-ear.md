---
title: "Curve 3pool Composition Threshold Cross — Stablecoin Depeg Early Warning Arb"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 6
frequency: 3
composite: 450
categories:
  - stablecoin
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a single stablecoin's share of Curve's 3pool (USDT/USDC/DAI) crosses 45% of total pool reserves, the market is revealing a preference to exit that stablecoin *before* the price discount is fully reflected on centralised exchanges or perpetual funding rates. The pool composition shift is a mechanically observable, on-chain revealed-preference signal. A mean-reversion trade — long the underweight stablecoin, short the overweight one — should capture the lag between on-chain price discovery and CEX/perp repricing.

**Causal chain:**

1. Large holder or routing algorithm wants to exit stablecoin X. They swap X → Y/Z via 3pool because it is the deepest venue.
2. Pool composition skews: X rises above 33% equilibrium. The AMM curve prices X at a discount *within the pool* immediately.
3. CEX spot and perp markets lag because: (a) most price feeds poll CEX data, not on-chain AMM reserves; (b) retail and smaller arb desks are not monitoring pool composition in real time.
4. Cross-venue arb eventually closes the gap, but the lag window (estimated minutes to hours) is the tradeable edge.
5. Entry at the 45% threshold captures the early phase of this lag. Exit when pool rebalances (arb complete) or 24h elapses (signal stale).

**What this is NOT:** This is not a prediction that a depeg will occur. It is a bet that the *information already embedded in pool composition* has not yet been fully priced elsewhere.

---

## Structural Mechanism — Why This Happens

**The AMM is a continuous, permissionless price discovery engine.** Curve's StableSwap invariant means that as composition skews, the marginal price of the excess coin drops within the pool. This is not a lagging indicator — it is instantaneous price discovery by capital at risk.

**CEX price feeds are structurally slower for stablecoins.** Stablecoin/stablecoin pairs on CEX (USDT/USDC, USDC/DAI) have thin order books and low trading volume under normal conditions. Market makers do not continuously update quotes based on Curve pool state. The information propagation path is: Curve pool skews → sophisticated arb bots notice → they trade CEX to close the gap → CEX price moves. Each step has latency.

**Perp funding rates are even slower.** Hyperliquid and other perp venues price stablecoin perps (where they exist) based on index prices derived from CEX. Funding rate adjustments lag spot price moves by the funding interval (typically 1–8 hours).

**The 45% threshold is a meaningful signal, not noise, because:**
- At 33% equal weight, normal swap routing causes ±2–3% fluctuations.
- Reaching 45% requires net inflow of approximately 18% of pool TVL in a single direction. At 3pool's typical TVL (~$300–500M), this is $54–90M of directional flow. This is not routing noise.
- Historical depeg events (USDC March 2023, DAI correlation event) show composition reaching 50–70% before CEX prices fully adjusted.

**What is NOT guaranteed:** The lag duration is not fixed. Sophisticated arb bots may close the gap in minutes. The strategy requires that *some* lag exists and is consistent enough to trade. This is the probabilistic element that drops the score from 8 to 6.

---

## Entry Rules


### Signal Definition

Monitor Curve 3pool reserves every 60 seconds via on-chain RPC or subgraph.

```
pool_share_X = reserve_X / (reserve_USDT + reserve_USDC + reserve_DAI)
```

**Trigger condition:** `pool_share_X > 0.45` for any X ∈ {USDT, USDC, DAI}

Identify:
- **Overweight coin** (X): the one above 45%
- **Underweight coin** (Y): the one with the lowest share at trigger time

### Entry

- **Instrument:** USDC/USDT spot pair on a CEX (Binance, Coinbase) OR stablecoin perp on Hyperliquid if available with sufficient liquidity (>$1M open interest).
- **Direction:** Long Y (underweight) / Short X (overweight) as a pair trade.
- **Entry price:** Mid-market at the time of threshold cross, not limit orders (to avoid missing the window).
- **Entry confirmation:** Require threshold to persist for 2 consecutive 60-second readings to filter single-block anomalies.

## Exit Rules

### Exit

**Primary exit (mean reversion):** `pool_share_X < 0.38` (pool has rebalanced toward equilibrium)

**Time stop:** 24 hours after entry, regardless of pool state.

**Stop-loss (depeg acceleration):** `pool_share_X > 0.65` — composition has blown through the arb zone into genuine depeg territory. Exit immediately. Do NOT hold through a real depeg event.

**Profit target:** No fixed TP. Let the mean-reversion exit rule handle it. The expected move is 10–30 bps on the USDC/USDT pair; do not hold for more.

### Position Management

- Do not add to a position if composition continues drifting after entry (no averaging down).
- If two coins simultaneously cross 45% (impossible by construction — they sum to 100% with a third), re-evaluate manually.
- Only one active trade at a time. If a second threshold cross occurs while in a position, log it but do not open a second leg.

---

## Position Sizing

**Base size:** 2% of portfolio NAV per trade.

**Rationale:** Stablecoin pair trades have very low volatility under normal conditions (moves are measured in bps). 2% NAV gives meaningful P&L without catastrophic exposure if a genuine depeg occurs and the stop-loss is hit.

**Stop-loss sizing:** At the 0.65 stop, maximum loss on a stablecoin pair trade is approximately 0.5–1.5% (based on historical USDC/USDT spread during the March 2023 event). 2% NAV × 1.5% max move = 3 bps of total portfolio loss per stopped-out trade. This is acceptable.

**Do not use leverage.** The edge is in the information lag, not in amplifying a small move. Leverage introduces liquidation risk if a genuine depeg occurs before the stop triggers.

**Scaling rule:** After 20 backtested trades with positive expectancy, consider scaling to 5% NAV. Do not scale before validation.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Period |
|---|---|---|---|
| Curve 3pool reserves | The Graph (Curve subgraph) or direct Ethereum RPC archive node | Per-block (~12s) | Jan 2021 – present |
| USDC/USDT spot price | Binance API (`/api/v3/klines`, symbol `USDCUSDT`) | 1-minute OHLCV | Jan 2021 – present |
| USDT/DAI spot price | Binance or Kraken API | 1-minute OHLCV | Jan 2021 – present |
| Curve 3pool swap events | Ethereum event logs (`TokenExchange` event, contract `0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7`) | Per-event | Jan 2021 – present |

### Data Sources — Specific Endpoints

- **Curve subgraph (The Graph):** `https://api.thegraph.com/subgraphs/name/messari/curve-finance-ethereum` — query `liquidityPool` entity for `inputTokenBalances` by block.
- **Ethereum archive RPC:** Alchemy or Infura — call `eth_call` on 3pool contract `get_balances()` at historical blocks.
- **Binance REST:** `https://api.binance.com/api/v3/klines?symbol=USDCUSDT&interval=1m`
- **Hyperliquid historical data:** `https://app.hyperliquid.xyz/api` — check if USDC perp exists with sufficient history.

### Backtest Steps

1. **Reconstruct pool composition time series** at 1-minute resolution by interpolating per-block reserve data.
2. **Identify all threshold cross events** (45% for any coin, sustained 2 minutes).
3. **For each event**, record:
   - Entry time and USDC/USDT mid-price at entry
   - Pool composition at entry
   - Time to mean reversion (pool_share < 38%)
   - USDC/USDT price at mean reversion exit
   - USDC/USDT price at 24h time stop
   - Whether 65% stop was hit, and price at that point
4. **Calculate per-trade P&L** in bps, net of estimated transaction costs (assume 2 bps round-trip for spot, 5 bps for perps).
5. **Separate analysis for each coin** (USDT events vs USDC events vs DAI events) — they may have different characteristics.
6. **Key metrics to compute:**
   - Win rate
   - Average P&L per trade (bps)
   - Average hold time
   - Maximum adverse excursion (MAE)
   - Sharpe ratio (annualised, using daily P&L)
   - Number of stop-loss events (65% threshold hits)
   - Correlation of signal frequency with market stress periods

### Baseline Comparison

Compare against a naive strategy: enter USDC/USDT long at a fixed time each day (e.g., midnight UTC) and exit 24h later. This tests whether the pool composition signal adds value over random entry.

### Critical Sub-Questions for Backtest

1. **Lag distribution:** What is the median and 90th percentile lag between 3pool composition cross and CEX price adjustment? If median lag < 2 minutes, the strategy is not executable without HFT infrastructure.
2. **False positive rate:** How often does composition cross 45% and then revert *without* any CEX price move? These are benign routing events and will be losers.
3. **Event frequency:** How many qualifying events occurred per year? If <10, the strategy has insufficient sample size.
4. **Asymmetry by coin:** USDT events (Tether risk) may behave differently from USDC events (Circle/banking risk) — analyse separately.

---

## Go-Live Criteria

The backtest must show ALL of the following before paper trading begins:

1. **Minimum 30 qualifying events** across the backtest period (not counting the March 2023 USDC event as representative — it is an outlier).
2. **Win rate ≥ 55%** on mean-reversion exits (excluding time stops).
3. **Average P&L ≥ 5 bps per trade** net of transaction costs.
4. **Sharpe ratio ≥ 1.0** on annualised daily P&L.
5. **Median lag ≥ 5 minutes** between 3pool signal and CEX price adjustment (confirms the window is executable without HFT).
6. **Stop-loss events ≤ 10% of total trades** (confirms the 65% threshold is a reliable circuit breaker).
7. **Strategy is profitable in at least 2 of 3 sub-periods** (2021–2022, 2022–2023, 2023–2025) — confirms it is not a one-period artefact.

If the backtest shows the strategy is only profitable during the March 2023 USDC event, **reject the strategy** — single-event dependence is not a structural edge.

---

## Kill Criteria

Abandon the strategy (in backtest, paper trade, or live) if any of the following occur:

1. **Backtest shows median lag < 3 minutes** — not executable without co-location.
2. **Win rate < 50% in backtest** — coin flip with transaction costs is a loser.
3. **More than 2 stop-loss events (65% threshold) in paper trading** — the stop is not protecting capital reliably.
4. **Paper trading shows consistent slippage > 3 bps** on entry — the window is too short to enter at a reasonable price.
5. **Curve 3pool TVL drops below $100M** — pool is no longer the primary stablecoin venue; signal loses validity.
6. **A competitor protocol (e.g., Uniswap v4 stablecoin pool) absorbs >50% of stablecoin swap volume** — the signal migrates; rebuild on the new venue or abandon.
7. **Live trading: 3 consecutive stopped-out trades** — re-evaluate threshold parameters before continuing.

---

## Risks

### Primary Risk: The Lag May Not Exist

The biggest risk is that sophisticated arb bots already close the Curve-to-CEX gap in seconds, not minutes. If the median lag is 30–60 seconds, this strategy requires co-location and is not viable for Zunid. **This is the most likely failure mode.** The backtest must measure this directly.

### Secondary Risk: Benign Composition Skews

Large DeFi protocols (e.g., MakerDAO PSM, Aave liquidations) route enormous swaps through 3pool for reasons unrelated to stablecoin risk. A $100M PSM rebalance will trigger the 45% threshold and immediately revert without any CEX price move. These will be losers. The 2-reading confirmation filter helps but does not eliminate this.

### Tail Risk: Genuine Depeg

If a real depeg occurs (USDT collapses, USDC depegs again), the 65% stop-loss will be hit. In a fast-moving depeg, the stop may execute at a significantly worse price than 65% composition implies. The March 2023 USDC event saw USDC/USDT trade at $0.87 briefly — a 13% loss on a "stablecoin" pair. **This is the catastrophic scenario.** Position sizing at 2% NAV limits total portfolio damage to ~26 bps in a worst-case scenario, which is acceptable.

### Liquidity Risk on Perps

If trading stablecoin perps on Hyperliquid rather than spot, open interest may be insufficient to enter/exit cleanly. Verify OI > $5M before using perps. Default to spot if perp liquidity is inadequate.

### Regulatory / Operational Risk

Monitoring on-chain data continuously requires reliable RPC infrastructure. A dropped connection during a fast-moving event means missing the entry or, worse, missing the stop-loss. Build redundant RPC endpoints (Alchemy + Infura + self-hosted) before going live.

### Model Risk: Threshold Calibration

The 45% entry and 65% stop thresholds are hypotheses, not empirically validated levels. The backtest may reveal that 50% entry and 70% stop perform better, or that the thresholds should be coin-specific. Do not assume the proposed thresholds are optimal — treat them as starting points for optimisation.

---

## Data Sources

| Source | URL / Endpoint | Notes |
|---|---|---|
| Curve 3pool contract | `0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7` (Ethereum mainnet) | Call `get_balances()` for reserves |
| Curve subgraph | `https://api.thegraph.com/subgraphs/name/messari/curve-finance-ethereum` | Historical pool balances by block |
| Dune Analytics | `https://dune.com/queries` | Pre-built Curve pool composition queries available; faster than raw RPC for backtest |
| Binance spot API | `https://api.binance.com/api/v3/klines?symbol=USDCUSDT&interval=1m` | 1-minute OHLCV, free, no auth required |
| Kraken OHLCV | `https://api.kraken.com/0/public/OHLC?pair=USDCUSDT` | Backup CEX feed |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | Perp OI and funding rates |
| Ethereum archive node | Alchemy (`https://eth-mainnet.g.alchemy.com/v2/`) or Infura | Required for per-block `get_balances()` calls |
| CoinGecko historical | `https://api.coingecko.com/api/v3/coins/usd-coin/market_chart` | Backup price data for USDC/USDT |

**Recommended backtest stack:** Pull Curve reserve data via Dune Analytics (fastest for historical), cross-reference with Binance 1-minute OHLCV. Use Python with `web3.py` for live monitoring once strategy is validated.
