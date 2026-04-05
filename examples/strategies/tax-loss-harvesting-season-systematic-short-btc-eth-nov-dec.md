---
title: "Tax-Loss Harvesting Season Systematic Short/Long (Crypto, Nov–Jan)"
status: HYPOTHESIS
mechanism: 3
implementation: 7
safety: 5
frequency: 1
composite: 105
categories:
  - calendar-seasonal
  - funding-rates
created: "2025-07-14"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

US taxable investors — retail and institutional — face a **hard statutory deadline of December 31** to realise capital losses for use in the current tax year. Crypto assets have no wash-sale rule (IRC §1091 does not currently apply to digital assets), meaning sellers can realise a loss and immediately repurchase the same asset. This removes the 30-day waiting period that dampens tax-loss harvesting in equities and makes crypto harvesting a **pure deadline-driven flow event**, not a sentiment signal.

**Causal chain:**

1. Crypto assets with large unrealised losses among taxable holders create a pool of harvestable losses.
2. As December 31 approaches, the cost of *not* harvesting (forfeited tax offset) rises monotonically — there is no benefit to waiting past December 31.
3. Sell pressure concentrates in November–December, with intensity increasing toward month-end.
4. Because sellers can immediately repurchase, the selling is mechanical and not driven by changed price outlook — it is flow, not information.
5. Post-December 31, the harvesting motive disappears. Sellers who exited for tax purposes re-enter in early January, creating a mechanical demand spike.
6. The January long leg is the cleaner trade: re-entry is also deadline-agnostic (no reason to delay past January 1) and the pool of re-buyers is the same cohort that sold.

**Second-order effect:** Altcoins with the largest YTD losses attract the most harvesting flow relative to their liquidity, potentially making them better candidates than BTC/ETH despite worse on-chain data availability.

---

## Structural Mechanism (WHY This Must Happen)

| Forcing function | Strength | Notes |
|---|---|---|
| December 31 tax deadline | Hard (statutory) | IRC §1222; no discretion on year-end |
| No wash-sale rule for crypto | Hard (current law) | IRS Notice 2014-21; no §1091 application confirmed as of 2025 |
| Re-entry incentive | Hard (rational actor) | If you want the position, selling and rebuying is strictly dominant — you get a tax asset for free |
| Fund fiscal year alignment | Hard for Dec-31 funds | Many US crypto funds have Dec-31 fiscal year; LP reporting forces realisation |

**What is NOT guaranteed:** The *magnitude* of the flow. This depends on:
- How many holders are in taxable (not tax-advantaged) accounts
- The size of unrealised losses in the current year (requires a down year)
- Whether the strategy is already priced in by other participants

The mechanism is real. The alpha is probabilistic, not guaranteed. This is why the score is 6, not 8+.

---

## Entry / Exit Rules

### Leg 1: Short (Tax-Loss Selling Pressure)

**Universe selection (run on November 1 each year):**
- Assets with liquid perpetual futures on Hyperliquid or Binance
- YTD return < −40% (measured from January 1 open to October 31 close)
- 30-day average daily volume > $50M (ensures harvestable pool is large enough to matter)
- Exclude stablecoins and wrapped assets

**Ranking:** Sort by YTD return, most negative first. Take top 5 assets.

**Entry:** Open short positions on **November 1** (or first trading day of November), equal-weighted across selected assets.

**Entry execution:** TWAP over the first trading session to avoid moving illiquid markets.

**Exit:** Close all short positions on **December 28** (three days before year-end to avoid the most volatile window and potential short squeezes from thin holiday liquidity).

**Stop-loss:** If any single position moves +25% against entry, close that leg only. Do not replace.

---

### Leg 2: Long (Re-Entry Buying Pressure)

**Universe:** Same assets that were shorted in Leg 1 (the cohort that was harvested).

**Entry:** Open long positions on **January 2** (first trading day of the new year), equal-weighted.

**Entry execution:** TWAP over first trading session.

**Exit:** Close all long positions on **January 10** (re-entry flow should be exhausted within the first 5–7 trading days).

**Stop-loss:** If any single position moves −20% against entry, close that leg only.

---

### Years When Strategy Does NOT Activate

- If fewer than 3 assets in the liquid universe meet the −40% YTD threshold by November 1, **do not trade either leg**. The harvestable loss pool is too small.
- If BTC is up >20% YTD by November 1, treat as a low-confidence year and reduce position size by 50%.

---

## Position Sizing

- **Total capital allocation:** 10% of portfolio per leg (Leg 1 and Leg 2 are sequential, not simultaneous, so max 10% at risk at any time).
- **Per-asset allocation:** Equal weight across selected assets (e.g., 5 assets = 2% each).
- **Leverage:** 1x–2x maximum. This is a low-conviction structural trade, not a high-conviction arb. Do not use leverage in the first backtest year of live trading.
- **Funding rate adjustment:** For Leg 1 (short perps), check funding rate before entry. If funding rate is strongly negative (shorts being paid), this is a tailwind. If funding rate is strongly positive (longs being paid, shorts paying), reduce position size by 50% — the carry cost may erode the edge.

---

## Backtest Methodology

### Target years
Run separately for each year: **2018, 2019, 2020, 2021, 2022, 2023, 2024**. Only 2018, 2019, 2022 are expected to show meaningful short-leg signal (down years). 2020, 2021, 2024 test false-positive rate.

### Data sources
| Data | Source | Endpoint / Notes |
|---|---|---|
| Daily OHLCV (spot) | CoinGecko API | `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=365` |
| Daily OHLCV (spot, broader) | CryptoCompare | `https://min-api.cryptocompare.com/data/v2/histoday` |
| Perp funding rates | Coinalyze or Laevitas | `https://coinalyze.net` — historical funding rate CSVs |
| BTC/ETH unrealised loss data | Glassnode (free tier) | `https://glassnode.com` — "Net Unrealised Profit/Loss" metric |
| On-chain cost basis distribution | Glassnode Studio | UTXO age bands, SOPR metric |
| Tax calendar | IRS.gov | Confirm Dec 31 deadline annually |

### Metrics to compute

**For each leg, each year:**
- Raw return (entry to exit, equal-weighted portfolio)
- Sharpe ratio (annualised, using daily returns within the window)
- Max drawdown within the holding period
- Win rate across individual assets

**Aggregate across years:**
- Mean return per leg
- % of years where Leg 1 is profitable
- % of years where Leg 2 is profitable
- Correlation between Leg 1 return and YTD BTC return (to confirm regime dependency)
- Correlation between Leg 1 return and size of unrealised loss pool (Glassnode NUPL)

### Baseline comparison
Compare each leg against:
1. **Passive hold:** Buy-and-hold BTC over the same window
2. **Random short:** Short a random set of 5 liquid assets (same dates) — run 1,000 Monte Carlo draws to establish whether the asset selection (most negative YTD) adds value beyond just "short anything in November"

### Key question the backtest must answer
Does selecting assets by YTD loss magnitude outperform random asset selection during the same window? If not, the structural mechanism is real but the asset selection rule adds no value.

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show **all** of the following:

1. **Leg 1 (short):** Profitable in ≥ 2 of the 3 expected down years (2018, 2022, and whichever of 2019/2023 qualifies). Mean return across all years > 0% after estimated transaction costs (0.05% per trade, funding rate costs).
2. **Leg 2 (long):** Profitable in ≥ 3 of the 4 years tested (including at least one up year, since re-entry buying should occur regardless of regime).
3. **Asset selection adds value:** Leg 1 return in down years must exceed the 60th percentile of the Monte Carlo random-selection baseline.
4. **No single year blows up:** Max drawdown within any single holding window must not exceed 40% (if it does, the stop-loss rules need tightening before live trading).
5. **Sharpe > 0.5** for the combined strategy (Leg 1 + Leg 2) across all tested years.

---

## Kill Criteria

Abandon the strategy (do not trade the following year) if **any** of the following occur:

1. **Legislative:** Wash-sale rules are extended to crypto by Congress or IRS guidance. The no-wash-sale mechanic is the core of the re-entry incentive. Without it, sellers must wait 30 days to repurchase, which breaks the January long leg and weakens the December short leg.
2. **Backtest failure:** Backtest does not meet go-live criteria above.
3. **Live paper trade failure:** Paper trading shows Leg 1 or Leg 2 loses money in a year where the structural conditions are met (down year, large unrealised loss pool). One year of failure is a warning; two consecutive years is a kill.
4. **Crowding signal:** If open interest in perp shorts on the target assets spikes >3x normal levels in the first week of November, the trade is crowded and expected alpha is likely already extracted. Do not enter.
5. **Funding rate cost exceeds expected return:** If the annualised funding rate cost of holding the short position (Leg 1) exceeds 15% annualised, close the position regardless of P&L — the carry is destroying the edge.

---

## Risks

### Honest assessment

| Risk | Severity | Mitigation |
|---|---|---|
| Strategy only works in down years | High | Accept this; size accordingly. Do not trade Leg 1 in up years. |
| Increasingly front-run | Medium | Monitor November 1 open interest for crowding. If crowded, skip. |
| Wash-sale rule extension | High (tail) | Kill criterion above. Monitor legislative calendar in Q3 each year. |
| Thin December liquidity amplifies moves against position | Medium | Exit by December 28, not December 31. Use stop-losses. |
| On-chain data (NUPL) only reliable for BTC/ETH | Medium | Use price-based YTD loss as primary selector; NUPL as confirmation only. |
| Tax behaviour varies by jurisdiction | Low-Medium | Strategy is US-centric. Non-US selling pressure does not follow Dec 31. However, US is the dominant taxable crypto market. |
| January long leg may be pre-empted by macro | Medium | January macro events (Fed meetings, CPI) can overwhelm re-entry flow. Check macro calendar before entering Leg 2. |
| Altcoin perp liquidity insufficient | Medium | Enforce $50M/day volume filter. Do not trade assets where position size > 0.5% of daily volume. |

### Honest summary
This is a **plausible structural trade with a real forcing function, but the alpha is regime-dependent and increasingly known**. The December 31 deadline is real. The no-wash-sale rule is real. Whether these produce tradeable price impact in any given year is uncertain. Treat this as a **low-allocation, opportunistic trade** — not a core strategy. The January long leg has a cleaner risk/reward profile than the November short leg and should be prioritised if forced to choose one.

---

## Data Sources

| Source | URL | What to pull |
|---|---|---|
| CoinGecko API (free) | `https://api.coingecko.com/api/v3/` | Daily OHLCV for all assets, Jan 1 – Dec 31 per year |
| CryptoCompare API (free) | `https://min-api.cryptocompare.com/data/v2/histoday` | Cross-check OHLCV, volume filter |
| Glassnode (free tier) | `https://glassnode.com/metrics` | BTC/ETH NUPL, SOPR, unrealised loss |
| Coinalyze | `https://coinalyze.net` | Historical funding rates for perp selection |
| Laevitas | `https://laevitas.ch` | Alternative funding rate source |
| Hyperliquid API | `https://app.hyperliquid.xyz/api` | Live perp data, funding rates for execution |
| IRS.gov | `https://www.irs.gov/taxtopics/tc409` | Confirm wash-sale applicability annually |
| Congress.gov | `https://www.congress.gov` | Monitor crypto tax legislation (search "wash sale digital assets") |

---

*This document is a hypothesis specification. No backtest has been run. Do not allocate capital until go-live criteria are met.*
