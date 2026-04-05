---
title: "Perp-Spot Basis Convergence"
status: KILLED
mechanism: 7
implementation: 8
safety: 6
frequency: 9
composite: 3024
categories:
  - basis-trade
  - funding-rates
created: "2025-01-31"
pipeline_stage: "Killed at backtest (step 4 of 9)"
killed: "2026-04-04"
kill_reason: "Basis spreads too thin after costs. Binance backtest: 1 signal across 7 tokens over months at 0.40% threshold. HL premium data confirms: spreads exceed 0.15% only 2-4% of the time, never reach 0.30%. Round-trip costs (0.09-0.19%) consume any convergence profit."
---

## Hypothesis

Crypto perpetual futures prices periodically diverge from their spot equivalents by measurable amounts. Because perps have no expiry, this basis is mechanically bounded: extreme divergences become expensive to hold (via funding payments), and arbitrageurs accelerate reversion. When the perp/spot basis exceeds a threshold and holds there for multiple consecutive readings, a short-perp (or long-perp) position captures both price reversion and incoming funding payments.

The entry signal is the **basis spread itself** — not the funding rate level — sampled at 15-minute intervals. This makes it a faster, more precise trigger than funding-rate-based strategies, which react to an 8-hour lagging metric.

*Hypothesis — needs backtest.*

---

## Why it's an edge

1. **Mechanical reversion force.** Perp prices are tethered to spot by the funding mechanism. Unlike statistical mean-reversion, the reversion here is structurally enforced: elevated basis increases the cost of holding the position that created it, incentivizing unwind.

2. **Dual return source.** A short-perp position opened when basis is significantly positive earns both (a) price convergence as the basis closes and (b) funding payments from longs during the holding period. These two sources are partially independent — even if price reversion is slow, funding accrues every 8 hours.

3. **Publicly observable, non-speed-dependent.** The basis is computable from two public API calls. No proprietary data feed or latency advantage is required. The edge comes from systematic monitoring and disciplined execution, not speed.

4. **Distinct from unlock shorts (Strategy 001).** Strategy 001 is a directional medium-term trade driven by a calendar event. This strategy is market-neutral in intention (it bets on convergence regardless of direction), operates on a shorter timeframe (hours to 48 hours), and is triggered by a price-structure condition rather than an event schedule.

5. **Distinct from raw funding-rate fade.** The trigger here is the **basis spread in price space**, not the funding rate. Funding rates are 8-hour lagging averages; basis can spike and revert within a single funding period. Entering on basis rather than on announced funding captures the reversion before the funding rate has fully adjusted.

---

## Backtest Methodology

### Data required

| Data | Source | Granularity | Notes |
|---|---|---|---|
| Perp OHLCV | Hyperliquid API (`/info` candles) or CoinGlass | 15-minute | BTC, ETH, SOL, ARB, SUI, OP, APT |
| Spot OHLCV | Binance `/api/v3/klines` | 15-minute | Same tokens; use mid (H+L)/2 or close |
| Historical funding rates | Hyperliquid `/info` → `fundingHistory` | 8-hour | For cost accounting |

**Backtest window:** 18 months (January 2024 – June 2025) where available. Use at least 12 months minimum.

**Tokens to include:** BTC, ETH, SOL, ARB, SUI, OP, APT. Start with higher-liquidity tokens (BTC, ETH, SOL) to establish baseline, then extend.

---

### Basis construction

```
basis_pct = (perp_mid - spot_mid) / spot_mid * 100
```

Use the close price of each 15-minute bar for both perp and spot. Where clocks differ across venues, align on UTC bar close.

---

### Signal definition

**Long signal (short perp):**
- `basis_pct > +0.40` on current bar AND
- `basis_pct > +0.30` on the immediately preceding bar

**Short signal (long perp):**
- `basis_pct < -0.40` on current bar AND
- `basis_pct < -0.30` on the immediately preceding bar

Requiring two consecutive readings above threshold filters single-candle data artifacts and flash events that revert before entry is possible.

---

### Exit rules (first triggered)

| Exit condition | Trigger |
|---|---|
| **Target:** Basis reverts | `abs(basis_pct) <= 0.10` |
| **Time stop** | 48 hours elapsed since entry |
| **Stop loss** | Basis moves 0.60% further against position (e.g., for a short-perp entry at basis +0.45%, stop at basis +1.05%) |

---

### Cost model

Apply to every simulated trade:

- **Trading fees:** 0.045% taker per leg → 0.09% round-trip
- **Funding:** Credit or debit funding payments that would accrue during hold period, using actual historical 8-hour funding rates from Hyperliquid. Short perp during positive funding collects; long perp during negative funding collects.
- **Slippage:** Add 0.05% per leg as a conservative estimate for liquid tokens (BTC, ETH, SOL). Use 0.10% per leg for smaller tokens (ARB, SUI, OP, APT).

Total cost floor per trade: ~0.19% round-trip before funding adjustment.

---

### Metrics to compute

| Metric | Target |
|---|---|
| Total trades | ≥ 100 across all tokens and period |
| Win rate | > 55% |
| Average P&L per trade (after all costs) | > 0.15% notional |
| Average hold time | < 24 hours (validates short-duration hypothesis) |
| Maximum consecutive losses | < 6 |
| Sharpe ratio (annualized, trade-level) | > 1.0 |
| Maximum drawdown | < 15% of allocated notional |
| % trades hitting time stop vs. target | Want ≤ 30% time stops |
| % trades hitting stop loss | Want ≤ 15% |

---

### Baseline comparison

Compare against:
1. **Random entry baseline:** Enter a short-perp position on a randomly selected 15-minute bar (no basis filter), hold for the same average duration as signal trades. This controls for directional drift.
2. **Naive funding-rate fade:** Enter any short when 8-hour funding rate > 0.05%. Directly tests whether the basis trigger adds value over the lagging funding signal.

The strategy must outperform both baselines net of costs to proceed.

---

### Regime segmentation

Separately report results for:
- **Trending periods** (price moved >15% in either direction over the prior 7 days at entry)
- **Sideways periods** (price movement ≤ 5% over the prior 7 days)
- **High-volatility periods** (30-day realized volatility above 75th percentile)

This tests whether the edge is regime-dependent and informs future conditional filters.

---

## Entry Rules


### Data inputs

- Perp mid: Hyperliquid `/info` → `allMids` (polled every 15 minutes at bar close)
- Spot mid: Binance `/api/v3/ticker/price` (same timestamp)
- Basis: computed locally

### Entry logic

```python
# Pseudo-code
basis_current = (perp_mid - spot_mid) / spot_mid * 100
basis_prev = basis_at_previous_15min_bar

if basis_current > 0.40 and basis_prev > 0.30:
    enter_short_perp(token, notional=POSITION_SIZE)
    record_entry(basis=basis_current, timestamp=now)

elif basis_current < -0.40 and basis_prev < -0.30:
    enter_long_perp(token, notional=POSITION_SIZE)
    record_entry(basis=basis_current, timestamp=now)
```

## Exit Rules

### Exit logic (checked every 15 minutes)

```python
current_basis = compute_basis(token)
hours_held = (now - entry_timestamp).hours
basis_move_against = current_basis - entry_basis  # for short-perp trades

if abs(current_basis) <= 0.10:
    exit_position(reason="target_reached")
elif hours_held >= 48:
    exit_position(reason="time_stop")
elif basis_move_against >= 0.60:  # for short-perp; flip sign for long-perp
    exit_position(reason="stop_loss")
```

### Concurrent position management

- **Maximum 1 open position per token** at any time. Do not pyramid into an existing trade.
- **Maximum 3 open positions across all tokens simultaneously.** Basis spikes tend to be correlated (they often happen market-wide during liquidation cascades), so concurrent exposure can be higher than it appears.

---

## Position Sizing

**Paper trading phase:** $200 notional per trade, no leverage.

**Rationale:**
- Consistent with Strategy 001 paper trading scale
- Keeps maximum loss per trade (stop loss at 0.60% basis move + fees) to ~$1.50 — negligible, sufficient to validate mechanics
- At 3 concurrent positions: $600 maximum deployed

**Live phase (post-validation):** Scale to $500–$1,000 per trade. Leverage is not recommended for this strategy — the edge is in the spread, not in amplification, and leverage increases the cost of adverse basis persistence.

---

## Go-Live Criteria

Deploy real capital when all of the following are met:

1. Backtest shows average net P&L > 0.15% per trade across ≥ 100 simulated trades
2. Backtest shows ≤ 30% of trades hitting the time stop (high time-stop rate implies basis persistence, not convergence)
3. At least 10 paper trades closed across at least 3 different tokens
4. Paper trade net P&L positive after all costs
5. No single paper trade lost more than 8% of notional (stop loss should prevent this; a breach suggests slippage or execution failure)
6. Founder approves capital allocation

---

## Kill Criteria

Abandon this strategy at any point if:

- **Backtest kill:** Average net P&L after costs < 0.10% per trade, OR win rate < 50%, OR time-stop rate > 40% → backtest fails, do not proceed to paper trading
- **Paper trading kill (early):** First 5 paper trades net negative after costs → redesign or kill
- **Paper trading kill (late):** After 15 paper trades, net P&L < 0.10% per trade after all costs → kill
- **Regime kill:** If basis spikes in trending markets (>20% 7d move) are consistently failing, add a trend filter in redesign — do not keep deploying into a known-bad regime
- **Execution kill:** If actual fill prices show >0.15% slippage per leg on BTC/ETH (should be rare), the cost model is broken — kill and reassess

---

## Risks

**1. Basis persistence in trending markets.**
The largest risk. During strong directional moves (e.g., rapid BTC rally), perp premium can stay above 0.5% for 12–72 hours as leveraged longs pile in. This hits the time stop repeatedly and erodes capital even when the eventual reversion is large. Mitigation: regime segmentation in backtest; consider adding a trend filter if data confirms.

**2. Stop-loss whipsaw.**
A basis spike to +0.45% that then expands to +1.05% before reverting would stop out the trade at a loss, then leave the "correct" trade on the table. This is the fundamental tension between having a stop loss and capturing eventual reversion. There is no clean solution — the stop loss is necessary to prevent catastrophic loss if a trend persists for days. Accept some whipsaw as the cost of risk control.

**3. Execution across two venues.**
The basis is computed using Hyperliquid perp and Binance spot. We only trade the perp leg on Hyperliquid. The spot price is a reference, not a traded leg. This means slippage and fill timing on the perp side are the only execution costs, but the basis signal may show phantom divergence if Binance spot is temporarily illiquid or has a data lag. Mitigation: use multiple spot references (Binance + Coinbase) and only trigger when both agree.

**4. Crowding and signal decay.**
Basis arbitrage is a well-known strategy. If many participants enter when basis is elevated, reversion accelerates — making the signal self-defeating over time as competition increases. Monitor whether average hold time until reversion is shortening over the paper trading period.

**5. Correlation with Strategy 001.**
If Strategy 001 is short a token due to an upcoming unlock AND this strategy is also short perp on the same token due to positive basis, the portfolio is unintentionally double-short. Track cross-strategy exposure. At paper trading scale this is immaterial; at live scale it needs a portfolio-level position check.

**6. Funding rate timing mismatch.**
Funding settles every 8 hours at fixed timestamps (00:00, 08:00, 16:00 UTC on Hyperliquid). A trade entered 30 minutes before the next settlement collects the first funding payment quickly; a trade entered 30 minutes after misses it for nearly 8 hours. This creates noise in per-trade P&L. The backtest should account for this by computing exact funding accrued based on entry timestamp.

---

## Data Sources

| Data | Source | Endpoint / Notes |
|---|---|---|
| Perp prices (15m OHLCV) | Hyperliquid | `POST /info` with `{"type": "candleSnapshot", "req": {"coin": "BTC", "interval": "15m", ...}}` |
| Spot prices (15m OHLCV) | Binance | `GET /api/v3/klines?symbol=BTCUSDT&interval=15m` |
| Spot prices (secondary reference) | Coinbase | `GET /products/BTC-USD/candles` |
| Historical funding rates | Hyperliquid | `POST /info` with `{"type": "fundingHistory", "coin": "BTC", ...}` |
| Current basis (live) | Computed | `allMids` from Hyperliquid + `ticker/price` from Binance |
| Token-pair mapping | Manual | BTC→BTCUSDT (Binance) + BTC (Hyperliquid perp) — maintained in config file |

All sources are free and publicly accessible. No API keys required for read-only market data on either venue.

---

## Next Steps

1. **Build basis time series reconstruction script** for BTC, ETH, SOL across 18 months (15-minute resolution, aligned UTC bars)
2. **Run backtest** per methodology above; primary focus on time-stop rate — if >35%, the strategy premise (fast reversion) is wrong
3. **Regime segmentation analysis** — determine whether the edge exists only in sideways markets or also in trending markets
4. **Compare against baselines** (random entry, naive funding fade)
5. **If backtest passes:** configure paper trading monitor alongside Strategy 001's daily GitHub Actions run
6. **Report findings** before committing to paper trading
