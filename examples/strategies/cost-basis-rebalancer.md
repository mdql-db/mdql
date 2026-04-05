---
title: "Strategy Specification: Cost-Basis Rebalancer"
status: HYPOTHESIS
mechanism: 4
implementation: 7
safety: 6
frequency: 10
composite: 1680
categories:
  - calendar-seasonal
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## EMA Band Filter (key variant)

The critical improvement over naive cost-basis rebalancing: **only operate when price is within a defined band around the N-period EMA** (e.g., ±1.5 standard deviations, or a fixed % like ±8%).

- **Price within band** → market is ranging → rebalance actively (buy below basis, sell above)
- **Price breaks above band** → strong uptrend → stop selling, hold position, don't fight it
- **Price breaks below band** → strong downtrend → stop buying, don't accumulate a falling knife

This solves both failure modes of the original strategy:
1. No selling into a bull run (misses the upside)
2. No buying into a crash (catches a falling knife)

The strategy only runs when conditions suit it. When it can't determine regime, it does nothing.

### Position sizing philosophy

This is not a primary strategy. It's a background loop — a fraction of a percent of the total purse per trade. Think of it as a savings account that skims tiny amounts on oscillations, always running, barely noticeable per trade, compounding over time. The goal is not to generate big returns but to keep a small slice of capital working when nothing else is active.

### Parameters to optimize in backtest
- EMA period (50, 100, 200 day)
- Band width (% or standard deviations)
- Trade size (fraction of a percent of total capital per rebalance)
- Re-entry rule: when does the strategy resume after a breakout? (price re-enters band? EMA slope flattens?)

---

## 1. Hypothesis

In a mean-reverting or range-bound asset, the current spot price repeatedly oscillates around some central tendency. By mechanically selling when price is above your average cost basis and buying when it is below — **but only when price is within a defined band around the EMA** — you:

1. Accumulate more units per dollar spent during drawdowns
2. Harvest margin when the price recovers above your basis
3. Lower your average cost basis over time relative to a passive hold

The edge claim is **not** that this beats buy-and-hold in a bull market. It does not. The edge claim is that it generates **risk-adjusted positive return in flat-to-oscillating markets where buy-and-hold returns near zero**, while also softening drawdowns compared to passive exposure. This makes it useful as a continuous 24/7 engine on liquid assets where no directional signal is available.

**Why it might have real edge:**
- Market microstructure: retail panic and FOMO create systematic oscillations around fair value even in trending assets
- No prediction required: the edge comes purely from variance harvesting, which is theoretically sound (similar in structure to constant-proportion portfolio rebalancing)
- Automation advantage: human traders cannot execute 24/7 mechanical rebalancing without drift; Zunid can

**Why it might not:**
- Variance harvesting only works if the asset mean-reverts on the relevant timescale. Crypto has multi-month trends. The strategy will be short units during the entire up-leg and long units during the entire down-leg — exactly backwards from buy-and-hold
- Cost basis is path-dependent. An asset that drops 80% and recovers to -30% will leave you with a cost basis near the top and an underwater position for years
- This strategy class is extremely well-documented. If it had durable edge it would be arbitraged away

---

## 2. Asset Selection

### Primary Test Assets
| Asset | Rationale |
|-------|-----------|
| BTC-USDT | Longest history, high liquidity, has shown multi-year ranging behavior (2018–2020, 2022–2023) |
| ETH-USDT | Similar rationale, slightly higher volatility and more oscillation |

### Asset Selection Filter (Production)
Before running on any asset, require **all** of the following:

1. **Mean-reversion score:** Rolling 90-day Hurst exponent < 0.55 (H < 0.5 = mean-reverting, H = 0.5 = random walk, H > 0.5 = trending). Disable strategy if Hurst > 0.55 for 2 consecutive 30-day windows
2. **Liquidity:** 30-day average daily volume > $500M USD on primary venue
3. **Not trending to zero:** Asset must have been in top 20 by market cap for at least 12 months
4. **No structural break:** Exclude any asset down >70% from all-time high on a 2-year trailing basis (indicates potential regime change, not mean reversion)

### Explicit Exclusions
- Long-tail altcoins (LUNA-style death spirals are indistinguishable from "buy the dip" until they aren't)
- Leveraged tokens (path-dependent decay destroys cost basis logic)
- Stablecoins (zero volatility, no trades trigger)

### Hyperliquid Perps vs. Spot

**Recommendation: Run on spot, not perps.**

Reasoning:
- On perps, a sustained downtrend means you're long into a declining position **and** paying funding rate. Funding on Hyperliquid BTC perps can be 10–50 bps/day in strong downtrends — this silently destroys the strategy
- Cost basis tracking on perps is conceptually messier (unrealized PnL vs. basis diverges)
- The accumulation mechanic (buying more = increasing position size) creates unlimited leverage risk on perps with no natural floor
- **If perps are required** (e.g., no spot infrastructure): cap leverage at 1x effective, treat it as a synthetic spot position, and factor funding into all PnL calculations

---

## 3. Strategy Rules

### 3.1 State Variables

```
cost_basis          = weighted average purchase price of all open units
total_units_held    = current position size in base asset
total_capital_deployed = total USD value invested (not counting realized PnL)
realized_pnl        = cumulative locked-in profit from sell legs
```

### 3.2 Initialization

- Deploy an **initial tranche** equal to 25% of total allocated capital at market price
- Set `cost_basis = entry_price`
- Set `total_units_held = initial_tranche_usd / entry_price`

### 3.3 Rebalancing Trigger

Use **price-threshold crossings**, not time-based rebalancing. Reason: time-based rebalancing fires randomly regardless of whether anything actionable has occurred; threshold-based rebalancing fires only when the market has moved enough to justify a trade.

**Threshold:** Define a grid step `G` as a percentage move from the current cost basis.

```
G = 2.5%   (base case — see sensitivity analysis in backtest)
```

Trigger a **sell** when: `spot_price >= cost_basis * (1 + G * n)` for integer n ≥ 1

Trigger a **buy** when: `spot_price <= cost_basis * (1 - G * n)` for integer n ≥ 1

Track which band the price last crossed to avoid re-triggering at the same level.

### 3.4 Trade Sizing

**Use deviation-proportional sizing**, not fixed dollar amounts. This scales naturally with how far price has moved.

**On a BUY signal** (price below basis):

```
deviation_pct = (cost_basis - spot_price) / cost_basis
buy_usd = base_order_size * (1 + deviation_pct * scaling_factor)
```

Where:
- `base_order_size` = 2% of total allocated capital per grid step
- `scaling_factor` = 2.0 (so a 10% deviation triggers ~1.2x normal buy, 20% deviation ~1.4x)
- Cap any single buy at 8% of total allocated capital

**On a SELL signal** (price above basis):

```
deviation_pct = (spot_price - cost_basis) / cost_basis
sell_units = total_units_held * 0.10 * (1 + deviation_pct * scaling_factor)
```

- Sell a portion of units, not a fixed USD amount — this preserves upside participation
- Cap any single sell at 20% of total units held
- **Never sell below cost basis.** If somehow a sell signal triggers below basis (this shouldn't happen by construction but handle edge cases), skip the trade

### 3.5 Cost Basis Update

After each BUY:
```
new_cost_basis = (total_units_held * cost_basis + buy_units * spot_price) 
                  / (total_units_held + buy_units)
total_units_held += buy_units
total_capital_deployed += buy_usd
```

After each SELL:
```
realized_pnl += (spot_price - cost_basis) * sell_units
total_units_held -= sell_units
# Cost basis does NOT change on a sell — this is intentional
# We only sold units above basis, so remaining units still carry same basis
```

Cost basis resets only if position goes to zero (full exit).

### 3.6 Entry/Exit Summary Table

| Condition | Action | Size |
|-----------|--------|------|
| Price crosses G% above cost basis | SELL portion | 10–20% of held units |
| Price crosses G% below cost basis | BUY more | 2–8% of allocated capital |
| Position hits max size limit | PAUSE buys | — |
| Kill criteria triggered | EXIT ALL | Market order |

---

## 4. Position Sizing and Risk Limits

### Capital Allocation
- **Total allocated capital:** Define as a fixed pool, e.g., $100,000. Never exceed this without explicit manual override
- **Initial deployment:** 25% of allocated capital at strategy start
- **Reserve:** Keep minimum 40% of allocated capital as dry powder for accumulation during drawdowns
- **Max deployed:** 60% of allocated capital at any time

### Position Limits
| Parameter | Limit |
|-----------|-------|
| Max position size (USD market value) | 60% of allocated capital |
| Max units accumulation vs. initial | 5x initial unit count |
| Max drawdown on deployed capital | 40% (kill switch) |
| Max single trade size (buy) | 8% of allocated capital |
| Max single trade size (sell) | 20% of current units |

### Why 40% reserve minimum?
A 60% drawdown on BTC (historically common) starting from initial deployment would require buying all the way down to maintain basis. Without reserve, the strategy is forced to watch the drawdown passively with no capital to average down — at that point it's just a bad long.

---

## 5. Backtest Methodology

### Data
- **Source:** Binance via CCXT or direct API export
- **Assets:** BTC-USDT, ETH-USDT
- **Timeframe:** January 2020 – December 2024 (5 full years, 3 distinct market regimes)
- **Resolution:** 1-hour OHLCV (sufficient for threshold-based triggers; overkill for daily but avoids missing intra-day crossings)
- **Fees:** 0.10% taker fee per trade (Binance standard; adjust to 0.025% for VIP/BNB discount in sensitivity run)

### Market Regimes to Evaluate Separately
| Period | Regime |
|--------|--------|
| Jan 2020 – Nov 2020 | Flat then breakout |
| Nov 2020 – Nov 2021 | Strong bull trend |
| Nov 2021 – Nov 2022 | Strong bear trend |
| Nov 2022 – Oct 2023 | Ranging/accumulation |
| Oct 2023 – Dec 2024 | Bull trend |

### Benchmark Comparisons
1. **Buy and hold** with same initial capital
2. **DCA** (fixed $X weekly, no selling)
3. **Static 60/40 rebalance** (BTC + cash, rebalance monthly)
4. **Zero strategy** (cash, 0% return)

### Metrics to Compute
| Metric | Description |
|--------|-------------|
| Total return (%) | Full period |
| Annualized return (%) | Geometric |
| Max drawdown (%) | Peak-to-trough on total equity |
| Sharpe ratio | Daily returns, risk-free = 4% |
| Sortino ratio | Downside deviation only |
| Calmar ratio | Annual return / max drawdown |
| Cost basis vs. spot over time | Visualize gap — key diagnostic |
| Trade count | Total buys and sells |
| Win rate on sell trades | % of sells above cost basis |
| Realized PnL vs. unrealized PnL | Is profit real or just on paper? |
| Units accumulated over time | Are we actually getting more units? |
| Strategy vs. buy-hold in each regime | Per-regime alpha/beta |

### Parameter Sensitivity Grid
Run full backtest across:

```
G (grid step): [1.5%, 2.5%, 5.0%, 7.5%, 10.0%]
base_order_size: [1%, 2%, 5%] of allocated capital
scaling_factor: [1.0, 2.0, 3.0]
initial_deployment: [10%, 25%, 50%]
```

Look for parameter robustness — edge should hold across a range of settings, not just be curve-fitted to one combination.

### Slippage Model
- Assume 0.05% slippage on BTC (deep market), 0.10% on ETH
- For large orders (>$50k), apply 0.02% additional market impact

### What the Backtest Must Answer
1. Does the strategy outperform buy-and-hold in flat/ranging periods? (Hypothesis: yes)
2. Does it underperform in strong trends? (Hypothesis: yes, this is acceptable)
3. What is the worst single drawdown and how long does recovery take?
4. How much of the return is realized PnL vs. unrealized (paper profit on held units)?
5. Does fee drag eat the edge at high-frequency parameter settings?
6. What is the strategy's return in the 2022 bear market — this is the critical failure-mode test

---

## 6. Go-Live Criteria

All of the following must be satisfied before deploying real capital:

| Criterion | Requirement |
|-----------|-------------|
| Backtest Sharpe (full period) | ≥ 0.8 on at least one asset |
| Backtest Sharpe (ranging regime only) | ≥ 1.2 |
| Max drawdown (backtest) | < 50% of allocated capital |
| Parameter robustness | Profitable in at least 60% of sensitivity grid runs |
| Bear market test (2022) | Strategy drawdown < buy-and-hold drawdown by ≥ 10 percentage points |
| Trade count (backtest) | > 200 total trades (sufficient for statistical significance) |
| Walk-forward validation | Final 6 months of backtest held out; out-of-sample Sharpe ≥ 0.6 |
| Paper trading | 30 days live paper trading with <5% deviation from simulated results |
| Fee model validated | Live fees match assumed fees within 20% |
| Manual review | At least one human review of strategy logic, position limits, kill switch |

---

## 7. Kill Criteria

Halt all activity and move to cash immediately if any of the following trigger:

| Trigger | Condition | Action |
|---------|-----------|--------|
| Hard drawdown | Total equity (deployed + realized) down >40% from starting capital | Full exit, stop all activity |
| Basis disconnect | Spot price > 3x cost basis and strategy has been selling — means massive missed upside | Pause buys, review manually |
| Reserve exhaustion | Available dry powder < 15% of allocated capital | Pause buys until price recovers or manual top-up |
| Downtrend detection | 50-day EMA < 200-day EMA on target asset AND Hurst > 0.6 | Pause buys; sells continue |
| Fee spike | Detected fee rate > 2x assumed (venue change, etc.) | Pause all trading |
| Execution errors | >3 failed orders in 24h | Halt and alert |
| Funding drain (perps only) | Cumulative funding paid > 0.5% of position per day for 5 consecutive days | Exit perps entirely |

**Note on the downtrend kill:** This is the most important one. The strategy's primary failure mode is buying into a sustained downtrend. The EMA cross + Hurst filter provides a coarse but effective brake. When this triggers, let existing positions ride (don't crystallize the loss), just stop accumulating.

---

## 8. Risks

### Primary Risks

**1. Sustained Downtrend (High Probability, High Impact)**
The strategy will mechanically buy every X% decline. In a 90% drawdown (possible for ETH, routine for altcoins), this exhausts the reserve and leaves a large underwater position. BTC and ETH survivorship bias makes this feel acceptable, but it is not guaranteed.
*Mitigation: Reserve floor, downtrend kill, strict asset selection*

**2. Fee Erosion at Tight Grid Spacing (Medium Probability, Medium Impact)**
At G=1.5% and 0.10% taker fees each way, you need the trade to capture >0.20% net just to break even. High-frequency parameters will churn the capital with negative expected value.
*Mitigation: Minimum G=2.5% in production; validate in backtest fee sensitivity*

**3. Basis Lock-In (Low Probability, High Impact)**
If an asset drops significantly below cost basis and never fully recovers, realized PnL is zero and the unrealized loss is permanent. This is not a temporary drawdown — it's a capital destruction event.
*Mitigation: Only run on BTC/ETH, enforce hard kill at 40
