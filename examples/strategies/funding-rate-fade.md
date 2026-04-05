---
title: "Funding Rate Fade"
status: "HYPOTHESIS"
mechanism: 6
implementation: 8
safety: 6
frequency: 9
composite: 2592
categories:
  - funding-rates
  - exchange-structure
pipeline_stage: "Pre-backtest (step 2 of 9)"
created: "2025-01-27"
modified: "2026-04-05"
---

## Hypothesis

When perpetual futures funding rates reach statistically extreme levels — top or bottom 5% of their 90-day rolling distribution — the position paying funding is overcrowded. A mean-reversion trade going against the crowded side should earn both directional P&L as the crowd unwinds and positive carry as funding normalizes back toward its median.

The edge is structural: funding rate extremes are self-correcting by mechanism. High positive funding increases the cost of holding longs, which attracts shorts, which compresses funding back toward zero. The trade earns carry while waiting for this mechanical reversion.

This is distinct from simple carry farming (holding the funded side indefinitely). Here the entry criterion is *extreme crowding*, and the exit criterion is *reversion to median* — a defined, time-bounded bet on mean reversion, not an open-ended carry harvest.

---

## Why it's an edge

1. **Mechanistic, not just statistical.** Funding rates must mean-revert — if they did not, arbitrageurs would drain the funded side indefinitely. The mechanism enforces reversion; history just tells us how quickly.

2. **Carry income partially hedges directional risk.** During the hold period, the position is on the receiving side of funding settlements every 8 hours. This is real cash income that offsets adverse price movement.

3. **Signal is objectively defined.** Entry is triggered by a percentile crossing on observable data — no discretion, no interpretation.

4. **High event frequency.** Across 20+ Hyperliquid perps, extreme funding events occur multiple times per month. Zunid can monitor all simultaneously and execute programmatically.

5. **No calendar dependency.** The strategy is always active. It does not require scheduled events, which makes it complementary to Strategy 001.

6. **Asymmetric information is not required.** The data (funding rates) is public and free. The edge comes from systematic execution, not information advantage.

---

## Backtest Methodology

### Data required

| Data | Source | Endpoint / Notes |
|---|---|---|
| Historical 8h funding rates | Hyperliquid API | `POST /info` with `{"type": "fundingHistory", "coin": "<TOKEN>", "startTime": <ms>}` |
| Historical perp OHLCV (1h candles) | Hyperliquid or Binance Futures | Binance: `GET /fapi/v1/klines` — use as cross-check |
| Funding settlement timestamps | Fixed schedule | 00:00, 08:00, 16:00 UTC daily |
| Token list | Hyperliquid `/info` → `meta` | Pull all listed perps, filter by 30d avg open interest > $10M |

Start with BTC and ETH to establish a clean baseline. Expand to SOL, AVAX, ARB, SUI once BTC/ETH results are understood. Do not start with low-liquidity alts.

### Backtest construction

**Step 1 — Compute rolling percentiles**
For each token, compute the 90-day rolling percentile rank of each 8h funding observation. Use an expanding window for the first 90 days (warm-up period). This gives a time series of `funding_percentile[t]` for each token.

**Step 2 — Identify entry events**
- Long signal: `funding_percentile[t] < 0.05` (shorts overcrowded, funding extreme negative)
- Short signal: `funding_percentile[t] > 0.95` (longs overcrowded, funding extreme positive)
- Only one open position per token at a time
- Entry price: open of the next 1h candle after the funding settlement that triggered the signal

**Step 3 — Simulate exits**
For each entry, track three possible exit conditions and take the first that triggers:
1. **Reversion exit:** `funding_percentile[t] crosses back through 0.50` (funding returns to median)
2. **Time stop:** 7 calendar days elapsed since entry — force close at next candle open
3. **Loss stop:** Position mark-to-market loss exceeds 5% of notional — force close at next candle open

**Step 4 — Compute P&L per trade**
```
trade_pnl = directional_pnl + carry_collected - fees - slippage_estimate

directional_pnl = (exit_price - entry_price) / entry_price * direction * notional
carry_collected = sum of funding_rate[t] * notional for each 8h period held (positive because we are on receiving side)
fees = 0.045% * notional * 2  (taker entry + taker exit)
slippage_estimate = 0.05% round-trip (conservative estimate for liquid tokens)
```

**Step 5 — Vary the threshold**
Run the full simulation at entry thresholds of 97.5th/2.5th, 95th/5th, 90th/10th percentile. Report results at each threshold separately. Do not pick the best one and report only that — report all three.

**Step 6 — Regime decomposition (critical)**
Split the backtest period into sub-periods:
- 2022 (bear market, high volatility)
- 2023 (recovery, ranging)
- 2024–2025 (bull run, trend)

Report performance separately for each sub-period. If the strategy works in 2022–2023 but not in 2024–2025, flag as potentially decaying or regime-dependent. This is the most important diagnostic.

### Metrics to report

| Metric | Definition |
|---|---|
| Total trades | Count of completed round-trips |
| Win rate | % of trades with positive net P&L |
| Average trade return | Mean net P&L as % of notional |
| Median hold time | In hours |
| Average carry collected | Mean funding income per trade as % of notional |
| Maximum drawdown | Peak-to-trough on cumulative P&L curve |
| Sharpe ratio (annualized) | Using trade-level returns, annualized |
| % trades exiting via reversion | vs. time stop vs. loss stop |
| Edge vs. baseline | Compare to random entry at same frequency — see below |

### Baseline comparison

Generate a random entry baseline: for each real entry event, generate 5 randomly timed entries within the same 90-day window, with the same exit rules applied. Compare average returns. The strategy must show statistically meaningful outperformance over this baseline, not just positive average returns. A positive average return in a bull market on a long-biased baseline is noise.

### Minimum acceptable result to proceed to paper trading

- ≥ 50 total trades across BTC + ETH combined
- Average net return per trade > 0.5% after all costs
- Win rate > 55%
- Max drawdown on cumulative P&L < 15%
- Performance in 2024–2025 not significantly worse than 2022–2023 (no more than 50% degradation in average return)
- Edge vs. random baseline: strategy average return at least 1.5x the random baseline

---

## Entry Rules

**Universe:** All Hyperliquid perps with 30-day average open interest > $10M. Start with BTC, ETH only; expand after baseline is validated.

**Signal computation (runs after each 8h funding settlement):**
1. Pull the most recent 8h funding rate for each token in universe
2. Pull the trailing 90-day history of 8h funding rates for each token
3. Compute the percentile rank of the current rate within the trailing 90-day distribution
4. If `percentile > 0.95`: short signal active for this token
5. If `percentile < 0.05`: long signal active for this token
6. If there is already an open position in this token: skip (no pyramid)

**Entry execution:**
- Trigger: funding settlement at 00:00, 08:00, or 16:00 UTC
- Execution: market order placed within 5 minutes of settlement timestamp
- Order type: taker (market or aggressive limit to ensure fill)
- Confirm fill before recording position open

---

## Exit Rules

Monitor open positions after every 8h funding settlement and every 1h candle close.

**Exit condition 1 — Reversion (primary):**
- Check current funding percentile for the token
- If the position is short and `funding_percentile < 0.50`: close the short
- If the position is long and `funding_percentile > 0.50`: close the long
- This means funding has returned to its median — the structural basis for the trade is resolved

**Exit condition 2 — Time stop:**
- If position has been open ≥ 7 calendar days: close at next candle open regardless of P&L
- Rationale: if funding has not reverted in 7 days, the signal is stale and holding longer increases directional risk without increasing expected carry significantly

**Exit condition 3 — Loss stop:**
- Monitor mark-to-market price continuously
- If unrealized loss on the position (mark-to-market, before carry) exceeds 5% of notional: close immediately at market
- This stop is based on *directional* loss only — carry income is not netted against it for the purpose of triggering the stop
- Rationale: a 5% directional loss suggests the trend is dominating the mean-reversion signal; the structural thesis is breaking down

**Exit execution:**
- All exits: market order (taker), executed within 5 minutes of trigger
- Record exit price, carry collected during hold, fees, net P&L

---

## Position Sizing

**Paper trading phase:**
- $200 notional per trade, no leverage
- Maximum 3 concurrent open positions across the full universe (prevents correlated drawdown when market is trending and all signals fire simultaneously)
- All positions equal weight — no scaling by signal strength in paper phase

**If/when live:**
- Start at $500 notional per trade, 1x leverage (no margin amplification until strategy is validated live)
- Maximum 5 concurrent open positions
- Do not exceed 20% of account in this strategy at any time
- Consider reducing max concurrent positions to 2 during high-volatility regimes (BTC realized vol > 80% annualized)

**Leverage note:** This strategy does not require leverage. The edge comes from carry + mean reversion, not leveraged price exposure. Leverage amplifies the directional risk (the main way this strategy loses) without proportionally amplifying the carry income. Keep leverage at 1x until there is a compelling reason to change this.

---

## Go-Live Criteria

Deploy real capital when all of the following are true:

1. Backtest on BTC + ETH meets minimum acceptable results (defined above)
2. At least 5 paper trades closed with net P&L positive after fees and funding
3. No single paper trade lost more than 8% of notional
4. At least one paper trade exited via the reversion condition (not just time stop or loss stop) — confirms the mechanism is working as hypothesized
5. Founder has reviewed backtest results and approved capital allocation
6. Hyperliquid API key and USDC deposit already in place (can share infrastructure with Strategy 001)

---

## Kill Criteria

**Kill immediately (any time):**
- Any single live trade loses > 8% of notional
- Funding API data feed fails for > 24 hours and cannot be restored (strategy is blind without it)
- Hyperliquid changes its funding mechanism in a way that breaks the signal definition

**Kill after paper trading:**
- After 5 paper trades: net P&L negative after all costs → kill or redesign threshold
- After 10 paper trades: average net return < 0.3% per trade → insufficient edge for capital allocation

**Kill after going live:**
- After 20 live trades: net P&L negative after all costs → kill
- After 20 live trades: edge vs. random baseline has disappeared (average return < 0.5x random baseline) → kill
- Any 30-day period where strategy loses > 10% of allocated capital → pause, review, require explicit re-approval to resume

**Flag for review (do not kill automatically):**
- Win rate drops below 45% over trailing 10 trades → review signal threshold
- Average hold time to reversion exceeds 5 days (signal taking longer to resolve) → review whether edge is decaying
- Proportion of exits via loss stop exceeds 40% → regime may be trending; consider disabling during high-trend periods

---

## Risks

**Trend persistence (primary risk)**
The most dangerous scenario: funding goes extreme positive because a token is in a sustained bull trend, and the trend continues for weeks. The position is short into a rising market. The 5% loss stop is the load-bearing risk control here — it must not be overridden. Without it, this strategy has unbounded downside in a strong trend.

Mitigation: Hard loss stop at 5% of notional, no exceptions. Consider adding a trend filter: if BTC 30-day realized return > 20%, reduce position sizing by 50% or disable new entries entirely.

**Correlated simultaneous signals**
During market-wide euphoria or panic, extreme funding may appear across many tokens at once. If the strategy enters 10 concurrent short positions during a bull run, the portfolio becomes a concentrated directional bet, not a diversified strategy.

Mitigation: Hard cap of 3 concurrent positions. When the cap is hit, queue additional signals and enter as positions close.

**Carry not covering directional loss**
In typical conditions, 8h funding at extreme levels might be 0.05–0.15% per period. Over a 7-day hold (21 periods), that is 1.0–3.15% carry. A single 5% adverse price move wipes this out entirely.

Mitigation: The loss stop at 5% ensures the maximum loss on any trade is bounded. The carry income is a bonus, not a buffer that should encourage holding losers.

**Exchange and smart contract risk**
Hyperliquid is a relatively new decentralized exchange. It has not been through a major market stress event at full scale. There is non-trivial smart contract risk and operational risk (API downtime, liquidation engine behavior under stress).

Mitigation: Keep position sizes small. Do not concentrate more than 20% of total capital in this strategy. Monitor Hyperliquid operational status.

**Signal crowding**
Funding rate mean-reversion is a known strategy. Sophisticated market makers and prop firms already trade this. The edge may be partially priced in, which would reduce average returns.

Mitigation: The backtest will reveal whether measurable edge remains after costs. The regime decomposition (2022 vs. 2024–2025) will show whether the edge is decaying over time as the market matures. If degradation is severe, kill before paper trading.

**Data quality**
Hyperliquid's funding history for newer tokens may be short or sparse. Percentile calculations require at least 90 days of history to be reliable; for tokens with less history, the signal should be disabled until sufficient data accumulates.

Mitigation: Enforce a minimum 90-day history requirement before enabling the signal for any token. BTC and ETH have years of history — start there.

---

## Data Sources

| Data | Source | Access method |
|---|---|---|
| Historical 8h funding rates | Hyperliquid | `POST https://api.hyperliquid.xyz/info` → `{"type": "fundingHistory", "coin": "BTC", "startTime": <epoch_ms>}` |
| Live funding rates | Hyperliquid | `POST /info` → `{"type": "metaAndAssetCtxs"}` — includes `funding` field per token |
| Historical OHLCV (perps) | Binance Futures | `GET https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1h` |
| Historical funding rates (cross-check) | Binance Futures | `GET https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=1000` |
| Token universe (all listed perps) | Hyperliquid | `POST /info` → `{"type": "meta"}` — returns all listed assets |

All data sources are free and require no authentication for read access. Binance Futures API is used as a cross-check for BTC/ETH funding history given its longer track record. Primary execution data should come from Hyperliquid since that is the execution venue.

---

## Implementation notes

**Shares infrastructure with Strategy 001.** The Hyperliquid wallet, API key, and paper trading framework already being built for Strategy 001 can be reused directly. The primary addition is a funding rate monitor that runs after each 8h settlement.

**Suggested backtest script structure:**
```
experiments/
  funding_backtest.py       # main backtest
  funding_paper_trader.py   # paper trading extension (after backtest passes)
  funding_history_cache/    # local cache of pulled
