---
title: "BTC ETF AP 4pm Cutoff — Pre-Close NAV Arbitrage Flow"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 7
composite: 1260
categories:
  - basis-trade
  - index-rebalance
  - exchange-structure
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

On days when spot Bitcoin ETFs (IBIT, FBTC) trade at a premium >0.10% to their implied NAV during the 3:00–3:25pm ET window, Authorized Participants are structurally incentivized to submit creation orders by the 4pm ET cutoff. To deliver BTC into creation baskets, APs must acquire spot BTC before 4pm ET. This mechanical buying pressure creates a detectable upward drift in BTC spot price in the 3:30–4:00pm ET window that is not present on low-premium or discount days.

**Causal chain:**

1. IBIT/FBTC trades at premium to NAV during afternoon session
2. AP identifies risk-free profit: buy BTC spot + create ETF shares → sell ETF shares at premium
3. AP must submit creation order by 4pm ET for same-day NAV settlement
4. AP must source BTC before 4pm ET to deliver into basket
5. AP spot BTC purchases concentrate in the 3:30–4:00pm ET window (last window before cutoff where they have confirmed premium signal)
6. Concentrated buying creates measurable upward price pressure on BTC spot
7. Strategy enters long BTC perp at 3:30pm ET, exits at 4:05pm ET, capturing this drift

**Null hypothesis to disprove:** BTC returns in the 3:30–4:05pm ET window are not statistically different on high-premium days vs. low-premium/discount days.

---

## Structural Mechanism — WHY This Must Happen

This is a **mechanical arbitrage deadline**, not a behavioral tendency.

**The hard constraint:** ETF creation/redemption orders must be submitted to the ETF custodian (BNY Mellon for IBIT) by 4:00pm ET for same-day NAV settlement. This is specified in each ETF's prospectus and is non-negotiable — it is a legal settlement deadline, not a convention.

**The NAV strike:** IBIT NAV is calculated using the CME CF Bitcoin Reference Rate — New York Variant (BRRNY), which references BTC spot prices at 4:00pm ET. This means the AP's creation cost is locked to the 4pm BTC price. An AP who buys BTC at 3:45pm and submits a creation order receives NAV based on 4pm price — they bear 15 minutes of price risk but capture the premium.

**Why APs can't hedge earlier and walk away:** The premium signal must be confirmed close to the cutoff. If an AP hedges at 2pm on a 0.10% premium, the premium may collapse by 3pm (other APs arb it), leaving them with an unhedged BTC position. APs therefore wait for premium confirmation before committing to the creation trade, concentrating their BTC purchases in the final 30–60 minutes.

**Why this is not fully arbitraged away:** The premium is not risk-free until the AP has confirmed the creation order will be accepted and NAV will be struck at a favorable price. Execution risk, BTC volatility risk during the 3:30–4:00pm window, and operational overhead mean APs require a minimum premium threshold (~0.10–0.15%) before acting. Below this threshold, no mechanical buying occurs.

**Scale of the flow:** IBIT alone has seen daily creation/redemption flows of $200M–$1B+ on active days. Even 50% of a $300M creation day = $150M of BTC spot purchases concentrated in 30 minutes. At BTC's typical 3:30–4pm liquidity, this is a non-trivial flow.

---

## Entry Rules


### Signal Calculation (3:25pm ET)

1. Pull IBIT last trade price at 3:25pm ET
2. Pull BTC spot mid-price at 3:25pm ET (Coinbase BTC-USD best bid/ask midpoint)
3. Calculate implied NAV: `NAV_implied = BTC_spot_price × 0.00099784` (IBIT holds ~0.001 BTC per share; exact ratio from prospectus, currently ~0.00099784 BTC/share — verify daily from iShares website)
4. Calculate premium: `Premium% = (IBIT_price - NAV_implied) / NAV_implied × 100`

### Entry Condition

- Premium% > 0.10% at 3:25pm ET **AND**
- IBIT 30-minute volume (2:55–3:25pm ET) > 20-day average 30-min volume for that window (confirms active AP participation, not thin market)
- **No entry** if BTC has moved >1.5% in either direction in the prior 30 minutes (volatility filter — AP arb breaks down in fast markets)

### Entry Execution

- **Instrument:** BTC-USD perpetual on Hyperliquid
- **Time:** Market order at 3:30pm ET (5 minutes after signal confirmation)
- **Direction:** Long

## Exit Rules

### Exit Execution

- **Time:** Market order at 4:05pm ET (5 minutes after NAV strike, allowing AP flow to complete)
- **Hard stop:** If BTC drops >0.75% from entry at any point before 4:05pm, exit immediately (stop-loss)

### No-Trade Days

- US federal holidays (ETF market closed)
- Days where BTC spot-perp funding rate > 0.05% per 8h (elevated carry cost distorts P&L)
- Days where IBIT premium is negative (discount) at 3:25pm — reverse signal, do not short (mechanism is asymmetric; redemption flow is less concentrated)

---

## Position Sizing

- **Base position:** 1% of portfolio NAV per trade
- **Maximum position:** 2% of portfolio NAV (never scale up based on premium size alone — premium magnitude does not linearly predict flow magnitude)
- **Rationale:** This is a 35-minute trade with a hard stop. Kelly sizing is inappropriate without a validated edge ratio. Use fixed fractional until backtest provides win rate and average R.
- **Leverage:** 2x maximum on Hyperliquid perp. The edge (if real) is small — do not amplify with leverage until edge is confirmed over 100+ trades.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Notes |
|---|---|---|---|
| IBIT intraday OHLCV | Polygon.io (paid) or Yahoo Finance (free, 1-min delay) | 1-minute bars | Yahoo: `yfinance` library, ticker `IBIT` |
| FBTC intraday OHLCV | Same as above | 1-minute bars | Cross-validate signal with FBTC |
| BTC-USD spot | Coinbase Advanced Trade API | 1-minute bars | Endpoint: `GET /api/v3/brokerage/market/candles` |
| BTC perp (execution proxy) | Hyperliquid public API | 1-minute bars | `https://api.hyperliquid.xyz/info` — `candleSnapshot` |
| IBIT shares outstanding / BTC per share | iShares daily NAV file | Daily | `https://www.ishares.com/us/products/333011/` → Holdings CSV |

**Backtest period:** January 11, 2024 (IBIT launch) through present. Minimum 12 months of data required before drawing conclusions. Target: ~250 trading days.

### Backtest Steps

1. **Reconstruct premium signal:** For each trading day, calculate IBIT premium at 3:25pm ET using 1-min bar close prices. Use the exact BTC/share ratio from iShares daily holdings file (it changes slightly over time due to fee accrual).

2. **Classify days:** 
   - `HIGH_PREMIUM`: Premium > 0.10% at 3:25pm ET
   - `LOW_PREMIUM`: Premium 0–0.10%
   - `DISCOUNT`: Premium < 0%
   - `NO_SIGNAL`: Volume filter fails or volatility filter triggers

3. **Measure BTC returns:** For each day, calculate BTC spot return from 3:30pm ET to 4:05pm ET (35-minute window). Use Coinbase 1-min bars.

4. **Primary test:** Is mean BTC return in `HIGH_PREMIUM` days statistically greater than mean BTC return on `LOW_PREMIUM` + `DISCOUNT` days? Use Welch's t-test (unequal variance). Require p < 0.05.

5. **Secondary test:** Plot cumulative P&L of the strategy vs. a baseline of "always long BTC 3:30–4:05pm ET regardless of signal." The signal should add value over the unconditional long.

6. **Tercile analysis:** Split `HIGH_PREMIUM` days into terciles by premium magnitude (0.10–0.20%, 0.20–0.40%, >0.40%). Does higher premium predict higher BTC drift? If not, the mechanism is not dose-responsive and the causal story is weaker.

7. **Time-of-day control:** Run the same analysis for the 2:30–3:00pm ET window (same duration, no AP deadline pressure). If BTC drifts equally in that window on high-premium days, the 4pm deadline is not the cause — it's a general momentum effect.

8. **Stop-loss impact:** Simulate the 0.75% hard stop. Calculate how many trades it triggers and whether removing it improves or worsens Sharpe.

### Key Metrics

| Metric | Minimum Acceptable | Target |
|---|---|---|
| Win rate | >52% | >58% |
| Mean return per trade (net of 0.05% round-trip cost) | >0.05% | >0.15% |
| Sharpe ratio (annualized, trade-level) | >0.8 | >1.5 |
| Max drawdown (consecutive losing trades) | <5 trades | <3 trades |
| t-stat vs. null hypothesis | >1.96 (p<0.05) | >2.5 |
| Number of qualifying signal days | >60 | >100 |

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Statistical significance:** t-stat > 1.96 on the primary test (HIGH_PREMIUM vs. other days)
2. **Economic significance:** Mean net return per trade > 0.05% after 0.05% round-trip transaction cost assumption
3. **Dose-response present:** Higher premium tercile shows higher mean return than lower premium tercile (confirms mechanism, not noise)
4. **Time-of-day control fails:** The 2:30–3:00pm ET control window does NOT show the same drift on high-premium days (confirms 4pm deadline is causal, not general momentum)
5. **Minimum sample:** ≥60 qualifying signal days in backtest
6. **No single-year dependency:** Edge holds in both 2024 and 2025 sub-periods (not a one-year artifact)

---

## Kill Criteria

Abandon the strategy (stop paper trading or live trading) if:

- **Backtest kill:** Any of the go-live criteria fail → do not proceed to paper trading
- **Paper trading kill:** After 30 paper trades, mean return is negative or Sharpe < 0.5
- **Live trading kill:** 10 consecutive losing trades, OR drawdown exceeds 3% of allocated capital
- **Structural change kill:** SEC changes ETF creation/redemption rules, or IBIT switches to in-kind creation (changes the BTC delivery mechanics fundamentally)
- **Premium compression kill:** Rolling 60-day average IBIT premium at 3:25pm ET drops below 0.05% (signal days become too rare to trade — fewer than 1/week)
- **ETF AUM kill:** If IBIT AUM drops below $5B (reduces daily flow magnitude to immaterial levels)

---

## Risks

### Risk 1: APs hedge continuously, not at 3:30pm (HIGH probability, HIGH impact)
The most likely failure mode. Sophisticated APs (Jane Street, Virtu) run delta-neutral books and may hedge BTC exposure throughout the day as ETF premium develops, not in a concentrated burst before 4pm. If hedging is spread across the day, the 3:30pm entry captures nothing special. **Mitigation:** The time-of-day control test in the backtest will detect this.

### Risk 2: Premium is already arbitraged by 3:25pm (MEDIUM probability, HIGH impact)
By 3:25pm, other market participants (not just APs) see the premium and buy BTC spot, compressing the premium before APs need to act. The signal may be real but the trade entry is too late. **Mitigation:** Test earlier entry times (3:00pm, 3:15pm) in backtest sensitivity analysis.

### Risk 3: Small sample of qualifying days (HIGH probability, MEDIUM impact)
IBIT premium >0.10% may occur on fewer days than expected. If only 20–30 qualifying days exist in the backtest period, statistical conclusions are unreliable. **Mitigation:** Lower threshold to 0.05% and re-run; also include FBTC to increase sample.

### Risk 4: BTC volatility overwhelms the signal (MEDIUM probability, MEDIUM impact)
BTC moves 0.5–2% randomly in any 35-minute window. The AP flow signal (if real) may be 0.10–0.20% of drift — easily swamped by noise. This strategy may require hundreds of trades to confirm edge with statistical confidence. **Mitigation:** Accept this — it's why we backtest before trading.

### Risk 5: Execution slippage on Hyperliquid (LOW probability, LOW impact)
BTC perp on Hyperliquid has deep liquidity. Market order slippage for <$100K position should be <0.02%. This is manageable. **Mitigation:** Use limit orders with 0.05% price tolerance at entry.

### Risk 6: Funding rate cost (LOW probability, MEDIUM impact)
Holding a long perp position for 35 minutes incurs a pro-rated funding cost. At typical BTC funding rates (0.01% per 8h), the 35-minute cost is ~0.001% — negligible. Only becomes material if funding spikes above 0.05% per 8h (covered by no-trade filter).

### Risk 7: Reverse causality (MEDIUM probability, HIGH impact)
BTC spot rising in the afternoon may CAUSE the ETF to trade at a premium (momentum buyers push ETF price up faster than NAV updates), rather than the premium causing BTC to rise. If this is the true direction, the strategy is trading momentum, not AP flow — a weaker and more crowded edge. **Mitigation:** The time-of-day control test partially addresses this; also examine whether premium leads or lags BTC spot moves using cross-correlation analysis in the backtest.

---

## Data Sources

| Resource | URL / Endpoint | Cost |
|---|---|---|
| IBIT 1-min bars | `yfinance`: `yf.download("IBIT", interval="1m", period="5d")` | Free (5-day rolling window only) |
| IBIT 1-min bars (historical) | `https://api.polygon.io/v2/aggs/ticker/IBIT/range/1/minute/{from}/{to}` | Polygon Starter $29/mo |
| BTC-USD spot 1-min | `https://api.exchange.coinbase.com/products/BTC-USD/candles?granularity=60&start=...&end=...` | Free |
| IBIT daily NAV + BTC/share ratio | `https://www.ishares.com/us/products/333011/ishares-bitcoin-trust-etf` → "Holdings" CSV download | Free |
| FBTC daily NAV | `https://www.fidelity.com/crypto/fidelity-wise-origin-bitcoin-fund` | Free |
| Hyperliquid BTC perp candles | `POST https://api.hyperliquid.xyz/info` body: `{"type":"candleSnapshot","req":{"coin":"BTC","interval":"1m","startTime":...,"endTime":...}}` | Free |
| BTC ETF flow data (creation/redemption volumes) | `https://farside.co.uk/bitcoin-etf-flow-all-data-table/` | Free (daily, not intraday) |

**Note on Polygon free tier:** The free Polygon tier provides end-of-day data only. The $29/mo Starter plan provides 1-minute historical bars for US equities/ETFs, which is required for this backtest. This is the minimum paid data cost for this strategy.

**Implementation note:** Build the backtest in Python. Use `pandas` for time alignment (be careful with timezone handling — Coinbase API returns UTC, convert to US/Eastern). Align IBIT bars and BTC bars on the same 1-minute timestamps before calculating premium. Verify BTC/share ratio daily from iShares holdings file, not a fixed constant — it drifts downward ~0.25% per year due to fee accrual.
