---
title: "Funding Rate Differential Forced Convergence (Hyperliquid vs. CEX)"
status: HYPOTHESIS
mechanism: 4
implementation: 4
safety: 6
frequency: 10
composite: 960
categories:
  - funding-rates
  - basis-trade
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's 8-hour perpetual funding rate exceeds the equivalent Binance or OKX rate by more than 0.05% per period on the same underlying asset, a risk-free (modulo execution friction) income stream exists by shorting the Hyperliquid perp and longing the CEX perp. The spread cannot persist indefinitely because rational capital will enter the same trade until the differential is consumed. The causal chain is:

1. Hyperliquid funding rate spikes above CEX rate (demand imbalance — retail longs piling in on HL)
2. Spread exceeds all-in friction cost (fees + slippage + capital cost)
3. Arbitrageurs enter: short HL perp, long CEX perp
4. Short pressure on HL reduces funding rate; long pressure on CEX may slightly elevate it
5. Spread compresses until it no longer covers friction
6. Position is closed; profit = collected funding differential minus all-in costs

The edge is **not** in predicting direction. It is in being systematic across 50+ Hyperliquid markets simultaneously, catching episodic spikes that are too small or too short-lived for large desks to bother with, but large enough to clear friction costs.

---

## Structural Mechanism — WHY This MUST Happen

Perpetual futures funding rates are a **mechanical equilibration device**. The protocol forces longs to pay shorts (or vice versa) every 8 hours to keep the perp price anchored to spot. This is not a tendency — it is a smart contract rule.

The convergence pressure is therefore structural:

- **Funding rate = f(open interest imbalance).** If longs dominate on HL but the same asset is balanced on Binance, HL's rate will be higher. Any capital that can sit long on Binance and short on HL earns the spread with zero directional exposure.
- **The spread is bounded above by friction.** Capital enters until the net yield (HL rate − CEX rate − fees − slippage) = 0. This is an accounting identity, not a forecast.
- **The spread is bounded below by zero** (ignoring negative funding, which creates the mirror trade). Negative spreads trigger the reverse arb.
- **Convergence lag** is determined by: (a) how much capital can be deployed across two venues simultaneously, (b) withdrawal/deposit friction between venues, and (c) whether HL's user base is structurally one-sided (retail long bias). Lag is the risk, not the direction of convergence.

The mechanism is well-understood. The Zunid edge is **breadth** (monitoring all HL markets, not just BTC/ETH) and **systematic execution** (catching spikes in illiquid HL markets where arb capital is slower to arrive).

---

## Entry / Exit Rules

### Universe
All Hyperliquid perpetual markets with a matching perpetual on Binance **or** OKX. As of 2025, this covers approximately 60–80 pairs. Exclude any pair where Binance/OKX 24h perp volume < $5M (slippage risk too high).

### Funding Rate Spread Calculation
```
spread_t = HL_funding_rate_t − CEX_funding_rate_t
```
Both rates are expressed as **% per 8-hour period**. Use the rate that was **set** at the start of the period (not the predicted rate), as this is the rate that will actually be paid.

### Entry Conditions (ALL must be true)
1. `spread_t > 0.05%` for **2 consecutive 8-hour funding periods** (16 hours of sustained divergence — filters noise)
2. HL 24h volume for the pair > $2M (minimum liquidity)
3. No active token unlock event within 7 days for the underlying (avoid confounding supply shocks — check token unlock calendars)
4. CEX perp for the same asset has open interest > $10M (ensures CEX side can absorb the long without moving the rate)
5. Position not already open in this pair

### Trade Construction
- **Leg A:** Short HL perp — size $X notional
- **Leg B:** Long CEX perp (Binance preferred; OKX as fallback) — size $X notional
- Both legs opened within the same 8-hour funding window, before the next funding settlement
- Target entry within **30 minutes of the window open** to capture the next funding payment

### Exit Conditions (first trigger wins)
1. **Primary:** `spread_t < 0.01%` for 1 consecutive period (spread has compressed)
2. **Time stop:** Position held for > 72 hours (9 funding periods) regardless of spread
3. **Stop-loss:** `spread_t > 0.15%` for 2 consecutive periods (spread is widening — structural issue, not noise; exit to prevent further capital lock-up)
4. **Liquidity stop:** If either leg's bid-ask spread exceeds 0.1% at time of exit, use limit orders and accept up to 15-minute fill window before using market orders

### Reverse Trade (Mirror)
If `spread_t < −0.05%` (HL rate significantly below CEX), enter the mirror: **long HL perp, short CEX perp**. Same entry/exit logic applies with signs flipped.

---

## Position Sizing

### Per-Trade Sizing
- Maximum notional per trade: **2% of total strategy capital** per leg (so 4% total capital deployed per pair, split across two venues)
- Rationale: Capital fragmentation risk — funds locked on two venues simultaneously. Keep individual positions small enough that a stuck position doesn't impair the rest of the book.

### Portfolio-Level Limits
- Maximum simultaneous open pairs: **10** (20% of capital per leg at full deployment)
- Maximum exposure to any single underlying: **1 trade** (no doubling up if spread widens)
- No leverage beyond 2x on either leg — this is a yield trade, not a directional bet. Leverage amplifies liquidation risk from basis moves.

### Margin Allocation
- Allocate 60% of capital to HL (higher margin requirements, less capital-efficient)
- Allocate 40% of capital to CEX (Binance cross-margin is more efficient)
- Keep 20% of total capital as undeployed buffer for margin calls during volatile periods

---

## Backtest Methodology

### Data Sources
| Data | Source | Endpoint / Notes |
|---|---|---|
| HL funding rate history | Hyperliquid API | `GET https://api.hyperliquid.xyz/info` with `{"type": "fundingHistory", "coin": "BTC", "startTime": <unix_ms>}` — returns 8h rates, ~2 years history |
| Binance funding rate history | Binance API | `GET https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=1000` — 8h rates, full history |
| OKX funding rate history | OKX API | `GET https://www.okx.com/api/v5/public/funding-rate-history?instId=BTC-USD-SWAP` |
| HL trade prices / volume | Hyperliquid API | `{"type": "candleSnapshot", "req": {"coin": "BTC", "interval": "1h", ...}}` |
| Binance perp prices | Binance API | `GET https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1h` |

### Backtest Period
- **Primary:** January 2023 – December 2024 (covers HL's growth phase and multiple market regimes)
- **Out-of-sample validation:** January 2025 – present (walk-forward, not used for parameter tuning)

### Simulation Steps
1. For each 8-hour funding period, compute `spread_t` for all pairs in universe
2. Apply entry conditions — log all triggered entries with timestamp
3. Simulate position P&L:
   - **Funding income:** Sum of `spread_t` collected each period while position is open
   - **Fee cost:** 0.035% per side per leg (HL taker fee ~0.035%; Binance taker ~0.04%) — apply at entry and exit = ~0.15% round-trip total
   - **Slippage:** Apply 0.05% per side as conservative estimate for mid-cap pairs; 0.02% for BTC/ETH
   - **Basis risk:** Track the difference between HL perp price and CEX perp price at entry vs. exit — this is the hidden P&L driver that most backtests miss
4. Apply exit conditions in order of priority
5. Aggregate: total return, Sharpe ratio, max drawdown, average holding period, win rate, average spread collected per trade

### Key Metrics to Compute
| Metric | Target | Kill threshold |
|---|---|---|
| Net yield per trade (after all costs) | > 0.08% per 8h period held | < 0.02% |
| Win rate (positive net P&L per trade) | > 65% | < 50% |
| Average holding period | 2–5 funding periods | > 8 (capital lock-up too long) |
| Annualised Sharpe (strategy-level) | > 1.5 | < 0.8 |
| Max drawdown | < 5% of strategy capital | > 10% |
| Basis risk contribution | < 30% of gross P&L variance | > 60% (basis is dominating) |

### Baseline Comparison
Compare against: (a) simple cash yield (USDC lending rate on Hyperliquid), and (b) single-venue funding rate farming (long spot + short HL perp). The cross-exchange arb must beat both baselines after accounting for capital fragmentation costs.

### Parameter Sensitivity Tests
- Vary entry threshold: 0.03%, 0.05%, 0.07%, 0.10% — check if 0.05% is optimal or arbitrary
- Vary confirmation periods: 1, 2, 3 consecutive periods
- Vary time stop: 48h, 72h, 96h
- Check performance by market cap tier: large cap (BTC/ETH) vs. mid cap vs. small cap HL markets

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. **Net positive expectancy** across all pairs in backtest: average net P&L per trade > 0.05% after all simulated costs
2. **Sharpe > 1.5** on in-sample period (2023–2024)
3. **Sharpe > 1.0** on out-of-sample period (2025 walk-forward)
4. **Basis risk < 30%** of total P&L variance — if basis is driving results more than funding, the strategy is actually a directional bet in disguise
5. **At least 200 trades** in backtest (statistical significance — with 50+ pairs over 2 years, this should be easily achievable)
6. **No single pair contributes > 25%** of total backtest P&L (concentration risk)
7. **Performance holds** across at least 3 distinct market regimes (bull 2023, ranging 2023–2024, bull 2024)

---

## Kill Criteria

Abandon the strategy (paper or live) if any of the following occur:

1. **Live Sharpe drops below 0.5** over any rolling 60-day window with > 20 trades
2. **Three consecutive trades** hit the stop-loss condition (spread widening > 0.15%) — indicates a structural change in HL's user base or fee structure
3. **Average holding period exceeds 6 funding periods** (48h) in live trading — convergence is slower than modelled, capital efficiency is broken
4. **Hyperliquid changes its funding rate formula** — the entire mechanism is predicated on the current 8h settlement structure; any protocol change requires full re-evaluation
5. **Basis risk exceeds 50%** of P&L variance in live trading over 30 days — the strategy has become a directional bet
6. **Execution failure rate > 10%** — if one leg fills and the other doesn't within the 30-minute window, the position is directionally exposed; if this happens repeatedly, the operational risk is too high

---

## Risks — Honest Assessment

### Execution Risk (HIGH)
The strategy requires simultaneous execution on two venues. If Leg A fills and Leg B doesn't (due to API failure, insufficient margin, or liquidity gap), the position is directionally exposed. This is the primary operational risk. Mitigation: always place Leg B first (CEX, more liquid), then Leg A.

### Basis Risk (MEDIUM-HIGH)
Even with zero directional exposure on funding, the HL perp price and CEX perp price can diverge. If HL perp trades at a premium to CEX perp at entry and that premium collapses at exit, the basis move can wipe out funding income. **This is the most underappreciated risk in this strategy.** The backtest must explicitly model basis P&L.

### Structural One-Sidedness (MEDIUM)
Hyperliquid's user base skews retail long. Some pairs may have persistently elevated funding rates that never converge to CEX levels because the arb capital required to compress them exceeds what's available. In this case, the strategy collects funding indefinitely — which sounds good but means capital is locked up and the time stop will trigger repeatedly.

### Capital Fragmentation (MEDIUM)
Funds must sit idle on two venues simultaneously. The opportunity cost of capital locked on Hyperliquid (which has limited yield on idle USDC) vs. deployed elsewhere is a real drag. Factor this into the net yield calculation.

### Regulatory / Withdrawal Risk (LOW-MEDIUM)
If Hyperliquid or the CEX imposes withdrawal restrictions during a volatile period, the ability to close one leg without the other creates directional exposure. Keep position sizes small enough that a stuck position is survivable.

### Known Trade / Competition (MEDIUM)
This is a well-known arbitrage. Large desks monitor BTC/ETH funding spreads continuously. The edge for Zunid is in the **long tail** — smaller HL markets where arb capital is slower to arrive. If Zunid's monitoring is not faster than competitors on mid/small-cap pairs, the opportunity window may be too short to capture.

### Fee Structure Changes (LOW)
Hyperliquid has changed its fee structure before. A fee increase would compress net yield and potentially make the strategy unviable. Monitor HL governance announcements.

---

## Data Sources

| Resource | URL / Endpoint |
|---|---|
| Hyperliquid funding rate history | `https://api.hyperliquid.xyz/info` — POST with `{"type": "fundingHistory", "coin": "<COIN>", "startTime": <unix_ms>}` |
| Hyperliquid market metadata (all coins) | `https://api.hyperliquid.xyz/info` — POST with `{"type": "meta"}` |
| Binance perp funding rate history | `https://fapi.binance.com/fapi/v1/fundingRate?symbol=<SYMBOL>&limit=1000` |
| Binance perp open interest | `https://fapi.binance.com/fapi/v1/openInterest?symbol=<SYMBOL>` |
| OKX funding rate history | `https://www.okx.com/api/v5/public/funding-rate-history?instId=<INST_ID>` |
| Token unlock calendar (for exclusion filter) | `https://token.unlocks.app` (manual check) or `https://www.tokenomist.ai` |
| Hyperliquid fee schedule | `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/fees` |
| Binance fee schedule | `https://www.binance.com/en/fee/futureFee` |

**Data collection note:** Hyperliquid's `fundingHistory` endpoint returns a maximum of 500 records per call. For a full 2-year backtest across 60+ pairs, write a paginated scraper that iterates `startTime` forward in 500-period chunks. Store locally — do not rely on live API calls during backtesting.
