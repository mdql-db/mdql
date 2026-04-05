---
title: "Hyperliquid Mark-Index Divergence Forced Convergence"
status: KILLED
mechanism: 7
implementation: 7
safety: 6
frequency: 8
composite: 2352
categories:
  - funding-rates
  - exchange-structure
created: "2026-04-03"
pipeline_stage: "Killed at backtest (step 4 of 9)"
killed: "2026-04-04"
kill_reason: "Spreads too thin after costs. Binance backtest (SOL): 30 trades, 3.3% win rate, -0.10%/trade. 97% hit time stop — spread doesn't converge within 4h. HL premium data confirms: mean |spread| ~0.05%, >0.15% only 2-4% of time. 0.17% round-trip costs exceed any convergence profit."
---

## Hypothesis

When Hyperliquid's perpetual mark price diverges from its oracle index price by more than 0.15% for a sustained period (≥30 minutes), the funding rate formula mechanically penalises the side causing the divergence. This penalty compounds every 8 hours (or at each hourly settlement, depending on HL's current schedule — confirm at backtest time). The penalty makes it economically irrational to hold the divergence-pushing side, forcing convergence. We enter on the convergence side, collect the punitive funding rate, and exit when the spread closes. The edge is not "spreads tend to close" — it is "the protocol charges an increasing toll on anyone preventing closure."

---

## Structural Mechanism

### Why This Is Not Pattern Trading

Hyperliquid's funding rate is computed as:

```
funding_rate = clamp((mark_price - index_price) / index_price, -0.05%, +0.05%)
```

*(Verify exact formula and clamp bounds against current HL documentation before backtesting — protocol parameters can change.)*

This means:

1. **Mark > Index by 0.15%:** Longs pay shorts at a rate proportional to the spread. At 0.15% divergence, longs pay ~0.15% per funding period. Annualised, this is punishing. No rational long-term holder stays long into this without a directional conviction that exceeds the carry cost.

2. **Mark < Index by 0.15%:** Shorts pay longs. Same logic inverted.

3. **The dam:** The divergence is the dam. Capital flowing into the punished side is paying a toll. The toll does not disappear — it accumulates every settlement. The only way to stop paying is to close the position, which mechanically pushes mark back toward index.

4. **Convergence is not guaranteed in a single window** — a new wave of directional buyers can rebuild the divergence after each settlement. This is why the score is 7, not 9. The mechanism forces convergence pressure, not instantaneous convergence.

### Why Mid/Small Caps Are the Target

BTC and ETH perps on Hyperliquid have deep OI and tight arbitrage from professional desks who close mark/index gaps within minutes. Mid/small cap perps have thinner OI, fewer arb desks watching them, and wider spreads that persist longer — giving a non-HFT participant time to enter and collect.

---

## Universe

- **In scope:** All Hyperliquid perpetual markets outside BTC and ETH.
- **Minimum OI filter:** >$500k open interest at signal time (prevents entering illiquid traps).
- **Exclude:** Any market with a known upcoming token unlock within 48 hours (confounding directional flow — see Zunid's unlock short strategy).
- **Review universe monthly** as new markets are listed.

---

## Entry Rules

### Signal Conditions (ALL must be true simultaneously)

| # | Condition | Threshold | Rationale |
|---|-----------|-----------|-----------|
| 1 | \|mark - index\| / index | > 0.15% | Below this, funding income doesn't cover fees |
| 2 | Spread sustained | ≥ 30 minutes | Filters transient spikes from single large trades |
| 3 | Spread direction | Consistent (not oscillating) | Oscillating spread = noise, not structural imbalance |
| 4 | OI on divergence side | Not declining rapidly | Rapid OI decline = convergence already happening, entry is late |
| 5 | Market OI | > $500k | Liquidity floor |
| 6 | Time to next funding | > 15 minutes | Ensures at least one full funding collection before exit pressure |

### Entry Execution

- **Mark > Index (premium):** Short the perp at market. Do NOT simultaneously long spot unless spot liquidity is confirmed deep enough to not move the market — for most mid-caps, spot is thin. Run as a directional carry trade (short perp only) unless spot arb is explicitly viable.
- **Mark < Index (discount):** Long the perp at market.
- **Entry price:** Use limit orders within 0.05% of mid to avoid paying full spread. If not filled within 5 minutes, cancel and re-evaluate — if spread has already closed, the opportunity is gone.
- **Entry size:** See Position Sizing section.

---

## Exit Rules

### Primary Exit Triggers (first condition hit closes position)

| Priority | Trigger | Action |
|----------|---------|--------|
| 1 | Spread closes to < 0.03% | Exit at market immediately |
| 2 | Next funding settlement completes AND spread < 0.08% | Exit after collecting funding |
| 3 | Stop loss: spread widens to > 0.5% | Exit at market, accept loss |
| 4 | OI on divergence side increases > 20% since entry | Exit — new capital is rebuilding divergence, thesis is fighting fresh flow |
| 5 | Maximum hold time: 4 hours | Exit regardless — prevents becoming a directional position |

### Exit Execution

- Use limit orders at 0.03% better than current mid for exits.
- If not filled within 3 minutes on exit, switch to market order — do not let a closing trade become a position management problem.

---

## Position Sizing

### Base Sizing Formula

```
position_size = (account_risk_per_trade) / (stop_distance_in_%)

account_risk_per_trade = 0.5% of total account per trade
stop_distance = 0.5% - 0.15% = 0.35% (spread widens from entry to stop)
```

**Example:** $100,000 account → risk $500 per trade → stop distance 0.35% → position size = $500 / 0.0035 = ~$143,000 notional.

**Hard caps:**
- Maximum notional per trade: 20% of account (leverage cap).
- Maximum simultaneous open positions: 5 (prevents correlated drawdown if broad market move causes multi-asset divergence simultaneously).
- If 5 positions are open, no new entries regardless of signal quality.

### Leverage

- Use 3–5x leverage maximum. This is a carry trade, not a directional bet. High leverage turns a funding income strategy into a liquidation risk.
- Confirm HL margin requirements for each asset before entry — maintenance margin varies by asset.

---

## Expected P&L Per Trade

*(These are estimates for backtest calibration, not guarantees.)*

| Component | Estimate |
|-----------|----------|
| Funding collected (1 period, 0.15% spread) | ~0.10–0.15% of notional |
| Entry + exit fees (taker, ~0.035% each side) | ~0.07% of notional |
| Slippage (mid-cap, estimated) | ~0.03–0.05% of notional |
| **Net per trade (best case)** | **~0.03–0.08% of notional** |
| **Net per trade (worst case, spread widens then closes)** | **-0.10% to -0.20% of notional** |

At 3–5 trades per week across the universe, net monthly return on notional is estimated at 0.5–2% before compounding — hypothesis only, backtest must validate.

---

## Backtest Methodology

### Data Requirements

| Data Field | Source | Frequency |
|------------|--------|-----------|
| Mark price | Hyperliquid API (`/info` endpoint, `markPx` field) | 1-minute bars minimum |
| Oracle/index price | Hyperliquid API (`oraclePx` field) | 1-minute bars minimum |
| Funding rate history | Hyperliquid API (`/info` → `fundingHistory`) | Per settlement |
| OI history | Hyperliquid API (`openInterest` field) | 1-minute bars |
| Trade history (for fee estimation) | Hyperliquid API | Per trade |

All data is publicly available via Hyperliquid's REST and WebSocket APIs at no cost. Archive at least 6 months of 1-minute mark/index data before running backtest.

### Backtest Steps

1. **Build mark/index spread time series** for all non-BTC/ETH markets over the backtest period.
2. **Identify all signal events** where spread > 0.15% sustained ≥ 30 minutes.
3. **Simulate entry** at the 30-minute mark using the prevailing mid price + 0.05% slippage assumption.
4. **Simulate exit** at first of: spread < 0.03%, post-funding spread < 0.08%, spread > 0.5% stop, 4-hour max hold.
5. **Calculate P&L** including: funding collected (from funding history), entry/exit fees (0.035% per side taker), slippage assumption.
6. **Segment results by:** asset, spread magnitude at entry, time of day, market regime (trending vs. ranging — use 24h BTC return as proxy).
7. **Key metrics to compute:** win rate, average P&L per trade, Sharpe ratio, max drawdown, average hold time, signal frequency per month.

### Backtest Red Flags (invalidate hypothesis if observed)

- Win rate < 55% (convergence is not reliably happening).
- Average losing trade > 3x average winning trade (stop is too wide or gap risk is real).
- Signal frequency < 5 events per month (not enough trades to be worth the infrastructure).
- P&L degrades significantly in the most recent 3 months vs. earlier periods (edge is being arbed away).

---

## Go-Live Criteria

All of the following must be satisfied before committing real capital:

1. **Backtest Sharpe > 1.5** on out-of-sample data (last 20% of dataset, not used in development).
2. **Minimum 50 backtest trades** across at least 10 different assets (not concentrated in one market).
3. **Paper trade for 30 days** with full signal logging — paper P&L must be within 30% of backtest expectation.
4. **Execution infrastructure confirmed:** automated spread monitoring, alert system, order execution — manual monitoring is not viable for a strategy that can trigger at any hour.
5. **Fee structure confirmed:** verify current HL taker/maker fees — if fees increase, recalculate minimum viable spread threshold.
6. **Funding formula verified:** re-read current HL documentation and confirm formula has not changed since this spec was written.

---

## Kill Criteria

Stop trading and return to backtest if ANY of the following occur:

| Trigger | Threshold | Action |
|---------|-----------|--------|
| Live Sharpe (rolling 60 days) | < 0.5 | Pause, investigate |
| Consecutive losing trades | 8 or more | Pause, investigate |
| Average spread at entry declining | < 0.10% over 30-day rolling window | Signal universe is being arbed — reduce size or pause |
| Single trade loss | > 2% of account | Immediate pause, review stop logic |
| HL protocol change | Any change to funding formula or settlement frequency | Immediate pause, re-derive all thresholds |

---

## Risks

### Risk 1: Gap Risk on Stop (HIGH)
A large directional news event (exchange hack, token exploit, macro shock) can push mark/index spread from 0.15% to 2%+ in minutes, blowing through the 0.5% stop. **Mitigation:** Hard position size caps (20% notional max), never hold through known high-risk events (major protocol upgrades, token unlocks on the specific asset).

### Risk 2: Funding Formula Change (MEDIUM)
Hyperliquid is a young protocol. The team can change the funding formula, clamp bounds, or settlement frequency. Any change invalidates all threshold calibrations in this spec. **Mitigation:** Monitor HL governance announcements and changelog; kill criteria item covers this.

### Risk 3: Oracle Manipulation (MEDIUM)
The index price is derived from external oracles. If the oracle is manipulated (e.g., thin CEX market for a small-cap asset), the "index" itself is wrong, and the spread signal is false. **Mitigation:** Only trade assets where the oracle sources are deep, liquid CEX markets. Avoid assets where HL's oracle relies on a single source.

### Risk 4: Convergence Delay (LOW-MEDIUM)
New directional flow can continuously rebuild the divergence across multiple funding windows, meaning the spread stays wide for hours or days. The 4-hour max hold and 0.5% stop are designed to cap this, but the position will lose funding income and potentially hit the stop. **Mitigation:** OI monitoring (exit rule #4) is designed to detect this early.

### Risk 5: Execution Latency (LOW)
By the time a 30-minute sustained signal is confirmed, the spread may already be closing. **Mitigation:** Backtest must measure the spread at the 30-minute confirmation point vs. the spread at actual close — if the average remaining spread at entry is < 0.08%, the signal is arriving too late and the threshold or timing must be adjusted.

### Risk 6: Correlated Multi-Asset Blowup (LOW-MEDIUM)
A broad market crash can cause simultaneous divergences across all mid-cap assets in the same direction. With 5 positions open, all stops could hit simultaneously. **Mitigation:** 5-position cap, 0.5% account risk per trade, 20% notional cap per trade — worst case simultaneous stop-out is 2.5% account drawdown, which is acceptable.

---

## Data Sources

| Source | URL / Endpoint | Data Type | Cost |
|--------|---------------|-----------|------|
| HL REST API | `https://api.hyperliquid.xyz/info` | Mark, index, OI, funding | Free |
| HL WebSocket | `wss://api.hyperliquid.xyz/ws` | Real-time mark/index stream | Free |
| HL Funding History | `/info` → `fundingHistory` action | Historical funding rates | Free |
| HL GitHub | `github.com/hyperliquid-dex` | Protocol documentation, formula verification | Free |

**Archive strategy:** Begin streaming and storing 1-minute mark/index snapshots for all HL markets immediately. Do not rely on being able to reconstruct historical data later — some endpoints may not provide full depth of history for all assets.

---

## Open Questions for Backtest Phase

1. What is the actual distribution of spread magnitudes and durations across all HL mid-cap markets over the past 12 months? (Determines signal frequency.)
2. Does spread persistence correlate with time of day (e.g., low-liquidity hours = more persistent divergences)?
3. Is there a minimum OI threshold below which the spread is noise (oracle lag) rather than genuine imbalance?
4. Does the strategy perform better entered immediately after a funding settlement (maximum time to collect before next settlement) vs. entered mid-period?
5. What percentage of signals are "false" — spread > 0.15% for 30 minutes but caused by oracle lag rather than genuine OI imbalance?

---

## Next Steps

| Step | Owner | Deadline |
|------|-------|----------|
| Begin archiving HL mark/index data for all markets | Data engineer | Immediate |
| Verify current funding formula against HL docs | Researcher | Before backtest |
| Build spread signal detector and backtester | Quant | 2 weeks |
| Run backtest on 6+ months of data | Quant | 3 weeks |
| Review backtest results against go-live criteria | Strategy committee | 4 weeks |
| Begin paper trading if criteria met | Trader | 5 weeks |
