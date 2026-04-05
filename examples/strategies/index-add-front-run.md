---
title: "Strategy Specification: Index Add Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 7
safety: 6
frequency: 2
composite: 504
categories:
  - index-rebalance
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a token is announced for addition to a major crypto index (CoinDesk 20, CF Benchmarks, Bloomberg Galaxy Crypto Index, etc.), passive funds and ETPs tracking those indices are obligated to purchase the token before the effective rebalancing date. This creates a predictable, time-bounded demand shock. Informed market participants who front-run this demand should be able to buy at announcement and sell into the passive fund buying pressure before or at rebalancing completion.

**The edge, if it exists, comes from:**

1. **Mechanical, non-discretionary demand**: Passive funds cannot defer their purchase. The buying is guaranteed and schedule-bound.
2. **Announcement pre-visibility**: The gap between announcement and effective date (typically 3–7 days) gives a defined entry window.
3. **Equities precedent**: The S&P 500 add effect is one of the most documented anomalies in academic finance (Shleifer 1986, Harris & Gurel 1986). Additions see average 3–8% abnormal returns in the announcement-to-effective window, with partial reversal after.
4. **Crypto-specific amplification candidates**: Lower liquidity tokens with larger index weight requirements may show amplified effects relative to equities.

**Primary concern**: Crypto index AUM is orders of magnitude smaller than equity index AUM. The CoinDesk 20 Index underlies a small number of products; total passive AUM tracking it is estimated in the low hundreds of millions USD versus individual token daily spot volumes often exceeding $500M–$10B. The signal-to-noise ratio may be insufficient to produce a measurable, tradeable effect. This is the central hypothesis to disprove or confirm in backtesting.

---

## Indices in Scope

| Index | Provider | Announcement Lead Time | Rebalancing Frequency | Notes |
|---|---|---|---|---|
| CoinDesk 20 (CD20) | CoinDesk Indices | ~5 business days | Quarterly | Formerly CoinDesk Market Index |
| CF Cryptocurrency Ultra Cap 5 | CF Benchmarks | ~5 business days | Monthly | Underlies CME products |
| Bloomberg Galaxy Crypto Index (BGCI) | Bloomberg / Galaxy | ~5 business days | Monthly | Limited passive AUM |
| Nasdaq Crypto Index (NCI) | Nasdaq | ~5 business days | Quarterly | Underlies Hashdex ETF |
| S&P Cryptocurrency Broad Digital Market | S&P / Lukka | ~5 business days | Quarterly | Nascent passive AUM |

**Initial focus**: CoinDesk 20 and Nasdaq Crypto Index, as these have the clearest ETF/ETP linkage and announcement paper trail.

---

## Backtest Methodology

### 3.1 Data Collection (Manual Phase First)

This is a bespoke, hand-curated dataset. There is no API for index addition announcements. The following sources must be scraped and manually verified:

**Announcement dates:**
- CoinDesk Indices official press releases and methodology documents: `indices.coindesk.com`
- Nasdaq Global Index Watch publications
- CF Benchmarks monthly rebalancing notices: `cfbenchmarks.com/data/indices`
- Bloomberg terminal alerts (if available)
- Archive.org snapshots of index constituent pages for historical verification
- Crypto press (The Block, Coindesk news) — used for cross-verification only, not as primary source

**Each event must record:**
- Token ticker and name
- Index name
- Announcement date (T=0): date the addition was publicly disclosed
- Effective date (T=E): date the rebalancing takes effect
- Announcement-to-effective gap in calendar days
- Whether the token was simultaneously removed from another index (confound)
- Any concurrent market events (token unlock, major protocol news) — flagged manually

**Target dataset size**: All additions across the above 5 indices from inception to present. Realistic expectation: 30–80 events total. This is a small sample; statistical significance will be limited and must be acknowledged.

### 3.2 Price Data

- **Source**: CoinGecko API (historical OHLCV, free tier sufficient), cross-validated against Messari and Kaiko for liquid tokens
- **Granularity**: Daily OHLC for event study; 1-hour OHLC for intraday entry/exit analysis
- **Benchmark**: BTC daily return used as market return for abnormal return calculation
- **Fallback benchmark**: Equal-weighted top-20 crypto index (constructed from CoinGecko data) if BTC-adjusted returns appear noisy

### 3.3 Event Study Framework

Use standard finance event study methodology:

**Abnormal Return (AR) calculation:**

```
AR(t) = R_token(t) - R_benchmark(t)
```

Where R is log return and benchmark is BTC daily log return (simpler) or market model (preferred if sample allows):

```
Expected Return = alpha + beta * R_BTC
```

Estimate alpha and beta from a 60-day estimation window ending 10 days before announcement (T=-70 to T=-10) to avoid contamination.

**Cumulative Abnormal Return (CAR):**

```
CAR(T1, T2) = sum of AR(t) for t in [T1, T2]
```

**Windows to test:**

| Window | Label | Rationale |
|---|---|---|
| [-5, -1] | Pre-leak | Test for information leakage |
| [0, 0] | Announcement day | Immediate reaction |
| [0, E-1] | Announcement to day before effective | Core trade window |
| [0, E] | Full announcement-to-effective | Including rebalancing day |
| [E+1, E+10] | Post-effective reversal | Test for S&P-style reversal |
| [-5, E+10] | Full window | Full picture |

**Statistical tests**: Two-tailed t-test on mean CAR; Wilcoxon signed-rank test (non-parametric, preferred given small sample and non-normal crypto returns). Report both.

### 3.4 Transaction Cost Modeling

- **Taker fee**: 10 bps per side (0.10%) — conservative for major exchange taker
- **Slippage**: Modeled as 5 bps for tokens with >$50M daily volume; 20 bps for tokens with $10M–$50M daily volume; 50 bps for <$10M daily volume. Applied at both entry and exit.
- **Funding rate (if using perps)**: Use historical funding from Coinglass; cap position duration at 7 days so funding cost is bounded
- **Total round-trip cost estimate**: 30–120 bps depending on liquidity tier

### 3.5 Subgroup Analysis

Split the event set and test separately:

- By index (CD20 vs NCI vs others)
- By token market cap tier at announcement (large cap >$5B, mid $500M–$5B, small <$500M)
- By announcement-to-effective gap length (3 days vs 7 days)
- By whether a spot BTC ETF approval era is included (pre/post Jan 2024) as a regime break given increased index product adoption
- By direction of broader market (BTC up >5% in window vs down >5%) — is effect masked by market drawdowns?

---

## Entry Rules


### 4.1 Entry

- **Trigger**: Public announcement of token addition to target index, confirmed on primary source (index provider's official release or methodology update page)
- **Entry timing**: Open of next trading session following announcement confirmation (i.e., if announced during US market hours on day T, enter at T+1 open on spot). If announced outside market hours, enter next 00:00 UTC open.
- **Entry price**: Use opening price of T+1 candle (daily); for backtest purposes this is the open of the first full daily candle after confirmation
- **Instrument**: Spot preferred. Perpetual futures acceptable if spot liquidity is insufficient (defined as <$5M 30-day average daily volume on top-3 exchanges). If perps used, document funding rate drag.
- **Exchange**: Largest liquid venue for the token by volume at time of event (typically Binance, Coinbase, Bybit)

### 4.2 Exit

**Primary exit (E-1 close):**
Exit at the close of the trading day immediately before the effective rebalancing date. Rationale: capture the run-up driven by front-running before passive fund buying is complete, then step aside before any post-effective reversal.

**Secondary exit test (E close):**
Also backtest exiting at close of effective date. Passive funds may purchase throughout effective day, providing continued price support.

**Stop-loss:**
Hard stop at -8% from entry price (daily close basis). Rationale: if the trade is moving against by >8%, either the market regime overwhelms the effect or the announcement was misclassified. The stop prevents a large drawdown on a low-probability scenario where the token declines sharply post-announcement.

**Time stop:**
Exit at E+1 open regardless of P&L if the primary exit condition has not been triggered. Do not hold past effective date.

**Take profit:**
No fixed take-profit. Let the position run to the scheduled exit. The edge is the full announcement-to-effective drift, not a fast scalp.

### 4.3 Exit Priority Order

1. Stop-loss (any time)
2. Primary scheduled exit (E-1 close)
3. Time stop (E+1 open)

---

## Exit Rules

Defined within Entry Rules section.
## Position Sizing

- **Base position size**: 2% of portfolio per event
- **Rationale**: Small sample, unproven edge, binary event risk; cannot risk more than a minor allocation
- **Scaling by liquidity**: If 2% of portfolio exceeds 5% of the token's average daily spot volume (ADV), scale down to 5% ADV. Execution beyond this level causes self-inflicted slippage.
- **Concentration cap**: Maximum 4% of portfolio in open index-add positions simultaneously (unlikely given rebalancing calendar spacing, but cap is needed)
- **Leverage**: None in spot. If using perps, maximum 1x (no leverage). The edge is expected to be small; leverage introduces funding and liquidation risk that is not justified at hypothesis stage.
- **Kelly sizing**: Not applied at hypothesis stage. Will revisit after backtest if edge is confirmed and sample size is sufficient to estimate edge probability and magnitude.

---

## Go-Live Criteria

All of the following must be satisfied before committing real capital:

| Criterion | Threshold |
|---|---|
| Sample size | Minimum 20 events in backtest with data meeting quality standards |
| Mean CAR significance | p < 0.10 (two-tailed) for CAR[0, E-1] — relaxed threshold given small sample |
| Mean CAR magnitude | Net-of-costs CAR > 1.5% across all events; > 0.5% excluding top-2 outliers |
| Win rate | > 55% of events profitable net of costs |
| Max single-event loss | No individual event exceeds -15% (indicating model break, not just noise) |
| Out-of-sample period | At least 5 events held out from backtest development; validate signal holds on holdout set |
| Operational readiness | Announcement monitoring pipeline live; execution can occur within 2 hours of announcement during any time zone |

**Go-live allocation**: 0.5% per event initially (50% of base size), scaling to full 2% after 10 live events with positive P&L.

---

## Kill Criteria

Stop trading immediately and return to hypothesis status if any of the following occur:

| Criterion | Threshold |
|---|---|
| Consecutive losses | 5 consecutive losing trades |
| Cumulative drawdown on strategy allocation | -15% drawdown on the capital allocated to this strategy |
| Live event sample | After 15 live events, mean net CAR < 0.5% (strategy not delivering) |
| AUM of tracking products collapses | Passive AUM tracking target indices drops >50% (the mechanical demand disappears) |
| Market structure change | Index provider changes announcement window to <24 hours (front-run window closes) |
| Crowding signal | Entry-day price move on announcement exceeds 5% before our entry (signal fully front-run by faster participants) |

---

## Risks

### 8.1 Core Risks

**R1 — Insufficient passive AUM (Highest probability risk)**
The fundamental concern. If total passive AUM tracking the index is, say, $200M and the new token is 2% of the index, the required purchase is ~$4M. For a token with $500M daily volume, this is less than 1% of a single day's trading. The price impact will be indistinguishable from noise. Mitigant: None directly; this is the primary null hypothesis. Focus subgroup analysis on smaller, less liquid tokens where $4M represents a larger fraction of ADV.

**R2 — Announcement timing and information leakage**
The exact moment of public announcement is difficult to determine precisely. If the announcement is made after-hours and widely noticed by 9:00 AM UTC, the entry price at next open may already include most of the move. Mitigant: Analyze intraday data around announcement time; if the entire move occurs in the first 30 minutes post-announcement, execution is not feasible for a human-operated strategy.

**R3 — Small sample statistical fragility**
30–80 events is a very small sample for a strategy relying on mean effects. A few outlier events can dominate the mean CAR. One or two large positive outliers may give the appearance of an edge that does not generalize. Mitigant: Report median CAR and trimmed mean; use bootstrapped confidence intervals; test robustness by dropping top and bottom 2 events.

**R4 — Data quality and survivorship bias**
Historical announcement dates may be incomplete. If additions that had negative outcomes (token later delisted) are missing from the event set, the backtest will be upward biased. Mitigant: Cross-check against archive.org and multiple sources; explicitly list events excluded and why.

**R5 — Market beta dominance**
Crypto returns are highly correlated. A 5-day announcement window during a bull run will show positive CAR even for random tokens. The BTC-adjusted abnormal return calculation is the primary control, but crypto beta is unstable. Mitigant: Use market model with rolling beta; test in down-market subperiods explicitly.

**R6 — Crowding and strategy decay**
If this strategy becomes known and widely traded, the entry-day move will exhaust the available return immediately, making subsequent execution at favorable prices impossible. Mitigant: Monitor announcement-day return as a leading crowding indicator; include as kill criterion.

**R7 — Index methodology changes**
Providers can change announcement windows, weighting methodologies, or rebalancing frequency. A shift to same-day announcements would eliminate the strategy entirely. Mitigant: Subscribe to all index methodology update notifications; treat methodology change as automatic strategy review trigger.

**R8 — Execution risk on illiquid tokens**
Some additions may be low-cap tokens with thin order books. The 5% ADV position cap helps, but slippage in stressed conditions (announcement day volume spike) may be 2–3x normal estimates. Mitigant: Conservative slippage model in backtest; consider limit order entry rather than market order entry for tokens below $20M ADV.

### 8.2 Risk Summary Table

| Risk | Probability | Impact | Mitigant |
|---|---|---|---|
| Insufficient passive AUM | High | Kills strategy | Subgroup by liquidity; test empirically |
| Information leakage / fast front-run | Medium | Reduces edge | Intraday analysis of entry timing |
| Small sample fragility | High | Misleading backtest | Robust stats; holdout set |
| Data quality / survivorship | Medium | Upward bias | Manual audit; document exclusions |
| Market beta dominance | Medium | False positive signal | BTC-adjusted returns throughout |
| Crowding / strategy decay | Low (near term) | Eventual decay | Monitor announcement-day returns |
| Index methodology change | Low | Kills strategy | Monitor methodology updates |
| Execution slippage | Medium | Reduces edge | Conservative cost model; ADV cap |

---

## Data Sources

| Data Type | Source | Access Method | Cost |
|---|---|---|---|
| Announcement dates | CoinDesk Indices (indices.coindesk.com) | Manual / web scrape | Free |
| Announcement dates | CF Benchmarks (cfbenchmarks.com) | Manual / PDF releases | Free |
| Announcement dates | Nasdaq Global Index Watch | Manual download | Free |
| Historical announcement verification | Archive.org Wayback Machine | API | Free |
|
