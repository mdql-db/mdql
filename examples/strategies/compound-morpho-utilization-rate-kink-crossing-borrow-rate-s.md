---
title: "Compound/Morpho Utilization Rate Kink Crossing — Borrow Rate Step-Change Short"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 3
composite: 375
categories:
  - lending
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When utilization on a major lending market (USDC, USDT, or ETH pools on Compound v3, Aave v3, or Morpho) crosses the protocol's hardcoded kink threshold upward, the borrow rate increases by 10–100× in a single block. This rate spike is a deterministic, on-chain-verifiable event. Leveraged yield farmers, basis traders, and looping strategies that borrowed at sub-kink rates are immediately unprofitable at post-kink rates and face a binary choice: repay or bleed. Repayment requires selling or unwinding collateral. This creates a short-duration, directional sell impulse on the collateral asset that is structurally caused, not pattern-based.

**Null hypothesis to disprove:** Kink crossings produce no statistically significant negative price return on the collateral asset in the 0–48 hours following the event, after controlling for broad market direction.

---

## Structural Mechanism

### The Kink Formula (Compound v3 / Aave v3 style)

Lending protocols implement a two-slope interest rate model:

```
If utilization U ≤ Kink:
    BorrowRate = BaseRate + (U / Kink) × Slope1

If utilization U > Kink:
    BorrowRate = BaseRate + Slope1 + ((U - Kink) / (1 - Kink)) × Slope2
```

Where Slope2 >> Slope1 by design. Typical parameters:

| Protocol | Market | Kink | Rate below kink | Rate above kink |
|---|---|---|---|---|
| Compound v3 | USDC | 92% | ~5% APR | ~50–150% APR |
| Aave v3 | USDC | 90% | ~6% APR | ~80% APR |
| Morpho | USDC/ETH | 80–92% | ~3–8% APR | ~30–100% APR |

These parameters are **hardcoded in the smart contract**. No governance vote, no oracle, no human decision is required for the rate to jump. The jump occurs in the same block that utilization crosses the threshold.

### The Causal Chain

```
Step 1: Utilization crosses kink (on-chain, verifiable, block-level precision)
         ↓
Step 2: Borrow rate increases 10–100× in same block (deterministic formula)
         ↓
Step 3: Existing leveraged positions become unprofitable (yield farmers,
        looping strategies, basis traders borrowing stables to buy spot)
         ↓
Step 4: Rational actors repay loans to exit the rate spike
        (repayment = returning borrowed stables = unwinding collateral)
         ↓
Step 5: Collateral (ETH, BTC, wstETH, etc.) sold or reduced
         ↓
Step 6: Sell pressure on collateral asset
         ↓
Step 7: Utilization drops as loans repaid → rate normalizes below kink
```

### Why This Is Structural, Not Pattern-Based

- Steps 1 and 2 are **mathematically guaranteed** by immutable contract code.
- Steps 3–6 are **strongly incentivized** but not contractually forced — this is why the score is 6/10, not 8+.
- The mechanism has a clear, falsifiable causal story: borrowers are paying more than they earn, so they must act or lose money continuously.
- The edge degrades if: (a) borrowers are irrational or inattentive, (b) they hedge rather than unwind, or (c) the rate spike is short-lived due to immediate new supply entering.

### Why New Supply Doesn't Always Neutralize Instantly

High utilization means the pool is nearly empty of lendable assets. New lenders must actively deposit. Depositing requires capital to be idle elsewhere, moved on-chain, and transacted — this takes hours to days, not seconds. The rate spike is the signal that attracts new supply, but supply response has latency. This latency window is the trade window.

---

## Markets in Scope

**Priority markets** (high TVL, liquid perp available on Hyperliquid):

| Lending Market | Collateral Asset | Perp to Short |
|---|---|---|
| Compound v3 USDC (Ethereum) | ETH, wBTC | ETH-PERP, BTC-PERP |
| Aave v3 USDC (Ethereum) | ETH, wstETH, wBTC | ETH-PERP, BTC-PERP |
| Morpho USDC/USDT (Ethereum) | ETH, wstETH | ETH-PERP |

**Minimum TVL filter:** $500M in the affected pool. Below this, the collateral base is too small to move the perp price meaningfully.

**Excluded markets:** Long-tail collateral assets (no liquid perp), pools under $100M TVL, stablecoin-only pools (no directional collateral to unwind).

---

## Entry Rules

### Trigger Conditions (ALL must be met)

1. **Kink crossing confirmed:** Utilization crosses kink threshold upward, confirmed in two consecutive blocks (prevents false triggers from single-block spikes).
2. **Rate delta significant:** Post-kink borrow rate is ≥ 15% APR higher than pre-kink rate (filters noise from kinks with shallow Slope2).
3. **TVL filter:** Affected pool TVL ≥ $500M at time of trigger.
4. **Perp liquidity filter:** Hyperliquid order book has ≥ $2M within 0.3% of mid for the relevant perp (prevents slippage eating the edge).
5. **Market regime filter:** BTC 4-hour trend is not strongly bullish (defined as: BTC has not made a new 4H high in the last 2 candles AND funding rate on BTC-PERP is not above +0.05% per 8h). This filter reduces false positives during broad market rips that override the mechanism.
6. **No concurrent macro event:** No FOMC, CPI, or major protocol announcement within 2 hours of trigger (these events dominate all other signals).

### Entry Execution

- **Instrument:** Short the collateral asset perp on Hyperliquid (ETH-PERP or BTC-PERP).
- **Entry timing:** Market order within 60 minutes of kink crossing confirmation. Do not chase if price has already moved >1.5% against the trade direction before entry.
- **Entry price:** Use TWAP over 5 minutes to reduce slippage on entry.

---

## Exit Rules

### Primary Exit Conditions (first triggered wins)

1. **Utilization normalization:** Utilization drops ≥ 3 percentage points below kink threshold AND stays there for 2 consecutive blocks. This signals the mechanical pressure has resolved.
2. **Timeout:** 48 hours after entry, regardless of P&L. The mechanism is short-duration; holding longer means you're no longer trading the kink event.
3. **Profit target:** +3% on the perp position (captures the typical move without overstaying).

### Stop Loss

- **Hard stop:** 2% adverse move on the perp from entry price, measured on a 15-minute close basis (not tick-by-tick, to avoid stop-hunting).
- **Soft stop:** If utilization drops back below kink within 4 hours of entry (mechanism failed to produce borrower response), exit at market regardless of P&L.

### Exit Execution

- Market order on Hyperliquid perp.
- Do not scale out — this is a binary event trade, not a trend trade.

---

## Position Sizing

### Base Sizing Formula

```
Position Size = (Account Equity × Risk Per Trade) / Stop Distance

Risk Per Trade = 1% of account equity (fixed)
Stop Distance = 2% (hard stop)
→ Position Size = 0.5× account equity notional
```

**Example:** $100,000 account → $50,000 notional short on ETH-PERP at 1× leverage (or $25,000 at 2× leverage with same dollar risk).

### Leverage Cap

- Maximum 3× leverage on any single kink trade.
- If position size formula implies >3× leverage, reduce risk per trade proportionally.

### Concurrent Position Limit

- Maximum 2 kink trades open simultaneously (different assets only — no doubling up on ETH).
- Total portfolio risk from kink trades capped at 2% of equity at any time.

### Scaling for Conviction

- **High conviction** (TVL > $1B, rate delta > 30% APR, market regime neutral): 1.5× base size.
- **Low conviction** (TVL $500M–$700M, rate delta 15–20% APR): 0.5× base size.

---

## Backtest Methodology

### Step 1: Build the Event Dataset

**Source:** Compound v3, Aave v3, Morpho subgraphs (The Graph) or direct RPC calls to `getUtilizationRate()` at each block.

**Process:**
1. Pull hourly utilization snapshots for USDC, USDT, ETH markets from January 2022 to present.
2. Identify all instances where utilization crossed kink threshold upward (U[t] > Kink AND U[t-1] ≤ Kink).
3. Apply TVL filter: keep only events where pool TVL > $500M.
4. Record: event timestamp, pool, utilization level, borrow rate before/after, collateral asset.

**Expected event count:** 30–80 events across all pools over 2 years (rough estimate — needs verification).

### Step 2: Match to Perp Price Data

**Source:** Hyperliquid historical data (available via API), or Binance perpetual futures as proxy for pre-Hyperliquid periods.

**Process:**
1. For each event, record ETH-PERP or BTC-PERP price at: T=0 (entry), T+4h, T+8h, T+12h, T+24h, T+48h.
2. Calculate raw return and market-adjusted return (subtract BTC return as market beta proxy).
3. Record whether utilization dropped ≥3% below kink within 48h (mechanism confirmation).

### Step 3: Statistical Analysis

**Primary metric:** Mean market-adjusted return at T+24h across all events.

**Secondary metrics:**
- Win rate (% of events where market-adjusted return > 0 at T+24h).
- Sharpe ratio of the event series.
- Return distribution by: rate delta magnitude, TVL size, market regime.

**Minimum bar for continuation:** Mean market-adjusted return at T+24h is negative (correct direction) with p-value < 0.10 on a one-tailed t-test. With ~50 events, this requires a consistent signal of ~0.5–1% mean return.

### Step 4: Subgroup Analysis

Test whether the signal is stronger when:
- Rate delta > 30% APR vs. 15–30% APR.
- Market regime is neutral vs. bullish.
- Event occurs during US business hours (09:00–17:00 ET) vs. off-hours.
- Utilization stays above kink for >4 hours vs. reverting quickly.

### Step 5: Slippage and Cost Modeling

- Assume 0.05% entry + 0.05% exit slippage on Hyperliquid perp.
- Assume funding rate cost of 0.01% per 8h (typical neutral rate) for 48h max hold = 0.06% total.
- Net cost per trade: ~0.16%. Subtract from gross returns.

---

## Paper Trading Protocol

### Duration

Minimum 60 days of paper trading before live capital deployment.

### Execution

- Monitor utilization in real-time using a simple script polling `getUtilizationRate()` every 5 minutes via Alchemy or Infura RPC.
- Log every trigger, entry, exit, and P&L in a structured spreadsheet.
- Compare paper trade outcomes to backtest distribution — flag if paper trade win rate deviates >15 percentage points from backtest.

### Alert System

Build a Telegram or Discord bot that fires when:
- Utilization within 1% of kink (pre-alert).
- Kink crossing confirmed (action alert).
- Utilization drops 3% below kink (exit alert).

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

1. **Backtest result:** Mean market-adjusted return at T+24h is negative with p < 0.10, across ≥ 30 events.
2. **Paper trade result:** ≥ 10 paper trades completed with positive expectancy (mean net return > 0 after costs).
3. **Paper vs. backtest consistency:** Paper trade win rate within 15 percentage points of backtest win rate.
4. **Mechanism confirmation rate:** In ≥ 70% of paper trade events, utilization dropped ≥ 3% below kink within 48h (confirms the mechanical response is occurring).
5. **No data snooping:** Backtest was run on a held-out test set (2024 data) after parameters were fixed on training set (2022–2023 data).

---

## Kill Criteria

Immediately halt trading and return to research if any of the following occur:

1. **Live trading drawdown:** 3 consecutive losing trades OR cumulative loss > 3% of account equity from this strategy.
2. **Mechanism failure:** In ≥ 3 consecutive live events, utilization does NOT drop below kink within 48h (suggests new structural lenders are absorbing supply faster than expected — mechanism has changed).
3. **Protocol parameter change:** Any of the target protocols changes kink threshold or Slope2 via governance (invalidates backtest parameters — requires re-backtest before resuming).
4. **Liquidity degradation:** Hyperliquid order book depth for ETH-PERP or BTC-PERP drops below $1M within 0.3% of mid on a sustained basis.
5. **Crowding signal:** If a public strategy or research note describing this exact mechanism is published by a well-followed source (Delphi, Messari, Gauntlet), re-evaluate edge decay within 30 days.

---

## Risks

### Risk 1: Borrower Inattention or Automation Lag
**Description:** Many borrowers use automated position managers (Instadapp, DeFi Saver) that may not react instantly to rate spikes. If automation is slow, the sell pressure is delayed beyond the 48h window.
**Mitigation:** Monitor on-chain repayment activity directly (track `Repay` events on Compound/Aave) as a secondary confirmation signal. Only enter if repayment volume increases within 2 hours of kink crossing.

### Risk 2: New Lender Supply Response
**Description:** High rates attract new lenders who deposit into the pool, pushing utilization back below kink without borrowers needing to repay. No collateral is sold; the trade fails.
**Mitigation:** Track deposit events in real-time. If large deposits (>$10M) hit the pool within 2 hours of kink crossing, exit the trade early — the mechanism is being neutralized from the supply side.

### Risk 3: Borrowers Hedge Rather Than Unwind
**Description:** Sophisticated borrowers (market makers, delta-neutral funds) may hedge their collateral exposure rather than sell it, producing no spot/perp sell pressure.
**Mitigation:** This is a genuine limitation. The strategy works best when borrowers are directional (yield farmers, leveraged longs) rather than delta-neutral. No clean mitigation — accept as a source of noise.

### Risk 4: Macro Override
**Description:** A broad market rally (risk-on event, ETF news, Fed pivot) can overwhelm the kink-driven sell pressure entirely.
**Mitigation:** Market regime filter at entry (see Entry Rules). Hard stop at 2% adverse move. Accept that macro events will produce losing trades — the edge is in the base rate, not every individual trade.

### Risk 5: Kink Crossing Is Transient (Single Block)
**Description:** Utilization spikes above kink for one block due to a large single borrow, then immediately drops back. No sustained rate pressure is created.
**Mitigation:** Two-consecutive-block confirmation rule at entry. Do not trade single-block crossings.

### Risk 6: Protocol Upgrade Changes Rate Model
**Description:** Compound, Aave, or Morpho governance votes to change kink parameters or switch to a dynamic rate model (e.g., Aave's interest rate strategy updates).
**Mitigation:** Monitor governance forums (Compound Forum, Aave Governance, Morpho Discord) for parameter change proposals. Kill criterion #3 handles confirmed changes.

### Risk 7: Hyperliquid Execution Risk
**Description:** Hyperliquid perp may have insufficient liquidity, or the platform may experience downtime during the execution window.
**Mitigation:** Liquidity filter at entry. Maintain a backup execution venue (dYdX or GMX) with pre-funded accounts for redundancy.

---

## Data Sources

| Data Type | Source | Access Method | Cost |
|---|---|---|---|
| Real-time utilization | Compound v3 / Aave v3 / Morpho contracts | Direct RPC call (`getUtilizationRate()`) | Free (Alchemy free tier) |
| Historical utilization | The Graph subgraphs (Compound, Aave, Morpho) | GraphQL API | Free |
| Historical borrow rates | DeFiLlama `/rates` API | REST API | Free |
| Pool TVL history | DeFiLlama `/tvl` API | REST API | Free |
| On-chain repayment events | Etherscan API or RPC log filters | `eth_getLogs` for `Repay` events | Free |
| Deposit events | Same as above | `eth_getLogs` for `Supply` events | Free |
| ETH/BTC perp price history | Hyperliquid API | REST/WebSocket | Free |
| Pre-Hyperliquid perp history | Binance Futures API | REST API | Free |
| Governance proposals | Compound Forum, Aave Governance portal | Manual monitoring + RSS | Free |
| Funding rates | Hyperliquid API, Coinglass | REST API | Free |

### RPC Query Template (Python pseudocode)

```python
# Poll every 5 minutes
def check_kink_crossing(pool_address, kink_threshold):
    current_util = compound_v3.getUtilizationRate(pool_address)
    prev_util = get_previous_block_util(pool_address)
    
    if current_util > kink_threshold and prev_util <= kink_threshold:
        current_rate = compound_v3.getBorrowRate(pool_address)
        rate_delta_apr = (current_rate - prev_rate) * BLOCKS_PER_YEAR
        
        if rate_delta_apr >= 0.15:  # 15% APR minimum delta
            tvl = get_pool_tvl(pool_address)
            if tvl >= 500_000_000:
                fire_alert(pool_address, current_util, rate_delta_apr, tvl)
```

---

## Open Research Questions

Before backtesting, the following questions should be answered to sharpen the hypothesis:

1. **What is the historical base rate of kink crossings per pool per quarter?** If crossings are rare (<5 per year per pool), the backtest sample will be too small for statistical confidence.
2. **What fraction of borrowers on Compound/Aave are automated vs. manual?** Higher automation → faster response → shorter trade window.
3. **Is there a size threshold below which kink crossings produce no measurable price impact?** A $600M pool crossing the kink may behave differently from a $5B pool.
4. **Do kink crossings cluster with other events** (e.g., do they tend to occur during broad market volatility, which would confound the signal)?
5. **What is the typical duration of above-kink utilization?** If crossings resolve in <1 hour on average, the 48h exit window is too wide and the 60-minute entry window may already be too late.

---

## Next Steps (Step 3 of 9)

1. **Data pull:** Extract all utilization snapshots for Compound v3 USDC (Ethereum mainnet) from January 2022 to present using The Graph subgraph. Estimated time: 4 hours.
2. **Event identification:** Run crossing detection algorithm, apply TVL filter, count events. If <20 events found, expand to Aave v3 and Morpho before proceeding.
3. **Price matching:** Join event timestamps to ETH-PERP hourly OHLCV data.
4. **Initial signal check:** Plot average cumulative return from T=0 to T+48h across all events. If directional signal is visible in the raw data, proceed to full statistical analysis.
5. **Report back:** Document event count, raw signal chart, and preliminary win rate before committing to full backtest infrastructure build.

---

*This document is a research hypothesis. No live capital should be deployed until all go-live criteria are satisfied. All backtest results must be documented with full methodology to prevent data snooping.*
