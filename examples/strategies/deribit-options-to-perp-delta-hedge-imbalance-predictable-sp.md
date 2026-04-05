---
title: "Deribit Options-to-Perp Delta Hedge Imbalance — Predictable Spot Flow After Large OI Print"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 5
composite: 625
categories:
  - options-derivatives
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## 1. Hypothesis

When a large options block trades on Deribit, the market-maker on the other side of that trade is structurally obligated to delta-hedge in the underlying spot or perpetual futures market. This hedge flow is not probabilistic — it is a mechanical requirement of running a delta-neutral book. The block trade is publicly observable in real time via Deribit's WebSocket feed. The delta of the block is calculable from public inputs (strike, expiry, spot price, implied volatility). Therefore, the direction and approximate magnitude of the imminent hedge flow are knowable before the hedge is fully executed.

The tradeable edge: enter a perp position in the direction of the expected hedge flow immediately after the block print, capture a fraction of the price impact caused by the dealer's hedge, and exit before the flow exhausts.

**This is not a prediction that dealers will hedge. It is a prediction that they must hedge, and that their hedging will temporarily move the market in a calculable direction.**

---

## 2. Structural Mechanism

### 2.1 Why Dealers Must Hedge

A market-maker who sells a call block to a buyer is short gamma and short delta. Holding that position unhedged exposes them to unlimited directional loss. Every professional options desk runs delta-neutral: they immediately buy delta in the underlying to offset the directional exposure. The hedge size is:

```
Hedge notional (USD) = Block notional (USD) × |Δ|
```

Where Δ is the Black-Scholes delta of the option at current spot, strike, expiry, and implied vol.

For a $5M notional call block with Δ = 0.5, the dealer must buy approximately $2.5M of underlying. This is not a choice — it is risk management protocol enforced by internal limits and, for regulated entities, regulatory capital rules.

### 2.2 Why the Hedge Moves the Market

A $2.5M market buy in BTC perps on Hyperliquid or in BTC spot on any exchange will move the price. The move is proportional to the order book depth at the time. For BTC, this is small but measurable. For altcoin options (ETH, SOL), the impact is larger relative to perp liquidity.

### 2.3 Why the Edge Is Observable Before Completion

Deribit's public WebSocket emits block trades in real time. The block print appears before the hedge is fully executed because:

1. The options trade settles instantly on Deribit
2. The hedge must be executed separately in spot/perp markets
3. Large hedges are often worked over 1–10 minutes to minimise market impact (TWAP or iceberg execution)

This creates a window — typically 30 seconds to 5 minutes — between the observable block print and the completion of the hedge flow.

### 2.4 Why This Is Not Already Fully Arbed Away

- Most systematic actors watching the Deribit tape are HFT firms focused on sub-second latency. A 30-second entry window is too slow for them but accessible to us.
- The strategy requires combining Deribit options data with perp execution on a separate venue — a cross-system integration step that filters out many participants.
- For altcoin options (SOL, AVAX, etc.), fewer sophisticated actors are watching the tape, widening the window.
- The edge is not a clean arbitrage — it requires probabilistic judgment about hedge timing and execution venue, which reduces competition from pure arb desks.

---

## 3. Entry Rules

### 3.1 Data Feed Requirements

- **Deribit public WebSocket:** Subscribe to `trades.{instrument_name}.raw` for all BTC, ETH, and SOL options instruments
- **Filter for block trades:** Deribit flags block trades in the trade feed with `block_trade_id` field (non-null = block trade)
- **Hyperliquid perp feed:** Real-time order book for execution venue

### 3.2 Signal Calculation

On detection of a block trade with non-null `block_trade_id`:

```
1. Extract: instrument (underlying, strike, expiry, call/put), 
   trade price, trade size (contracts), direction (buy/sell)
   
2. Calculate notional:
   Notional (USD) = contracts × contract_size × spot_price
   
3. Apply notional filter:
   IF notional < $2,000,000 → DISCARD (noise, insufficient hedge flow)
   
4. Calculate Black-Scholes delta:
   Inputs: spot (from Deribit index), strike, T (time to expiry in years),
   r (risk-free rate, use 5% annualised), σ (use Deribit mark IV for 
   that instrument)
   Output: Δ ∈ [-1, +1]
   
5. Apply delta filter:
   IF |Δ| < 0.3 → DISCARD (deep OTM options have small hedge flow)
   
6. Determine hedge direction:
   IF trade_direction = "buy" (customer bought call): dealer sold call → 
     dealer must BUY delta → LONG perp signal
   IF trade_direction = "sell" (customer sold call): dealer bought call → 
     dealer must SELL delta → SHORT perp signal
   IF trade_direction = "buy" (customer bought put): dealer sold put → 
     dealer must SELL delta → SHORT perp signal
   IF trade_direction = "sell" (customer sold put): dealer bought put → 
     dealer must BUY delta → LONG perp signal
```

**Direction logic summary:**

| Customer action | Option type | Dealer position | Dealer hedge |
|----------------|-------------|-----------------|--------------|
| Buy | Call | Short call | Buy underlying |
| Sell | Call | Long call | Sell underlying |
| Buy | Put | Short put | Sell underlying |
| Sell | Put | Long put | Buy underlying |

### 3.3 Entry Execution

- **Entry window:** Within 30 seconds of block trade timestamp
- **Venue:** Hyperliquid perp for BTC, ETH, SOL (deepest non-HFT accessible perp liquidity)
- **Order type:** Market order (speed matters; slippage is a known cost)
- **Entry condition:** Do not enter if bid-ask spread on Hyperliquid perp is >0.05% at time of entry (wide spread indicates stressed conditions where edge is unreliable)

---

## 4. Exit Rules

### 4.1 Primary Exit: Time-Based

- **Exit after 3 minutes** from entry timestamp
- Rationale: Dealer hedge flow for blocks in the $2M–$20M range typically completes within 1–5 minutes. Holding beyond 3 minutes means riding noise, not hedge flow.

### 4.2 Secondary Exit: Stop-Loss

- **Exit immediately if adverse move exceeds 0.4%** from entry price
- Rationale: If price moves against the expected hedge direction by 0.4%, either (a) the dealer is hedging on a different venue, (b) a larger opposing flow exists, or (c) the signal was misread. Cut the loss.

### 4.3 Tertiary Exit: Target

- **Exit if gain reaches 0.3%** from entry price before the 3-minute timer
- Rationale: Lock in the edge; do not overstay.

### 4.4 Exit Order Type

- Market order at exit trigger. Do not use limit orders for exit — the 3-minute window is too short to risk non-fill.

---

## 5. Position Sizing

- **Maximum per trade:** 0.5% of total capital
- **Rationale:** This is a high-frequency, low-edge strategy. Individual trade expectancy is small. Position size must be small enough that a string of 10 consecutive losses (realistic in a 30-second window strategy) does not exceed 5% drawdown.
- **Scaling rule:** Do not scale up until 200+ live trades are logged with positive expectancy. The edge may be smaller than estimated.
- **Concurrent positions:** Maximum 1 open position at a time. If a second block prints while a position is open, discard the second signal.
- **Daily loss limit:** If daily P&L reaches -2% of capital, stop trading for the day. Reset next calendar day.

---

## 6. Backtest Methodology

### 6.1 Data Requirements

| Data | Source | Notes |
|------|--------|-------|
| Deribit block trade history | Deribit API (historical trades endpoint) or archive from `deribit.com/api/v2/public/get_last_trades_by_instrument` | Must filter `block_trade_id != null`. Historical depth: ~2 years available |
| Deribit mark IV (implied vol) | Deribit historical mark price API | Required for delta calculation at time of trade |
| Deribit index price | Deribit historical index API | Spot price at time of block trade |
| BTC/ETH/SOL perp OHLCV (1-second) | Hyperliquid historical data or Tardis.dev | Required for simulating entry/exit fills |

### 6.2 Backtest Procedure

```
For each historical block trade:
  1. Apply notional filter (>$2M)
  2. Apply delta filter (|Δ| > 0.3)
  3. Calculate delta using mark IV and index price at trade timestamp
  4. Determine expected hedge direction
  5. Simulate entry: use 1-second perp price 30 seconds after block timestamp
     (conservative: assumes 30-second delay to process signal)
  6. Simulate exit: 
     - Check 1-second prices for next 3 minutes
     - Apply stop at -0.4%, target at +0.3%, time exit at 3 minutes
  7. Record: entry price, exit price, exit reason, P&L in bps
  8. Apply transaction costs: 0.05% per side (taker fee on Hyperliquid)
```

### 6.3 Backtest Metrics to Report

- Win rate (%)
- Average P&L per trade (bps, net of fees)
- Average P&L on wins vs. losses
- Expectancy per trade (bps)
- Sharpe ratio (annualised, assuming ~3 trades/day based on block frequency)
- Maximum consecutive losses
- P&L by underlying (BTC vs. ETH vs. SOL)
- P&L by delta bucket (0.3–0.5, 0.5–0.7, 0.7–1.0)
- P&L by notional bucket ($2M–$5M, $5M–$20M, >$20M)
- P&L by time of day (UTC) — dealer staffing varies by session

### 6.4 Known Backtest Limitations

1. **Venue assumption:** Historical hedge flow may have occurred on Binance, Bybit, or CME, not Hyperliquid. Backtest will show price impact on Hyperliquid perp regardless — this is a proxy, not a perfect reconstruction.
2. **1-second granularity:** True entry would be within 30 seconds of block print. 1-second data is the finest granularity available for most sources. Tick data from Tardis.dev would improve accuracy.
3. **Delta calculation timing:** Mark IV at the exact block trade timestamp may differ slightly from what was available to a live trader. Use the closest available mark IV snapshot.
4. **Survivorship:** Deribit block trade history may be incomplete for older periods. Validate data completeness before drawing conclusions.
5. **Cannot observe dealer identity:** We cannot confirm which side of the trade was the dealer. The assumption is that the non-initiating side is the dealer/MM. This is standard but not guaranteed.

---

## 7. Go-Live Criteria

All of the following must be satisfied before allocating real capital:

| Criterion | Threshold | Rationale |
|-----------|-----------|-----------|
| Backtest expectancy | > +3 bps per trade net of fees | Below this, fees and slippage will likely erase edge in live trading |
| Backtest win rate | > 52% | Minimum for positive expectancy given 0.3% target / 0.4% stop ratio |
| Backtest sample size | > 300 qualifying block trades | Minimum for statistical significance |
| Backtest Sharpe | > 1.0 annualised | Minimum acceptable risk-adjusted return |
| Paper trade period | 30 calendar days, minimum 50 live signals | Validate live signal detection and execution infrastructure |
| Paper trade expectancy | > 0 bps (directionally correct) | Live execution must not destroy the backtest edge |
| Infrastructure test | End-to-end latency (block print → order sent) < 5 seconds | Ensure 30-second window is achievable |

---

## 8. Kill Criteria

Stop trading and return to research if any of the following occur:

| Trigger | Action |
|---------|--------|
| 20 consecutive losing trades in live trading | Halt, investigate, do not resume without research review |
| Live expectancy < 0 bps over 100+ trades | Strategy is not working; kill |
| Live Sharpe < 0.5 over 90-day rolling window | Edge has degraded; kill |
| Single trade loss > 1% of capital (position sizing breach) | Investigate execution system; halt until resolved |
| Deribit changes block trade feed format or removes `block_trade_id` field | Halt immediately; data dependency broken |
| Hyperliquid perp spread consistently > 0.05% at entry time | Market conditions incompatible with strategy; pause |
| Regulatory change making options-to-perp cross-venue trading restricted | Kill |

---

## 9. Risks

### 9.1 Execution Risk (HIGH)
The 30-second window is tight. Any latency in the signal pipeline (WebSocket lag, computation delay, order routing) compresses the window further. If entry is delayed beyond 60 seconds, the hedge may already be complete and the edge is gone. **Mitigation:** Co-locate signal processing close to Deribit WebSocket endpoint; pre-compute delta lookup tables; use direct API order submission, not GUI.

### 9.2 Venue Risk (MEDIUM)
The dealer may hedge on Binance, CME, or OKX rather than Hyperliquid. If the hedge flow does not touch Hyperliquid, the price impact we are trying to capture will not appear there. **Mitigation:** Monitor whether BTC/ETH price moves on Hyperliquid correlate with block prints during paper trading. If correlation is weak, the strategy does not work on this venue.

### 9.3 Direction Risk (MEDIUM)
The model assumes the non-initiating side of the block trade is the dealer. In practice, two customers may trade a block directly (customer-to-customer). In this case, neither side is obligated to delta-hedge. **Mitigation:** No clean filter exists for this. Accept it as noise in the signal. The backtest will implicitly price this in if the historical data includes customer-to-customer blocks.

### 9.4 Competition Risk (MEDIUM)
Other actors watch the Deribit block tape. If enough participants front-run the same signal, the price impact occurs before our entry, and we are buying the top of the move. **Mitigation:** Monitor entry slippage over time. If average entry slippage increases, competition has increased and the edge is being competed away. Kill criterion above captures this via live Sharpe degradation.

### 9.5 Gamma Risk on Large Blocks (LOW)
For very large blocks (>$50M notional), the dealer may hedge in tranches over hours, not minutes. Our 3-minute exit would capture only the first tranche. This is not a risk per se — it means we exit early — but it also means we may be leaving edge on the table. **Mitigation:** Test extended hold times (10 minutes, 30 minutes) in backtest for large-notional blocks specifically.

### 9.6 Implied Vol Input Risk (LOW)
Delta calculation requires implied vol (mark IV). If mark IV is stale or anomalous at the time of the block trade, delta will be miscalculated. **Mitigation:** Use Deribit's mark IV (their own calculation, updated continuously). Cross-check against model IV if mark IV appears anomalous (e.g., >200% IV).

### 9.7 Funding Rate Drag (LOW)
Holding a perp position for 3 minutes incurs negligible funding (funding is charged every 8 hours on most venues). Not a material risk at this hold time.

---

## 10. Data Sources

| Source | Data | Access | Cost |
|--------|------|--------|------|
| Deribit WebSocket (live) | Real-time block trades, mark IV, index price | Public, no auth required for market data | Free |
| Deribit REST API (historical) | Historical trades with `block_trade_id`, historical mark IV | Public | Free |
| Hyperliquid WebSocket (live) | Real-time perp order book and trades | Public | Free |
| Hyperliquid REST API (historical) | Historical perp OHLCV | Public | Free |
| Tardis.dev (optional) | Tick-level historical data for both venues | Paid subscription | ~$500/month; use only if 1-second data proves insufficient |

---

## 11. Open Research Questions

Before committing to backtest build, answer these:

1. **How many qualifying block trades ($2M+, |Δ|>0.3) occur per day on Deribit?** If fewer than 1/day, the strategy has insufficient trade frequency to be worth the infrastructure cost.
2. **What fraction of Deribit block trades are customer-to-customer vs. customer-to-dealer?** Deribit does not publish this. Estimate from market structure research or industry contacts.
3. **Does Hyperliquid BTC/ETH perp price lead or lag Deribit index price?** If Hyperliquid lags, the hedge flow may already be priced in before our order reaches the market.
4. **What is the average price impact of a $2M–$5M market order on Hyperliquid BTC perp?** This determines whether the hedge flow is large enough to create a tradeable move net of our own entry impact.
5. **Are altcoin options (SOL, AVAX) on Deribit liquid enough to generate qualifying block trades?** If yes, these may offer better edge due to thinner perp markets and fewer competing actors.

---

## 12. Next Steps

| Step | Owner | Timeline |
|------|-------|----------|
| Pull 12 months of Deribit block trade history; count qualifying trades per day | Researcher | Week 1 |
| Build delta calculator (Black-Scholes, vectorised) | Engineer | Week 1 |
| Build backtest engine: block print → delta calc → simulated perp entry/exit | Engineer | Week 2 |
| Run backtest; report metrics by underlying, delta bucket, notional bucket | Researcher | Week 3 |
| Decision gate: proceed to paper trade or kill | Zunid PM | End of Week 3 |
| If proceed: build live signal pipeline (Deribit WS → delta calc → Hyperliquid order) | Engineer | Week 4–5 |
| Paper trade for 30 days | Researcher | Weeks 6–9 |
| Go/no-go for live capital | Zunid PM | End of Week 9 |

---

*This document is a hypothesis. No backtest has been run. No live trading has occurred. All claims about mechanism are theoretical and require empirical validation before capital is committed.*
