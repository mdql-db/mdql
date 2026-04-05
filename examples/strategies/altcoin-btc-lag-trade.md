---
title: "Strategy Specification: Altcoin BTC Lag Trade"
status: HYPOTHESIS
mechanism: 3
implementation: 7
safety: 5
frequency: 7
composite: 735
categories:
  - calendar-seasonal
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

### Core Claim
When Bitcoin makes a sharp directional move exceeding 3–5% within a single 4-hour candle, altcoins systematically underreact at the time of the move and then "catch up" over the subsequent 2–12 hours. This creates a predictable, tradeable echo window.

### Proposed Mechanism
The lag arises from **liquidity microstructure asymmetry**:

1. **Liquidity concentration**: During risk events, market participants rotate through BTC first because it has the deepest order books and lowest slippage. Capital then cascades into altcoins sequentially.
2. **Attention sequencing**: Retail and smaller algorithmic traders monitor BTC price action as a primary signal. Alert/execution pipelines trigger with a delay for alt positions.
3. **Cross-exchange arbitrage latency**: BTC price discovery happens on CME, Coinbase, and Binance simultaneously. Alt price discovery is more fragmented and slower to equilibrate.
4. **Correlation structure**: In aggregate, large-cap alts (ETH, SOL, BNB, etc.) show 0.6–0.85 rolling 30-day correlation to BTC. They mean-revert toward BTC's new price level, not away from it.

### Why It Might Be an Edge
- The lag is structural, not random — it arises from identifiable microstructure constraints.
- The event is objectively observable (BTC ≥ X% in 4h) with no look-ahead.
- The catch-up window (2–12h) is wide enough to enter without perfect timing.
- High-frequency events (BTC moves >3% occur multiple times per month) provide statistical power.

### Why It Might Not Work (Pre-Backtest Skepticism)
- As markets mature, algorithmic market-makers reprice alts within minutes, not hours. The edge may have existed in 2019–2021 but compressed since.
- The relationship may be directionally asymmetric (alts catch down faster than they catch up in bull moves, or vice versa).
- Funding rates and perpetual swap mechanics on exchanges like Binance may add noise or adverse carry costs.

---

## Backtest Methodology

### 2.1 Time Period
- **Primary**: January 2020 – December 2024 (5 years, full cycle including bull 2020–21, bear 2022, recovery 2023–24)
- **Sub-period analysis**: Split into 12-month rolling windows to detect whether edge has decayed over time. This is the most important diagnostic.

### 2.2 Universe
**Tier 1 (primary test):** Top 10 altcoins by market cap excluding stablecoins and wrapped tokens, rebalanced quarterly. Approximate list as of spec date: ETH, BNB, SOL, XRP, ADA, AVAX, DOGE, DOT, LINK, MATIC.

**Tier 2 (secondary test):** Ranks 11–30 by market cap. Expected to show stronger lag but higher slippage cost.

**Exclusions:**
- Stablecoins (USDT, USDC, BUSD, DAI)
- Wrapped tokens (WBTC)
- Any token with average daily volume < $50M USD at time of trade (liquidity filter applied at signal time, not hindsight)

### 2.3 Data Source
- **Primary**: Binance OHLCV 4-hour candles via public REST API (`GET /api/v3/klines`)
- **Backup / cross-validation**: CryptoCompare historical data for pre-Binance-listing assets
- **Volume data**: Binance 4h candle volume in USDT pairs (BTC/USDT, ETH/USDT, etc.)
- **Funding rates** (for cost modeling): Binance perpetual funding rate history (`GET /fapi/v1/fundingRate`)

All timestamps in UTC. Data should be stored locally before backtesting to avoid API rate-limit artifacts.

### 2.4 Backtest Engine Requirements
- Event-driven (not vectorized), to correctly model the sequential signal → entry → exit timing.
- Simulate fills at open of next 4h candle after signal (conservative; no fill-at-signal-candle-close).
- Apply per-trade transaction cost of **0.10% per side** (taker fee on Binance, standard tier).
- Simulate funding cost for perpetual positions: apply realized 8-hour funding rate at each 8h settlement that overlaps with the hold window.
- No look-ahead bias: signal detection uses only candles whose close has completed.

---

## Signal Definition

### 3.1 BTC Trigger Candle
A **BTC Trigger Event** is defined as:

```
abs(BTC_close[t] - BTC_open[t]) / BTC_open[t] >= threshold
```

Where:
- `t` = index of the completed 4h candle
- `threshold` ∈ {0.03, 0.04, 0.05} — test all three as parameters
- Direction is signed: `+` for up-trigger, `−` for down-trigger

**Additional filter (volume confirmation):**
```
BTC_volume[t] >= 1.5 × BTC_volume_20period_SMA[t-1]
```
This excludes low-liquidity weekend moves or thin-book spikes.

### 3.2 Altcoin Lag Condition (Entry Qualifier)
For each altcoin in the universe, at time `t` (after BTC trigger confirmed):

```
alt_candle_return[t] < 0.5 × BTC_candle_return[t]
```

Meaning: the alt has moved less than half of what BTC moved in the same candle. This confirms it is "lagging" rather than having already repriced. If an alt has already moved ≥ 50% of BTC's move, skip it — the catch-up potential is diminished.

---

## Entry Rules

### 4.1 Entry Timing
- Signal is confirmed at **close of BTC trigger candle** (time `t`).
- Entry is executed at the **open of the next 4h candle** (time `t+1`), approximately the next 4 hours.
- This models realistic execution: trader/algorithm receives close price, places market order at next open.

### 4.2 Entry Direction
- If BTC trigger is **bullish** (+3%+): go **LONG** the qualifying altcoin.
- If BTC trigger is **bearish** (−3%+): go **SHORT** the qualifying altcoin (via Binance perpetuals).

### 4.3 Multi-Alt Handling
- If multiple alts qualify simultaneously, enter all qualifying alts from Tier 1 (up to 5 concurrent positions).
- Allocate position sizing equally across qualifying alts (see Section 6).
- If already holding a position in an alt from a prior trigger, **do not add** to it (no pyramiding).

---

## Exit Rules

### 5.1 Primary Exit: Catch-Up Target
Exit when the altcoin's cumulative return since entry reaches **80% of BTC's trigger candle return**.

```
alt_cumulative_return_since_entry >= 0.80 × BTC_trigger_candle_return
```

This is the "catch-up achieved" condition. Check at the close of each 4h candle after entry.

### 5.2 Time-Based Exit (Hard Stop)
If catch-up target is not reached within **3 candles (12 hours)** of entry, exit at the **open of candle t+4**.

Rationale: The lag hypothesis asserts 2–12h catch-up. If it hasn't happened within the window, the trade is invalidated. Staying in longer exposes to unrelated alt-specific risk.

Test variants: 2-candle (8h), 3-candle (12h), 4-candle (16h) exit windows.

### 5.3 Stop Loss
Hard stop at **−2.0% from entry price**, checked at candle close. Exit at open of the following candle.

Rationale: A −2% adverse move suggests either (a) the alt is diverging from BTC's signal rather than lagging, or (b) the move has reversed. Risk/reward minimum at 3%+ target vs. 2% stop is approximately 1.5:1 before fees.

### 5.4 Invalidation Exit (BTC Reversal)
If BTC itself reverses more than **2% in the opposite direction** from the original trigger during the hold window, exit all open positions on the next candle open.

Rationale: The catch-up trade is predicated on BTC holding its new level. A reversal invalidates the premise.

### 5.5 Exit Priority (in order)
1. Stop loss triggered → exit next open
2. BTC reversal invalidation → exit next open
3. Catch-up target hit at candle close → exit next open
4. Time stop (3 candles) → exit next open

---

## Position Sizing

### 6.1 Base Model
Risk a fixed **1% of portfolio equity per trade** (risk-based sizing).

```
Position Size = (Portfolio Equity × 0.01) / Stop Distance in %
```

Where Stop Distance = 0.02 (2% stop loss).

```
Position Size = Portfolio Equity × 0.50
```

(i.e., up to 50% of equity per trade at face value, but in practice constrained by the multi-position cap below)

### 6.2 Multi-Position Cap
Maximum concurrent exposure: **5 simultaneous positions**.

Per-position maximum allocation: **15% of portfolio equity** (hard cap, regardless of formula output).

Maximum gross notional exposure: **75% of equity** at any time.

### 6.3 Leverage
Backtest with **1× (spot equivalent)** as the base case.

Secondary test at **2× leverage** (modest leverage via perpetuals) to assess fee/funding drag vs. return amplification.

Do **not** test above 3× during hypothesis validation.

### 6.4 Drawdown Scaling
If portfolio equity drawdown from peak exceeds **10%**, reduce per-trade risk to **0.5%** until equity recovers to 5% below peak.

If drawdown exceeds **20%**, halt new entries (see Kill Criteria).

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

| Criterion | Threshold | Rationale |
|---|---|---|
| Total trades (backtest) | ≥ 200 | Sufficient statistical power |
| Sharpe Ratio (annualized) | ≥ 1.0 | Risk-adjusted return meaningful |
| Win rate | ≥ 45% | Combined with R:R, expect profitability |
| Average profit/loss ratio | ≥ 1.5:1 | Must cover fees and slippage |
| Max drawdown | ≤ 20% | Acceptable operational risk |
| Profitable in ≥ 3 of 5 annual sub-periods | Yes | Not a single-regime artifact |
| Edge present in BOTH bull and bear regimes | Yes | Not purely directional beta |
| 2023–2024 sub-period Sharpe | ≥ 0.5 | Edge has not fully decayed in recent data |
| Slippage sensitivity test passing | Yes | Results hold at 0.20% per side (2× base) |

If the 2023–2024 sub-period shows Sharpe < 0.5 but earlier periods are strong, the strategy is classified as **DECAYED** and does not go live.

---

## Kill Criteria

Halt live trading and return to research if any of the following occur:

| Trigger | Action |
|---|---|
| Live drawdown from peak > 15% | Halt new entries, review |
| Live drawdown from peak > 20% | Full halt, kill all positions |
| 20 consecutive losing trades | Halt, structural review |
| Rolling 3-month Sharpe (live) < 0 | Halt, flag for parameter re-estimation |
| Average realized catch-up time > 24h (30-trade rolling) | Signal lag window has expanded; review hypothesis |
| BTC/alt correlation structure breaks (rolling 30d corr < 0.40 for ≥ 3 alts) | Universe review; may need rebalancing |
| Exchange API downtime / fill uncertainty event | Halt until audit complete |

Kill criteria are **non-negotiable**. No overrides without a documented post-mortem and updated spec version.

---

## Risks

### 9.1 Edge Decay Risk (HIGH — Primary Concern)
The lag may have been largely arb'd away by 2022–2024 as:
- Market makers deploy cross-asset delta hedging
- Copy-trading bots replicate BTC moves to alts in milliseconds
- Crypto-native funds run this exact strategy, compressing returns

**Mitigation**: Sub-period analysis in backtest is mandatory. Treat 2023–2024 results as the most forward-looking signal.

### 9.2 Regime Risk
The strategy is implicitly long volatility (requires sharp BTC moves). In low-volatility, sideways regimes, signal frequency drops and fee drag becomes a higher percentage of P&L.

**Mitigation**: Backtest should report signals-per-month by year. Below 4 signals/month, the strategy may not cover fixed operational costs.

### 9.3 Correlation Breakdown Risk
During extreme market stress (exchange failures, regulatory shocks), BTC and alt correlations can spike to 1.0 or collapse. The lag structure may be non-stationary.

**Example**: FTX collapse (Nov 2022) saw alts gap down before BTC in some cases. The mechanism temporarily reversed.

**Mitigation**: BTC reversal invalidation rule (Section 5.4) and the 12h time stop limit exposure.

### 9.4 Liquidity / Slippage Risk
Large-cap alts (ETH, BNB) will have acceptable slippage. Mid-cap alts (DOT, MATIC) can have 0.15–0.30% market impact at $100K+ sizes.

**Mitigation**: Apply 2× fee sensitivity test as a go-live criterion. Size position based on ADV: do not exceed **0.5% of the alt's 24h average volume** per trade.

### 9.5 Funding Rate Risk (Shorts)
On bear-trigger trades, short perpetual positions incur funding. In bear markets, funding can be negative (profitable for shorts) but in recovering markets, shorts pay funding costs of 0.01–0.05% per 8 hours.

**Mitigation**: Model funding explicitly in the backtest. If the short-side of the strategy is unprofitable after funding, restrict to long-only.

### 9.6 False Signal / Wash Move Risk
A BTC move of exactly 3.5% on thin volume (e.g., Sunday 4 AM UTC) may not represent genuine market repricing. The alt "lag" in this case is simply correct pricing, not underreaction.

**Mitigation**: Volume confirmation filter (1.5× SMA, Section 3.1). Test whether removing this filter materially worsens results.

### 9.7 Overfitting Risk
Three threshold parameters (3/4/5%), two exit windows (8/12/16h), and a stop level (2%) constitute a moderate parameter space. With 200 trades, overfitting is possible.

**Mitigation**: Select parameters using 2020–2022 data only (in-sample). Validate on 2023–2024 (out-of-sample). Do not optimize on full backtest period.

---

## Data Sources

| Data | Source | Endpoint / Notes |
|---|---|---|
| BTC 4h OHLCV | Binance | `GET /api/v3/klines?symbol=BTCUSDT&interval=4h` |
| Alt 4h OHLCV | Binance | Same endpoint, per-symbol |
| Perpetual funding rates | Binance Futures | `GET /fapi/v1/fundingRate` — history available from 2020 |
| Market cap rankings (universe selection) | CoinGecko | Historical snapshots via `/coins/markets` with `date` param |
| 24h volume (liquidity filter) | Binance | From OHLCV candle volume column |
| Cross-validation OHLCV | CryptoCompare | Free tier,
