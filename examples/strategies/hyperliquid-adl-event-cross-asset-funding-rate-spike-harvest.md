---
title: "Hyperliquid ADL Contagion — Cross-Asset Funding Rate Harvesting"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 6
frequency: 3
composite: 432
categories:
  - funding-rates
  - liquidation
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's Auto-Deleveraging (ADL) system fires on a large position in Asset A, traders who held correlated hedges across Asset A and Asset B are suddenly unbalanced. Their hedge in B is now naked, and they must either close it or re-hedge. This mechanical rebalancing flow creates a temporary imbalance in B's open interest skew, which distorts B's funding rate. The distortion over-corrects (because the rebalancing is reactive and non-linear), then mean-reverts as the market re-equilibrates. The window between distortion and reversion is the harvesting opportunity.

**Causal chain:**
1. ADL fires on Asset A (on-chain event, verifiable, timestamped)
2. Profitable positions in A are forcibly closed at mark price → OI in A drops sharply
3. Traders who were long A / short B (or short A / long B) as a spread trade now have an unhedged leg in B
4. Forced rebalancing of B leg creates directional flow in B within minutes to hours
5. B's funding rate spikes as OI imbalance shifts (longs > shorts or vice versa)
6. Spike is transient — no new fundamental information has arrived, only a mechanical flow event
7. Funding rate in B mean-reverts to baseline within 8–48 hours
8. Strategy earns the elevated carry during the reversion window

---

## Structural Mechanism

**Why this CAN happen (not MUST):**

This is a second-order effect, not a first-order guarantee. The first-order mechanism (ADL closes positions in A) is contractually guaranteed by Hyperliquid's protocol. The second-order effect (funding distortion in B) depends on:

- Whether a meaningful fraction of A's OI was held as part of cross-asset spread trades
- Whether those traders rebalance quickly (creating the spike) rather than slowly or not at all
- Whether B's funding rate calculation is sensitive enough to the resulting OI shift

**Why it's plausible:**
Hyperliquid's funding rate is calculated as a function of the mark-price premium over index and the OI imbalance between longs and shorts. A sudden forced closure of, say, $2M in SOL longs (via ADL) that were hedged with $1.5M in AVAX shorts creates $1.5M in unhedged AVAX short exposure. If those traders close their AVAX shorts simultaneously, AVAX OI skews long, and the funding rate for AVAX longs rises. This is a real mechanical pathway — but its magnitude and consistency are empirical questions.

**Honest classification:** Structural trigger (ADL) + plausible but unproven transmission mechanism. Score 5/10 is appropriate. Do not treat this as a guaranteed edge until backtest confirms.

---

## Entry/Exit Rules

### Pre-computation (done offline, updated weekly)
1. Build a correlation matrix of all Hyperliquid-listed perps using 30-day rolling hourly returns
2. For each asset, identify its top-3 correlated peers (Pearson r > 0.65)
3. Store as a lookup table: `{asset_A: [asset_B1, asset_B2, asset_B3]}`

### Trigger Detection
- Monitor Hyperliquid's ADL event stream continuously
- ADL event qualifies if:
  - Notional closed ≥ $500,000 USD equivalent
  - Asset has at least one correlated peer with r > 0.65 in the lookup table
  - No other ADL event fired in the same asset within the prior 4 hours (avoid stacking)

### Entry Conditions (check within 30 minutes of ADL confirmation)
For each correlated peer B of the ADL asset A:
- Compute B's current 8h funding rate
- Compute B's 30-day rolling mean and standard deviation of 8h funding rate
- **Enter if:** current funding rate > (mean + 1.5 × std dev) OR < (mean − 1.5 × std dev)
- **Direction:** Take the side that EARNS the elevated funding
  - If funding is abnormally positive → go SHORT the perp (shorts receive funding)
  - If funding is abnormally negative → go LONG the perp (longs receive funding)
- Enter at market on Hyperliquid perp

### Exit Conditions (check every 8 hours after entry)
- **Primary exit:** Funding rate returns to within 0.75 std dev of 30-day mean
- **Time stop:** Exit at market after 48 hours regardless of funding level
- **Loss stop:** Exit if mark-to-market loss on position exceeds 1.5× the expected carry earned over 48h at entry funding rate (i.e., if price moves against you enough to wipe 1.5 periods of carry)

### Do Not Enter If
- B's funding rate spike occurred more than 2 hours before ADL event was detected (pre-existing distortion, not caused by ADL)
- B has its own ADL event in the prior 8 hours
- B's 24h volume is below $5M (insufficient liquidity for clean entry/exit)

---

## Position Sizing

**Base size:** 0.5% of total portfolio per trade

**Rationale:** ADL events are rare and the cross-asset mechanism is unproven. Small size allows accumulation of statistical evidence without meaningful drawdown risk during the hypothesis-testing phase.

**Leverage:** Maximum 3× on the perp position. The carry earned is the return; leverage amplifies carry but also amplifies price risk. At 3×, a 1% adverse price move costs 3% of position — roughly equivalent to 6–12 periods of carry at 0.05%/8h. Keep leverage low.

**Scaling rule:** If backtest shows Sharpe > 1.5 and ≥ 30 events, scale to 1% of portfolio per trade. Do not scale further until live paper-trade confirms.

**Concurrent positions:** Maximum 3 simultaneous positions (one per correlated peer of a single ADL event). If two ADL events fire within 4 hours, treat as separate signals but cap total exposure at 1.5% of portfolio.

---

## Backtest Methodology

### Data Sources
- **ADL event log:** Hyperliquid on-chain data via `https://api.hyperliquid.xyz/info` — use `clearinghouseState` and trade history endpoints. ADL trades are flagged in the trade stream. Alternatively, parse from Hyperliquid's public data dumps at `https://hyperliquid.xyz/data` (if available) or community archives.
- **Funding rate history:** `POST https://api.hyperliquid.xyz/info` with `{"type": "fundingHistory", "coin": "SOL", "startTime": <unix_ms>}` — returns 8h funding snapshots. Pull for all listed assets.
- **OHLCV / mark price:** Same API, `candleSnapshot` endpoint.
- **Correlation matrix inputs:** Hourly close prices from `candleSnapshot` for all listed perps.

### Data Range
- Use all available Hyperliquid history (mainnet launched ~November 2023). As of early 2025, this gives ~14 months of data.
- Expect: ADL events qualifying at ≥$500K notional are likely rare — estimate 10–40 events total. This is a known limitation.

### Backtest Steps
1. Parse full ADL event log; filter to ≥$500K notional events; record timestamp, asset, notional
2. For each event, look up correlated peers from the correlation table (computed on data prior to the event date — no lookahead)
3. For each peer, check entry conditions at T+0 to T+30min post-ADL
4. If entry triggered, simulate position: record funding payments every 8h, record mark price at entry and exit
5. Apply exit rules; record P&L = (funding earned) − (price slippage at entry/exit, estimated at 0.05% per side) − (price move P&L)

### Metrics to Compute
- **Total trades:** Must be ≥ 15 to draw any conclusions
- **Win rate** (trades where total P&L > 0)
- **Average carry earned per trade** (funding payments received)
- **Average price P&L per trade** (separate from carry — want to confirm price is not systematically moving against the position)
- **Sharpe ratio** (annualized, using per-trade P&L)
- **Max drawdown** (across all trades, mark-to-market)
- **Average hold time to exit**
- **Funding spike magnitude distribution** (how often does the spike exceed 1.5 std dev?)

### Baseline Comparison
- **Null hypothesis:** Enter a carry trade in Asset B at a random time (not triggered by ADL) when funding exceeds 1.5 std dev. Compare P&L distribution to ADL-triggered entries. If ADL-triggered entries are not better than random high-funding entries, the cross-asset contagion hypothesis is false — the funding spikes are coincidental, not caused by ADL.
- This baseline test is critical. Run it.

---

## Go-Live Criteria (Paper Trading)

All of the following must be true before moving to paper trade:

1. **Sample size:** ≥ 15 qualifying ADL events with entry signals triggered
2. **Win rate:** ≥ 55% of trades profitable (total P&L including price move)
3. **Carry dominance:** Average carry earned per trade > average adverse price move per trade (funding is the return driver, not lucky price direction)
4. **ADL-triggered vs. random baseline:** ADL-triggered entries show statistically higher carry-adjusted P&L than random high-funding entries (p < 0.10 acceptable given small sample)
5. **No single event dominates:** Remove the best single trade; strategy remains profitable
6. **Max drawdown:** < 3× average trade P&L (strategy is not a few big wins masking many losses)

---

## Kill Criteria

Abandon the strategy (do not proceed to live trading) if any of the following:

1. **Backtest sample < 15 events** — insufficient data; park the hypothesis and revisit in 12 months when more ADL history exists
2. **ADL-triggered entries are statistically indistinguishable from random high-funding entries** — the cross-asset contagion mechanism is not real; the funding spikes are coincidental
3. **Price move losses consistently exceed carry earned** — the position is a directional bet in disguise, not a carry trade
4. **Funding normalization takes > 48h on average** — financing costs during extended holds erode the edge
5. **During paper trading:** 3 consecutive losses where price move loss > 2× carry earned — live market behavior diverges from backtest

---

## Risks

### Primary Risk: Mechanism May Not Exist
The cross-asset contagion is a hypothesis. It is entirely possible that ADL events in Asset A do not meaningfully affect funding rates in Asset B — either because spread traders are a small fraction of OI, or because they rebalance slowly and don't create a detectable spike. The baseline comparison in the backtest will reveal this.

### Secondary Risk: Small Sample Size
Hyperliquid is a young exchange. Large ADL events (≥$500K) are infrequent. A backtest with 10–15 events has wide confidence intervals. A strategy that looks profitable on 12 events could easily be noise. Be conservative about scaling.

### Tertiary Risk: Adverse Price Move During Hold
The strategy earns carry but holds a directional position. If the market moves sharply against the position during the 8–48h hold window, price losses can exceed carry earned. The loss stop (1.5× expected carry) limits this but does not eliminate it. Stress-test: what happens if a second ADL event fires in Asset B while you're holding?

### Execution Risk: ADL Detection Latency
ADL events must be detected quickly (within 30 minutes) for the entry window to be valid. If the funding spike in B occurs within minutes of the ADL event, a 30-minute detection window may miss the optimal entry. Measure the lag between ADL timestamp and funding spike onset in the backtest.

### Structural Risk: Hyperliquid Protocol Changes
Hyperliquid may modify its ADL mechanism, funding rate formula, or insurance fund rules. Any such change invalidates the mechanism. Monitor protocol upgrade announcements.

### Correlation Instability Risk
Correlations between crypto assets are unstable, especially during stress events (which is exactly when ADL fires). The correlation table computed on normal-market data may not reflect correlations during the ADL event itself. Consider using a shorter lookback (14 days) for the correlation table to capture more recent regime.

---

## Data Sources

| Data | Endpoint | Notes |
|------|----------|-------|
| ADL event stream | `POST https://api.hyperliquid.xyz/info` → `{"type": "userFills", ...}` with ADL flag | ADL trades have `liquidation: true` and specific flags in trade stream; verify exact field names in API docs |
| Funding rate history | `POST https://api.hyperliquid.xyz/info` → `{"type": "fundingHistory", "coin": "<ASSET>", "startTime": <unix_ms>}` | Returns array of `{time, coin, fundingRate, premium}` |
| Mark price / OHLCV | `POST https://api.hyperliquid.xyz/info` → `{"type": "candleSnapshot", "req": {"coin": "<ASSET>", "interval": "1h", ...}}` | Use for price P&L calculation |
| Open interest | `POST https://api.hyperliquid.xyz/info` → `{"type": "metaAndAssetCtxs"}` | Returns current OI per asset; for historical OI, may need to poll and store or use community archives |
| API documentation | `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api` | Official reference |
| Community data archives | `https://github.com/hyperliquid-dex` | Check for community-maintained historical dumps |

**Note on ADL identification:** Verify the exact mechanism for identifying ADL trades in the Hyperliquid API before building the backtest. ADL trades may appear as liquidations with a specific counterparty address (the ADL engine) or a specific trade type flag. Confirm this against known historical ADL events (several large ADL events occurred in 2024 and are documented in community forums).

---

*This specification is sufficient to build a backtest. The critical unknown is whether the cross-asset contagion mechanism exists at all — the baseline comparison test in the backtest methodology is the most important single test to run. If ADL-triggered entries are not better than random high-funding entries, stop here.*
