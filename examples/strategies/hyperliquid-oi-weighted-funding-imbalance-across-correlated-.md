---
title: "Hyperliquid OI-Weighted Funding Imbalance Across Correlated Assets"
status: HYPOTHESIS
mechanism: 6
implementation: 7
safety: 6
frequency: 10
composite: 2520
categories:
  - funding-rates
  - basis-trade
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a macro move hits crypto, retail flow concentrates unevenly across correlated assets — one ticker absorbs the directional crowd while a correlated peer remains relatively unaffected. This creates a funding rate differential between assets that historically move together. The crowded leg pays elevated funding (a guaranteed, metered cost) while the uncrowded leg pays near-zero or negative funding. A pairs trade — short the high-funding leg, long the low-funding leg — earns the funding differential as carry while the spread between two correlated assets mean-reverts. The structural edge is the funding rate itself: it is a contractually enforced transfer payment that drains the crowded side every 8 hours regardless of price direction. The correlation provides a second force — a center of gravity that the spread must eventually return to. Neither element alone guarantees convergence on a fixed schedule, but together they create a carry-positive trade with a mechanical reversion pull.

---

## Structural Mechanism

**Layer 1 — Funding rate as guaranteed carry:**
Hyperliquid perpetual funding rates are settled every 8 hours. When a position is short on a high-funding-rate asset, the protocol transfers the funding payment from longs to shorts. This is not probabilistic — it is a smart-contract-enforced cash flow. If the funding rate on Asset A is +0.15%/8hr and Asset B is +0.02%/8hr, the short-A/long-B pair earns +0.13%/8hr in carry, compounding every 8 hours the differential persists. At 0.13%/8hr, the annualised carry is approximately 142% — the carry alone justifies the trade if the spread does not move adversely.

**Layer 2 — Correlation as spread gravity:**
Two assets with 30-day rolling daily return correlation >0.85 share a common factor (broad crypto beta, L1 narrative, macro risk-on/off). Their price ratio has a statistical center of gravity. When retail flow pushes one asset's OI and funding to extremes, the price of that asset is being bid up relative to its correlated peer by leveraged longs — not by a fundamental divergence. As funding drains those longs, or as the narrative normalises, the price ratio reverts. This is not guaranteed to happen within any fixed window, which is why the score is 7 not 9.

**Layer 3 — OI imbalance as a leading indicator of funding persistence:**
High OI on the crowded leg means more longs paying funding. As long as OI remains elevated, the carry continues. OI data on Hyperliquid is public and real-time. A high-OI/high-funding combination signals that the carry is likely to persist for multiple funding periods, not just one.

**Why this is less competed:**
Most funding arb strategies are single-asset (long spot, short perp on the same asset). This strategy is cross-asset, requiring correlation tracking and pairs infrastructure. The additional complexity filters out most participants. Hyperliquid-specific data (OI, funding) is less widely scraped than Binance data, reducing competition further.

---

## Universe Definition

**Eligible pairs:**
- Must both be listed as perpetual futures on Hyperliquid.
- 30-day rolling daily return correlation must exceed 0.85 at signal time.
- Both assets must have minimum 24h volume > $5M on Hyperliquid to ensure fills.
- Exclude stablecoin pairs, wrapped asset pairs (e.g., WBTC/BTC), and any pair where one leg is a leveraged token.

**Candidate pair groups (to be validated in backtest):**
- BTC / ETH
- ETH / SOL
- ETH / AVAX
- SOL / AVAX
- BTC / SOL
- Any L1 pair with correlation >0.85 in the backtest window

**Pair selection at signal time:** If multiple pairs qualify simultaneously, rank by funding rate differential (highest differential first) and take the top 2 pairs maximum to limit concentration.

---

## Signal Definition

**Primary signal — Funding Rate Differential (FRD):**
```
FRD = Funding_Rate_Asset_A (8hr) - Funding_Rate_Asset_B (8hr)
```
Where Asset A is the high-funding leg (to be shorted) and Asset B is the low-funding leg (to be longed).

**Entry trigger:** FRD > +0.10%/8hr AND 30-day rolling correlation > 0.85.

**Confirmation filter (reduces false entries):**
- OI on Asset A has increased >15% in the prior 24 hours (confirms crowding is fresh, not stale).
- Funding rate on Asset A has been elevated (>0.05%/8hr) for at least two consecutive 8-hour periods (confirms persistence, not a spike).

**Signal check frequency:** Every 8 hours, aligned with funding settlement times (00:00, 08:00, 16:00 UTC).

---

## Entry Rules

1. At the first funding settlement after all entry conditions are met, enter simultaneously:
   - **Short** Asset A (high funding) on Hyperliquid perp.
   - **Long** Asset B (low funding) on Hyperliquid perp.
2. Enter at market using limit orders within 0.1% of mid to avoid slippage on illiquid books.
3. Size both legs to equal **notional USD value** at entry (dollar-neutral, not beta-neutral — see sizing section).
4. Record entry prices, entry funding rates, entry OI for both legs, and entry correlation.
5. Maximum 2 pairs open simultaneously.

---

## Exit Rules

**Exit condition 1 — Funding convergence (primary target):**
FRD closes to < 0.02%/8hr. Exit both legs at market within the next 8-hour window.

**Exit condition 2 — Time stop:**
7 calendar days elapsed since entry regardless of FRD. Exit both legs. Rationale: funding differentials that persist beyond 7 days without convergence suggest a structural regime shift (e.g., one asset has decoupled from the pair), not a temporary imbalance.

**Exit condition 3 — Spread adverse move stop:**
Net P&L on the combined pair position reaches -3% of initial notional. Exit both legs immediately. This stop is on the combined position, not each leg individually — one leg moving against you is acceptable if the other leg offsets.

**Exit condition 4 — Correlation breakdown:**
If 30-day rolling correlation drops below 0.70 during the trade, exit within the next funding period. The spread gravity assumption has broken down.

**Partial exit rule:** If FRD drops to <0.05%/8hr (halfway to target) AND the spread has moved in your favour by >1%, close 50% of the position to lock in gains and let the remainder run to full convergence.

---

## Position Sizing

**Base size:** 2% of total portfolio notional per pair (1% per leg).

**Rationale:** The -3% stop on the pair means maximum loss per trade is approximately 3% × 2% = 0.06% of portfolio per trade. This is conservative given the carry income partially offsets adverse spread moves.

**Leverage:** Use 3x leverage maximum on each leg. At 3x, a 1% adverse move in the spread costs 3% of margin — consistent with the stop loss. Do not use cross-margin; use isolated margin on each leg to prevent one leg's loss from liquidating the other.

**Notional cap:** No single pair trade should exceed $50,000 notional (combined both legs) until the strategy has 20+ completed trades in live paper trading with positive expectancy confirmed.

**Scaling rule:** After 20 completed trades with Sharpe > 1.0 in paper trading, scale to 5% of portfolio per pair, maximum 3 pairs simultaneously.

---

## Backtest Methodology

**Data required:**
- Hyperliquid historical funding rates: pull from Hyperliquid public API (`/info` endpoint, `fundingHistory` field) for all available assets.
- Hyperliquid historical OHLCV: pull from public API for daily close prices to compute rolling correlations.
- Hyperliquid historical OI: pull from public API (`openInterest` field in market data).
- Data availability: Hyperliquid launched in 2023; expect 12–18 months of usable history as of mid-2024.

**Backtest steps:**

1. **Build funding rate database:** For each asset on HL, collect all 8-hour funding rate observations. Flag any gaps or anomalies (funding rate > 1%/8hr is likely a data error or extreme event — investigate separately).

2. **Build correlation matrix:** Compute 30-day rolling pairwise correlations for all eligible pairs using daily close returns. Store the correlation value at each signal check time.

3. **Identify all signal events:** For each 8-hour period, scan all pairs for FRD > 0.10%/8hr AND correlation > 0.85 AND OI increase > 15% in prior 24h AND funding elevated for 2+ consecutive periods.

4. **Simulate trades:** For each signal event, simulate entry at the next available price (use open of next 1-hour candle as fill approximation). Apply the exit rules in order of priority. Record: entry date, exit date, exit reason, gross P&L from spread, funding income earned, net P&L.

5. **Compute funding income separately:** For each 8-hour period the trade is open, add FRD × notional to P&L. This is the carry component. Separate carry P&L from spread P&L to understand which component drives returns.

6. **Slippage assumption:** Apply 0.05% slippage per leg per side (entry and exit) = 0.20% total round-trip slippage cost per pair trade.

7. **Key metrics to compute:**
   - Win rate (% of trades with net positive P&L)
   - Average carry earned per trade (in % of notional)
   - Average spread P&L per trade
   - Average holding period
   - Sharpe ratio (annualised)
   - Maximum drawdown
   - Breakdown of exit reasons (convergence vs. time stop vs. spread stop vs. correlation breakdown)
   - Performance by pair (BTC/ETH vs. ETH/SOL etc.)

8. **Stress test:** Re-run backtest with FRD threshold raised to 0.15%/8hr and lowered to 0.07%/8hr to test sensitivity to the entry threshold.

9. **Correlation sensitivity test:** Re-run with correlation threshold at 0.80 and 0.90 to test how much the correlation filter matters.

---

## Go-Live Criteria

All of the following must be satisfied before committing real capital:

1. Backtest shows positive net P&L after slippage on at least 30 completed trades.
2. Backtest Sharpe ratio > 1.0 annualised.
3. Backtest maximum drawdown < 5% of strategy allocation.
4. Carry income (funding differential earned) is positive in >80% of trades — confirms the structural mechanism is working, not just lucky spread moves.
5. No single pair accounts for >60% of total backtest P&L (concentration risk).
6. Paper trading on Hyperliquid testnet or with small real size ($500 notional per leg) for minimum 30 days with at least 5 completed trades showing positive expectancy.
7. Correlation stability confirmed: the 30-day rolling correlation for the top pairs has not dropped below 0.80 for more than 5 consecutive days in the backtest period.

---

## Kill Criteria

Stop trading this strategy immediately if any of the following occur:

1. **Live trading drawdown > 5%** of strategy allocation (not per-trade, total strategy).
2. **Three consecutive losing trades** where the carry was positive but spread moved adversely — suggests correlation has broken down structurally, not temporarily.
3. **Average funding differential at entry drops below 0.07%/8hr** across all signals in a rolling 30-day window — the opportunity set has compressed, likely due to competition or regime change.
4. **Hyperliquid changes its funding rate mechanism** (e.g., moves to time-weighted average, changes settlement frequency) — the structural mechanism must be re-evaluated from scratch.
5. **Correlation between top pairs drops below 0.75** on a 30-day rolling basis for more than 10 consecutive days — the pairs universe has broken down.
6. **OI data or funding rate data becomes unavailable** from the public API — the strategy cannot be monitored without this data.

---

## Risks

**Risk 1 — Spread divergence before convergence (primary risk):**
The high-funding asset can continue to be bid up by new longs even as existing longs pay funding. If a genuine narrative divergence occurs (e.g., SOL gets a major protocol upgrade while ETH does not), the spread may not revert within the 7-day window. The -3% stop and 7-day time stop are the mitigants. Magnitude: moderate. Frequency: low if correlation filter is respected.

**Risk 2 — Funding rate spike on the long leg:**
If the low-funding leg (the long) suddenly attracts its own crowd and funding spikes, the carry advantage narrows or reverses. Monitor funding on both legs every 8 hours. If the long leg's funding rises above 0.05%/8hr, reassess the trade. Magnitude: low to moderate. Frequency: moderate.

**Risk 3 — Liquidity / slippage on smaller HL markets:**
AVAX, smaller L1s on Hyperliquid may have thin order books. A 2% notional position in a thin market can move the price on entry/exit. Mitigant: enforce the $5M/24h volume minimum. Stick to BTC, ETH, SOL pairs initially. Magnitude: low for large pairs, high for small pairs. Frequency: low if volume filter is applied.

**Risk 4 — Correlation instability:**
Crypto correlations are notoriously unstable. A 30-day rolling correlation of 0.85 can drop to 0.60 within days during a sector rotation or idiosyncratic event. The correlation breakdown exit rule (exit if correlation drops below 0.70 during the trade) is the primary mitigant. Magnitude: moderate. Frequency: moderate.

**Risk 5 — Crowded strategy / competition:**
If this strategy becomes widely known, the funding differential will be arbitraged away faster, reducing the window between signal and convergence. Monitor average holding period: if it drops below 24 hours consistently, competition has increased. Magnitude: moderate long-term. Frequency: low initially, increasing over time.

**Risk 6 — Hyperliquid-specific smart contract or operational risk:**
Hyperliquid is a relatively new exchange. Smart contract bugs, oracle failures, or exchange downtime could prevent exits. Mitigant: use isolated margin, never exceed 3x leverage, maintain a manual monitoring schedule every 8 hours aligned with funding settlements. Magnitude: potentially high (total loss of margin). Frequency: low but non-zero.

**Risk 7 — Funding rate manipulation or anomalies:**
Extremely high funding rates (>0.5%/8hr) may indicate a market anomaly, oracle issue, or deliberate manipulation rather than genuine crowding. Apply a sanity cap: do not enter if either leg's funding rate exceeds 0.5%/8hr — investigate manually first. Magnitude: moderate. Frequency: rare.

---

## Data Sources

| Data Type | Source | Endpoint / Method | Cost |
|---|---|---|---|
| Hyperliquid funding rates (historical) | Hyperliquid public API | `POST /info` with `type: fundingHistory` | Free |
| Hyperliquid OHLCV (historical) | Hyperliquid public API | `POST /info` with `type: candleSnapshot` | Free |
| Hyperliquid open interest (real-time) | Hyperliquid public API | `POST /info` with `type: metaAndAssetCtxs` | Free |
| Hyperliquid order book depth | Hyperliquid public API | `POST /info` with `type: l2Book` | Free |
| Cross-exchange funding rate comparison | Coinglass.com | Web UI or API (free tier) | Free / $29/mo for API |
| Correlation validation | Built from OHLCV above | Rolling pandas `.corr()` on daily returns | Free |

**Data pipeline recommendation:** Build a Python script that pulls funding rates and OHLCV from the Hyperliquid API every 8 hours, stores to a local SQLite or Postgres database, computes rolling correlations and FRD, and sends an alert (Telegram bot or email) when entry conditions are met. This pipeline is buildable in <200 lines of Python using the `hyperliquid-python-sdk` library.

---

## Open Questions for Backtest Phase

1. What is the empirical distribution of FRD values across all HL pairs historically? Is 0.10%/8hr the right threshold, or does it filter out too many trades?
2. How often does the spread move adversely by >3% before funding convergence occurs? This determines whether the carry income is sufficient to offset stop-loss frequency.
3. Which specific pairs show the most stable correlation (>0.85) over rolling 30-day windows? BTC/ETH is the hypothesis; this needs empirical confirmation on HL specifically.
4. What is the average duration of elevated funding differentials (>0.10%/8hr) historically? If the average is <24 hours, the 7-day time stop is too loose and should be tightened.
5. Does OI increase >15% in 24h add predictive value for funding persistence, or is it redundant given the 2-period funding confirmation filter?

---

*This document is a hypothesis specification. No backtest results exist yet. All claims about mechanism are logical deductions from protocol design, not empirically validated. Proceed to backtest (step 3 of 9) before allocating capital.*
