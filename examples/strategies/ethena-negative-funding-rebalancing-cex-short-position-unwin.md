---
title: "Ethena Negative Funding Rebalancing — CEX Short Position Unwind Signal"
status: HYPOTHESIS
mechanism: 6
implementation: 6
safety: 5
frequency: 3
composite: 540
categories:
  - funding-rates
  - defi-protocol
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Ethena's reserve fund is being drawn down and USDe supply contracts materially, Ethena is mechanically reducing its perpetual short positions across CEXs. This short covering creates net long pressure on ETH, BTC, and SOL perpetual markets. The pressure is proportional to the size of the unwind and is observable in advance via on-chain data before it is fully priced into perp markets. We can front-run (or co-run) this mechanical buying by going long the relevant perps when the signal triggers.

The edge is not that "negative funding tends to precede rallies" — that is a commoditised observation. The edge is specifically that **Ethena's position reduction is a forced, size-able, observable mechanical flow event** that must occur in proportion to USDe supply contraction.

---

## Structural Mechanism

### Why This Must Happen

Ethena's USDe is backed 1:1 by a delta-neutral portfolio:
- Spot ETH/BTC/SOL (or LSTs) held as collateral
- Equivalent notional short perpetual positions on CEXs to cancel delta

When a user redeems USDe:
1. Ethena receives USDe, burns it, reduces supply
2. Ethena must return collateral — it unwinds the corresponding spot long AND the corresponding perp short
3. The perp short unwind = Ethena buying back short contracts = net long flow into the perp market

This is **not probabilistic**. The unwind is contractually and mechanically required by the redemption architecture. The only uncertainty is:
- **Timing**: Ethena may batch redemptions or stagger unwinds across venues
- **Price impact**: Depends on order book depth and how aggressively they unwind
- **Venue routing**: Which CEX absorbs the buy flow is not perfectly predictable

### The Negative Funding Amplifier

Under normal conditions, redemptions are idiosyncratic and small. The signal becomes high-conviction when:

1. **Funding rates go persistently negative** (longs pay shorts): Ethena's short positions are now *costing* money rather than earning it
2. **Reserve fund drawdown accelerates**: Ethena is paying out from reserves to cover negative funding on positions they haven't yet unwound
3. **USDe supply contracts**: Redemptions are outpacing mints — the portfolio is shrinking

These three conditions together indicate Ethena is under structural pressure to reduce short exposure. The larger and faster the supply contraction, the larger the forced unwind flow.

### Scale Context

As of early 2025, Ethena held ~$3–5B in notional short positions across Binance, Bybit, OKX, Deribit, and others. A 5% supply contraction = ~$150–250M in short covers. This is non-trivial relative to typical daily perp volume on those venues. It will not move markets dramatically in isolation, but it is a directional flow bias that is observable before it is fully absorbed.

### Information Asymmetry

The signal is visible on-chain (USDe supply, reserve fund wallet balances) but requires active monitoring. Most market participants are not watching Ethena's reserve fund daily. The lag between on-chain signal and price impact is the tradeable window — estimated 12–72 hours based on how quickly Ethena operationally executes unwinds.

---

## Signal Construction

### Primary Signal: USDe Supply Contraction Rate

```
supply_contraction_72h = (USDe_supply_T0 - USDe_supply_T-72h) / USDe_supply_T-72h
```

**Trigger threshold**: `supply_contraction_72h < -0.05` (supply down >5% in 72 hours)

### Confirming Signal 1: Reserve Fund Drawdown

```
reserve_drawdown_7d = (reserve_fund_T0 - reserve_fund_T-7d) / reserve_fund_T-7d
```

**Confirming threshold**: `reserve_drawdown_7d < -0.03` (reserve down >3% in 7 days)

This confirms Ethena is paying out on negative funding, not just experiencing normal redemption churn.

### Confirming Signal 2: Funding Rate Environment

- Average 7-day funding rate across ETH-PERP and BTC-PERP on Binance + Bybit + OKX
- **Confirming threshold**: Average 7-day funding < -0.01% per 8-hour period (annualised: approximately -13%)

Negative funding is the *cause* of the reserve drawdown. If reserve is falling but funding is positive, the mechanism is different — do not trade.

### Full Signal Trigger (ALL THREE required)

| Condition | Threshold | Source |
|-----------|-----------|--------|
| USDe supply 72h change | < -5% | Etherscan / Ethena dashboard |
| Reserve fund 7d change | < -3% | On-chain reserve wallet |
| 7-day avg funding rate | < -0.01%/8h | Binance/Bybit/OKX API |

---

## Entry Rules

### Instrument Selection

Trade the assets Ethena is most heavily short. As of writing, priority order:
1. **ETH-USDC perp** (Hyperliquid) — largest Ethena hedge notional
2. **BTC-USDC perp** (Hyperliquid)
3. **SOL-USDC perp** (Hyperliquid) — smaller Ethena exposure, lower conviction

### Entry Execution

- **Entry trigger**: All three signal conditions met at daily close (00:00 UTC check)
- **Entry timing**: Open position at next available price after signal confirmation — no chasing, use limit orders within 0.1% of mid
- **Entry split**: Enter 50% of target position at signal trigger, remaining 50% at next daily close if signal persists
  - Rationale: Ethena unwinds may take 24–48h; averaging in captures more of the flow window

### Entry Veto Conditions (do not enter if)

- Broader market is in acute liquidation cascade (BTC down >8% in 24h) — Ethena unwind may be overwhelmed by directional selling
- Ethena has publicly announced a protocol change or pause
- USDe supply contraction is >20% in 72h — this may indicate a depeg event, not a normal rebalancing (different risk profile entirely)

---

## Exit Rules

### Primary Exit: Signal Reversal

Exit when the primary signal normalises:
- USDe supply 72h change returns to > -2% (contraction slowing/stopped)
- OR funding rate returns to > +0.005%/8h (positive funding — Ethena no longer incentivised to unwind)

Exit 100% of position at next daily close after signal reversal.

### Time-Based Exit (Hard Stop)

- **Maximum hold**: 10 calendar days from entry
- Rationale: If the unwind hasn't moved price in 10 days, either the flow was absorbed without impact or our timing was wrong. Do not hold indefinitely waiting for a thesis to play out.

### Profit Target

- **Soft target**: +4% on ETH, +3% on BTC from entry price
- At soft target: reduce position by 50%, trail stop on remainder at -1.5% from high
- Rationale: Forced unwinds create flow-driven moves, not trend changes. Take profits before the flow exhausts.

### Stop Loss

- **Hard stop**: -3% from average entry price (ETH), -2.5% (BTC)
- Stop is based on price, not signal — if price moves against us sharply, exit regardless of whether signal is still active
- Do not widen stops. The thesis is about flow timing, not directional conviction.

---

## Position Sizing

### Base Sizing

- **Maximum allocation per trade**: 8% of portfolio NAV per asset (ETH + BTC combined max 15%)
- **Leverage**: 2x maximum on Hyperliquid perps
  - Rationale: This is a flow-timing trade, not a high-conviction directional bet. Low leverage preserves capital through false signals.

### Scaling by Signal Strength

| Supply Contraction (72h) | Reserve Drawdown (7d) | Position Size (% NAV) |
|--------------------------|----------------------|----------------------|
| -5% to -8% | -3% to -5% | 4% per asset |
| -8% to -12% | -5% to -8% | 6% per asset |
| >-12% | >-8% | 8% per asset |

### Correlation Adjustment

If entering both ETH and BTC simultaneously (high correlation), treat combined position as single risk unit. Do not exceed 15% NAV combined. Weight 60% ETH / 40% BTC by default (Ethena's ETH exposure is historically larger).

---

## Backtest Methodology

### Data Requirements

| Data Series | Source | Availability |
|-------------|--------|-------------|
| USDe total supply (daily) | Etherscan ERC-20 transfer events / Ethena API | Available from USDe launch (Feb 2024) |
| Ethena reserve fund wallet balance (daily) | Ethena published wallet addresses, Etherscan | Available from launch |
| 8h funding rates: ETH, BTC | Binance, Bybit, OKX historical API | Available 2020–present |
| ETH/BTC perp OHLCV (daily) | Hyperliquid, Binance | Available |
| Ethena collateral composition | Ethena dashboard / on-chain | Partially available |

### Backtest Period

- **Primary**: February 2024 – present (USDe existence)
- **Note**: This is a short history (~14 months as of writing). Backtest will have limited signal occurrences — treat results as directional, not statistically conclusive.

### Signal Occurrence Estimation

Negative funding episodes of sufficient depth to trigger all three conditions have occurred approximately 3–5 times since USDe launch (notably: August 2024 market crash, early 2025 funding compression periods). Expect **3–6 historical signal instances** — too few for statistical significance alone. Backtest is primarily for:
1. Confirming the mechanical relationship holds directionally
2. Calibrating timing (how many hours/days after signal does price move?)
3. Identifying false positives and their characteristics

### Backtest Procedure

```
FOR each day T in backtest period:
  1. Calculate supply_contraction_72h, reserve_drawdown_7d, avg_funding_7d
  2. IF all three thresholds met AND no veto conditions:
     a. Record signal trigger
     b. Simulate entry at T+1 open (50%) and T+2 open (50%)
     c. Track daily P&L with 2x leverage
     d. Apply exit rules: signal reversal, time stop (10d), hard stop (-3%)
  3. Record: entry price, exit price, hold duration, exit reason, P&L
  4. Aggregate: win rate, avg P&L per trade, max drawdown, Sharpe
```

### Benchmark

Compare against: "buy ETH whenever 7-day funding < -0.01%/8h" (no Ethena-specific signal). This isolates whether the Ethena supply/reserve signal adds alpha beyond the raw funding signal alone.

---

## Go-Live Criteria

All of the following must be satisfied before committing real capital:

1. **Backtest shows positive expectancy**: Average P&L per signal > +1.5% net of fees (0.05% per side on Hyperliquid) across all historical instances
2. **Win rate > 55%** on historical signals (acknowledging small sample)
3. **No single loss > -5%** in backtest (validates stop loss discipline)
4. **Timing analysis confirms**: Price move begins within 48h of signal in majority of cases (validates the flow-timing thesis, not just directional funding bet)
5. **Paper trade**: Run signal live for 30 days minimum, recording all triggered signals and hypothetical P&L before committing capital
6. **Manual review of each historical signal**: Confirm that Ethena's on-chain position data actually shows short reduction during each episode (this is the causal check — if supply contracted but Ethena didn't actually unwind, the mechanism is broken)

---

## Kill Criteria

Abandon strategy immediately if any of the following occur:

| Condition | Reason |
|-----------|--------|
| 3 consecutive losses at full stop | Signal is not working; do not average down on a broken thesis |
| Ethena changes redemption architecture | Mechanical basis of strategy is altered |
| Ethena moves to on-chain hedging (e.g., via Hyperliquid directly) | Flow becomes visible and front-runnable by others, edge disappears |
| USDe supply falls below $500M total | Position sizes too small to move markets; signal loses power |
| Timing analysis shows price moves BEFORE signal (not after) | Market is already pricing the signal faster than we can observe it |
| Drawdown exceeds 12% of allocated capital | Strategy is not performing; cut and reassess |

---

## Risks

### Risk 1: Ethena Batches or Delays Unwinds (HIGH PROBABILITY)
Ethena may not unwind immediately upon redemption. They may accumulate redemptions and execute in batches, or use OTC desks to minimise market impact. This would delay or diffuse the price signal. **Mitigation**: The 10-day time stop limits exposure to delayed unwinds. The 50/50 entry split captures more of the window.

### Risk 2: Market Direction Overwhelms Flow (HIGH PROBABILITY in bear markets)
If the market is in a sharp downtrend, Ethena's short covering (bullish flow) may be overwhelmed by broader selling. The unwind happens but price still falls. **Mitigation**: Entry veto if BTC down >8% in 24h. Accept that this strategy has negative correlation with acute bear markets.

### Risk 3: Small Historical Sample (CERTAIN)
14 months of USDe history with ~3–6 signal instances is not statistically robust. Backtest results are illustrative, not conclusive. **Mitigation**: Paper trade extensively before going live. Treat first 5 live trades as extended paper trading with reduced size (25% of target).

### Risk 4: Ethena Venue Routing Uncertainty (MEDIUM)
We don't know which CEX Ethena unwinds on first. If they unwind on Binance but we're trading Hyperliquid, there may be a lag before arbitrage transmits the flow. **Mitigation**: Hyperliquid ETH/BTC perps are tightly arbed to Binance; lag is typically <1 minute for large moves. Not a material risk for daily-timeframe strategy.

### Risk 5: USDe Depeg Scenario (LOW PROBABILITY, HIGH SEVERITY)
If USDe supply contracts >20% rapidly, this may indicate a depeg event (bank run on USDe). In this scenario, Ethena is unwinding under duress, not orderly rebalancing. Price dynamics are unpredictable. **Mitigation**: Hard veto on entry if 72h supply contraction >20%. If already in position and supply contraction accelerates past 20%, exit immediately regardless of P&L.

### Risk 6: Ethena Transparency Changes (MEDIUM)
Ethena currently publishes reserve fund addresses and collateral composition. If they stop publishing this data or move to opaque structures, the signal disappears. **Mitigation**: Monitor Ethena governance and announcements. Kill strategy if data access degrades.

### Risk 7: Crowding (LOW NOW, INCREASING)
As Ethena grows and this signal becomes more widely known, the front-running window will compress. **Mitigation**: Monitor signal-to-price-move timing. If price begins moving before signal triggers consistently, the edge has been crowded out.

---

## Data Sources

| Data | Source | Access Method | Update Frequency |
|------|--------|---------------|-----------------|
| USDe total supply | Etherscan (contract: `0x4c9EDD5852cd905f086C759E8383e09bff1E68B3`) | ERC-20 totalSupply() call | Real-time / daily snapshot |
| Reserve fund balance | Ethena published addresses (check ethena.fi/transparency) | Etherscan wallet balance API | Daily |
| Ethena collateral breakdown | ethena.fi dashboard | Manual / scrape | Daily |
| Funding rates (8h) | Binance API: `GET /fapi/v1/fundingRate` | REST API | Per funding period |
| Funding rates (8h) | Bybit API: `GET /v5/market/funding/history` | REST API | Per funding period |
| Funding rates (8h) | OKX API: `GET /api/v5/public/funding-rate-history` | REST API | Per funding period |
| ETH/BTC perp OHLCV | Hyperliquid API | REST/WebSocket | Real-time |
| Ethena position sizes by venue | Ethena dashboard (partially) | Manual review | Weekly |

### Monitoring Stack (Recommended)

```
Daily cron job (00:05 UTC):
  1. Pull USDe totalSupply from Etherscan
  2. Pull reserve fund ETH/stablecoin balances from Etherscan
  3. Pull 7-day average funding from Binance/Bybit/OKX
  4. Calculate all three signal metrics
  5. Alert if any threshold is within 20% of trigger (early warning)
  6. Alert if all three thresholds breached (trade signal)
  7. Log to time-series DB for backtest data accumulation
```

---

## Open Questions for Research

Before backtesting, the following questions need answers:

1. **Can we reconstruct Ethena's historical short positions by venue from on-chain data?** If yes, we can directly verify that short reduction occurred during each signal episode — this is the causal validation step.

2. **What is Ethena's actual redemption-to-unwind latency?** Do they unwind same-day, next-day, or in batches? This determines optimal entry timing.

3. **Is the reserve fund drawdown observable in real-time or with a lag?** If Ethena uses multisig with time delays, our signal may be stale.

4. **Has Ethena ever NOT unwound shorts during a supply contraction?** (e.g., did they use reserve fund to cover without reducing positions?) If yes, the mechanical link is weaker than assumed.

5. **What percentage of Ethena's shorts are on Binance vs. Bybit vs. OKX?** Venue concentration affects where the flow impact lands.

---

## Summary Scorecard

| Dimension | Assessment |
|-----------|------------|
| Structural basis | ✅ Mechanically causal — redemption requires unwind |
| Observability | ✅ On-chain data available |
| Timing certainty | ⚠️ Batching/latency unknown |
| Price impact certainty | ⚠️ Depends on market conditions |
| Historical sample size | ❌ Very small (~3–6 instances) |
| Data availability | ✅ Mostly available from Feb 2024 |
| Execution complexity | ✅ Low — daily signal, no HFT required |
| Crowding risk | ✅ Low currently |
| **Overall** | **6/10 — Proceed to backtest with low capital commitment** |

---

*Next step: Build data pipeline for USDe supply + reserve fund monitoring. Reconstruct historical signal dates. Manually verify Ethena position changes during each episode. Run backtest procedure. Target completion: 3 weeks.*
