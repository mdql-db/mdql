---
title: "Post-Funding-Spike OI Collapse Momentum"
status: HYPOTHESIS
mechanism: 5
implementation: 7
safety: 5
frequency: 7
composite: 1225
categories:
  - funding-rates
  - liquidation
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When perpetual futures funding rates spike to extreme levels (>3x the 30-day rolling average AND >0.1%/8h absolute), a subset of leveraged positions become mathematically cash-flow-negative beyond their margin buffer tolerance. These holders face a binary forced choice: voluntarily close or get liquidated. The resulting OI collapse is not behavioural — it is the arithmetic consequence of carry cost exceeding marginal position profitability. As leveraged longs (or shorts) are flushed, the artificial demand (or supply) they represented is removed, and price reverts toward the index/spot price.

**Causal chain:**

1. Funding rate spikes → carry cost per 8h period increases sharply
2. Marginal holders (thin margin buffers, near break-even positions) become cash-flow-negative on a per-period basis
3. These holders must close or face liquidation within 1–3 funding periods
4. OI drops as positions close → artificial directional pressure unwinds
5. Price reverts toward spot/index as the crowded side's demand/supply is removed
6. Funding rate normalises as the imbalance clears

**Key distinction from behavioural hypothesis:** The flush is not "people tend to close" — it is "at some funding rate, the marginal holder's P&L math forces closure." The uncertainty is *when* the marginal holder hits their tolerance threshold, not *whether* they eventually will.

---

## Structural Mechanism

### Why positions MUST close

A leveraged long on a perpetual future pays funding every 8 hours. For a position to remain open, the holder must either:

- (a) Believe the expected price appreciation exceeds the carry cost, OR
- (b) Have sufficient margin buffer to absorb the carry cost while waiting

At 0.1%/8h funding, the annualised carry cost is approximately **109% APR**. At 0.3%/8h (a common spike level), it is **328% APR**. No rational holder with a thin margin buffer can sustain this indefinitely. The marginal holder — defined as the holder with the smallest expected-return-to-carry-cost ratio — is forced out first.

### Why OI is the confirmation signal

OI at a 30-day high combined with extreme funding means:
- The crowded side is maximally extended
- New entrants are still arriving (momentum), but the *existing* marginal holders are already underwater on carry
- The OI high is the "peak crowding" marker — the point at which the flush pool is largest

### Why price reverts

Leveraged longs create synthetic demand that is not backed by spot conviction. When they close (sell the perp), that synthetic demand disappears. The perp price, which was trading at a premium to spot (reflected in positive funding), converges back toward spot as the premium-sustaining demand is removed. This is a mechanical convergence, not a prediction about spot price direction.

### The timing problem (honest)

The mechanism is real but the trigger timing is uncertain. A funding spike can persist if:
- New leveraged entrants continuously replace flushed ones (momentum markets)
- The move is fundamentals-driven and new entrants have high carry tolerance (e.g., well-capitalised funds)

This is why the score is 5/10, not 8/10. The flush is guaranteed *eventually* but the window could be 8h or 72h.

---

## Entry Rules


### Universe
All perpetual futures on Hyperliquid with >$10M average daily OI over the prior 30 days. Exclude stablecoins and synthetic assets. Minimum 30 days of OI + funding history required.

### Entry Conditions (ALL must be true simultaneously)

| Condition | Threshold | Rationale |
|-----------|-----------|-----------|
| Funding rate (current 8h period) | >3x the 30-day rolling average funding rate | Relative spike — filters out assets that structurally carry high funding |
| Funding rate (absolute) | >0.10%/8h | Absolute floor — ensures carry cost is genuinely punishing |
| OI | >30-day rolling high at time of signal | Confirms maximum crowding — flush pool is largest |
| Funding direction | Positive → short the perp; Negative → long the perp | Trade against the crowded side |
| Spot-perp premium | Perp trading >0.3% above spot (for long funding) | Confirms premium exists to revert; skip if premium is absent |

**Entry timing:** Enter at the open of the *next* 8h funding period after all conditions are met. Do not chase mid-period — wait for the period boundary to get a clean funding cost read.

## Exit Rules

### Exit Conditions (first trigger wins)

| Exit | Condition |
|------|-----------|
| **Primary exit (profit)** | OI drops >15% from the entry-day peak OI reading |
| **Secondary exit (normalisation)** | Funding rate falls below 1.5x the 30-day average for two consecutive 8h periods |
| **Hard stop (loss)** | Price moves >5% against the position within the first 8h period after entry |
| **Time stop** | Position held >72h (9 funding periods) with neither profit exit nor stop hit → close at market |

**Exit timing:** Check exit conditions at each 8h funding period boundary. Do not monitor intra-period unless hard stop is being tracked (which requires continuous monitoring or a limit order placed at entry).

### Hard Stop Implementation
Place a limit order (or conditional stop) at entry price ±5% immediately upon entry. This handles the hard stop mechanically without requiring continuous monitoring.

---

## Position Sizing

### Base sizing
- **Risk per trade:** 1% of portfolio NAV
- **Position size:** Risk amount / hard stop distance (5%)
- **Example:** $100,000 portfolio → $1,000 risk → $20,000 notional position

### Adjustments
- **Reduce by 50%** if the asset has had a funding spike in the prior 14 days (repeat spikes indicate structural momentum, not flush dynamics)
- **Reduce by 50%** if the asset is in the top 3 by market cap (BTC, ETH) — these have deeper pockets of new entrants and spikes persist longer
- **Maximum single position:** 3% of portfolio NAV notional
- **Maximum concurrent positions:** 3 (strategy can trigger on multiple assets simultaneously during market-wide leverage events)
- **No leverage beyond 5x** — this is a mean-reversion trade; excess leverage on a reversion that takes 72h to play out is account-threatening

---

## Backtest Methodology

### Data sources
- **Hyperliquid funding rate history:** `https://api.hyperliquid.xyz/info` — endpoint: `fundingHistory` (returns 8h funding rates per asset)
- **Hyperliquid OI history:** Same API — endpoint: `openInterest` snapshots; note: historical OI snapshots may require reconstruction from trade data or third-party aggregators
- **Spot price (for premium calculation):** Hyperliquid spot API or CoinGecko historical OHLCV
- **Supplementary OI data:** Coinglass API (`https://open-api.coinglass.com/public/v2/open_interest`) for cross-exchange OI validation

### Backtest period
- **Primary:** January 2023 – present (covers multiple leverage cycle regimes)
- **Stress test:** November 2021 – June 2022 (sustained momentum bull then crash — tests whether the strategy bleeds in trending markets)

### Assets to test
Start with the 10 highest-OI perpetuals on Hyperliquid. Run each asset independently, then combine into portfolio simulation.

### Signal identification
For each asset, scan the full history and flag every 8h period where:
- Funding > 3x 30-day rolling average
- Funding > 0.10%/8h absolute
- OI > 30-day rolling max

Record: asset, timestamp, funding rate, OI level, spot-perp premium at signal time.

### Outcome measurement per signal
For each flagged signal, record:
- **Max adverse excursion (MAE):** Worst price move against position in first 8h, 24h, 72h
- **Max favourable excursion (MFE):** Best price move in favour in first 8h, 24h, 72h
- **OI at T+8h, T+24h, T+72h:** Did OI drop >15%? When?
- **Funding at T+8h, T+24h, T+72h:** When did it normalise?
- **P&L at each exit trigger:** Which exit fired first? What was the P&L?
- **Carry cost paid:** Sum of funding rates paid during hold period (this is a real cost)

### Key metrics to report
| Metric | Minimum acceptable |
|--------|-------------------|
| Win rate | >45% |
| Average win / average loss ratio | >1.8 |
| Expectancy per trade | >0.5% of notional |
| Max drawdown (portfolio simulation) | <15% |
| Sharpe ratio (annualised) | >0.8 |
| Signal frequency | >2 signals/month across universe |

### Baseline comparison
Compare against a naive strategy: short any asset when funding >0.1%/8h (no OI filter, no relative threshold). The OI filter should demonstrably improve win rate or expectancy vs. the naive version. If it does not, the OI condition adds no value and should be dropped.

### Slippage assumption
Model 0.05% slippage on entry and exit (Hyperliquid taker fee is 0.035% + market impact). Use this as the floor; if the strategy is not profitable after 0.1% round-trip cost, it is not viable.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. **Expectancy positive** after fees and slippage across the full backtest period
2. **Win rate >45%** with average win/loss ratio >1.8 (both required — a high win rate with tiny wins is not acceptable)
3. **OI filter adds value:** Expectancy with OI filter > expectancy without OI filter by >20%
4. **No single asset dominates:** Remove the best-performing asset from the universe — strategy remains profitable on the remaining assets
5. **2022 bear market test:** Strategy does not produce >20% drawdown during the November 2021 – June 2022 period
6. **Minimum signal count:** >30 signals in the backtest period (below this, results are not statistically meaningful)
7. **Carry cost accounted for:** P&L figures must include funding paid during hold period — a position held for 3 funding periods at 0.1%/8h costs 0.3% in carry

**Paper trading duration before live:** Minimum 8 weeks or 10 signals (whichever is longer), with live paper trade results within 1 standard deviation of backtest expectancy.

---

## Kill Criteria

Abandon the strategy (paper or live) if any of the following occur:

| Trigger | Action |
|---------|--------|
| 10 consecutive losing trades | Stop, review — likely regime change |
| Drawdown exceeds 15% of allocated capital | Stop, full review before resuming |
| Win rate drops below 35% over trailing 20 trades | Stop |
| Average hold time exceeds 60h (time stop firing frequently) | Mechanism is not working — flush is not happening in the expected window |
| OI filter shows no predictive value in live paper trading (vs. naive baseline) | Drop OI condition or abandon strategy |
| Funding spikes become structurally persistent (>5 consecutive days of elevated funding on multiple assets) | Regime has changed — strategy is not designed for sustained momentum; pause |

---

## Risks

### Primary risks (honest assessment)

**1. Timing risk (HIGH)**
The flush is guaranteed eventually but could take days. During that time, the position pays funding (if short a positive-funding perp, you *receive* funding — this is actually a tailwind, but if long a negative-funding perp, you pay). More critically, price can move significantly against the position before the flush occurs. The 5% hard stop may be hit before the mechanism plays out.

**2. Momentum continuation (HIGH)**
In strong trending markets, new leveraged entrants continuously replace flushed ones. Funding can spike and hold for days (see BTC in March 2021, November 2021). The OI filter helps (OI at 30-day high means the pool is large) but does not prevent new entrants from sustaining the spike.

**3. Fundamentals-driven moves (MEDIUM)**
If the funding spike is caused by a genuine fundamental catalyst (major protocol upgrade, ETF approval, exchange listing), new entrants have high carry tolerance and the flush may not occur at the expected magnitude. No filter fully screens this out.

**4. Carry cost on losing trades (MEDIUM)**
If short a perp with positive funding, you *receive* funding — this is a tailwind. But if the position is a long (negative funding environment), you pay funding during the hold. At 0.1%/8h for 9 periods (72h time stop), that is 0.9% in carry cost before price movement. This must be modelled explicitly.

**5. OI data quality (MEDIUM)**
Historical OI snapshots on Hyperliquid may have gaps or inconsistencies. The backtest is only as good as the OI data. Validate OI data against Coinglass for cross-reference before trusting backtest results.

**6. Liquidity at exit (LOW-MEDIUM)**
During an OI flush, bid-ask spreads widen. The 0.05% slippage assumption may be optimistic during the exact moments the strategy wants to exit. Model 0.15% slippage on exit in stress scenarios.

**7. Correlation during market-wide events (MEDIUM)**
Funding spikes often occur simultaneously across multiple assets during market-wide leverage events. Maximum 3 concurrent positions cap helps, but all 3 positions may be correlated and all may hit the hard stop simultaneously.

---

## Data Sources

| Data | Source | Endpoint / URL |
|------|--------|----------------|
| Hyperliquid funding rate history | Hyperliquid API | `POST https://api.hyperliquid.xyz/info` body: `{"type": "fundingHistory", "coin": "BTC", "startTime": <unix_ms>}` |
| Hyperliquid OI snapshots | Hyperliquid API | `POST https://api.hyperliquid.xyz/info` body: `{"type": "openInterest"}` — note: point-in-time only; historical reconstruction needed |
| Hyperliquid trade/price history | Hyperliquid API | `POST https://api.hyperliquid.xyz/info` body: `{"type": "candleSnapshot", "req": {"coin": "BTC", "interval": "1h", ...}}` |
| Cross-exchange OI history | Coinglass | `https://open-api.coinglass.com/public/v2/open_interest` (API key required, free tier available) |
| Spot price reference | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` (free, rate-limited) |
| Funding rate cross-reference | Coinglass | `https://open-api.coinglass.com/public/v2/funding` |

**Note on OI history gap:** Hyperliquid's API returns current OI snapshots but historical OI reconstruction requires either (a) storing snapshots in real-time from the API going forward, or (b) using Coinglass historical OI data as a proxy. For the backtest, use Coinglass OI data for assets available there, and flag any asset where OI history is unavailable as untestable. Do not backtest on reconstructed/estimated OI — the signal quality assumption would be invalid.

**Recommended first step:** Pull 90 days of funding rate history for the top 10 Hyperliquid perpetuals and identify all signal dates manually before building automated backtest infrastructure. This manual scan will reveal whether the signal fires at a useful frequency and give a rough sense of outcomes before committing engineering time.
