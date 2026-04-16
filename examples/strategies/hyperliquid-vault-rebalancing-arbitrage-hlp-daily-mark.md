---
title: "Hyperliquid Vault Rebalancing Arbitrage (HLP Daily Mark)"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 5
frequency: 4
composite: 480
categories:
  - liquidation
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's HLP vault accumulates directional delta exposure beyond its normal operating range — typically after a large liquidation event or sustained funding anomaly — it faces structural pressure to reduce that exposure back toward delta-neutral. Because HLP's position state is publicly readable on the Hyperliquid L1 in near-real-time, and because HLP's rebalancing behavior is rule-based rather than discretionary, an observer can identify the imbalance before the rebalancing trade executes and take an opposing position that profits from the mean-reversion in HLP's delta. The edge is not that HLP *tends* to rebalance — it is that HLP *must* rebalance to preserve its market-making function, and the trigger state is publicly observable before the action occurs.

---

## Structural Mechanism

### 2.1 What HLP Is

HLP (Hyperliquid Provider) is the protocol-native market-making vault on Hyperliquid. It earns fees by providing liquidity across perpetual markets. It is not a passive index — it actively takes on the opposite side of large trades and liquidations, which means it accumulates directional inventory as a byproduct of its function.

### 2.2 Why Rebalancing Is Structural, Not Discretionary

HLP operates under a defined mandate: provide liquidity and remain approximately delta-neutral over time. Three mechanical forces make rebalancing non-optional:

1. **Liquidation absorption:** When a large position is liquidated, HLP absorbs the inventory. A $50M ETH long liquidation leaves HLP net short ETH. This is not a choice — it is the mechanical consequence of being the backstop liquidity provider.
2. **Funding rate pressure:** If HLP holds a large directional position, it pays or receives funding continuously. A net long position in a negative-funding environment bleeds P&L every 8 hours, creating time-pressure to reduce exposure.
3. **Vault NAV protection:** HLP depositors can withdraw. A visibly directional HLP position creates withdrawal risk (depositors flee before a loss), which incentivizes the vault operators to rebalance faster, not slower.

### 2.3 The Information Asymmetry

HLP's position state — net delta per asset, total notional, unrealized P&L — is queryable via the Hyperliquid public API and L1 state. This data is available to anyone. The asymmetry is not that the data is hidden; it is that most market participants are not watching it systematically. The rebalancing trade is visible in the queue before it executes.

### 2.4 The Causal Chain

```
Large liquidation event
        ↓
HLP absorbs inventory → net delta spikes beyond normal range
        ↓
HLP delta state becomes publicly readable via API
        ↓
[ENTRY WINDOW: observer takes opposing position]
        ↓
HLP rebalancing pressure builds (funding bleed, NAV risk, mandate)
        ↓
HLP executes mean-reversion trade → price moves against HLP's prior exposure
        ↓
Observer exits with profit
```

---

## Entry Rules

### 3.1 Signal Definition

**Primary signal:** HLP net delta on any single asset exceeds **2.0× its 30-day rolling median absolute delta** for that asset.

- Compute: `signal_threshold = 2.0 × median(|HLP_delta_t|, lookback=30d)`
- If `|HLP_delta_current| > signal_threshold` → signal is active
- Direction: if HLP is net long → SHORT the asset; if HLP is net short → LONG the asset

**Secondary confirmation (all three must be true before entry):**
1. The delta spike occurred within the last **4 hours** (stale signals are discarded — HLP may have already begun rebalancing)
2. The asset's 1-hour price momentum is **not** aligned with HLP's exposure (i.e., do not fade HLP into a strong trending move — wait for momentum to stall)
3. Funding rate on the asset is **not** strongly favorable to HLP's current position (if HLP is net long and funding is strongly positive, HLP is being paid to hold — rebalancing pressure is lower)

### 3.2 Entry Execution

- Enter via **Hyperliquid perpetual futures** on the same asset as the HLP imbalance
- Use **limit orders** at mid-price ± 0.05% to avoid crossing the spread
- If limit order is not filled within **10 minutes**, cancel and re-evaluate — do not chase
- Maximum entry slippage tolerance: **0.15%** of notional

### 3.3 Assets in Scope

Initially limit to: **BTC, ETH, SOL** — the three assets where HLP carries the largest notional exposure and where Hyperliquid perp liquidity is deepest. Expand to other assets only after 60 days of live observation.

---

## Exit Rules

### 4.1 Primary Exit — Signal Normalization

Exit when HLP net delta on the asset returns to within **1.2× the 30-day rolling median** (i.e., the imbalance has substantially resolved). Check HLP state every **15 minutes** during an open position.

### 4.2 Time-Based Exit

If HLP delta has not normalized within **24 hours** of entry, exit at market regardless of P&L. Rationale: if HLP has not rebalanced in 24 hours, either (a) the rebalancing is not coming, (b) HLP is intentionally holding the position, or (c) the market has moved against the thesis. Do not extend the holding period.

### 4.3 Stop Loss

Hard stop at **1.5% adverse move** from entry price, measured on the perpetual. This is not a trailing stop — it is a fixed level set at entry. Rationale: if the market moves 1.5% against the position before HLP rebalances, the thesis is likely wrong for this instance.

### 4.4 Take Profit (Optional Partial)

At **0.8% profit**, take off 50% of the position and move the stop on the remainder to breakeven. This locks in partial profit while allowing the full rebalancing move to play out.

---

## Position Sizing

### 5.1 Base Sizing Rule

Risk **0.5% of total portfolio** per trade (defined as: maximum loss if stop is hit = 0.5% of portfolio NAV).

Given a 1.5% stop, position size in notional = `(0.005 × Portfolio_NAV) / 0.015`

Example: $100,000 portfolio → risk $500 per trade → notional position = $33,333.

### 5.2 Leverage

Use **3× leverage maximum** on Hyperliquid. At $33,333 notional with $100K portfolio, this requires $11,111 margin — well within the 3× limit. Do not increase leverage to chase larger positions.

### 5.3 Concentration Cap

Maximum **two simultaneous open positions** across different assets. If BTC and ETH both trigger simultaneously, take the one with the larger delta-to-threshold ratio. Do not open a third position until one closes.

### 5.4 Scaling Up

After 30 confirmed live trades with positive expectancy, scale to **1.0% portfolio risk per trade**. Do not scale before this milestone.

---

## Backtest Methodology

### 6.1 Data Collection (Weeks 1–4)

Before backtesting, collect the following raw data:

| Dataset | Source | Format | Lookback |
|---|---|---|---|
| HLP vault positions per asset | Hyperliquid public API (`/info` endpoint, `clearinghouseState` for HLP vault address) | JSON, polled every 15 min | 180 days |
| HLP vault delta history | Derived from position data above | Computed | 180 days |
| Hyperliquid perp OHLCV (BTC, ETH, SOL) | Hyperliquid API or Kaiko | 1-min bars | 180 days |
| Liquidation event log | Hyperliquid explorer / API | Per-event | 180 days |
| Funding rate history | Hyperliquid API | 8-hour intervals | 180 days |

**Note:** Hyperliquid launched its mainnet in late 2023. Realistically, 12–15 months of HLP position history is available as of April 2026. This is sufficient for a preliminary backtest but not for multi-regime analysis.

### 6.2 Signal Reconstruction

1. Reconstruct HLP net delta per asset at each 15-minute interval from historical position snapshots.
2. Compute the 30-day rolling median absolute delta for each asset at each timestamp.
3. Identify all timestamps where `|HLP_delta| > 2.0 × rolling_median` — these are candidate entry signals.
4. Apply secondary confirmation filters (momentum filter, funding filter) to reduce the candidate set.
5. Record entry price, stop level, and exit conditions for each candidate.

### 6.3 Simulation Rules

- Simulate limit order fills at mid-price with a **0.10% slippage assumption** (conservative for Hyperliquid BTC/ETH depth).
- Apply **0.035% taker fee** per leg (Hyperliquid standard fee tier).
- Do not assume fills at the exact signal timestamp — add a **15-minute execution lag** to simulate realistic detection and order placement.
- Apply the 24-hour time stop and 1.5% hard stop mechanically — no look-ahead.

### 6.4 Metrics to Compute

| Metric | Minimum Acceptable | Target |
|---|---|---|
| Win rate | >45% | >55% |
| Average win / average loss ratio | >1.5 | >2.0 |
| Expectancy per trade (in % of notional) | >0.20% | >0.40% |
| Maximum drawdown (on 0.5% risk sizing) | <8% portfolio | <5% portfolio |
| Sharpe ratio (annualized) | >0.8 | >1.5 |
| Number of trades in sample | >40 | >80 |

### 6.5 Robustness Checks

Run the backtest under three threshold variants: **1.5×, 2.0×, 2.5×** median delta. If the edge disappears at 1.5× (too many false signals) or 2.5× (too few trades), the 2.0× threshold is not robust and must be re-examined. A genuine structural edge should show positive expectancy across a range of threshold values, not just at one cherry-picked level.

---

## Forward Observation Period (Pre-Backtest Gate)

Before committing to a full backtest, run a **30-day forward observation** (paper only, no capital):

- Poll HLP delta state every 15 minutes via API.
- Log every instance where the 2.0× threshold is breached.
- Record what HLP actually does in the subsequent 24 hours (does it rebalance? how fast? how much?).
- Record what the asset price does in the same window.

**Gate criteria to proceed to backtest:** At least **8 signal events** observed in 30 days, with HLP delta normalizing within 24 hours in **≥60% of cases**. If fewer than 8 events occur, extend observation to 60 days. If normalization rate is below 60%, re-examine the mechanism before proceeding.

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

1. **Backtest expectancy ≥ 0.20%** per trade after fees and slippage, across ≥40 simulated trades.
2. **Forward observation gate passed** (see Section 7).
3. **API monitoring infrastructure is live** — automated polling of HLP state every 15 minutes with alerting on threshold breach. Manual monitoring is not acceptable for live trading.
4. **Paper trading period:** 20 live paper trades executed with documented entry/exit rationale before first real trade.
5. **Drawdown ceiling confirmed:** Backtest maximum drawdown on 0.5% risk sizing does not exceed 8% of portfolio.

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

| Trigger | Action |
|---|---|
| 5 consecutive losing trades | Pause, review whether HLP behavior has changed |
| Realized drawdown exceeds 4% of portfolio on this strategy | Pause, reduce size to 0.25% risk per trade, re-evaluate |
| HLP vault undergoes a protocol upgrade or rule change | Pause immediately — the structural mechanism may have changed |
| Win rate over trailing 30 trades falls below 35% | Kill — the edge has likely been arbitraged away or the mechanism is broken |
| HLP delta normalization rate (observed live) falls below 50% over 20 signals | Kill — HLP is no longer rebalancing predictably |
| Hyperliquid introduces a delay or obfuscation to HLP position reporting | Kill — the information asymmetry is gone |

---

## Risks

### 10.1 HLP Does Not Rebalance on Schedule
**Risk:** HLP vault operators may choose to hold a directional position intentionally (e.g., they believe the move will reverse). The rebalancing is structurally incentivized but not contractually forced on a fixed schedule.
**Mitigation:** The 24-hour time stop limits exposure to any single non-rebalancing event. The 1.5% hard stop limits loss if the market moves against the position.

### 10.2 Front-Running Is Already Priced In
**Risk:** Other participants may already be monitoring HLP state and front-running the same signal, compressing or eliminating the edge.
**Mitigation:** The forward observation period will reveal this — if the price moves against HLP's exposure immediately upon threshold breach (before our entry), the edge is already crowded. Monitor the time between threshold breach and price movement.

### 10.3 HLP Position Data Latency
**Risk:** The public API may have a reporting lag, meaning the position state we observe is already stale by the time we act.
**Mitigation:** Test API latency empirically during the observation period. If lag exceeds 30 minutes consistently, the strategy is not viable without a direct node connection.

### 10.4 Liquidation Cascade Risk
**Risk:** Large liquidation events that cause HLP delta spikes are also high-volatility events. The 1.5% stop may be hit by noise before HLP rebalances.
**Mitigation:** Apply the momentum filter strictly — do not enter during the first 30 minutes after a large liquidation event. Wait for initial volatility to subside before entering.

### 10.5 HLP Protocol Changes
**Risk:** Hyperliquid may modify HLP's rebalancing rules, reporting format, or vault structure in a protocol upgrade.
**Mitigation:** Monitor Hyperliquid governance and changelog. Any HLP-related upgrade triggers an immediate strategy pause (see Kill Criteria).

### 10.6 Adverse Selection on Limit Orders
**Risk:** Limit orders may only fill when the market is moving against the position (i.e., we get filled precisely when the thesis is failing).
**Mitigation:** Cancel unfilled limit orders after 10 minutes. Do not use market orders to chase entry. Accept missed trades rather than bad fills.

---

## Data Sources

| Source | URL / Access Method | Cost | Use |
|---|---|---|---|
| Hyperliquid public API | `https://api.hyperliquid.xyz/info` | Free | HLP position state, funding rates, OHLCV |
| Hyperliquid explorer | `https://app.hyperliquid.xyz/explorer` | Free | Historical vault state, liquidation events |
| HLP vault address | Published by Hyperliquid team (verify on-chain) | Free | Required to query specific vault state |
| Kaiko / Tardis | Subscription | Paid | High-resolution tick data for slippage modeling (optional, use only if Hyperliquid API data is insufficient) |

**Infrastructure requirement:** A Python polling script running every 15 minutes, storing HLP delta snapshots to a local database, with alerting (Telegram or email) on threshold breach. Estimated build time: 2–4 hours. No proprietary data required.

---

## Open Questions (Must Resolve Before Backtest)

1. **What is the actual API latency** for HLP position updates? Is it real-time, 1-minute delayed, or longer?
2. **Does HLP rebalance in one large trade or gradually?** If gradual, the price impact is spread over hours and the entry timing matters more.
3. **What is HLP's normal delta range** for BTC, ETH, SOL? Need 30 days of baseline data before the 2.0× threshold is meaningful.
4. **Are there other vaults or market makers on Hyperliquid** that also rebalance predictably and could be monitored simultaneously?
5. **Has HLP ever held a large directional position for >48 hours?** If yes, under what conditions? This defines the failure mode.

---

## Next Actions

| Action | Owner | Deadline | Blocker |
|---|---|---|---|
| Build HLP delta polling script | Engineering | Day 3 | Confirm HLP vault address |
| Begin 30-day forward observation | Research | Day 4 | Polling script live |
| Establish BTC/ETH/SOL baseline delta ranges | Research | Day 14 | 10 days of polling data |
| Evaluate API latency empirically | Engineering | Day 7 | Polling script live |
| Design backtest simulation framework | Research | Day 20 | Baseline ranges established |
| Review observation gate criteria | Research | Day 34 | 30-day observation complete |
