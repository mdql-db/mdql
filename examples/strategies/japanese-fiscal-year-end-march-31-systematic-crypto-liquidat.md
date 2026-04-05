---
title: "Japanese Fiscal Year-End Crypto Tax Harvest"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 6
frequency: 1
composite: 144
categories:
  - calendar-seasonal
  - exchange-structure
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Japanese retail crypto holders face a hard March 31 fiscal year-end deadline. Crypto gains in Japan are taxed as "miscellaneous income" at marginal rates up to 55% (national + local), calculated on a per-fiscal-year basis with **no loss carryforward** to future years. This creates two forced behaviors:

1. **Loss harvesting:** Holders with unrealized losses MUST sell before March 31 to offset gains realized earlier in the fiscal year. After April 1, those losses expire worthless for tax purposes.
2. **Gain realization for cash flow:** Holders who need yen liquidity to pay prior-year tax bills (due by March 15 for withholding, or mid-March for self-assessment) must liquidate crypto holdings.

**Causal chain:**
- Tax law creates a hard deadline (March 31, non-negotiable)
- Deadline creates asymmetric incentive to sell in the final 2 weeks of March
- Concentrated selling on JPY-denominated pairs creates detectable volume/price anomalies
- After April 1, the incentive reverses: tax slate is clean, new-year capital deployment begins
- Therefore: short BTC/ETH in the March 20–31 window, long April 1–7

**This is NOT a "tends to happen" pattern claim.** The tax law is the mechanism. The question is whether Japanese retail volume is large enough to move global BTC price, or whether the effect is only visible in JPY pair spreads/volume.

---

## Structural Mechanism (WHY This Must Happen)

**The constraint is real and legally binding:**

- Japan's National Tax Agency (NTA) classifies crypto as miscellaneous income under Article 35 of the Income Tax Act
- Tax year is April 1 – March 31 (fiscal year), not calendar year
- Losses cannot be carried forward to the next fiscal year (unlike stock losses, which can be carried 3 years)
- This means a loss realized on April 1 has **zero tax value** for offsetting March gains — it must happen by March 31
- Tax payment deadlines cluster in mid-March (withholding) and mid-March to mid-April (self-assessment), creating cash demand

**The dam analogy:** The March 31 deadline is a dam. Capital that would otherwise flow freely (hold or sell at will) is forced through a narrow window. The dam does not guarantee a flood, but it guarantees abnormal flow concentration.

**Why the effect may be detectable despite small market share:**
- JPY/BTC pair volume on bitFlyer and Coincheck historically represented 5–15% of global BTC spot volume during 2017–2021 bull markets
- Even at current lower levels (~2–5%), concentrated selling within a 10-day window creates a measurable volume spike relative to baseline
- The effect should be most visible in: (a) JPY pair premium/discount vs USD pairs, (b) bitFlyer volume anomalies, (c) BTC/JPY price relative to BTC/USD

**Why this is NOT guaranteed (honest):**
- Japanese retail is a fraction of global volume; a single large institutional trade can swamp the signal
- The no-carryforward rule creates incentive but not obligation — holders may choose to hold and pay tax
- Macro conditions (bull/bear cycle) may dominate any tax-driven selling

---

## Entry Rules


### Short Leg (Pre-March 31)

| Parameter | Rule |
|-----------|------|
| **Instrument** | BTC-USDC perpetual on Hyperliquid (primary); ETH-USDC as secondary |
| **Entry trigger** | Enter short on March 20 at daily close, OR on the first daily close above the 7-day rolling average in the March 15–20 window (whichever comes first) |
| **Entry condition filter** | Only enter if bitFlyer BTC/JPY 7-day volume is ≥ 20% above its 30-day trailing average (confirms Japanese activity is elevated) |
| **Exit** | Close short at March 31 23:59 UTC (hard close, no exceptions) |
| **Stop loss** | 4% adverse move from entry price (hard stop, not trailing) |
| **Take profit** | No fixed TP — hold to March 31 close unless stop hit |

### Long Leg (Post-April 1)

| Parameter | Rule |
|-----------|------|
| **Instrument** | BTC-USDC perpetual on Hyperliquid |
| **Entry trigger** | Enter long at April 1 00:01 UTC open |
| **Entry condition filter** | Only enter if short leg was profitable OR if BTC/JPY volume on bitFlyer shows a drop ≥ 15% on March 31 vs March 30 (selling exhaustion signal) |
| **Exit** | Close long at April 7 23:59 UTC (hard close) |
| **Stop loss** | 4% adverse move from entry |
| **Take profit** | No fixed TP — hold to April 7 unless stop hit |

### Execution Notes
- Enter at daily close (00:00 UTC for Hyperliquid daily candle), not intraday
- Do not chase — if March 20 is missed, do not enter after March 25 (insufficient time for the trade to work)
- Both legs are independent trades; the long leg is NOT contingent on the short leg being open

---

## Exit Rules

Defined within Entry Rules section.
## Position Sizing

- **Base allocation:** 2% of total portfolio per leg (short leg = 2%, long leg = 2%)
- **Maximum allocation:** 3% per leg if volume filter confirms elevated JPY activity (bitFlyer 7-day volume ≥ 40% above 30-day average)
- **Leverage:** 2x maximum. This is a low-conviction, calendar-driven trade — not a high-confidence structural arb
- **Rationale:** The signal is probabilistic, not guaranteed. Sizing reflects the 5/10 score. A 4% adverse move at 2x leverage = 8% of allocated capital lost = 0.16% of total portfolio. Acceptable.
- **No pyramiding:** Single entry, single exit. Do not add to position.

---

## Backtest Methodology

### Data Required

| Dataset | Source | URL/Endpoint |
|---------|--------|--------------|
| BTC/JPY daily OHLCV (2017–2024) | bitFlyer API | `https://api.bitflyer.com/v1/getexecutions?product_code=BTC_JPY` |
| BTC/JPY daily volume | bitFlyer API | Same endpoint; aggregate daily |
| BTC/USD daily OHLCV (2017–2024) | Binance API | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d` |
| BTC/USD hourly OHLCV | Binance API | Same, `interval=1h` |
| bitFlyer historical volume (monthly) | bitFlyer public stats | `https://bitflyer.com/en-jp/s/statistics` |
| Japan crypto tax law reference | NTA Japan | `https://www.nta.go.jp/taxes/shiraberu/taxanswer/shotoku/1524.htm` |

### Backtest Window
- **Primary:** March 2018 – March 2024 (6 annual cycles)
- **Exclude:** March 2020 (COVID crash — macro event overwhelms any tax signal; treat as outlier)
- **Minimum viable sample:** 5 clean cycles

### What to Measure

**For the short leg (March 20–31):**
1. BTC/USD return over March 20–31 each year (raw)
2. BTC/JPY return over same window
3. BTC/JPY volume vs 30-day trailing average (volume anomaly score)
4. BTC/JPY price vs BTC/USD price (JPY premium/discount — does JPY pair underperform USD pair during this window?)
5. Win rate of short (did BTC fall March 20–31?)
6. Average return, median return, Sharpe of the short leg across all years

**For the long leg (April 1–7):**
1. BTC/USD return April 1–7 each year
2. Win rate of long
3. Average return, median return

**Baseline comparison:**
- Compare March 20–31 returns to: (a) random 11-day windows throughout the year, (b) same window in prior month (Feb 20 – Mar 2)
- If March 20–31 is not statistically different from random 11-day windows, the hypothesis is rejected

**Volume filter validation:**
- Test whether the volume filter (bitFlyer 7-day ≥ 20% above 30-day average) is predictive — does elevated JPY volume correlate with larger BTC drawdowns in the window?

### Key Metrics to Report
- Win rate (short leg, long leg, combined)
- Average return per leg
- Maximum drawdown per leg
- Sharpe ratio (annualized, using 6 annual observations — acknowledge small sample)
- JPY pair underperformance vs USD pair during window (the "JPY discount" metric)
- Correlation between bitFlyer volume spike and BTC return during window

---

## Go-Live Criteria

All of the following must be true before moving to paper trading:

1. **Short leg win rate ≥ 4/6 years** (67%) in the backtest window (excluding March 2020)
2. **Average short leg return ≥ +1.5%** (must exceed estimated funding cost + slippage of ~0.3%)
3. **JPY pair shows measurable underperformance** vs USD pair in at least 4/6 years during the window (confirms the mechanism is JPY-specific, not just global BTC weakness)
4. **Volume filter is predictive:** Years where bitFlyer volume spike ≥ 20% above baseline must show larger average BTC decline than years without the spike
5. **Long leg win rate ≥ 3/6 years** with positive average return (lower bar — this leg is more speculative)

If criteria 1–4 are met but criterion 5 fails, run short leg only.

---

## Kill Criteria

Abandon the strategy (do not trade or stop trading) if:

1. **Backtest fails go-live criteria** — do not paper trade, do not force it
2. **bitFlyer + Coincheck combined BTC/JPY volume drops below 1% of global BTC spot volume** for 3 consecutive months before March — Japanese retail is no longer large enough to matter
3. **Japan changes crypto tax law** to allow loss carryforward or shifts to calendar-year taxation — the structural mechanism is gone
4. **Two consecutive live years show losses on both legs** — the signal has decayed or macro is permanently overwhelming it
5. **Paper trading shows slippage > 0.5% per entry** on Hyperliquid BTC perp — execution cost destroys the edge

Monitor NTA Japan announcements annually (typically October–December for next fiscal year changes).

---

## Risks

### Primary Risks (Honest Assessment)

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Global macro overwhelms signal** | High | The 4% stop loss limits damage; March 2020 shows this can happen |
| **Japanese retail volume too small** | Medium-High | Volume filter pre-screens for years where JPY activity is elevated; if filter never triggers, don't trade |
| **Tax law change** | Low (annual monitoring needed) | Kill criterion #3; monitor NTA announcements |
| **Funding rate costs on short** | Low | At 2x leverage, 11-day funding cost ~0.1–0.3%; acceptable if return ≥ 1.5% |
| **Small sample size (6 years)** | High | Cannot achieve statistical significance with 6 observations; this is a hypothesis, not a proven edge |
| **Survivorship / selection bias** | Medium | Backtest includes bear years (2018, 2022) and bull years — not cherry-picked |
| **Timing uncertainty** | Medium | Japanese retail may front-run the deadline (sell March 10–15) or delay; the March 20 entry may miss the actual selling window |
| **Coincheck/bitFlyer API reliability** | Low | Use Binance BTC/USDT as primary price feed; JPY data is supplementary |

### Honest Assessment of Signal Strength
This is a **5/10 strategy** because the mechanism is real but the transmission to global BTC price is weak and noisy. The best-case scenario is that this is a small, consistent edge (1–3% per year) that compounds modestly. The worst case is that it's a ghost — visible in JPY pair data but too small to trade profitably after costs. The backtest will determine which.

Do not size up this trade. Do not trade it without the volume filter confirming elevated JPY activity. The structural mechanism justifies investigation, not conviction.

---

## Data Sources

| Source | URL | Notes |
|--------|-----|-------|
| bitFlyer API (BTC/JPY executions) | `https://api.bitflyer.com/v1/getexecutions?product_code=BTC_JPY&count=500` | Paginate for full history; rate limit applies |
| bitFlyer ticker | `https://api.bitflyer.com/v1/ticker?product_code=BTC_JPY` | Real-time; use for live monitoring |
| Coincheck API | `https://coincheck.com/api/trades?pair=btc_jpy` | Secondary JPY volume source |
| Binance OHLCV | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d&limit=1000` | Primary BTC/USD price feed |
| CryptoCompare historical | `https://min-api.cryptocompare.com/data/v2/histoday?fsym=BTC&tsym=JPY&limit=2000` | Alternative JPY OHLCV source |
| NTA Japan crypto tax guidance | `https://www.nta.go.jp/taxes/shiraberu/taxanswer/shotoku/1524.htm` | Monitor for law changes |
| Hyperliquid perp data | `https://app.hyperliquid.xyz/trade/BTC` | Execution venue; API at `https://api.hyperliquid.xyz/info` |
| CoinGecko global volume | `https://api.coingecko.com/api/v3/global` | Use to calculate JPY pair % of global volume |
