---
title: "Insurance Fund Depletion as ADL Overhang Signal — Contrarian Fade"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - liquidation
  - exchange-structure
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's insurance fund drops >5% within a 24-hour window, Auto-Deleveraging (ADL) is occurring or has recently occurred. ADL mechanically force-closes profitable positions on the winning side of the market to cover insolvent positions that the insurance fund cannot fully absorb. These forced closures are non-consensual, price-insensitive sells (in an uptrend) or buys (in a downtrend), creating a temporary artificial counter-trend price distortion. This distortion should partially reverse within 24–48 hours as the mechanical pressure dissipates and organic price discovery resumes.

**Causal chain:**
1. Large position(s) become insolvent → liquidation engine triggers
2. Insurance fund absorbs losses → fund balance drops measurably
3. If losses exceed fund capacity → ADL activates
4. ADL engine ranks profitable counterparties by profit percentage + leverage, force-closes them at mark price
5. Profitable longs (in uptrend) or profitable shorts (in downtrend) are closed at market → creates mechanical counter-trend flow
6. Price temporarily overshoots in the counter-trend direction
7. Once ADL clears, organic buyers/sellers resume → partial mean reversion

The edge is not "markets mean-revert after volatility." The edge is specifically that ADL closes positions at mark price regardless of the holder's intent, creating identifiable, non-economic selling or buying pressure with a known mechanical cause.

---

## Structural Mechanism — WHY This Must Happen

ADL is a **protocol-level contractual rule**, not a discretionary decision. Hyperliquid's ADL mechanism is deterministic:

- When a liquidation results in a loss exceeding the insurance fund balance, ADL **must** trigger — there is no human override
- ADL **must** close positions ranked highest on the ADL priority queue (highest profit % × leverage), regardless of market conditions
- These closures happen at **mark price**, not limit orders — they are guaranteed fills that do not "wait" for favorable prices
- The direction of ADL pressure is **mechanically determined** by the direction of the winning trade: if longs are winning (uptrend), ADL closes longs → net sell pressure

This is structurally similar to forced redemptions in a fund: the selling is not driven by a view on price, it is driven by a rule. The rule creates predictable, temporary, non-economic flow.

**Why the insurance fund is the signal:** ADL only triggers when the fund is insufficient. A rapid fund drawdown (>5% in 24h) is the observable proxy for "ADL is happening or about to happen." The fund balance is the canary.

**Caveat on "must":** The price impact of ADL is not guaranteed — it depends on the size of positions being closed relative to available liquidity. This is why the score is 5, not 8. The mechanism is contractual; the magnitude of price distortion is probabilistic.

---

## Entry/Exit Rules

### Signal Detection
- **Trigger:** Hyperliquid insurance fund balance decreases by ≥5% within any rolling 24-hour window
- **Direction determination:** Look at the 24-hour price return of the asset with the largest open interest on Hyperliquid at time of trigger (typically BTC or ETH perp)
  - If 24h return > +2%: assume longs are profitable → ADL is closing longs → **SHORT**
  - If 24h return < -2%: assume shorts are profitable → ADL is closing shorts → **LONG**
  - If 24h return is between -2% and +2%: **NO TRADE** (direction ambiguous, ADL pressure unclear)

### Entry
- Enter at the **open of the next 4-hour candle** after signal confirmation
- Use the asset with the highest open interest on Hyperliquid at time of signal (use HL API to confirm — typically BTC-PERP)
- Entry is a **market order** (or aggressive limit within 0.1% of mid)

### Exit
- **Primary exit:** Close position at the open of the candle 48 hours after entry
- **Stop loss:** 3% adverse move from entry price (hard stop, no exceptions)
- **Early exit trigger:** Insurance fund balance returns to within 1% of pre-event level AND price has moved ≥1.5% in the trade direction — take profit early
- **No trailing stop** — this is a mean-reversion trade with a defined time window, not a trend follow

### Filters (apply ALL before entering)
1. Insurance fund drop must be ≥5% — not cumulative over multiple days, must be within a single 24h window
2. Funding rate on the target asset must not be extreme (|funding rate| < 0.1% per 8h) — extreme funding suggests a different structural dynamic is dominant
3. No major scheduled protocol events (token unlocks, governance votes affecting collateral) within 24h that could confound the signal
4. Minimum insurance fund starting balance: $1M (below this, small absolute moves can create false percentage signals)

---

## Position Sizing

- **Base size:** 0.5% of total portfolio per trade
- **Rationale:** Low-frequency signal, uncertain magnitude of price impact, small sample size expected — this is a monitoring/learning position, not a core allocation
- **Leverage:** 2x maximum on the perp position
- **Effective portfolio exposure:** 1% of portfolio per event
- **No pyramiding** — single entry, single exit
- **Maximum concurrent positions:** 1 (if two signals fire simultaneously, take the one with larger insurance fund % drop)

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Notes |
|---|---|---|---|
| Hyperliquid insurance fund balance | HL API: `https://api.hyperliquid.xyz/info` (POST, type: `"clearinghouseState"`) or on-chain via Arbitrum | Daily minimum, hourly preferred | **Critical gap: verify historical depth. HL launched ~2023. Check if fund balance is stored on-chain or only current state is queryable.** |
| Hyperliquid perp OHLCV | HL API: `https://api.hyperliquid.xyz/info` (type: `"candleSnapshot"`) | 4-hour candles | Available for all listed perps |
| Open interest by asset | HL API: type `"openInterest"` | Daily | To identify which asset to trade |
| Funding rates | HL API: type `"fundingHistory"` | 8-hour | For filter #2 |
| ADL event logs | Hyperliquid Discord announcements / on-chain event logs | Event-level | Cross-reference to validate insurance fund signal |

### Historical Data Availability — Known Risk
Hyperliquid's insurance fund balance may not have a queryable historical API endpoint — the current API may only return the live balance. **First task before any backtest: verify whether historical fund balance data exists.** Options if it doesn't:
- Check if Hyperliquid stores fund balance in smart contract state history (queryable via Arbitrum archive node)
- Check community-built dashboards: Dune Analytics (`https://dune.com` — search "Hyperliquid insurance fund"), Parsec Finance
- If no historical data exists, begin **prospective data collection immediately** and run a forward-looking paper trade study

### Backtest Steps

1. **Reconstruct insurance fund balance time series** at daily (or finer) granularity from earliest available date
2. **Identify all events** where fund dropped ≥5% in a 24h window
3. **For each event:** record entry price (open of next 4h candle), direction (based on 24h return rule), stop level, and outcomes at 24h, 48h, and 72h
4. **Apply all filters** retroactively and note how many events survive
5. **Calculate per-trade P&L** in % terms (not dollar terms — normalize for comparability)
6. **Metrics to compute:**
   - Win rate
   - Average win / average loss
   - Expectancy per trade (win rate × avg win − loss rate × avg loss)
   - Maximum drawdown across the event series
   - Sharpe ratio (if sample size ≥ 20 events; if < 20, report raw stats only — do not compute Sharpe on small samples)
   - Time-in-market (this strategy will be out of market >95% of the time)

### Baseline Comparison
- **Null hypothesis:** Random 48h directional trades on BTC-PERP with same sizing
- **Baseline:** Buy-and-hold BTC over the same period
- The strategy must beat random direction selection at the same frequency to demonstrate the ADL signal adds information

### Sample Size Warning
Hyperliquid launched in 2023. Severe ADL events (fund drop >5% in 24h) may have occurred fewer than 10 times in the available history. **If N < 10 events, the backtest is illustrative only — do not draw statistical conclusions.** The primary value of the backtest at this stage is to verify the causal mechanism and establish a data collection pipeline.

---

## Go-Live Criteria (Paper Trading)

Before moving to paper trade, the backtest must show:

1. **Minimum 5 historical events** identified (if fewer, skip to prospective paper trade immediately)
2. **Win rate ≥ 55%** across all historical events (above random chance)
3. **Average win ≥ 1.5× average loss** (positive expectancy even at 50% win rate)
4. **No single event** produced a loss exceeding 5% of the position (validates stop logic)
5. **Direction rule is correct** in ≥60% of cases (i.e., the 24h return correctly identifies which side ADL is closing)
6. **Data pipeline is live** — insurance fund balance is being polled at minimum every 4 hours with alerts configured

If historical data is insufficient (N < 5), go directly to **prospective paper trading** with a 6-month observation window before any live capital deployment.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Historical backtest shows negative expectancy** (even accounting for small sample size, if every event is a loser, the mechanism is wrong)
2. **Hyperliquid changes ADL mechanics** — any protocol upgrade that modifies how ADL is triggered or executed invalidates the causal chain. Monitor governance proposals at `https://hyperliquid.xyz/blog` and Discord
3. **Insurance fund balance becomes unqueryable** — if HL removes API access to fund balance data, the signal cannot be detected
4. **After 10 live paper trades:** win rate < 45% AND expectancy is negative — signal is not working in live conditions
5. **Hyperliquid introduces a socialized loss mechanism** that replaces ADL — this would eliminate the forced-close pressure entirely
6. **Insurance fund grows to >$50M** — at very large fund sizes, the fund can absorb most liquidations without triggering ADL, making the signal rare to the point of uselessness

---

## Risks — Honest Assessment

### Data Risk (HIGH)
The entire strategy depends on historical insurance fund balance data that may not exist in queryable form. This is the single biggest blocker. Do not spend engineering time on the backtest until data availability is confirmed.

### Sample Size Risk (HIGH)
Severe ADL events are rare by design — Hyperliquid's insurance fund exists precisely to prevent ADL. A well-capitalized fund means few events. Small N means any backtest results are statistically unreliable. This strategy may take 12–24 months of live observation to accumulate meaningful data.

### Confounding Causes of Fund Drawdown (MEDIUM)
The insurance fund can decrease for reasons other than ADL:
- Protocol fee changes
- Manual withdrawals by the team (if fund is not fully autonomous)
- Accounting adjustments
Each apparent signal must be cross-referenced against ADL event logs (Hyperliquid Discord posts ADL notices) to confirm the cause.

### Magnitude Uncertainty (MEDIUM)
Even when ADL occurs, the price impact depends on the size of positions being closed relative to market depth. A small ADL event in a liquid market may produce no measurable price distortion. The 5% fund drop threshold is a guess — the correct threshold needs empirical calibration.

### Timing Lag (MEDIUM)
By the time the insurance fund drop is detected (polling interval), the ADL may already be complete and the price distortion may have already partially reversed. The entry rule (next 4h candle open) may be entering after the best price. Consider whether a shorter polling interval (1h or 15min) would improve entry timing.

### Hyperliquid-Specific Risk (LOW-MEDIUM)
Hyperliquid is a single exchange. If the exchange has operational issues, is hacked, or changes its mechanics, the strategy fails entirely. Do not size this as a core position.

### Adverse Selection Risk (LOW)
The assets being ADL'd are the ones with the most extreme moves. Fading an extreme move means fading momentum, which has a known negative carry in trending markets. The stop loss at 3% is essential — without it, this strategy can have catastrophic losses in sustained trends.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|---|---|---|
| Hyperliquid REST API | `https://api.hyperliquid.xyz/info` | Fund balance (POST `{"type": "clearinghouseState", "user": "<insurance_fund_address>"}`) — **verify the insurance fund wallet address first** |
| Hyperliquid candle data | `https://api.hyperliquid.xyz/info` POST `{"type": "candleSnapshot", "req": {"coin": "BTC", "interval": "4h", "startTime": <ms>, "endTime": <ms>}}` | OHLCV for BTC, ETH perps |
| Hyperliquid funding history | `https://api.hyperliquid.xyz/info` POST `{"type": "fundingHistory", "coin": "BTC"}` | 8h funding rates |
| Dune Analytics | `https://dune.com/search?q=hyperliquid+insurance` | Community dashboards with historical fund balance — check for pre-built queries |
| Hyperliquid Discord | `https://discord.gg/hyperliquid` | ADL event announcements (manual cross-reference) |
| Hyperliquid docs | `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/adl` | ADL mechanics specification — read before building anything |
| Arbitrum archive node | Via Alchemy/Infura Arbitrum endpoint | On-chain fund balance history if API doesn't provide it |

**First action item:** Query `https://api.hyperliquid.xyz/info` with the insurance fund address and confirm (a) the address is correct, (b) historical balance states are accessible, and (c) the data goes back to at least mid-2023. If historical data is not available via API, file a Dune query to extract it from on-chain state.
