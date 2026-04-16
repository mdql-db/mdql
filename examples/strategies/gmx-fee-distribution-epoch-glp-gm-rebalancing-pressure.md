---
title: "GMX GM Pool APR Divergence — Liquidity Rotation Capture"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 5
frequency: 3
composite: 360
categories:
  - defi-protocol
  - funding-rates
  - liquidation
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a GMX v2 GM pool's fee APR exceeds the median peer pool APR by ≥2x for 48 consecutive hours, capital will rotate into that pool within the following 3–10 days. This rotation increases the high-APR pool's TVL, which increases its max open interest ceiling, which increases utilization pressure on the underlying asset. Simultaneously, the capital-losing pool sees TVL decline, OI ceiling compression, and potential forced position reductions on its underlying asset.

**Causal chain (long leg):**
1. High-APR GM pool observed (e.g., ARB-USD GM pool at 40% APR vs. median 18%)
2. Rational LPs notice yield differential via GMX stats dashboard or on-chain monitoring
3. LPs migrate liquidity from low-APR pools into high-APR pool over 3–10 days
4. High-APR pool TVL increases → max OI ceiling rises → more trader demand can be absorbed → utilization ratio rises → fee APR compresses back toward median
5. Increased liquidity depth in the pool marginally supports the underlying asset's price on GMX (tighter spreads, lower price impact for longs)

**Causal chain (short leg):**
1. Low-APR pool loses TVL as LPs exit
2. Max OI ceiling on that market decreases
3. If current open interest exceeds new ceiling, GMX protocol forces position size reductions (existing positions cannot be increased; new longs/shorts restricted)
4. OI compression creates directional selling pressure on the underlying asset as traders are forced to reduce or close positions

**Honest caveat:** Step 3 of the short leg (forced OI reduction) is the only hard forcing function in this strategy. Steps 1–2 and the long leg are probabilistic, not guaranteed. This is why the score is 5/10, not 7+.

---

## Structural Mechanism — WHY This Happens

### What is mechanically guaranteed:
- GMX v2 GM pools have a protocol-enforced `maxOpenInterest` parameter per market, set as a function of pool TVL (specifically, the pool's long token + short token value). This is a smart contract invariant: `currentOI ≤ maxOI`. Source: GMX v2 contracts on Arbitrum (`MarketUtils.sol`, `getMaxOpenInterest()`).
- If TVL drops and `maxOI` falls below `currentOI`, GMX's keeper system will reject new position increases and may trigger partial liquidations to restore compliance. This is contractually enforced.

### What is structural but probabilistic:
- LP capital rotation in response to APR differentials. LPs are rational actors with gas costs, opportunity costs, and inertia. Rotation happens but timing is variable (days to weeks).
- Fee APR itself is endogenous — it rises when utilization rises and falls as TVL increases. The signal and the outcome are coupled, creating mean-reversion pressure that limits how long the divergence persists.

### What is NOT guaranteed:
- That LPs will rotate at all (some are passive, some are locked in strategies)
- That the high-APR pool's underlying asset will appreciate
- That OI compression on the low-APR pool will cause measurable price impact on spot/perp markets outside GMX

### The honest structural edge:
The only hard edge is the **OI ceiling mechanic on the short leg**: if a GM pool loses enough TVL that `currentOI > maxOI`, the protocol *must* restrict new positions. This is binary and contractually enforced. Everything else is a soft tendency.

---

## Entry Rules


### Signal Detection

**Primary signal (APR divergence):**
- Pull GM pool APRs every 4 hours from GMX v2 stats API
- Calculate: `spread_ratio = max_pool_APR / median_pool_APR`
- Signal fires when `spread_ratio ≥ 2.0` persists for ≥ 48 hours (12 consecutive 4-hour readings)
- Only consider pools with TVL ≥ $5M (filter out illiquid pools where APR spikes are noise)

**Secondary signal (OI ceiling proximity, for short leg only):**
- Calculate: `oi_utilization = currentOI / maxOI` for the low-APR pool
- Short leg only activates if `oi_utilization ≥ 0.80` on the losing pool (i.e., within 20% of ceiling — TVL drop of 20% would trigger forced OI compression)

### Long Leg Entry
- **Instrument:** Perpetual future on the high-APR pool's underlying asset on Hyperliquid
- **Entry:** Market order at next 4-hour candle open after signal confirmation (48h of spread ≥ 2x)
- **Size:** See position sizing section
- **Direction:** Long

### Short Leg Entry
- **Instrument:** Perpetual future on the low-APR pool's underlying asset on Hyperliquid
- **Entry:** Only if secondary signal also fires (`oi_utilization ≥ 0.80` on the losing pool). Market order at same candle as long leg entry.
- **Direction:** Short

## Exit Rules

### Exit Rules (apply to both legs independently)

| Condition | Action |
|---|---|
| APR spread compresses to `< 1.3x` | Close both legs at next 4h open |
| 7 calendar days elapsed since entry | Close both legs regardless of APR |
| Long leg drawdown exceeds 8% from entry | Stop-loss close long leg only |
| Short leg drawdown exceeds 8% from entry | Stop-loss close short leg only |
| High-APR pool TVL increases ≥ 30% (rotation confirmed) | Close long leg, hold short if still active |
| Low-APR pool `oi_utilization` drops below 0.60 | Close short leg (OI ceiling risk has passed) |

**No re-entry** within 72 hours of closing a position on the same underlying asset.

---

## Position Sizing

- **Account risk per trade:** 1.5% of total account equity per leg (long and short are separate risk allocations, not netted)
- **Leverage:** 2x maximum. This is a slow-moving structural play, not a momentum trade. High leverage defeats the purpose.
- **Position size formula:**
  - `stop_distance = 0.08` (8% stop)
  - `position_size_USD = (account_equity × 0.015) / stop_distance`
  - Example: $100,000 account → `($100,000 × 0.015) / 0.08 = $18,750` notional per leg at 2x leverage = $9,375 margin per leg
- **Maximum concurrent positions:** 2 pairs (4 legs total). APR divergence events are rare enough that this cap is unlikely to bind.
- **Correlation check:** Do not hold simultaneous long legs on assets with >0.85 rolling 30-day correlation (e.g., do not be long ETH and long BTC simultaneously under this strategy).

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Notes |
|---|---|---|---|
| GM pool APRs (fee yield) | GMX v2 subgraph (TheGraph) | Hourly | Query `marketStats` entity |
| GM pool TVL | GMX v2 subgraph | Hourly | Long token + short token value |
| GM pool currentOI and maxOI | GMX v2 subgraph | Hourly | `openInterestLong`, `openInterestShort`, `maxOpenInterest` |
| Underlying asset OHLCV | Hyperliquid historical API or Coingecko | 4-hour | For P&L calculation |
| LP deposit/withdrawal events | GMX v2 subgraph | Event-level | To verify rotation actually occurred |

### Subgraph Endpoints
- GMX v2 Arbitrum: `https://subgraph.satsuma-prod.com/gmx/synthetics-arbitrum-stats/api` (or via TheGraph hosted service)
- GMX v2 Avalanche: `https://subgraph.satsuma-prod.com/gmx/synthetics-avalanche-stats/api`
- GMX stats dashboard API (unofficial): `https://gmx-stats.com/api/gmxv2` — useful for sanity-checking APR calculations

### Backtest Period
- **Start:** GMX v2 launch on Arbitrum (August 2023) — earliest available GM pool data
- **End:** Most recent complete month
- **Minimum required:** 18 months of data to capture multiple market regimes

### Signal Identification
1. For each 4-hour timestamp, compute APR for all GM pools with TVL ≥ $5M
2. Compute `spread_ratio = max_APR / median_APR`
3. Flag timestamps where `spread_ratio ≥ 2.0`
4. Identify contiguous runs of ≥ 12 flagged timestamps (48h) as signal events
5. Record: which pool is high-APR, which is low-APR, entry timestamp, underlying assets

### P&L Calculation
- Entry price: 4h OHLCV open at signal confirmation timestamp
- Exit price: 4h OHLCV open at whichever exit condition triggers first
- Include estimated transaction costs: 0.05% per side (Hyperliquid taker fee) + 0.01% per 8h funding (estimate; use actual funding history where available)
- Do NOT include GMX LP yield in P&L — this strategy trades the underlying asset, not the LP position itself

### Metrics to Compute

| Metric | Target | Minimum Acceptable |
|---|---|---|
| Win rate | > 50% | > 42% |
| Average win / average loss ratio | > 1.5 | > 1.2 |
| Sharpe ratio (annualized) | > 1.2 | > 0.8 |
| Max drawdown | < 15% | < 25% |
| Number of signal events | > 20 | > 10 (below 10, results are not statistically meaningful) |
| Average holding period | 3–7 days | — |

### Baseline Comparison
- Compare against: random entry/exit on same assets with same holding period (Monte Carlo, 10,000 simulations)
- If strategy Sharpe is not meaningfully above the Monte Carlo 75th percentile, the APR signal adds no value

### Sub-hypotheses to Test Separately
1. **Long leg only** — does high-APR pool underlying outperform?
2. **Short leg only** — does low-APR pool underlying underperform, and only when `oi_utilization ≥ 0.80`?
3. **OI ceiling proximity as standalone signal** — is `oi_utilization ≥ 0.80` alone predictive of price decline, without requiring APR divergence?
4. **Lag sensitivity** — does entry at 24h, 48h, or 72h of divergence produce different results?

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. **≥ 15 signal events** identified in backtest period (below this, no statistical inference is possible)
2. **Sharpe ratio ≥ 0.8** on out-of-sample data (use last 6 months of available data as holdout; backtest only on earlier data)
3. **Win rate ≥ 42%** with **avg win / avg loss ≥ 1.3** (these are jointly necessary — high win rate with tiny wins is not acceptable)
4. **Short leg shows independent edge** when `oi_utilization ≥ 0.80` filter is applied — if the short leg has negative expectancy even with the filter, drop it entirely and run long-only
5. **APR data pipeline is live and automated** — manual APR checking is not acceptable for live trading; the 4-hour polling loop must be running before paper trading starts
6. **Paper trade for minimum 30 days** before any real capital allocation

---

## Kill Criteria

Abandon the strategy (stop paper trading, do not proceed to live) if any of the following occur:

| Condition | Action |
|---|---|
| Backtest produces < 10 signal events over 18 months | Kill — insufficient opportunity frequency |
| Out-of-sample Sharpe < 0.5 | Kill — no edge above noise |
| Monte Carlo test shows strategy is not above 60th percentile of random entries | Kill — APR signal is not informative |
| GMX v2 migrates to a new fee distribution model that eliminates per-pool APR differentiation | Kill — structural mechanism no longer exists |
| During paper trading: 5 consecutive losses | Pause, re-examine signal definition |
| During paper trading: drawdown exceeds 12% of paper account | Kill paper trade, return to backtest review |
| GMX v2 TVL falls below $200M total | Kill — insufficient liquidity for the mechanism to function |

---

## Risks

### Mechanism Risks
- **APR is endogenous and self-correcting.** High APR attracts capital, which compresses APR. By the time a 48h signal fires, the rotation may already be partially complete. The signal may be lagging, not leading.
- **No hard cliff.** Unlike a token unlock (which happens at a specific block), LP rotation is continuous and lumpy. A single large LP depositing can spike APR without any subsequent rotation from others.
- **GM pool composition drift.** GM pool TVL changes with the price of the underlying asset, not just LP flows. A 20% price drop in the pool's long token reduces TVL mechanically, potentially triggering OI ceiling effects without any LP rotation at all. This confounds the signal.
- **maxOI parameter is governance-controlled.** GMX governance can raise or lower `maxOI` independently of TVL. A governance vote to raise `maxOI` on the low-APR pool would invalidate the short leg thesis.

### Execution Risks
- **Hyperliquid liquidity for small-cap GM pool underlyings.** If the high-APR pool is for a small token (e.g., DOGE-USD GM pool), Hyperliquid may have insufficient liquidity for meaningful position sizes. Filter: only trade underlyings with Hyperliquid 24h volume ≥ $10M.
- **Funding rate drag.** If the trade direction aligns with the crowd (e.g., everyone is long the high-APR token), funding rates may be significantly negative, eroding the edge. Check funding rate at entry; do not enter if funding exceeds 0.05% per 8h against the position.
- **Correlation between legs.** In risk-off events, all crypto assets fall together. The long/short pair does not provide true market neutrality — both legs can lose simultaneously.

### Data Risks
- **APR calculation methodology.** GMX's displayed APR is a trailing average; the subgraph data may use different smoothing windows. Verify that the backtest APR calculation matches what LPs actually see on the dashboard (the signal that drives their behavior).
- **Subgraph data gaps.** TheGraph subgraphs for GMX have had historical outages and data gaps. Validate data completeness before trusting backtest results; flag any gaps > 8 hours as potentially corrupting nearby signal events.
- **Historical maxOI values.** The `maxOI` parameter has been changed by governance multiple times. Backtest must use the historically correct `maxOI` at each timestamp, not the current value. This requires reconstructing governance change history from on-chain events.

### Strategic Risks
- **GMX v3 or major protocol upgrade.** GMX is actively developing. A significant architecture change could eliminate GM pools entirely.
- **Competitor protocols.** If a competing protocol offers higher yields, LP rotation may go to the competitor rather than to the high-APR GM pool, breaking the rotation assumption.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|---|---|---|
| GMX v2 Arbitrum Subgraph | `https://subgraph.satsuma-prod.com/gmx/synthetics-arbitrum-stats/api` | `marketStats`, `poolAmounts`, `openInterest` entities |
| GMX v2 Avalanche Subgraph | `https://subgraph.satsuma-prod.com/gmx/synthetics-avalanche-stats/api` | Same entities |
| GMX Official Stats | `https://stats.gmx.io` | Dashboard cross-reference for APR sanity check |
| TheGraph Explorer (GMX) | `https://thegraph.com/explorer/subgraphs/` (search "GMX v2") | Backup subgraph endpoint |
| Hyperliquid Historical Data | `https://app.hyperliquid.xyz/api` (REST) or `https://hyperliquid.xyz/docs` | OHLCV for underlying assets, funding rate history |
| Coingecko API | `https://api.coingecko.com/api/v3/coins/{id}/ohlc` | Backup OHLCV if Hyperliquid data is incomplete |
| GMX GitHub (contracts) | `https://github.com/gmx-io/gmx-synthetics` | `MarketUtils.sol` — verify `maxOpenInterest` calculation logic |
| Arbiscan (governance txns) | `https://arbiscan.io` | Reconstruct historical `maxOI` parameter changes via `setMaxOpenInterest` events |

### Key Subgraph Query (example — GM pool stats)
```graphql
{
  marketStats(
    where: { period: "1h", timestamp_gte: 1690000000 }
    orderBy: timestamp
    orderDirection: asc
    first: 1000
  ) {
    id
    market
    timestamp
    totalFees
    poolValue
    longOpenInterest
    shortOpenInterest
    maxLongOpenInterest
    maxShortOpenInterest
  }
}
```

---

## Summary Assessment

This strategy has a real structural mechanism (GMX's `maxOI` ceiling) but the primary signal (APR divergence driving LP rotation) is probabilistic and self-correcting. The strongest sub-hypothesis is the **short leg with OI ceiling proximity filter** — this is the closest thing to a hard forcing function in the strategy. The long leg is weaker and should be treated as a secondary, optional component.

**Recommended backtest priority:** Test short leg with `oi_utilization ≥ 0.80` filter first, in isolation. If that shows edge, add the APR divergence requirement as a secondary filter. Only add the long leg if the short leg independently validates.

**Do not allocate real capital** until both backtest and 30-day paper trade criteria are met. The mechanism is plausible but the timing uncertainty is significant enough that this could easily be a break-even strategy after fees.
