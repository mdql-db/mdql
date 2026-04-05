---
title: "Inverted Funding Term Structure Basis Exit Signal"
status: HYPOTHESIS
mechanism: 5
implementation: 7
safety: 5
frequency: 5
composite: 875
categories:
  - funding-rates
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a perpetual futures funding rate flips from sustained positive to negative, the carry trade that made leveraged long + short-spot basis positions profitable is mechanically reversed. Participants who entered the basis trade (long spot, short perp) to earn positive funding now face a carry drain — they are paying funding instead of receiving it. This creates a forced unwind: spot is sold, perp shorts are closed (perp bought). The net effect is perp outperformance vs. spot in the immediate post-flip window, and a mean-reversion of funding back toward zero as the leveraged long base is flushed.

**Causal chain:**

1. Funding rate is positive → basis traders enter: buy spot, short perp, earn funding yield
2. Funding flips negative (longs pay shorts) → basis trade carry inverts, now cash-negative
3. Rational basis traders exit: sell spot + buy back perp short simultaneously
4. Spot sell pressure + perp buy pressure → perp premium over spot compresses or inverts
5. Funding rate mean-reverts toward zero as net long OI on perp decreases
6. A trader who is long perp at the moment of flip earns the negative funding paid by remaining longs AND benefits from the perp/spot basis compression

**What this is NOT:** A prediction that price goes up or down. This is a relative value trade on the perp/spot basis, with a directional funding income component.

---

## Structural Mechanism — WHY This Must Happen

The mechanism has two components, one near-mechanical and one probabilistic:

**Near-mechanical (score: 7):**
Negative funding is a direct cash cost on long perp positions, charged every 8 hours. For any position funded by borrowed capital (e.g., a basis trader using leverage on the spot leg), the P&L equation flips sign. There is no discretion — the funding payment is deducted automatically by the exchange protocol. Basis traders with thin margin cannot sustain indefinite negative carry. The longer funding stays negative, the more forced exits accumulate.

**Probabilistic (score: 5):**
The *timing* of the unwind is not guaranteed. A basis trader with deep pockets can absorb negative funding for days or weeks. The hypothesis is that the *aggregate* of many such traders creates measurable flow within 2–5 days of a regime flip, not that any individual trader is forced out immediately.

**Why the perp outperforms spot during unwind:**
When basis traders close, they execute: SELL spot + BUY perp. This is a simultaneous two-leg trade. The perp buy leg creates direct upward pressure on perp price. The spot sell leg creates downward pressure on spot. The perp/spot spread therefore narrows or inverts. A long perp / short spot position profits from this spread compression.

**Why funding mean-reverts:**
Negative funding persists because net long OI exceeds net short OI on the perp. As longs close (basis unwind + directional longs capitulating), net OI rebalances. Funding is mechanically calculated from the mark/index price spread and OI imbalance — as OI normalises, funding normalises. This is a protocol-level calculation, not a behavioural tendency.

---

## Entry/Exit Rules

### Entry Conditions (ALL must be true)

| Condition | Threshold | Rationale |
|-----------|-----------|-----------|
| Funding rate (current 8h period) | < -0.01% | Negative regime confirmed |
| Prior 7-day average funding | > +0.005% per 8h | Confirms regime flip, not continuation of bear |
| Consecutive negative periods | ≥ 2 (i.e., 16h sustained) | Filters noise, confirms flip is holding |
| 24h spot volume | > $50M (for the asset) | Ensures sufficient liquidity for basis trade |
| Perp OI | Not declining >30% in prior 24h | Avoids entering mid-capitulation flush |

### Entry Execution

- **Leg 1:** Long perp at market open of the 3rd consecutive negative funding period
- **Leg 2 (optional, for full basis trade):** Short spot on a CEX simultaneously
- If running perp-only (no spot hedge): treat as a directional + funding income trade, size accordingly (see Position Sizing)

### Exit Conditions (FIRST trigger wins)

| Exit Trigger | Action |
|--------------|--------|
| Funding returns to ≥ +0.005% for 2 consecutive 8h periods | Close full position — carry has normalised |
| 5-day (15 × 8h periods) max hold | Close regardless — thesis window expired |
| Funding goes more negative than -0.1% per 8h | Close — extreme negative funding signals structural bear, not basis unwind |
| Perp OI drops >40% from entry | Close — mass liquidation event, not orderly unwind |
| Spot price drops >8% from entry (perp-only leg) | Stop loss — directional risk override |

### Exit Execution

- Close perp leg at market on the 8h period open following trigger
- If running spot short leg: close simultaneously with perp
- Do not leg out — close both legs within the same 15-minute window

---

## Position Sizing

**Base position size:** 2% of portfolio NAV per trade (perp leg)

**Scaling by funding magnitude:**

| Funding Rate (per 8h) | Position Multiplier | Max Position |
|-----------------------|--------------------|----|
| -0.01% to -0.02% | 1.0× | 2% NAV |
| -0.02% to -0.05% | 1.5× | 3% NAV |
| -0.05% to -0.10% | 2.0× | 4% NAV |
| > -0.10% | 0.5× (reduce) | 1% NAV |

**Rationale for reduction at extreme negative:** Rates above -0.10% per 8h signal a structural bear or a market in distress — the unwind thesis breaks down and directional risk dominates.

**Leverage:** Maximum 3× on the perp leg. The funding income is the edge, not leverage amplification.

**Spot short leg sizing (if used):** Equal notional to perp long. This creates a delta-neutral basis trade. If spot short is not available or too expensive to borrow, run perp-only at 50% of normal size.

**Concurrent positions:** Maximum 3 simultaneous positions across different assets. Correlation check: do not hold BTC and ETH basis trades simultaneously — they are too correlated to count as independent.

---

## Backtest Methodology

### Data Sources

| Data | Source | Endpoint/URL |
|------|--------|--------------|
| Hyperliquid funding rates (historical) | Hyperliquid API | `https://api.hyperliquid.xyz/info` — `fundingHistory` endpoint |
| Hyperliquid perp OHLCV | Hyperliquid API | `https://api.hyperliquid.xyz/info` — `candleSnapshot` endpoint |
| Spot price (BTC, ETH, SOL, etc.) | Binance public API | `https://api.binance.com/api/v3/klines` |
| Spot price (backup) | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| OI data | Coinglass | `https://open-api.coinglass.com/public/v2/open_interest` |

### Backtest Universe

- **Assets:** BTC, ETH, SOL, ARB, DOGE — prioritise assets with >6 months of Hyperliquid funding history
- **Period:** Full Hyperliquid history (launch ~Nov 2023 to present)
- **Frequency:** 8-hour bars aligned to funding settlement (00:00, 08:00, 16:00 UTC)

### Backtest Steps

1. **Identify all regime flip events:** Scan funding history for transitions from ≥7-day positive average to 2+ consecutive negative periods. Log each event with timestamp, asset, entry funding rate, and prior 7-day average.

2. **Simulate entry:** At the open of the 3rd consecutive negative 8h period, record entry price (perp and spot).

3. **Simulate exit:** Apply exit rules in priority order. Record exit price, hold duration, and exit trigger type.

4. **Calculate P&L per trade:**
   - Funding income: sum of funding payments received during hold (negative funding × position size, paid to shorts)
   - Price P&L: (exit perp price − entry perp price) / entry perp price × position size
   - Spot leg P&L (if modelled): (entry spot price − exit spot price) / entry spot price × position size
   - Transaction costs: assume 0.035% taker fee per leg (Hyperliquid standard)

5. **Aggregate metrics to compute:**
   - Win rate (% of trades where total P&L > 0)
   - Average hold duration
   - Average funding income per trade (as % of notional)
   - Average price P&L per trade
   - Sharpe ratio (annualised, using 8h P&L series)
   - Maximum drawdown
   - Profit factor (gross profit / gross loss)
   - Breakdown by exit trigger type (how often does each exit fire?)

### Baseline Comparison

Compare against two baselines:
1. **Naive long perp hold:** Buy and hold perp for the same 5-day window regardless of funding signal
2. **Random entry control:** Enter at random 8h periods (same asset, same hold duration) — tests whether the funding flip signal adds value vs. random entry

### What to Look For

- Funding income should be consistently positive (it is near-mechanical — if it isn't, data pipeline is broken)
- Price P&L should be positive on average but will have high variance — this is the probabilistic component
- The regime flip filter (7-day positive prior) should show better outcomes than entering on any negative funding period

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

| Criterion | Minimum Threshold |
|-----------|------------------|
| Number of qualifying events | ≥ 30 across all assets |
| Win rate | ≥ 55% |
| Average trade P&L (net of fees) | > +0.3% per trade |
| Sharpe ratio (annualised) | > 1.0 |
| Maximum drawdown | < 15% of strategy NAV |
| Profit factor | > 1.3 |
| Funding income component | Positive in ≥ 85% of trades |
| Outperformance vs. random entry baseline | Statistically significant (p < 0.10) |

**If fewer than 30 qualifying events exist in Hyperliquid history:** Extend backtest to Binance perpetuals funding data (available from 2020) using the same entry/exit logic. Note that Binance funding mechanics differ slightly (8h fixed vs. Hyperliquid's variable) — flag this in the backtest report.

---

## Kill Criteria

Abandon the strategy (do not proceed to live trading) if ANY of the following are true:

| Kill Condition | Reason |
|----------------|--------|
| Win rate < 50% in backtest | Signal has no edge over coin flip |
| Funding income is negative in >20% of trades | Data or logic error — funding income is near-mechanical |
| Average hold duration consistently hits 5-day max | Thesis window is wrong; funding doesn't mean-revert in time |
| Backtest P&L is not statistically different from random entry (p > 0.20) | No signal in the regime flip filter |
| Negative funding periods cluster only in 2022 bear market | Edge is regime-specific, not structural |
| Paper trading Sharpe < 0.5 after 60 days | Live execution is degrading the edge |

---

## Risks — Honest Assessment

**1. Negative funding can persist for weeks in bear markets (HIGH risk)**
In 2022, BTC funding was negative for 30+ consecutive days. The 5-day max hold exit protects against this, but the strategy will take repeated small losses during sustained bear regimes. Mitigation: the 7-day positive prior filter should exclude entries during established bear trends, but it is not foolproof.

**2. Basis traders may not exist in sufficient size on Hyperliquid (MEDIUM risk)**
Hyperliquid is a newer venue. If the basis trade community is small, the mechanical unwind flow may be too small to move prices. The $50M daily volume filter partially addresses this, but Hyperliquid spot volumes are lower than Binance — the spot leg of the basis trade may be executed on Binance, not Hyperliquid, making the perp/spot spread harder to capture cleanly.

**3. Perp-only version carries directional risk (HIGH risk)**
If running without the spot short hedge, a funding flip that coincides with a genuine price crash will produce losses on the perp leg that exceed funding income. The 8% stop loss is essential and must be enforced.

**4. Funding rate calculation differences across venues (LOW-MEDIUM risk)**
Hyperliquid uses a variable funding rate mechanism that differs from Binance's fixed 8h settlement. Cross-venue backtesting (if needed to get 30+ events) introduces basis risk in the backtest itself. Flag all cross-venue results separately.

**5. Crowding (LOW risk currently, monitor)**
This is not a widely-known systematic strategy on Hyperliquid. However, if it works and is published, the edge will compress. Monitor entry funding rates over time — if the strategy stops working at -0.01% but still works at -0.03%, crowding at the margin is occurring.

**6. Execution slippage on simultaneous two-leg entry (MEDIUM risk)**
The basis trade requires near-simultaneous execution on two venues (Hyperliquid perp + CEX spot). Manual execution introduces leg risk. Automate or accept that the perp-only version is the practical implementation for now.

---

## Data Sources — Summary

| Resource | URL | Notes |
|----------|-----|-------|
| Hyperliquid API docs | `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api` | Full API reference |
| Hyperliquid funding history | `POST https://api.hyperliquid.xyz/info` body: `{"type": "fundingHistory", "coin": "BTC", "startTime": <ms>}` | Returns 8h funding rate history |
| Hyperliquid candles | `POST https://api.hyperliquid.xyz/info` body: `{"type": "candleSnapshot", "req": {"coin": "BTC", "interval": "8h", ...}}` | OHLCV aligned to funding periods |
| Binance spot klines | `GET https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=8h` | Free, no auth required |
| Binance perp funding (backup) | `GET https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT` | Historical funding, free |
| Coinglass OI | `https://open-api.coinglass.com/public/v2/open_interest` | Requires free API key |
| Coinglass funding heatmap | `https://www.coinglass.com/funding` | Manual reference for regime identification |

---

## Next Steps

1. Pull Hyperliquid funding history for BTC, ETH, SOL via API — count qualifying regime flip events
2. If <30 events: extend to Binance perp funding history from 2020, note venue difference
3. Build event study: plot perp/spot spread and funding rate for ±10 days around each flip event (visual sanity check before full backtest)
4. Code backtest with the exact entry/exit rules above
5. Run sensitivity analysis on the "7-day positive prior" and "2 consecutive negative periods" parameters — test ±1 day and ±1 period to check robustness
6. Report results against go-live criteria — proceed to paper trading or kill
