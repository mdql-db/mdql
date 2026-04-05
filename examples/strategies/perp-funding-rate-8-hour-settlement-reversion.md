---
title: "Perp Funding Rate 8-Hour Settlement Reversion"
status: HYPOTHESIS
mechanism: 3
implementation: 9
safety: 6
frequency: 8
composite: 1296
categories:
  - funding-rates
  - exchange-structure
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Funding rate settlement at fixed 8-hour UTC timestamps (00:00, 08:00, 16:00) creates a mechanical, repeating flow event: long-biased speculators in high-funding environments face a binary choice — pay the accrued funding or close before the timestamp. The aggregate of these individual rational decisions produces a predictable micro-dip in the 5–15 minutes before settlement and a snap-back in the 2–10 minutes after. The edge is not "prices tend to fall before funding" — it is that the settlement timestamp is contractually fixed and the incentive to avoid payment is calculable in real time. In regimes where funding exceeds 0.05% per 8-hour window, the dollar cost of holding through settlement is large enough to motivate institutional-scale position rotation, making the signal detectable above noise.

---

## Structural Mechanism

### Why the flow MUST happen (the causal chain)

1. **Funding accrual is time-weighted within the window.** On Binance, Bybit, and Hyperliquid, funding is calculated as a TWAP of the mark-basis over the preceding 8-hour window. A trader who opens a long position 30 minutes before settlement pays the full 8-hour accrued rate as if they held the entire window — they receive zero credit for the time they were not in the position.

2. **This creates a rational exit incentive.** If funding is 0.10% per window and a trader holds $10M long, paying through settlement costs $10,000 in 12 minutes. Closing 10 minutes before settlement and reopening 2 minutes after saves approximately $9,500 (accounting for two round-trip transaction costs at ~$250 each at 0.05% taker). The math is unambiguous at scale.

3. **The incentive is asymmetric by position size.** Retail traders with $1,000 positions save $1 — not worth the friction. Traders with $1M+ positions save $1,000+ — worth two round trips. This means the flow is concentrated among larger accounts, producing detectable price impact rather than noise.

4. **The timestamp is public and immovable.** Unlike earnings dates or governance votes, the settlement timestamp cannot be delayed, cancelled, or front-run by insiders. Every market participant knows exactly when it fires. This makes the flow predictable but also means it is partially competed away — the edge exists in the residual between the rational flow and the bots already trading it.

5. **The snap-back is mechanical.** Traders who closed to avoid funding must reopen to maintain their directional exposure. If they are trend-followers or basis traders, their thesis has not changed — only their funding cost has been managed. Reopening demand hits the ask within minutes of settlement, creating the reversion.

### Why this is NOT fully arbitraged away

- The window is only 12 minutes wide (10 min pre + 2 min post). Execution risk within this window deters many systematic funds.
- The edge is conditional on high funding — it does not fire in neutral markets, reducing the signal-to-noise ratio for strategies that run continuously.
- Hyperliquid's funding mechanism has subtle differences from Binance (continuous accrual vs. discrete settlement) that create cross-venue basis opportunities not yet fully exploited.

---

## Universe & Filters

### Asset selection
- **Primary:** BTC-PERP, ETH-PERP on Hyperliquid (execution venue)
- **Secondary backtest universe:** BTC, ETH, SOL, BNB perpetuals on Binance and Bybit
- **Rationale:** High liquidity reduces slippage within the 12-minute execution window; large open interest amplifies the dollar value of funding avoidance.

### Activation filter (ALL conditions must be true to enter)
| Filter | Threshold | Rationale |
|--------|-----------|-----------|
| Funding rate (current window) | > 0.05% per 8h | Below this, dollar incentive to rotate is too small to move price |
| Open interest | > $500M notional | Ensures sufficient position size among large accounts to create detectable flow |
| 1h volume (pre-settlement) | > 20th percentile of trailing 30-day 1h volumes | Confirms market is active enough to absorb the trade |
| Basis momentum (last 30 min) | Basis NOT expanding (i.e., mark price not accelerating away from index) | Momentum override — if a directional move is in progress, the reversion thesis is overridden |

### Momentum kill filter (detailed)
- Calculate: `basis_30min_change = (mark_price - index_price) / index_price` at T-30min vs T-10min
- If `basis_30min_change > +0.02%` (basis widening into settlement in a long-biased market), **skip this settlement window entirely**
- Rationale: A widening basis means new longs are entering aggressively — the reversion flow is being overwhelmed by directional momentum.

---

## Entry Rules


### Timing reference
All times relative to settlement timestamp T (00:00, 08:00, or 16:00 UTC).

### Entry
- **Time:** T − 10 minutes (i.e., 23:50, 07:50, or 15:50 UTC)
- **Direction:** SHORT the perp
- **Order type:** Limit order posted at mid-price; if unfilled within 60 seconds, convert to aggressive limit (cross the spread by 0.5 ticks); if still unfilled at T − 8 minutes, cancel and skip this window
- **Do not use market orders** — the 12-minute window is tight but not so tight that slippage from market orders is acceptable

## Exit Rules

### Exit
- **Primary exit:** T + 2 minutes (i.e., 00:02, 08:02, or 16:02 UTC)
- **Order type:** Limit order at mid-price; if unfilled within 90 seconds, convert to aggressive limit; if unfilled at T + 5 minutes, market close
- **Stop loss:** If position moves against entry by 0.15% at any point during the hold, close immediately at market — this caps the loss at approximately 3× the expected gain per trade

### Hold period
- Maximum hold: T − 10min to T + 5min = 15 minutes total
- Do not hold through the next settlement window under any circumstances

---

## Position Sizing

### Base sizing
- **Risk per trade:** 0.25% of portfolio NAV
- **Stop distance:** 0.15% from entry
- **Position size formula:** `size = (0.0025 × NAV) / 0.0015`
- Example: $100,000 NAV → risk $250 → stop at 0.15% → position = $166,667 notional

### Scaling rules
- **Maximum position:** 2× base size when funding > 0.10% per window (double the incentive, double the expected flow)
- **Minimum position:** Do not trade if base size results in notional < $10,000 (slippage will consume the edge)
- **Concentration limit:** Never exceed 20% of NAV in a single settlement trade

### Leverage
- Use 3–5× leverage on Hyperliquid to achieve target notional without tying up full capital
- Do not exceed 5× — the 15-minute hold period does not justify higher leverage given liquidation risk

---

## Backtest Methodology

### Data requirements
| Dataset | Source | Format | Cost |
|---------|--------|--------|------|
| 1-minute OHLCV (BTC, ETH, SOL) | Binance public REST API (`/api/v3/klines`) | JSON → CSV | Free |
| Funding rate history (per 8h window) | Binance (`/fapi/v1/fundingRate`), Bybit (`/v5/market/funding/history`) | JSON → CSV | Free |
| Mark price history (1-min) | Binance (`/fapi/v1/markPriceKlines`) | JSON → CSV | Free |
| Open interest history (1-min) | Binance (`/fapi/v1/openInterestHist`) | JSON → CSV | Free |
| Hyperliquid funding + trades | Hyperliquid public API (`/info` endpoint, `fundingHistory`) | JSON | Free |

### Backtest period
- **Primary:** January 2021 – present (covers multiple high-funding bull regimes and low-funding bear regimes)
- **Segment analysis required:** Backtest must be run separately on (a) high-funding regimes (>0.05%), (b) moderate-funding regimes (0.01–0.05%), and (c) low/negative-funding regimes (<0.01%) — the hypothesis only applies to segment (a)

### Backtest construction (step by step)

1. **Load** 1-minute mark price and index price for BTC-PERP from Binance futures, January 2021 to present.
2. **Tag** each 1-minute bar with the funding rate for its containing 8-hour window.
3. **Identify** all settlement timestamps where funding > 0.05%.
4. **Apply momentum filter:** For each qualifying timestamp T, check if basis expanded in the T−30min to T−10min window. Flag and exclude those events.
5. **Simulate entry** at T−10min: record the 1-minute open price of the T−10min bar as the entry price. Apply 0.05% taker fee (conservative for limit orders — use 0.02% if assuming maker fill).
6. **Simulate exit** at T+2min: record the 1-minute open price of the T+2min bar as the exit price. Apply 0.05% taker fee.
7. **Apply stop loss:** If any 1-minute low (for shorts) exceeds entry + 0.15% during the hold period, record the stop price as the exit.
8. **Calculate PnL** per trade: `(entry_price - exit_price) / entry_price - fees`.
9. **Aggregate** by regime, by asset, by time-of-day (00:00 vs 08:00 vs 16:00 UTC — these may differ due to Asian vs US session liquidity).
10. **Report:** Win rate, average PnL per trade, Sharpe ratio of trade returns, maximum drawdown per regime, and number of qualifying events per month.

### Key metrics to compute
- **Win rate by funding regime:** Expect >55% in high-funding regime; if <50%, hypothesis is rejected
- **Average gross PnL per trade:** Must exceed 0.10% to survive fees (two round trips at 0.05% each = 0.10% cost)
- **Sharpe of trade-level returns:** Target >1.5 on the high-funding subsample
- **Decay analysis:** Does the edge diminish over time (2021 vs 2022 vs 2023 vs 2024)? If yes, quantify the decay rate
- **Session analysis:** 00:00 UTC (Asia open) vs 08:00 UTC (Europe open) vs 16:00 UTC (US afternoon) — liquidity profiles differ materially

### Slippage model
- Assume 0.05% round-trip slippage on top of fees for BTC/ETH (liquid)
- Assume 0.10% round-trip slippage for SOL and smaller assets
- Run a sensitivity analysis: at what slippage level does the strategy break even?

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

| Criterion | Threshold |
|-----------|-----------|
| Backtest win rate (high-funding regime) | ≥ 55% |
| Backtest average net PnL per trade | ≥ 0.05% after fees and slippage |
| Backtest Sharpe (trade-level) | ≥ 1.5 |
| Minimum qualifying events in backtest | ≥ 200 trades (to establish statistical significance) |
| Edge present in at least 2 of 3 assets | BTC, ETH, SOL must each be tested independently |
| No single calendar year shows negative expectancy | Edge must not be entirely concentrated in one regime |

Paper trading go-live criteria (before real capital):

| Criterion | Threshold |
|-----------|-----------|
| Paper trade sample | ≥ 30 live qualifying events |
| Paper trade win rate | ≥ 50% (lower bar due to small sample) |
| Paper trade average PnL | Positive net of simulated fees |
| Fill rate on limit orders | ≥ 70% (if fills are consistently missed, execution model is broken) |

---

## Kill Criteria

Stop trading immediately if any of the following occur:

| Trigger | Action |
|---------|--------|
| 10 consecutive losing trades in live trading | Halt, review whether funding regime has changed |
| Live win rate drops below 45% over trailing 50 trades | Suspend and re-examine momentum filter calibration |
| Average slippage in live execution exceeds 0.10% per round trip | Halt — edge is consumed by execution costs |
| Funding rate structure changes on Hyperliquid (e.g., move to continuous settlement) | Immediate halt — structural mechanism no longer applies |
| Open interest on target pair drops below $200M | Reduce size by 50%; below $100M, halt entirely |
| A competing strategy or known bot is detected front-running the T−10min entry | Shift entry to T−8min and re-evaluate; if still front-run, retire strategy |

---

## Risks

### Risk 1: Edge is already fully arbitraged
**Description:** High-frequency bots may already be trading this exact window, compressing the micro-dip to below fee threshold.
**Mitigation:** The backtest decay analysis will reveal this. If post-2023 data shows no edge, do not go live.
**Residual risk:** Medium — this is the primary risk to the strategy.

### Risk 2: Execution risk within the 12-minute window
**Description:** Limit orders may not fill if the market moves away from mid immediately after entry signal. Missed fills mean the strategy fires on some events but not others, creating selection bias in live results.
**Mitigation:** Track fill rate during paper trading. If fill rate < 70%, the limit order strategy is broken and must be redesigned.
**Residual risk:** Medium — manageable with careful order management.

### Risk 3: Flash crash or liquidation cascade during hold period
**Description:** The 15-minute hold period is short but not zero. A large liquidation event during the window could move the market 1–2% against the position before the stop triggers.
**Mitigation:** Stop loss at 0.15% limits loss to ~3× expected gain. Do not trade during periods of elevated volatility (e.g., if 1h realized vol > 3× trailing 30-day average, skip the window).
**Residual risk:** Low-medium — stop loss provides hard protection.

### Risk 4: Funding regime dependency
**Description:** The strategy only fires when funding > 0.05%. In bear markets or low-volatility regimes, this condition may not be met for weeks or months, producing long periods of zero activity.
**Mitigation:** Accept this as a feature, not a bug — the strategy is designed to be selective. Do not lower the funding threshold to generate more trades; doing so destroys the edge.
**Residual risk:** Low — this is a known and acceptable characteristic.

### Risk 5: Exchange rule changes
**Description:** Binance, Bybit, or Hyperliquid could change their funding settlement mechanics (e.g., move to hourly settlement, change the TWAP window, or introduce partial accrual credit for mid-window entries).
**Mitigation:** Monitor exchange announcements. Subscribe to official changelog feeds for all three venues.
**Residual risk:** Low — changes of this type are announced in advance and would be immediately visible.

### Risk 6: Correlation with broader market moves
**Description:** Settlement timestamps coincide with high-activity periods (00:00 UTC = Asia open, 16:00 UTC = US afternoon). Macro news events at these times could overwhelm the funding reversion signal.
**Mitigation:** Add a news filter: skip any settlement window within 30 minutes of a scheduled macro event (FOMC, CPI, etc.) using a public economic calendar API.
**Residual risk:** Low-medium — rare but potentially large impact events.

---

## Data Sources

| Source | Endpoint | Data | Update frequency |
|--------|----------|------|-----------------|
| Binance Futures REST | `/fapi/v1/fundingRate` | Historical funding rates (8h) | Per settlement |
| Binance Futures REST | `/fapi/v1/markPriceKlines` | 1-min mark price OHLCV | Real-time |
| Binance Futures REST | `/fapi/v1/openInterestHist` | Open interest (5-min buckets) | Per 5 min |
| Bybit REST v5 | `/v5/market/funding/history` | Historical funding rates | Per settlement |
| Hyperliquid API | `/info` → `fundingHistory` | Funding rate history | Per settlement |
| Hyperliquid API | `/info` → `candleSnapshot` | 1-min OHLCV | Real-time |
| CoinGlass | Web scrape or API | Cross-exchange funding rate dashboard | Real-time |
| Economic calendar | `https://economic-calendar.tradingeconomics.com` or Forex Factory API | Macro event schedule | Daily |

All data sources are free and publicly accessible. No data vendor relationships required for backtesting. Hyperliquid API requires no authentication for public market data.

---

## Implementation Notes

### Execution on Hyperliquid
- Hyperliquid uses a continuous funding model (accrued per second, settled continuously) rather than discrete 8-hour settlement — **this changes the mechanism materially for Hyperliquid-native positions.**
- The backtest should be run on Binance/Bybit data where discrete settlement applies.
- For live execution on Hyperliquid, the strategy trades the *price impact* of Binance/Bybit settlement flows spilling into Hyperliquid via arbitrageurs — not the direct funding avoidance incentive. This cross-venue transmission hypothesis requires separate validation.
- Alternative: Execute on Binance or Bybit directly where the discrete settlement mechanism applies natively.

### Automation requirements
- Cron job or event-driven scheduler firing at T−12min, T−10min, T+2min, T+5min
- Real-time funding rate monitor polling every 60 seconds to confirm filter conditions have not changed since last check
- Order management system capable of limit → aggressive limit → cancel logic within 2-minute windows
- PnL and fill rate logging per trade for ongoing monitoring

### Next steps (ordered)
1. Pull Binance BTC-PERP 1-min mark price data from January 2021 to present via REST API
2. Pull corresponding funding rate history for the same period
3. Implement backtest engine per the step-by-step methodology above
4. Run segment analysis (high/moderate/low funding regimes)
5. Run session analysis (00:00 vs 08:00 vs 16:00 UTC)
6. Run decay analysis by calendar year
7. If backtest passes go-live criteria, begin paper trading on Binance testnet
8. After 30 qualifying paper trade events, evaluate against paper trading go-live criteria
9. If paper trading passes, deploy with minimum position size ($10,000 notional) for first 30 live trades
