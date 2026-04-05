---
title: "Hyperliquid New Listing Initial Funding Squeeze"
status: HYPOTHESIS
mechanism: 6
implementation: 8
safety: 5
frequency: 3
composite: 720
categories:
  - funding-rates
  - exchange-structure
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a new perpetual futures market opens on Hyperliquid, retail FOMO creates an immediate long-side OI imbalance with no established hedger base to absorb it. The funding mechanism is contractually obligated to charge the dominant side every 8 hours. Because longs dominate early OI, they pay shorts a carry rate that frequently reaches 0.1–0.5%/8hr in the first 24–72 hours. Shorting the perp within the first two funding periods captures this carry while the imbalance persists, before arbitrageurs and market makers normalize the rate. The edge is not directional — it is a carry harvest on a structural imbalance that must exist by protocol design until hedgers arrive.

---

## Structural Mechanism

**Why this must happen, not just tends to happen:**

1. **Funding formula is deterministic.** Hyperliquid's funding rate is computed as: `funding = clamp(premium_index, -0.05%, 0.05%) + interest_rate_component`. The premium index is the spread between the mark price and the oracle index price. When retail longs push the perp above spot, the premium is positive, and longs are mechanically charged every 8 hours. This is not discretionary — it is a smart contract rule.

2. **No hedger base at listing.** Established markets have arbitrageurs, delta-neutral market makers, and basis traders who short the perp and buy spot to harvest funding, compressing the rate toward zero. At listing, these participants have not yet built infrastructure (spot inventory, risk limits, API integrations) for the new token. The imbalance persists until they do.

3. **Retail FOMO is structurally predictable.** New listings on Hyperliquid are announced publicly. Retail participants buy perps (not spot) because perps require no custody of the underlying token. This creates a systematic long bias in the first hours of trading that is not random — it is the predictable result of the listing event itself.

4. **Convergence is guaranteed, not just probable.** Funding rates cannot remain elevated indefinitely. As the premium compresses (either price falls or hedgers arrive), the rate normalizes. The question is timing, not direction of convergence.

**The edge is:** Collect funding carry during the window between listing and hedger arrival. The structural mechanism guarantees the carry exists; the risk is price movement overwhelming the carry before normalization.

---

## Entry Rules

| Parameter | Rule |
|-----------|------|
| **Trigger** | New perpetual market goes live on Hyperliquid |
| **Entry window** | Enter short after the **second funding payment** has been confirmed (i.e., 8–16 hours post-listing) |
| **Entry condition** | Funding rate at entry must be ≥ 0.08%/8hr (annualizes to ~87%). Below this threshold, carry does not justify stop risk |
| **Entry condition 2** | OI must be ≥ $500k notional (below this, slippage and manipulation risk are too high) |
| **Entry condition 3** | Mark price must be within 20% of the listing open price (extreme early pumps increase stop risk disproportionately) |
| **Entry method** | Market order or limit order within 0.5% of mid. Do not chase — if fill requires >0.5% slippage, skip the trade |
| **Entry timing rationale** | Waiting for the second funding period confirms the imbalance is real and not a data artifact, while still capturing the majority of the elevated carry window |

---

## Exit Rules

**Primary exits (whichever triggers first):**

| Exit Type | Rule |
|-----------|------|
| **Funding normalization** | Exit when the 8hr funding rate drops below 0.04%/8hr for two consecutive periods |
| **Time stop** | Exit at market close of day 5 post-listing regardless of funding level |
| **Profit target** | No hard profit target — let carry accumulate until normalization or time stop |

**Risk exits:**

| Exit Type | Rule |
|-----------|------|
| **Hard stop** | Exit immediately if mark price moves 15% adverse from entry (short squeeze risk on new listings is acute) |
| **Soft stop** | If price moves 8% adverse AND funding rate has dropped below 0.06%/8hr, exit — carry no longer justifies the drawdown |
| **OI collapse** | If OI drops >50% from peak within the holding period, exit — indicates market structure breakdown |

**Exit method:** Market order. Do not use limit orders on exit — new listing liquidity can evaporate instantly and a missed exit on a squeeze is catastrophic.

---

## Position Sizing

**Core principle:** This is a carry trade with a fat left tail. Size must reflect that a 15% stop is possible on every trade.

| Parameter | Rule |
|-----------|------|
| **Max notional per trade** | 1% of total portfolio NAV |
| **Max loss per trade** | 0.15% of NAV (15% stop × 1% notional) |
| **Concurrent positions** | Maximum 2 simultaneous new listing shorts (listings rarely overlap, but cap exposure) |
| **Leverage** | Use 2–3× leverage maximum. Higher leverage amplifies stop risk beyond acceptable bounds |
| **Scaling** | Do not scale in. Enter full size at trigger. New listings are too volatile for averaging |

**Sizing rationale:** At 0.1%/8hr funding and a 3-day hold, gross carry is ~0.9% on notional (9 periods × 0.1%). On 1% NAV position at 2× leverage, that is ~1.8% gross carry on notional, or 0.018% of NAV per trade. After stops and slippage, expected value per trade is small. Volume of trades (2–6/month) and compounding matter more than per-trade sizing.

---

## Backtest Methodology

### Data Collection

1. **Listing dates:** Compile all Hyperliquid perpetual listing dates from Hyperliquid's public announcement channels (Discord, Twitter/X) and cross-reference with the first timestamp of funding rate data appearing in the API. Source: `https://api.hyperliquid.xyz/info` endpoint `fundingHistory`.

2. **Funding rate history:** Pull 8-hour funding rates for each market from listing date through day 10. Endpoint: `POST /info` with `{"type": "fundingHistory", "coin": "<COIN>", "startTime": <unix_ms>}`.

3. **OHLCV data:** Pull 1-hour candles from listing date through day 10 for each market. Endpoint: `{"type": "candleSnapshot", "req": {"coin": "<COIN>", "interval": "1h", ...}}`.

4. **OI data:** Pull open interest snapshots at hourly intervals from listing. Endpoint: `{"type": "openInterest"}` or derived from `metaAndAssetCtxs`.

### Simulation Steps

**Step 1 — Universe construction:** List every Hyperliquid perpetual market that has listed since Hyperliquid launched. Exclude markets with fewer than 5 days of history (insufficient data). Expected universe: 30–80 markets depending on launch date of data pull.

**Step 2 — Entry simulation:** For each market, identify the timestamp of the second funding payment. Check entry conditions: funding ≥ 0.08%/8hr, OI ≥ $500k, price within 20% of open. Record whether trade is entered or skipped.

**Step 3 — Exit simulation:** For each entered trade, simulate exits in priority order: hard stop (15% adverse), soft stop (8% adverse + funding < 0.06%), funding normalization (< 0.04% for 2 periods), time stop (day 5). Record exit reason, holding period, and PnL.

**Step 4 — PnL calculation:** `PnL = (entry_price - exit_price) / entry_price × notional + sum(funding_received_during_hold)`. Funding received = funding_rate × notional × number_of_periods_held. Subtract estimated slippage of 0.1% round-trip.

**Step 5 — Sensitivity analysis:** Re-run with entry funding threshold at 0.05%, 0.08%, 0.10%, 0.15% to find optimal filter. Re-run with stop at 10%, 15%, 20% to find optimal risk parameter. Re-run with time stop at 3, 5, 7 days.

**Step 6 — Segmentation:** Split results by: (a) market cap tier of listed token at listing, (b) whether listing was accompanied by a major announcement, (c) time of day of listing. Look for subsets with materially better Sharpe.

### Key Metrics to Compute

| Metric | Target for Go-Live |
|--------|-------------------|
| Win rate (funding collected > stop loss) | > 55% |
| Average carry collected per winning trade | > 0.5% on notional |
| Average loss per losing trade | < 1.5% on notional |
| Expectancy per trade | > 0.1% on notional |
| Max consecutive losses | < 5 |
| Sharpe ratio (annualized) | > 1.0 |

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

1. **Backtest expectancy is positive** across the full historical universe with no parameter overfitting (out-of-sample test on most recent 20% of listings must also show positive expectancy).
2. **Minimum 20 historical trades** in backtest universe. Fewer than 20 trades means the sample is too small to distinguish edge from noise.
3. **No single trade accounts for >30% of total backtest PnL.** If one outlier drives the result, the edge is not repeatable.
4. **Paper trade for 30 days** with at least 3 live trades executed on paper. Paper trade PnL must be within 50% of backtest expectancy (accounting for sample variance).
5. **Execution infrastructure confirmed:** API connection to Hyperliquid, automated funding rate monitoring, alert system for new listing detection (Discord webhook or API polling every 5 minutes), automated stop-loss order placement.
6. **Listing detection latency < 30 minutes.** The entry window is 8–16 hours post-listing, so detection speed is not critical, but a 30-minute maximum ensures no missed entries.

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

| Trigger | Action |
|---------|--------|
| 5 consecutive losing trades in live trading | Suspend, review, do not resume without researcher sign-off |
| Live Sharpe drops below 0.5 over trailing 90 days (minimum 10 trades) | Suspend and re-evaluate |
| Hyperliquid changes funding rate formula or calculation frequency | Suspend immediately — structural mechanism may be invalidated |
| Average entry funding rate across last 10 listings drops below 0.06%/8hr | Market has adapted; edge may be arbitraged away. Suspend and re-evaluate |
| Any single trade loss exceeds 2% of NAV | Position sizing or stop logic has failed. Suspend and audit |
| New listing frequency drops below 1/month for 3 consecutive months | Insufficient trade frequency to maintain statistical validity |

---

## Risks

### Primary Risks

**1. Short squeeze on new listings (HIGH severity, MEDIUM probability)**
New tokens with low float and high retail excitement can squeeze 50–100% in hours. A 15% stop does not protect against a gap through the stop level. *Mitigation:* Hard position size cap at 1% NAV. Accept that some trades will gap through stops. This is a known cost of the strategy.

**2. Funding rate normalization faster than expected (MEDIUM severity, HIGH probability)**
Sophisticated arbitrageurs may arrive within the first funding period, collapsing the rate before entry. *Mitigation:* Entry condition requires ≥ 0.08%/8hr at entry, not at listing. If rate has already normalized, no trade is taken.

**3. Liquidity collapse during exit (HIGH severity, LOW probability)**
New listing markets can become one-sided. A market order exit during a squeeze may execute at a price far worse than the stop level. *Mitigation:* Use market orders (not limits) on exit. Accept slippage as a cost. Do not hold through extreme illiquidity.

**4. Edge arbitraged away (MEDIUM severity, MEDIUM probability)**
As more participants discover this pattern, they will short new listings earlier, compressing the funding rate faster. The entry window shrinks over time. *Mitigation:* Monitor average entry funding rate as a kill criterion. If the edge degrades, the kill criterion triggers before significant capital is lost.

**5. Hyperliquid protocol changes (LOW probability, HIGH severity)**
Hyperliquid could change funding rate mechanics, listing procedures, or fee structures. *Mitigation:* Monitor Hyperliquid governance and announcements. Kill criterion triggers on any funding formula change.

**6. Correlation of losses (MEDIUM severity, LOW probability)**
If multiple new listings occur simultaneously during a broad market rally, all shorts lose simultaneously. *Mitigation:* Cap concurrent positions at 2. Do not increase size during market-wide euphoria.

### Secondary Risks

- **Tax treatment:** Short perp positions with frequent funding receipts may create complex tax events depending on jurisdiction. Consult tax advisor before live deployment.
- **Counterparty risk:** Hyperliquid is a decentralized exchange but has centralized components. Smart contract risk and exchange risk are non-zero.
- **Data quality:** Hyperliquid API historical data may have gaps or errors near listing dates. Validate all data points manually for the first 5 listings in the backtest universe.

---

## Data Sources

| Data Type | Source | Endpoint / Method | Cost |
|-----------|--------|-------------------|------|
| Funding rate history | Hyperliquid public API | `POST https://api.hyperliquid.xyz/info` → `{"type": "fundingHistory", "coin": X, "startTime": Y}` | Free |
| OHLCV candles | Hyperliquid public API | `{"type": "candleSnapshot", "req": {"coin": X, "interval": "1h"}}` | Free |
| Open interest | Hyperliquid public API | `{"type": "metaAndAssetCtxs"}` → `openInterest` field | Free |
| Listing dates | Hyperliquid Discord / Twitter | Manual compilation + cross-reference with first API data timestamp | Free (manual) |
| Mark price history | Hyperliquid public API | Derived from candle data or `{"type": "l2Book"}` snapshots | Free |
| Oracle/index price | Hyperliquid public API | `{"type": "allMids"}` for spot reference prices | Free |

**Data validation requirement:** For each listing in the backtest universe, manually verify the listing date against at least one external source (Discord announcement, Twitter post, or block explorer timestamp). API data alone is insufficient for listing date confirmation.

---

## Open Questions for Researcher Review

1. **Is the 8–16 hour entry window optimal?** Waiting for the second funding period sacrifices the highest-rate periods. Backtest should test entry at listing open, first funding, and second funding to quantify the tradeoff between rate capture and confirmation.

2. **Should we filter by token category?** Meme tokens may have more extreme and persistent funding imbalances than infrastructure tokens. Segmentation analysis in the backtest should answer this.

3. **Is there a spot hedge available?** If the listed token trades on a CEX (Binance, Bybit), a simultaneous spot long would convert this to a pure funding harvest with no directional risk. This would raise the score to 8/10 if executable. Investigate spot availability for each historical listing.

4. **What is the detection mechanism for new listings?** Hyperliquid does not have a public webhook for new market creation. Options: (a) poll `metaAndAssetCtxs` every 5 minutes and alert on new coins appearing, (b) monitor Hyperliquid Discord via bot. Both need to be built before go-live.

5. **Does the edge persist after Hyperliquid's validator set expands?** As Hyperliquid grows, more sophisticated market makers will list faster. The entry window may compress from 8–16 hours to 1–2 hours. Monitor this as a structural degradation signal.

---

*Next step: Assign to data engineer to pull full funding rate history for all Hyperliquid listings. Backtest target completion: 3 weeks. Researcher review of backtest results required before paper trade authorization.*
