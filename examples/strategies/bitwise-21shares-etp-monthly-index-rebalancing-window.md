---
title: "Bitwise/21Shares ETP Monthly Index Rebalancing Window"
status: HYPOTHESIS
mechanism: 6
implementation: 6
safety: 5
frequency: 4
composite: 720
categories:
  - index-rebalance
  - defi-protocol
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Publicly disclosed index methodologies for crypto basket ETPs (Bitwise BITW, 21Shares basket products) create calculable, directional flow obligations that must be executed by authorized participants (APs) on a fixed schedule. By computing the required rebalance trades in advance and positioning in the same direction before the execution window, we can extract a predictable price impact premium — particularly in smaller-cap, lower-liquidity constituents where AP flow represents a meaningful fraction of daily volume.

This is the crypto equivalent of S&P 500 index addition front-running, a well-documented TradFi edge. The crypto version is structurally less efficient because: (a) index methodologies are less widely tracked, (b) AUM is smaller and less institutionally monitored, (c) niche basket products have illiquid underlying constituents, and (d) AP execution is less standardised than equity index rebalancing.

**The edge is not predictive — it is calculable.** The required trades are a mathematical output of public data. The only uncertainty is execution timing within the published window.

---

## Structural Mechanism

### Why this MUST happen (not just tends to happen)

1. **Contractual obligation:** Index ETPs are legally required to track their stated index. Deviation beyond tolerance bands triggers mandatory rebalancing. The methodology document is a binding operational constraint, not a suggestion.

2. **AP mechanics:** Authorized participants create/redeem ETP shares by delivering or receiving the underlying basket. To maintain NAV alignment, APs must trade the underlying assets in the proportions dictated by the current index weights. This is not discretionary.

3. **Weight drift forces trades:** Between rebalance dates, constituent prices move, causing actual weights to drift from target weights. On rebalance date, the index resets to target weights. The direction and approximate magnitude of required trades is calculable from:
   - Current constituent prices (public, real-time)
   - Current target weights (published in methodology)
   - New target weights (published on or before rebalance date)
   - Total AUM of the product (published daily via NAV)

4. **Execution concentration:** Unlike a large equity index fund that rebalances continuously, these ETPs execute over a compressed 1–3 day window, concentrating flow and creating a detectable price impact in illiquid constituents.

### The causal chain

```
Index methodology published (public)
        ↓
Rebalance date announced (fixed calendar, public)
        ↓
New target weights published (T-1 to T-3 days before rebalance)
        ↓
Required trades calculable: ΔWeight × AUM = Dollar flow per constituent
        ↓
AP executes on CEX (Coinbase Prime, Kraken, etc.) over 1–3 day window
        ↓
Price impact in direction of AP flow, especially in illiquid constituents
        ↓
We exit into AP buying/selling pressure
```

### Why the edge persists

- Most quants focus on large-cap crypto; niche basket products are below the radar
- The AUM is too small to attract dedicated institutional front-runners
- Manual data collection (PDF methodology parsing, daily NAV tracking) creates friction that deters casual participants
- Multiple products compound the signal on overlapping schedules

---

## Target Products

### Primary targets (highest signal quality)

| Product | Issuer | AUM Estimate | Rebalance Frequency | Constituent Liquidity |
|---------|--------|-------------|--------------------|-----------------------|
| 21Shares DeFi ETP | 21Shares | ~$20–50M | Monthly | Low–Medium |
| 21Shares Layer 2 ETP | 21Shares | ~$10–30M | Monthly/Quarterly | Low |
| 21Shares Crypto Basket ETP | 21Shares | ~$100M+ | Monthly | Medium |
| Bitwise 10 Crypto Index (BITW) | Bitwise | ~$800M–1B | Monthly | Medium–High |
| Bitwise DeFi Crypto Index | Bitwise | ~$20–50M | Monthly | Low–Medium |

**Best candidates:** 21Shares niche products (DeFi, L2 baskets) where:
- AUM is $10–50M
- Constituents have daily spot volume of $1–20M
- AP flow represents >0.5% of constituent daily volume

**Avoid for this strategy:** BITW (too large, too liquid, more efficient) unless focusing on newly added small-cap constituents.

### Constituent selection filter

For each rebalance event, rank constituents by:

```
Impact Score = (Dollar flow required) / (Constituent 30-day avg daily volume)
```

Only trade constituents where Impact Score > 0.5%. Below this threshold, the AP flow is noise against normal volume.

---

## Data Sources

### Required data (all public, mostly free)

| Data Type | Source | Update Frequency | Cost |
|-----------|--------|-----------------|------|
| Index methodology PDFs | Bitwise.com, 21Shares.com | On methodology change | Free |
| Current target weights | Methodology PDF + product factsheet | Monthly | Free |
| Daily NAV with constituent breakdown | Product issuer website, Bloomberg (paid) | Daily | Free (issuer) / Paid (Bloomberg) |
| Constituent spot prices | CoinGecko API, CoinMarketCap API | Real-time | Free tier |
| Constituent daily volume | CoinGecko API, CoinMarketCap API | Daily | Free tier |
| Rebalance calendar | Methodology PDF (fixed schedule) | Stable | Free |
| CEX order book depth | Exchange APIs (Binance, Coinbase) | Real-time | Free |

### Data pipeline

```
1. Parse methodology PDFs → extract rebalance dates, weight rules, constituent universe
2. Daily: scrape NAV pages → current weights, AUM
3. Daily: pull prices/volumes from CoinGecko API
4. T-3 before rebalance: compute ΔWeight for each constituent
5. Compute Dollar Flow = ΔWeight × AUM
6. Compute Impact Score = Dollar Flow / 30d avg daily volume
7. Rank constituents → select trades above threshold
```

**Note:** Some 21Shares products publish constituent weights in machine-readable format (CSV/JSON on their website). Bitwise publishes daily holdings for some products. Verify current data availability before building pipeline — this changes.

---

## Entry Rules


### Entry

**Trigger:** Rebalance date is known (fixed calendar). New target weights published by issuer (typically T-1 to T-3 before rebalance date).

**Entry timing:** T-3 to T-2 days before the rebalance execution window opens.

**Entry condition checklist:**
- [ ] New target weights confirmed from issuer source (not estimated)
- [ ] Impact Score for constituent > 0.5%
- [ ] Constituent has liquid spot market on Binance or Coinbase (>$500K daily volume)
- [ ] Perp available on Hyperliquid or Binance for the constituent (preferred for shorts/leverage)
- [ ] No major protocol event or token unlock within the rebalance window (would contaminate signal)
- [ ] Bid-ask spread on entry < 0.3% (illiquid names excluded if spread too wide)

**Direction:**
- **Long:** Constituents with positive ΔWeight (AP must buy)
- **Short:** Constituents with negative ΔWeight (AP must sell) — deletions or weight reductions

**Entry execution:** TWAP over 4–6 hours to avoid moving the market on entry. We are not the price-mover here; the AP is. We just need to be positioned before them.

## Exit Rules

### Exit

**Primary exit:** During the rebalance execution window (T+0 to T+2), as AP flow creates price impact. Exit into the AP-driven move.

**Exit timing logic:**
- Monitor on-chain/CEX volume for unusual spikes in constituent volume — this signals AP execution has begun
- Begin scaling out when volume exceeds 2× 30-day average daily volume
- Full exit by T+3 regardless of P&L (do not hold through post-rebalance mean reversion)

**Stop loss:** -3% from entry on any individual position. The structural edge is timing-dependent; if the trade moves against us before AP execution, the thesis is broken or timing is off.

**Hard exit rule:** If rebalance is delayed or methodology change is announced after entry, exit immediately at market. The structural basis for the trade has changed.

---

## Position Sizing

### Per-trade sizing

```
Max position size per constituent = MIN(
    0.5% of portfolio,
    10% of constituent's average daily volume (to avoid self-impact),
    Dollar flow required by AP × 5% (we are a small fraction of the AP trade)
)
```

**Rationale for 10% of ADV cap:** We must not become the price-mover. Our edge is riding AP flow, not creating our own impact. If our position exceeds 10% of ADV, we risk moving the price before the AP arrives, reducing our edge and creating adverse selection.

### Portfolio-level sizing

- Maximum simultaneous open positions: 6 (across all rebalance events)
- Maximum total exposure to this strategy: 15% of portfolio
- Maximum exposure to any single rebalance event: 5% of portfolio
- Correlation note: Constituents within the same basket are correlated; treat same-basket positions as a single risk unit

### Leverage

- Spot preferred for long positions in illiquid constituents (no liquidation risk)
- Perp with max 2× leverage for short positions (weight reductions/deletions)
- No leverage on constituents with <$1M daily volume

---

## Backtest Methodology

### Scope

- **Period:** January 2022 – present (covers multiple market regimes)
- **Products:** Start with BITW (longest history, most data available) then extend to 21Shares products
- **Events:** Target minimum 30 rebalance events for statistical validity

### Data reconstruction

1. Retrieve historical BITW holdings from Bitwise website (they publish historical NAV/holdings)
2. For each historical rebalance date: reconstruct what the required trades were using T-1 holdings vs. T+1 holdings
3. Identify constituents where ΔWeight × AUM > $500K (minimum meaningful flow)
4. Record constituent prices at T-3, T-2, T-1, T+0, T+1, T+2, T+3

### Metrics to compute

| Metric | Target | Kill Threshold |
|--------|--------|---------------|
| Win rate | >55% | <45% |
| Average return per trade | >0.8% | <0.2% |
| Sharpe ratio (annualised) | >1.5 | <0.8 |
| Max drawdown | <15% | >25% |
| Average holding period | 3–6 days | — |
| Slippage-adjusted return | >0.5% | <0.0% |

### Slippage model

Apply realistic slippage estimates:
- Entry: 0.15% for liquid names (>$5M ADV), 0.4% for illiquid names (<$2M ADV)
- Exit: Same as entry (assume symmetric)
- Funding costs for perp positions: use historical funding rates from Hyperliquid/Binance

### Segmentation analysis

Break results down by:
- Constituent liquidity tier (high/medium/low ADV)
- Direction (long additions vs. short deletions)
- Market regime (bull/bear/sideways — use BTC 90-day trend as proxy)
- Impact Score bucket (0.5–1%, 1–2%, >2%)

**Hypothesis:** Edge should be strongest in low-liquidity constituents with high Impact Score, and in bear/sideways markets where fewer momentum traders are competing for the same move.

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

- [ ] Backtest covers ≥30 rebalance events across ≥2 products
- [ ] Slippage-adjusted Sharpe > 1.5 in backtest
- [ ] Win rate > 55% in backtest
- [ ] Strategy is profitable in at least 2 of 3 market regimes (bull/bear/sideways)
- [ ] Data pipeline is automated and tested (no manual PDF parsing in production)
- [ ] Paper trading for ≥3 rebalance cycles with results matching backtest within 30%
- [ ] Execution tested: entry/exit slippage in paper trading matches model assumptions
- [ ] Legal review: confirm no regulatory issues with front-running public index data in target jurisdictions (this is legal in crypto; confirm for completeness)

---

## Kill Criteria

**Immediate kill (stop trading, review):**

- Any single trade loses >5% (suggests timing or data error)
- Two consecutive rebalance events produce negative returns
- AP execution window becomes unobservable (issuer stops publishing daily holdings)
- Issuer changes methodology to obscure rebalance dates or weights

**Strategy retirement (permanent kill):**

- Rolling 6-month Sharpe drops below 0.5
- Average slippage-adjusted return per trade drops below 0.2% (edge has been arbitraged away)
- Competing products with larger AUM begin tracking the same indices (increases efficiency)
- Regulatory change requires ETPs to randomise execution timing (would destroy the edge)

---

## Risks

### Risk 1: AP execution timing uncertainty ⚠️ HIGH
**Description:** We know the rebalance window (e.g., "first 3 business days of the month") but not the exact hour or day within that window. AP may execute on day 1 or day 3.
**Mitigation:** Enter T-3 (before window opens). Accept that we may hold through the full window. Hard exit at T+3 regardless.
**Residual risk:** If AP executes on day 1 and we entered T-3, we hold through the move and exit into mean reversion. This is the primary P&L risk.

### Risk 2: AP execution via OTC/dark pools ⚠️ MEDIUM
**Description:** Large APs (Coinbase Prime, Cumberland) may execute rebalance trades OTC or via dark pools, creating no visible price impact on public markets.
**Mitigation:** Focus on smaller products where OTC execution is less likely (smaller trade sizes). Monitor for volume spikes as confirmation signal.
**Residual risk:** If OTC execution is common, the strategy has no edge. This must be validated in backtest by checking whether price impact is actually observable.

### Risk 3: Methodology changes ⚠️ MEDIUM
**Description:** Issuer changes index methodology (constituent universe, rebalance frequency, weight calculation) without adequate notice.
**Mitigation:** Monitor issuer websites for methodology updates. Subscribe to product newsletters. Hard exit rule if methodology changes after entry.
**Residual risk:** Sudden methodology change mid-cycle could invalidate open positions.

### Risk 4: AUM too small for meaningful impact ⚠️ MEDIUM
**Description:** If product AUM shrinks (redemptions, market downturn), the dollar flow per constituent may fall below the threshold needed to move prices.
**Mitigation:** Recompute Impact Score before every rebalance event. Do not trade if Impact Score < 0.5%.
**Residual risk:** AUM can decline between our calculation date and execution date.

### Risk 5: Constituent liquidity deterioration ⚠️ LOW-MEDIUM
**Description:** Small-cap constituents can experience sudden liquidity drops (exchange delistings, protocol issues), making exit difficult.
**Mitigation:** Only trade constituents with >$500K daily volume. Monitor for exchange delisting announcements.
**Residual risk:** Flash crashes in illiquid constituents during holding period.

### Risk 6: Strategy crowding ⚠️ LOW (currently)
**Description:** If this strategy becomes widely known, front-runners of the front-runners emerge, compressing the edge.
**Mitigation:** Monitor pre-rebalance price action for evidence of earlier positioning. If prices move significantly before T-3, the edge is being front-run.
**Residual risk:** Gradual edge decay as crypto markets mature. Monitor via rolling Sharpe.

### Risk 7: Correlation with broader crypto market ⚠️ LOW
**Description:** In a sharp market downturn, all constituents fall regardless of AP buying pressure. The structural edge is overwhelmed by macro flow.
**Mitigation:** Position sizing limits (15% max portfolio exposure). Consider hedging with BTC short during high-volatility periods.
**Residual risk:** Strategy is not market-neutral; it has positive beta during rebalance windows.

---

## Open Questions (Pre-Backtest)

These must be answered before the backtest is meaningful:

1. **Is historical holdings data available for 21Shares products?** Bitwise publishes this; 21Shares availability is unclear. If not available, backtest is limited to BITW.

2. **What fraction of AP execution happens OTC vs. on public CEXes?** This is the central empirical question. If >50% is OTC, the strategy has no observable edge.

3. **What is the actual execution window?** Methodology says "first 3 business days" — does AP consistently execute on day 1, or is it spread? This determines optimal entry/exit timing.

4. **Is the price impact statistically significant in historical data?** Run event study: average constituent returns T-3 to T+3 for additions vs. deletions. If no signal, kill immediately.

5. **Do multiple products rebalance simultaneously?** If BITW and 21Shares DeFi ETP both rebalance on the same date, does the combined flow amplify the signal?

---

## Next Steps

| Step | Action | Owner | Timeline |
|------|--------|-------|----------|
| 1 | Audit data availability: confirm 21Shares historical holdings accessibility | Researcher | Week 1 |
| 2 | Build data pipeline: NAV scraper + CoinGecko price/volume pull | Engineer | Week 1–2 |
| 3 | Reconstruct historical rebalance events for BITW (2022–present) | Researcher | Week 2 |
| 4 | Run event study: constituent returns T-3 to T+3 around rebalance dates | Researcher | Week 3 |
| 5 | If event study shows signal: build full backtest with slippage model | Engineer | Week 4–5 |
| 6 | Segment analysis by liquidity tier, direction, market regime | Researcher | Week 5 |
| 7 | Paper trade next 3 rebalance cycles | Trader | Month 2–3 |
| 8 | Go/no-go decision based on paper trading results | PM | Month 3 |

---

## Relationship to Other Zunid Strategies

**Complementary to token unlock shorts:** Both strategies exploit predictable, scheduled supply/demand events. Token unlocks are supply shocks; ETP rebalances are demand shocks (for additions) or supply shocks (for deletions). They can run simultaneously with low correlation.

**Shares the "forced action" thesis:** The AP is a forced actor — they must execute regardless of market conditions. This is the same structural logic as a vesting cliff or an unbonding queue. The constraint creates the edge.

**Differentiated from DPI front-run (Candidate 1):** DPI is on-chain (transparent execution, front-runnable by bots); this strategy targets TradFi-wrapped products where execution is off-chain and less bot-accessible. Different execution environment, same structural logic.

---

*This document is a hypothesis. No backtest has been run. Do not allocate capital until go-live criteria are satisfied. All AUM figures are estimates and must be verified before use.*
