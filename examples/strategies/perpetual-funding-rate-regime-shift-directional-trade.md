---
title: "Funding Rate Regime Crossover — Directional Trade"
status: HYPOTHESIS
mechanism: 3
implementation: 8
safety: 6
frequency: 5
composite: 720
categories:
  - funding-rates
  - calendar-seasonal
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When crypto perpetual futures funding rates cross zero and sustain in the new direction for two consecutive 8-hour periods, the resulting 48-hour price move has positive expectation in the direction of the new funding regime.

This strategy trades the *structural regime crossover* — the moment the aggregate market flips from net long to net short positioning (or vice versa) — as a directional signal. It is distinct from:

- **Funding rate fade (existing pipeline):** Fades *extreme* single-period funding spikes as mean reversion plays
- **Catalogue 4.4 — Funding Rate Trend Riding:** Enters into an *existing* funding trend; requires funding already to be moving in one direction
- **This strategy:** Trades the *transition event itself* — the zero-crossing confirmed over two periods — as the trigger

---

## Why it's an edge

Perpetual funding rates are not sentiment proxies. They are mechanically derived from the gap between the perpetual price and the spot index price. A positive funding rate means longs are paying shorts; a negative rate means shorts are paying longs. The sign of the funding rate therefore encodes which side of the market is net overweight and is paying a premium to hold that position.

A funding regime crossover — positive to negative, or negative to positive — reflects a genuine structural repositioning of the aggregate market:

**Positive → Negative:**
Net longs have closed or been liquidated. The market is now net short. Remaining shorts are paying to hold, meaning they have conviction sufficient to bear a recurring cost. The prior long bias that supported prices has been removed. Price has structural headwind.

**Negative → Positive:**
Net shorts have covered. New net longs are entering and paying a premium to hold exposure. Fresh buying conviction is entering the market. Price has structural tailwind.

The key distinguishing claim is that the *crossover* captures a structural flip in positioning, not merely a sentiment spike. Requiring confirmation over two consecutive 8-hour periods (16 hours total) filters noise and ensures the signal reflects a sustained repositioning rather than a transient liquidity event.

This is a hypothesis — the causal chain (funding regime → price continuation) is not guaranteed. Funding rates lag price to some degree. The backtest will determine whether the forward price return after crossover confirmation has positive expectation, or whether the move has already occurred before the signal fires.

---

## Backtest Methodology

### Data

| Dataset | Source | Parameters |
|---|---|---|
| 8-hour funding rate history | Binance Futures API `/fapi/v1/fundingRate` | BTC, ETH, SOL; 2020–2024; all available history |
| Perpetual OHLCV (8-hour bars) | Binance Futures API `/fapi/v1/klines` | Same assets, aligned timestamps |
| Spot OHLCV for reference | Binance Spot API `/api/v3/klines` | Same assets, same period |

Use Binance data for the backtest (longer history, more crossover events). Execution will move to Hyperliquid; funding rates are broadly correlated across major exchanges for BTC and ETH, but note divergences as a caveat.

### Sample Size Estimate

Funding regime crossovers are relatively infrequent. On BTC perps, funding changes sign approximately 2–4 times per month on average, but many of these are single-period reversals that will be filtered by the 2-period confirmation requirement. Expected confirmed crossover events after filtering:

- BTC: ~1–2 per month × 48 months (2020–2024) ≈ 50–100 events
- ETH: similar
- SOL: fewer (shorter history, more volatile funding)

Total backtest sample: approximately 150–250 events across the three assets. Sufficient for preliminary hypothesis testing; not sufficient for high-confidence production deployment without additional validation.

### Crossover Detection Algorithm

```
For each 8-hour period t:
  funding_sign[t] = sign(funding_rate[t])
  
  crossover_confirmed[t] = (
    funding_sign[t-2] == OLD_SIGN
    AND funding_sign[t-1] == NEW_SIGN
    AND funding_sign[t] == NEW_SIGN
  )
  
  If crossover_confirmed[t]:
    signal = direction of NEW_SIGN
      (NEW_SIGN > 0 → LONG signal)
      (NEW_SIGN < 0 → SHORT signal)
    entry_time = open of period t+1
```

Where:
- `OLD_SIGN` = sign of funding in the period immediately before the first NEW_SIGN period
- Two consecutive periods of NEW_SIGN required before trigger fires
- Entry is at the open of the *next* period after confirmation (period t+1), not at the confirmation close

This means total latency from the first zero-crossing to entry = 24 hours (2 confirmation periods × 8 hours, plus entering at the next open). This is intentionally conservative to avoid noise at the exact crossover point.

### Metrics to Compute

**Primary:**
- Mean 48-hour forward return in the signal direction (vs. unconditional 48-hour return as baseline)
- Win rate (percentage of trades where price moved in signal direction by close)
- Mean R (average return divided by stop loss distance, using -8% stop)
- Sharpe ratio of the signal's return stream

**Secondary:**
- Return distribution by asset (BTC vs ETH vs SOL)
- Return by funding regime entered (long vs short signals separately)
- Decay analysis: does the signal edge concentrate in hours 0–24, 24–48, or is it uniform?
- Performance in bull vs bear market regimes (2020–2021 bull; 2022 bear; 2023 recovery; 2024 bull)

**Baseline comparisons:**
1. Unconditional 48-hour returns on the same assets (no signal)
2. Random 48-hour entry on the same days as the signal fires (same-day random entry)
3. Entering on *single-period* funding crossovers (no 2-period confirmation) — tests whether the confirmation filter adds value

### Validity Threshold for Proceeding to Paper Trading

The backtest must show:
- Mean signal-direction return > 1.5% over 48 hours (before fees and funding costs)
- Win rate > 52% (better than coin flip by a meaningful margin)
- Performance does not concentrate exclusively in one regime period (not a 2021 bull market artifact)
- Confirmation filter version outperforms single-period version (confirms the filter is adding value, not just reducing sample size)

If any of these are not met, kill or redesign before paper trading.

---

## Entry Rules


### Universe

BTC, ETH, SOL perpetuals on Hyperliquid. These are the three most liquid perps with reliable, continuous funding rate data and tight bid-ask spreads on Hyperliquid.

Do not expand to smaller-cap assets in the paper trading phase. Funding rates on illiquid perps are noisier and more susceptible to manipulation.

### Signal Detection

1. Pull the last 3 funding rate periods (24 hours of history) for each asset every 8 hours, immediately after each funding settlement
2. Check for crossover confirmation:
   - Period t-2: sign A
   - Period t-1: sign B (B ≠ A)
   - Period t: sign B (second consecutive period of new sign)
   - → Confirmed crossover toward sign B
3. If confirmed: queue entry at the open of period t+1

### Entry

- **Timing:** Market order at the open of the next 8-hour period following confirmation
- **Direction:** Long if new regime is positive funding; Short if new regime is negative funding
- **Leverage:** 2x maximum
- **Position size:** See sizing section below
- **Slippage budget:** Assume 0.05% slippage on entry for BTC/ETH; 0.10% for SOL

## Exit Rules

### Exits (in priority order)

1. **Signal invalidation exit:** If funding crosses zero again in the opposite direction and holds for one period (does not require 2-period confirmation — one period of reversal is enough to invalidate the current signal), close immediately at the next period open. Rationale: the underlying regime signal has reversed; holding is no longer justified.

2. **Stop loss:** Close if mark price moves -8% against the position from entry. This is a hard stop, executed as a limit order 8% below (long) or above (short) entry, set immediately upon entry.

3. **Time-based exit:** Close at the open of the 7th period after entry (48 hours after entry) if neither of the above has triggered. This is the primary expected exit path.

4. **Conflict exit (optional):** If a large token unlock event (Strategy 001) fires for the same asset during the holding period and contradicts the current direction (e.g., Strategy 002 is long, Strategy 001 signals short), close the Strategy 002 position early. This avoids correlated conflicting exposure.

### Filters (Do Not Enter If)

- A large scheduled unlock (≥2% supply) for the same asset occurs within 5 days
- The asset has experienced a price move of ≥15% in the preceding 24 hours (funding crossover may be a lagging artifact of a whipsaw, not a genuine repositioning)
- There is an active position already open on this asset from Strategy 001

---

## Position Sizing

**Paper trading phase:** $300 notional per trade at 2x leverage ($150 margin per trade).

**Rationale for 2x:** The edge here is a medium-frequency regime signal, not a high-conviction single event. Lower leverage limits ruin risk while preserving measurability. The -8% stop loss at 2x leverage means maximum loss per trade is approximately 16% of margin ($24 on a $150 margin position), or ~8% of notional.

**Maximum concurrent positions:** 2 (one per asset, but not all three simultaneously). If signals fire on multiple assets within the same period, take the signal on the asset with the most decisive funding crossover (largest absolute funding rate in the new direction on the confirmation period). Queue the others and enter only if the first position closes before the next signal expires.

**Capital allocation:** In live trading, size this strategy at no more than 20% of allocated strategy capital until it has 15+ completed trades with positive net P&L. The uncertainty in the causal mechanism warrants conservative sizing relative to Strategy 001.

---

## Go-Live Criteria

Deploy real capital when ALL of the following are satisfied:

1. Backtest meets validity thresholds defined above (mean 48h return > 1.5%, win rate > 52%, not regime-concentrated)
2. Minimum 10 paper trades closed (not just opened)
3. Net P&L across paper trades is positive after:
   - Trading fees (0.09% round-trip)
   - Funding costs accrued during holding period
   - Estimated slippage
4. No single paper trade lost more than 8% of notional (consistent with stop loss functioning correctly)
5. At least one signal has fired in both directions (long and short) during paper trading — a strategy that has only been tested in one direction is not ready for live deployment
6. Founder reviews and approves live capital deployment

---

## Kill Criteria

**Kill during backtest (before paper trading):**
- Mean signal-direction return ≤ 1.5% over 48 hours
- Win rate ≤ 52%
- Performance explained entirely by one market regime period
- Confirmation filter does not outperform single-period version
- Any of these: kill or fundamentally redesign before proceeding

**Kill during paper trading:**
- After 10 closed paper trades: net P&L negative after all costs → kill
- After 15 closed paper trades: edge (mean return in signal direction) < 1.0% after all costs → kill
- Any time: three consecutive stop-loss hits → pause and investigate regime dependency before resuming
- Any time: funding data source becomes unreliable or methodology changes at the exchange level

**Kill conditions that trigger immediate review (not automatic kill):**
- A major market regime shift (e.g., prolonged near-zero volatility) that produces no crossover events for 60+ days → strategy is inactive, not broken; flag for review
- Funding rates on Hyperliquid diverge persistently from Binance (>50bps difference at crossover events) → reassess whether backtest-derived signal applies to the live execution venue

---

## Risks

### 1. Funding lags price — the move may already be in
**Nature:** The primary causal risk. Funding rates respond to the perp/spot spread, which is itself driven by price action. By the time funding regime crosses zero with two-period confirmation, 16–24 hours of the directional move may have already occurred. The remaining forward return may be zero or negative.

**Mitigation:** The backtest will directly test this. If the edge concentrates entirely in hours -24 to 0 (before entry), the strategy is uninvestable. If there is genuine forward edge in hours 0–48, the lag is acceptable.

**Severity:** High if confirmed by backtest. This is the thesis-killer risk.

### 2. Crowded signal — front-running of the crossover
**Nature:** Funding rates are public and widely monitored. Sophisticated traders may enter positions in anticipation of the confirmed crossover, front-running the signal and compressing the post-entry return.

**Mitigation:** The 2-period confirmation requirement is itself a form of intentional delay. If crowding has eliminated the edge, the backtest will show it. Additionally, Zunid is not competing on speed here — the edge, if real, comes from systematic execution, not latency.

**Severity:** Medium. Likely partially priced in, but whether it's fully priced in is an empirical question.

### 3. Exchange-specific funding distortion
**Nature:** Funding rates differ between Hyperliquid and Binance/Bybit/OKX. A confirmed negative-to-positive crossover on Hyperliquid may not correspond to the same event on other venues, which represents a different balance of longs and shorts. The Hyperliquid funding rate encodes Hyperliquid-specific positioning, not the global market.

**Mitigation:** Run the backtest on Hyperliquid funding data specifically (available via API), not Binance. Accept that the signal reflects Hyperliquid microstructure. If Hyperliquid data history is too short for a reliable backtest, use Binance as a proxy but note this as a caveat.

**Severity:** Medium. Worth validating whether Hyperliquid and Binance crossovers are correlated. If they are (likely for BTC/ETH), this risk is lower.

### 4. Funding manipulation by large traders
**Nature:** A sufficiently large trader can temporarily push funding rates across zero by opening a large directional position and then closing it after triggering algorithmic followers. Two-period confirmation reduces but does not eliminate this.

**Mitigation:** The 2-period filter requires 16 hours of sustained new-regime funding, which makes brief manipulation more costly. Additionally, requiring the price filter (no entry after ≥15% move in 24h) reduces the risk of entering into a manipulated post-pump environment.

**Severity:** Low to medium for BTC/ETH (too liquid to manipulate at scale); higher for SOL (assess in backtest).

### 5. Funding costs during holding period
**Nature:** If the strategy goes long during positive funding, the position accrues funding costs (longs pay when funding is positive). This creates a headwind for the holding period. A 48-hour long during a sustained positive funding regime could cost 0.10–0.30% in funding payments, reducing net return.

**Mitigation:** Track funding costs explicitly in the backtest and paper trading P&L. Size the required gross return accordingly (the 1.5% threshold already provides buffer). In extreme funding environments (>0.10% per period), consider reducing hold time to 24 hours.

**Severity:** Low to medium. Manageable at 2x leverage with 48-hour max hold.

### 6. Small backtest sample
**Nature:** Even with 4 years of BTC history, the 2-period confirmation requirement may produce only 50–80 confirmed crossover events per asset. This is a thin sample for statistical inference. The backtest results may be overfit to a specific market period.

**Mitigation:** Do not proceed to live capital deployment based on backtest alone. The paper trading requirement (10+ closed trades) provides out-of-sample validation. Additionally, segment the backtest by year and verify the edge is not concentrated in one period.

**Severity:** Medium. Inherent limitation. Honest disclosure of sample size in any review.

---

## Data Sources

| Data | Source | Endpoint / Access |
|---|---|---|
| Historical funding rates (backtest) | Binance Futures API | `GET /fapi/v1/fundingRate?symbol=BTCUSDT&limit=1000` |
| Historical funding rates (live
