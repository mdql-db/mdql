---
title: "FASB Fair Value Wave — Corporate Crypto Quarter-End Rebalancing"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 5
frequency: 2
composite: 240
categories:
  - calendar-seasonal
  - regulatory
  - exchange-structure
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Public companies holding BTC on their balance sheet under ASC 350-60 (FASB fair value accounting, mandatory for fiscal years beginning after Dec 15, 2024) must record unrealised gains and losses each quarter. When aggregate unrealised gains across known corporate holders are large (>20% above blended cost basis), CFOs face a menu of incentives to reduce exposure before quarter-end: smoothing earnings volatility, managing debt covenant ratios tied to reported asset values, and avoiding analyst scrutiny of large mark-to-market swings. This creates a **predictable, calendar-driven selling window** in the 5–7 days before fiscal quarter-end.

**Causal chain:**

1. FASB ASC 350-60 is law → companies must revalue crypto each quarter-end
2. Large unrealised gains → earnings volatility risk, covenant pressure, CFO incentive to reduce position
3. CFO action window is bounded → must execute before quarter-close to affect reported figures
4. Aggregate corporate BTC holdings are publicly disclosed → signal is constructable in advance
5. Selling pressure from even a subset of holders → short-term price suppression → mean reversion after quarter-end

**Null hypothesis to disprove:** BTC price behaviour in the 7 days before fiscal quarter-end is indistinguishable from any other 7-day window.

---

## Structural Mechanism

### Why this *should* happen (not *must*)

This is a **regulatory pressure edge**, not a contractually guaranteed one. The distinction matters:

- The FASB rule **forces revaluation** — that part is mandatory
- The **selling decision** remains at CFO discretion — that part is probabilistic
- The **timing** is bounded by the reporting calendar — that part is fixed

The mechanism is strongest when:
- Aggregate unrealised gain is high (CFO has most to "protect" by selling)
- Companies are near debt covenants (forced reduction, not discretionary)
- The holding is large relative to the company's market cap (material to earnings)

**Known corporate holders as of early 2025 (seed list):**

| Company | Ticker | Approx BTC Holdings | Avg Cost Basis (est.) |
|---|---|---|---|
| MicroStrategy (now Strategy) | MSTR | ~450,000 BTC | ~$30,000 |
| Tesla | TSLA | ~9,720 BTC | ~$32,000 |
| Marathon Digital | MARA | ~40,000 BTC | ~$18,000 |
| Coinbase | COIN | ~9,000 BTC | ~$20,000 |
| Block (Square) | SQ | ~8,027 BTC | ~$22,000 |

> Note: MicroStrategy has explicitly stated it will not sell. Exclude from "likely seller" calculation but include in aggregate unrealised gain metric as a market sentiment signal.

**Mechanism weakens when:**
- Companies hedge with derivatives (no spot selling, no price impact)
- Companies have already disclosed a HODL policy in 10-K risk factors
- BTC is falling into quarter-end (unrealised losses → no selling incentive)

---

## Entry / Exit Rules

### Pre-trade checklist (run 10 days before each quarter-end)

1. Pull latest disclosed BTC holdings from SEC EDGAR for all tracked companies
2. Calculate **aggregate unrealised gain** = Σ[(current BTC price − avg cost basis) × BTC held] across all non-HODL-committed holders
3. Calculate **gain ratio** = aggregate unrealised gain / aggregate cost basis
4. Check whether any tracked company is within 15% of a disclosed debt covenant threshold (from 10-K/10-Q notes)

### Entry signal

**All three conditions must be true:**

- [ ] Gain ratio > 20% (aggregate unrealised gain exceeds 20% of cost basis)
- [ ] At least one company is within 15% of a debt covenant tied to asset values or leverage
- [ ] BTC spot price has not fallen >10% in the prior 14 days (falling market removes selling incentive)

**Entry:** Short BTC-USDC perpetual on Hyperliquid at market open, **7 calendar days before fiscal quarter-end** (typically March 24, June 24, Sept 24, Dec 24 — adjust for weekends)

### Exit rules

**Take profit:** Cover at quarter-end close (last hour of trading on the final day of the fiscal quarter, typically March 31, June 30, Sept 30, Dec 31)

**Mean reversion exit:** If BTC drops >8% from entry before quarter-end, cover 50% of position and trail stop on remainder at entry price

**Stop loss:** Hard stop at +4% adverse move from entry price (i.e., BTC rises 4% above entry → exit full position)

**Time stop:** If 7 days pass and price has not moved >2% in either direction, exit at market — the signal did not materialise

### Post-quarter re-entry (optional)

After quarter-end, monitor for mean reversion long: if BTC dropped >5% into quarter-end, enter long on the first trading day of the new quarter with a 3-day hold window. Hypothesis: selling pressure dissipates immediately after reporting deadline.

---

## Position Sizing

**Base position:** 2% of portfolio NAV per trade

**Scaling rules:**
- If gain ratio > 40%: scale to 3% NAV
- If gain ratio > 60%: scale to 4% NAV (cap)
- If only one condition is met (not all three): do not trade

**Leverage:** 2–3x maximum on Hyperliquid perp. This is a low-conviction, calendar-driven trade — not a high-conviction structural arb. Do not over-lever.

**Correlation note:** If running token unlock shorts simultaneously, reduce this position by 50% during overlapping windows (both strategies are short BTC, correlation risk is additive).

---

## Backtest Methodology

### Data sources

| Data | Source | URL / Endpoint |
|---|---|---|
| BTC daily OHLCV | Binance public API | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d` |
| Corporate BTC holdings history | SEC EDGAR full-text search | `https://efts.sec.gov/LATEST/search-index?q=%22bitcoin%22&dateRange=custom&startdt=2020-01-01&forms=10-K,10-Q,8-K` |
| Fiscal year calendars | SEC EDGAR company facts | `https://data.sec.gov/submissions/CIK{number}.json` |
| Debt covenant disclosures | SEC EDGAR 10-K filings (manual review) | EDGAR full-text search for "covenant" + company ticker |
| BTC cost basis per company | 10-K/10-Q disclosures + Bitcoin Treasuries | `https://bitcointreasuries.net` |

### Backtest period

**Primary:** Q1 2021 – Q4 2024 (16 quarters; pre-FASB but tests the behavioural hypothesis)

**Note on pre-FASB data:** Before ASC 350-60, companies used impairment-only accounting (could only write down, not write up). The selling incentive was *weaker* pre-2025. Expect the signal to be noisier in historical data. The backtest tests whether the *behavioural pattern* existed even under weaker incentives — if it did, the new rule should strengthen it.

**Post-FASB live period:** Q1 2025 onward — treat as out-of-sample validation

### Backtest construction

1. For each quarter-end from Q1 2021 to Q4 2024:
   - Reconstruct corporate BTC holdings as of 10 days before quarter-end using EDGAR filings dated prior to that window
   - Calculate gain ratio using BTC price 10 days before quarter-end
   - Record whether entry signal triggered (gain ratio > 20%)
   - Record BTC return from entry day (T-7) to quarter-end (T-0)
   - Record BTC return from T-0 to T+3 (post-quarter mean reversion test)

2. Compare signal-triggered windows vs. all other 7-day windows (baseline)

3. Calculate:
   - Win rate (% of triggered trades that were profitable)
   - Average return per trade (gross, before fees)
   - Sharpe ratio of trade series
   - Maximum drawdown within trade windows
   - Slippage estimate: assume 0.05% per side on Hyperliquid perp

### Key metrics to compute

| Metric | Target threshold |
|---|---|
| Win rate | > 55% |
| Average return per triggered trade | > 1.5% (net of 0.1% round-trip fees) |
| Sharpe (trade series) | > 0.8 |
| Max drawdown per trade | < 6% |
| Signal frequency | ≥ 6 triggered quarters out of 16 |

### Baseline comparison

Compare against: random 7-day short BTC windows (same number of trades, randomly sampled from the same period). If the strategy's average return is not statistically distinguishable from the random baseline at p < 0.10, the signal is noise.

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show **all** of the following:

1. Win rate ≥ 55% on triggered trades
2. Average net return per trade ≥ 1.5%
3. Strategy average return statistically different from random baseline (p < 0.10, one-tailed t-test)
4. No single losing trade exceeds −6% (confirms stop loss is functioning)
5. Signal triggered in at least 6 of 16 historical quarters (confirms it's not a one-off)
6. Post-quarter mean reversion (T+0 to T+3 long) shows positive expectancy in ≥ 60% of cases where the short was profitable — this validates the causal mechanism, not just the pattern

**Paper trade duration:** Minimum 2 live quarters (Q1 and Q2 2025) before allocating real capital.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| Two consecutive paper trade losses exceeding stop loss | Pause, re-examine mechanism |
| Backtest win rate < 50% after full reconstruction | Kill — no edge |
| MicroStrategy and Marathon both publicly commit to no-sell policies | Reduce universe; re-score to 3/10 |
| BTC ETF options market shows large put buying before quarter-end (suggests hedging, not spot selling) | Mechanism has shifted; kill spot-selling hypothesis |
| FASB issues guidance allowing derivative hedging to offset mark-to-market (reducing selling incentive) | Kill — structural mechanism weakened |
| Three consecutive live quarters with no signal trigger (gain ratio never exceeds 20%) | Strategy is dormant; suspend until BTC price recovers above aggregate cost basis |

---

## Risks

### Critical risks (could invalidate the strategy entirely)

**1. CFO discretion breaks the chain**
The FASB rule forces *revaluation*, not *selling*. Most CFOs will simply disclose the mark-to-market and move on. The selling hypothesis requires a specific CFO incentive (covenant pressure, earnings smoothing) that may not exist at any given quarter-end. **Mitigation:** Only trade when covenant pressure signal is present.

**2. Derivative hedging absorbs the pressure**
Sophisticated treasury teams (Tesla, Coinbase) may hedge with BTC options or futures rather than selling spot. This creates no price impact on spot/perp markets. **Mitigation:** Monitor options open interest and put/call ratio in the 2 weeks before quarter-end; if large put buying is visible, the mechanism has shifted and the trade should not be entered.

**3. MicroStrategy dominates the signal but won't sell**
MSTR holds ~450,000 BTC — roughly 40–50% of all known corporate holdings. Their unrealised gain dominates the gain ratio calculation, but they have explicitly committed to a HODL strategy. **Mitigation:** Exclude MSTR from the "likely seller" calculation. Use MSTR holdings only as a sentiment/market signal, not as a selling pressure signal.

**4. Rule is too new for clean historical data**
ASC 350-60 is mandatory from fiscal years beginning after Dec 15, 2024. Pre-2025 data tests a weaker version of the incentive. The backtest may show no signal, not because the mechanism is wrong, but because the incentive was weaker. **Mitigation:** Treat pre-2025 backtest as a lower bound; expect the live signal to be stronger.

**5. BTC market is too large for corporate selling to move price**
Even if every tracked company sold their entire holdings simultaneously, the aggregate (~$30–40B at $80K BTC) is a fraction of daily BTC volume (~$20–30B/day on major exchanges). **Mitigation:** The strategy does not require large price moves — a 2–4% directional drift is sufficient for the target return. Corporate selling may be one of several factors creating a directional bias, not the sole cause.

### Moderate risks

- **Earnings management in the opposite direction:** Companies with large unrealised *losses* near quarter-end may *buy* BTC to reduce the reported loss (unlikely but possible for companies with flexible treasury mandates)
- **Fiscal year misalignment:** Not all S&P 500 companies have Dec 31 fiscal year-ends. Microsoft (June), Apple (September) — need to track each company's specific fiscal calendar
- **Signal crowding:** If this strategy becomes known, other traders front-run the quarter-end window, pulling the effect earlier and reducing the edge

---

## Data Sources

| Resource | URL | Notes |
|---|---|---|
| SEC EDGAR full-text search | `https://efts.sec.gov/LATEST/search-index?q=%22bitcoin%22&forms=10-K,10-Q,8-K` | Search for BTC holding disclosures |
| SEC EDGAR company submissions API | `https://data.sec.gov/submissions/CIK0000789019.json` | Replace CIK with target company |
| SEC EDGAR company facts API | `https://data.sec.gov/api/xbrl/companyfacts/CIK0000789019.json` | XBRL financial data |
| Bitcoin Treasuries tracker | `https://bitcointreasuries.net` | Aggregated corporate holdings; cross-check against EDGAR |
| Binance OHLCV API | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d&limit=1000` | BTC daily price history |
| Hyperliquid perp data | `https://app.hyperliquid.xyz/trade/BTC` | Live trading; historical data via `https://hyperliquid.xyz/api` |
| FASB ASC 350-60 text | `https://asc.fasb.org/350-60` | Rule reference; requires FASB subscription |
| FASB ASU 2023-08 summary | `https://www.fasb.org/page/PageContent?pageId=/standards/accounting-standards-updates/2023-08.html` | Free summary of the rule |
| Fiscal year end calendar | SEC EDGAR `dei:DocumentPeriodEndDate` XBRL tag | Pull via company facts API |

---

## Implementation Notes

### Monitoring workflow (run quarterly, 10 days before each quarter-end)

```
1. Pull latest 10-Q/8-K for each tracked company from EDGAR API
2. Extract: BTC quantity held, average cost basis, any covenant disclosures
3. Fetch BTC spot price from Binance API
4. Calculate gain ratio for non-HODL-committed holders
5. Check covenant proximity (manual review of 10-K notes section)
6. If all three entry conditions met → set calendar alert for T-7 entry
7. At T-7: enter short on Hyperliquid BTC-USDC perp, 2-4% NAV, 2-3x leverage
8. Monitor daily; apply stop loss and time stop rules
9. Exit at T-0 (quarter-end close)
10. Log result; update backtest database
```

### Tracked company seed list (expand as new disclosures emerge)

Monitor EDGAR for new 8-K filings containing "bitcoin" or "digital asset" from S&P 500 and Russell 1000 companies. New corporate adopters under ASC 350-60 will file 8-Ks disclosing initial adoption — these are new signal sources.

---

*This document is a hypothesis specification. No backtest has been run. Do not allocate capital until go-live criteria are met.*
