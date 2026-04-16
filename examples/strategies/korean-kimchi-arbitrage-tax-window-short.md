---
title: "Korean Kimchi Arbitrage Tax Window Short"
status: HYPOTHESIS
mechanism: 3
implementation: 7
safety: 6
frequency: 1
composite: 126
categories:
  - calendar-seasonal
  - exchange-structure
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

South Korean retail investors holding unrealised crypto losses sell positions before December 31 to realise losses for tax purposes, compressing the Kimchi premium (the persistent price premium on Korean exchanges vs. global spot). This creates a predictable, calendar-driven directional bias: short BTC and ETH perps on global venues (Hyperliquid) during the Dec 20–31 window, or short the Kimchi basis directly if cross-border execution is feasible.

---

## Structural Mechanism

**Why this might be forced, not just correlated:**

1. **Hard regulatory deadline.** South Korea's crypto capital gains tax (enacted under the 2020 tax law amendments, repeatedly delayed, now scheduled for enforcement) uses December 31 as the tax year cutoff. Loss harvesting must occur before this date — it cannot be deferred. This is a calendar-forced action, not a discretionary one.

2. **Retail-dominated Korean exchanges.** Upbit and Bithumb are overwhelmingly retail. Institutional tax-loss harvesting is more spread out; retail investors cluster near the deadline due to procrastination and awareness campaigns in Korean financial media. This creates a demand shock concentrated in the final 10 days of December.

3. **Kimchi premium as the signal.** The Kimchi premium (Upbit BTC price vs. Binance BTC price, expressed in KRW/USD adjusted terms) reflects net Korean retail demand. When Korean retail sells, the premium compresses. This compression is the measurable signal. If the premium compresses faster than global BTC falls, the edge is in the basis, not the outright direction.

4. **Capital controls amplify the effect.** South Korea maintains capital controls that prevent easy arbitrage of the Kimchi premium. Arbitrageurs cannot instantly close the gap — they must move fiat through regulated banking channels, which takes days. This means the premium can stay compressed for the full window, not just minutes.

5. **Causal chain:** Tax deadline → forced retail selling on KRW venues → Upbit/Bithumb price falls relative to global → Kimchi premium compresses → if global BTC is correlated, global perps also fall, but the primary edge is in the basis compression, not outright direction.

**Honest caveat:** The tax has been delayed multiple times by the Korean National Assembly. As of 2024, enforcement was pushed to January 2025, then again to January 2028 (per the December 2024 amendment). **If the tax is not in force, the structural mechanism weakens significantly.** The strategy must be re-evaluated each year based on current Korean tax law status. Even without the formal tax, the *expectation* of the tax may still drive behaviour — this is a weaker, behavioural version of the edge (score: 4/10 in that scenario).

---

## Entry Rules


### Instrument Options (in order of preference)

| Option | Instrument | Venue | Notes |
|---|---|---|---|
| A | BTC-PERP short | Hyperliquid | Liquid, no KYC friction, direct execution |
| B | ETH-PERP short | Hyperliquid | Secondary position, smaller size |
| C | Kimchi basis short | Upbit spot long + Binance/Hyperliquid perp short | Requires Korean bank account; operationally complex |

**Option C is the purest expression of the edge but is operationally infeasible for most non-Korean entities. Default to Option A/B.**

### Entry Rules

- **Entry date:** December 20, 00:00 UTC (or first 1-hour candle open after 00:00 UTC Dec 20)
- **Entry condition:** Kimchi premium must be ≥ +1.0% at time of entry (confirms Korean premium exists to compress). If premium is already negative or below +0.5%, skip the trade — the compression has already occurred or the mechanism is absent.
- **Entry confirmation (optional filter):** 7-day rolling Kimchi premium must be declining vs. the 30-day average. This confirms the compression trend has begun.
- **Entry price:** Market order on Hyperliquid BTC-PERP at open of Dec 20 candle. No limit chasing — the edge is calendar-based, not price-level based.

## Exit Rules

### Exit Rules

- **Hard exit:** December 31, 20:00 UTC (before Asian midnight, capturing any last-day selling)
- **Profit target:** None — hold full position to exit date. The edge is time-bounded, not price-bounded.
- **Stop loss:** 8% adverse move from entry price on the perp (e.g., if BTC rises 8% from entry, close position). This is a risk management stop, not a signal stop.
- **Early exit trigger:** If Kimchi premium rises above +3.0% after entry (premium expanding, not compressing), close 50% of position — the mechanism is not playing out.

### Annual Re-entry Checklist (run December 15 each year)

- [ ] Confirm Korean crypto capital gains tax is in force for the current tax year
- [ ] Confirm Kimchi premium data feed is live and accurate
- [ ] Confirm Upbit/Bithumb are operational (no regulatory shutdown)
- [ ] Check Korean financial media for tax-loss harvesting coverage (qualitative signal)

---

## Position Sizing

- **Base allocation:** 3% of portfolio NAV per trade (BTC-PERP short)
- **Optional ETH allocation:** 1.5% of portfolio NAV (ETH-PERP short), only if ETH Kimchi premium also ≥ +1.0%
- **Leverage:** 2x maximum. The edge is not high-conviction enough to justify higher leverage. At 2x, a 50% adverse move in BTC wipes the position — acceptable given the 8% stop loss exits well before that.
- **Rationale for small size:** This is a calendar trade with a weak-to-moderate structural mechanism. It should be sized as a satellite position, not a core holding. The Sharpe ratio is unknown pre-backtest.
- **Do not scale up** until 3+ years of live or backtested data confirm the edge.

---

## Backtest Methodology

### Data Required

| Dataset | Source | URL |
|---|---|---|
| Kimchi premium daily/hourly | CryptoQuant | https://cryptoquant.com/asset/btc/chart/market-data/kimchi-premium |
| Upbit BTC/KRW OHLCV | Upbit API | https://docs.upbit.com/reference/candle-day |
| Bithumb BTC/KRW OHLCV | Bithumb API | https://apidocs.bithumb.com |
| Binance BTC/USDT OHLCV | Binance API | https://api.binance.com/api/v3/klines |
| USD/KRW FX daily | Bank of Korea | https://www.bok.or.kr/eng/main/contents.do?menuNo=400069 |
| BTC-PERP funding rates | Hyperliquid / Coinalyze | https://coinalyze.net/bitcoin/funding-rate/ |

### Backtest Period

- **Primary:** December 2018 – December 2024 (6 years, 6 observations)
- **Note:** Korean crypto tax law was not in force before 2025 (repeatedly delayed), so pre-2025 observations test the *behavioural* version of the hypothesis (retail fear of future taxation, media coverage effects), not the structural version. Label these separately.
- **Minimum viable sample:** 3 years with consistent Kimchi premium data.

### Backtest Steps

1. **Construct Kimchi premium series:** `(Upbit BTC/KRW) / (Binance BTC/USDT × USD/KRW) - 1`, expressed as percentage. Use daily close data.
2. **Simulate entry:** Short BTC-PERP at Dec 20 close price each year, conditional on Kimchi premium ≥ +1.0%.
3. **Simulate exit:** Close at Dec 31 close price.
4. **Apply stop loss:** If intraday high exceeds entry × 1.08, close at that level.
5. **Deduct costs:** 0.05% taker fee each way (Hyperliquid), plus average funding rate cost for the 11-day hold period (check historical funding rates for Dec — if BTC is in contango, shorts receive funding; if backwardated, shorts pay).
6. **Measure:** Raw P&L per year, Kimchi premium change over the window, correlation between premium compression and BTC perp P&L.
7. **Separate analysis:** Run the same backtest on ETH-PERP. Check if ETH Kimchi premium compresses in sync with BTC.

### Key Metrics to Report

- Win rate (years profitable / total years)
- Average return per trade (net of fees)
- Maximum drawdown within the window
- Kimchi premium compression (bps) vs. BTC price change: are they correlated? If BTC falls but premium doesn't compress, the mechanism isn't the driver.
- Funding rate drag over the 11-day window

### Red Flags in Backtest

- If BTC falls in Dec but Kimchi premium does NOT compress → the edge is not Korean-specific, it's just a December BTC seasonal (different, weaker hypothesis)
- If premium compresses but BTC perp is flat or up → the edge exists only in the basis (Option C), not in outright shorts
- If results are driven by 1–2 outlier years → not robust

---

## Go-Live Criteria

All of the following must be true before allocating real capital:

1. Backtest shows ≥ 4 of 6 years profitable (net of fees)
2. Average net return ≥ 1.5% on allocated capital per trade
3. Kimchi premium compression is statistically correlated with trade P&L (Pearson r ≥ 0.5 across years)
4. Korean capital gains tax is confirmed in force for the current tax year (check National Assembly legislation each November)
5. Kimchi premium on December 15 is ≥ +1.0% (confirming the premium exists to compress)
6. Funding rate environment is not severely adverse (funding cost over 11 days must not exceed 0.5% of notional)

---

## Kill Criteria

Abandon the strategy permanently if any of the following occur:

- **Regulatory:** Korean government permanently exempts retail crypto from capital gains tax with no sunset clause
- **Market structure:** Kimchi premium collapses to near-zero on a structural basis (capital controls removed, or major arbitrage infrastructure deployed), eliminating the basis
- **Performance:** Strategy loses money in 2 consecutive live years after go-live
- **Data:** Upbit or Bithumb API becomes unavailable or unreliable, making real-time premium monitoring impossible
- **Mechanism failure:** Backtest shows no correlation between Kimchi premium compression and December window (mechanism was never real)

---

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Tax law delayed again (as happened 2020–2024) | High | Check legislation status each November; skip trade if tax not in force |
| BTC rallies in December (Santa Claus rally) | Medium | 8% stop loss; small position size |
| Kimchi premium already compressed before Dec 20 | Medium | Entry condition requires ≥ +1.0% premium at entry |
| Funding rate turns sharply negative for shorts | Low-Medium | Check funding before entry; abort if projected 11-day cost > 0.3% |
| Korean exchange outage / regulatory action | Low | Monitor Upbit/Bithumb status; use as signal only, not execution venue |
| Global macro shock overrides seasonal effect | Medium | Cannot hedge; accept as tail risk |
| Sample size too small (6 observations) | High | Do not over-allocate; treat as speculative satellite position |
| Behavioural effect without tax enforcement | Medium | Acknowledge weaker edge; reduce position size to 1.5% NAV if tax not in force |

---

## Data Sources

| Source | Use | URL |
|---|---|---|
| CryptoQuant | Kimchi premium historical series | https://cryptoquant.com/asset/btc/chart/market-data/kimchi-premium |
| Upbit Open API | BTC/KRW OHLCV candles | https://docs.upbit.com/reference/candle-day |
| Bithumb API | BTC/KRW cross-check | https://apidocs.bithumb.com |
| Binance API | BTC/USDT global reference price | https://api.binance.com/api/v3/klines |
| Bank of Korea | USD/KRW official FX rate | https://www.bok.or.kr/eng/main/contents.do?menuNo=400069 |
| Coinalyze | Historical funding rates for BTC-PERP | https://coinalyze.net/bitcoin/funding-rate/ |
| Korean National Assembly | Tax law status | https://likms.assembly.go.kr/bill/main.do |
| Naver Finance / Korean media | Qualitative retail sentiment check | https://finance.naver.com |

---

## Open Questions for Researcher

1. **Has the Korean crypto capital gains tax actually been enforced for any tax year yet?** If not, all historical observations are behavioural, not structural — the score drops to 4/10.
2. **Is there a way to measure Korean retail selling volume directly** (e.g., Upbit net outflows to wallets in December vs. other months) rather than relying on price-based Kimchi premium as a proxy?
3. **Does the effect appear in altcoins with high Korean retail concentration** (e.g., XRP, DOGE, specific Korean-listed tokens)? If so, a broader basket short may be more powerful than BTC/ETH alone.
4. **What is the typical funding rate on BTC-PERP in December?** If the market is consistently in contango in December (shorts pay), this erodes the edge materially.
5. **Is there a mirror effect in January** (tax-loss sellers re-enter positions after year-end)? If so, a long entry on Jan 1 may be a companion trade worth speccing separately.

---

*This document is a strategy hypothesis. No backtest has been run. Do not allocate capital until go-live criteria are met.*
