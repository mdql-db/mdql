---
title: "Aave Utilization Kink Rate Spike — Borrow Unwind Short"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 5
frequency: 3
composite: 450
categories:
  - defi-protocol
  - lending
  - liquidation
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Aave's utilization rate crosses the hard-coded kink threshold upward, borrow rates spike parabolically within a single block. Borrowers paying punishing rates face an immediate economic incentive to repay. Repayment of stablecoin loans requires borrowers to either sell volatile collateral (ETH, BTC, altcoins) or buy stablecoins — both create directional selling pressure on the collateral asset. This pressure is mechanically predictable because the rate spike is contractually guaranteed by the smart contract's interest rate model. The trade: short the collateral asset on Hyperliquid perps immediately after a confirmed kink crossing, exit when utilization retreats below the kink.

The edge is **not** that borrowers panic — it is that the rate arithmetic makes continued borrowing economically irrational for leveraged positions above a calculable threshold, creating a structural flow event with a known direction.

---

## Structural Mechanism

### The Kink Model (Aave V2/V3)

Aave's interest rate model (`DefaultReserveInterestRateStrategy`) computes borrow APR as a two-slope function hard-coded in the deployed smart contract:

```
If utilization (U) ≤ U_optimal:
    Borrow Rate = Base Rate + (U / U_optimal) × Slope1

If utilization (U) > U_optimal:
    Borrow Rate = Base Rate + Slope1 + ((U - U_optimal) / (1 - U_optimal)) × Slope2
```

**Concrete USDC example (Aave V3 Ethereum, as deployed):**
- `U_optimal` = 0.80 (80%)
- `Slope1` = 4% APR (gradual)
- `Slope2` = 75% APR (punishing)
- At U = 0.79: Borrow APR ≈ 5%
- At U = 0.81: Borrow APR ≈ 9% (Slope2 kicks in)
- At U = 0.95: Borrow APR ≈ 64%

The transition is discontinuous in economic impact — a borrower at 79% utilization pays ~5% APR; the same borrower one block later at 81% pays ~9% and climbing. At 90%+ utilization, rates exceed 30% APR, making leveraged positions economically untenable within days.

### The Forced Flow Chain

1. **Kink crossed upward** → borrow rate spikes in the same block (on-chain, verifiable)
2. **Rate-sensitive borrowers** (leveraged yield farmers, recursive borrowers, delta-neutral desks) face immediate P&L deterioration
3. **Repayment requires** either: (a) buying stablecoins in spot/perp markets, or (b) selling collateral to generate stablecoins
4. **Net effect**: selling pressure on collateral assets (ETH, BTC, LSTs, altcoins) and/or buying pressure on stablecoins
5. **Utilization retreats** as repayments reduce total borrows → rate normalizes → pressure abates
6. **Window**: typically 4–48 hours based on historical kink events (hypothesis — needs backtest)

### Why This Is Structural, Not Pattern-Based

The rate spike is **not probabilistic** — it is computed deterministically by the smart contract every block. The economic incentive to repay is **not behavioral** — it is arithmetic: a borrower with $1M USDC borrowed at 60% APR loses $1,644/day in interest. The direction of collateral selling is **not assumed** — it follows from the only available repayment path for undercollateralized or leveraged borrowers.

The **probabilistic component** (score: 6 not 8) is: how quickly borrowers respond, whether they are rate-tolerant (e.g., hedged elsewhere), and whether new supply enters the pool before repayments occur (liquidity providers adding supply can resolve the kink without any collateral selling).

---

## Markets in Scope

| Pool | Collateral Asset | Perp Instrument | U_optimal | Slope2 |
|------|-----------------|-----------------|-----------|--------|
| USDC (Aave V3 Ethereum) | ETH, wBTC, wstETH | ETH-PERP, BTC-PERP | 80% | 75% |
| USDT (Aave V3 Ethereum) | ETH, wBTC | ETH-PERP, BTC-PERP | 80% | 75% |
| USDC (Aave V3 Arbitrum) | ETH, wBTC, ARB | ETH-PERP, BTC-PERP, ARB-PERP | 80% | 75% |
| DAI (Aave V3 Ethereum) | ETH, wBTC | ETH-PERP, BTC-PERP | 80% | 75% |

**Priority order for backtesting:** USDC Ethereum first (largest pool, most historical events), then Arbitrum USDC.

**Excluded pools:** Pools where the borrowed asset is itself volatile (e.g., borrowing ETH against BTC) — the flow direction is ambiguous and the position sizing is harder to calibrate.

---

## Entry Rules

### Signal Detection

**Step 1 — Monitor utilization every block (12 seconds on Ethereum):**
```
Source: Aave V3 Pool contract → getReserveData() → liquidityRate + variableBorrowRate
OR: Aave subgraph → reserves(id: "USDC") → utilizationRate
```

**Step 2 — Kink crossing confirmed when ALL of the following are true:**
- `U_current > U_optimal` (e.g., > 0.80 for USDC)
- `U_previous ≤ U_optimal` (crossed upward in this block or within last 3 blocks)
- `variableBorrowRate_current > variableBorrowRate_previous × 1.5` (rate increased ≥50% — filters noise from gradual drift)
- `totalVariableDebt_pool > $50M` (minimum pool size — small pools have insufficient flow to move perp prices)

**Step 3 — Collateral composition check:**
- Pull top-5 collateral assets by USD value from Aave subgraph
- Confirm ETH or BTC constitutes ≥40% of total collateral in the pool
- If altcoin collateral dominates (e.g., LINK, UNI), skip trade — perp liquidity on Hyperliquid insufficient for clean execution

**Step 4 — Market filter (do not enter if):**
- Hyperliquid ETH-PERP or BTC-PERP funding rate is already < -0.05% per 8h (market already short-biased — crowded)
- Spot price has already dropped >3% in the 30 minutes preceding the signal (late entry, flow may be exhausted)
- A major protocol announcement or macro event is scheduled within 2 hours (confounding factor)

### Entry Execution

- **Instrument:** ETH-PERP or BTC-PERP on Hyperliquid (whichever asset is dominant collateral)
- **Direction:** SHORT
- **Entry price:** Market order within 2 minutes of signal confirmation (do not use limit orders — the edge is time-sensitive)
- **Slippage budget:** Accept up to 0.15% slippage on entry; abort if order book depth is insufficient for position size at this tolerance

---

## Exit Rules

### Primary Exit (Signal-Based)

Exit SHORT when **any** of the following:

1. **Utilization retreats below kink:** `U_current < U_optimal × 0.97` (3% buffer to avoid whipsaw exits) — indicates repayments completed or new liquidity supplied
2. **Borrow rate normalizes:** `variableBorrowRate_current < variableBorrowRate_entry × 0.60` — rate has dropped significantly, pressure abating
3. **Time stop:** 48 hours from entry — if utilization has not resolved, the trade thesis has failed (borrowers are rate-tolerant or new supply is incoming)

### Secondary Exit (Risk-Based)

4. **Stop-loss:** Price moves against position by 2.5% from entry (hard stop, no exceptions)
5. **Funding rate stop:** If Hyperliquid funding rate on the shorted perp exceeds +0.10% per 8h (market is paying longs heavily — structural headwind to short)

### Exit Execution

- **Instrument:** Close SHORT via market order on Hyperliquid
- **Do not scale out** — exit full position at once to avoid partial exposure during rate normalization

---

## Position Sizing

### Base Formula

```
Position Size ($) = min(
    Pool_Borrow_Excess × Collateral_ETH_Share × Sensitivity_Factor,
    Max_Position_Cap
)

Where:
    Pool_Borrow_Excess = totalVariableDebt × (U_current - U_optimal) / U_current
    Collateral_ETH_Share = ETH_collateral_USD / Total_collateral_USD
    Sensitivity_Factor = 0.005  (0.5% of excess borrow value — conservative starting point)
    Max_Position_Cap = $50,000 (hard cap during hypothesis testing phase)
```

**Example calculation:**
- USDC pool total borrows: $800M
- U_current: 0.85, U_optimal: 0.80
- Pool_Borrow_Excess: $800M × (0.05/0.85) = $47M
- ETH collateral share: 60%
- Raw size: $47M × 0.60 × 0.005 = $141,000
- Capped at: $50,000

**Rationale for 0.5% sensitivity factor:** We have no validated data on what fraction of excess borrowers actually repay within the trade window. Start at 0.5%, adjust upward after 20+ backtested events confirm the flow magnitude.

### Leverage

- **Maximum leverage:** 3x on Hyperliquid
- **Preferred leverage:** 2x (preserves margin buffer against stop-loss)
- **Margin currency:** USDC (held on Hyperliquid, not in Aave — no recursive exposure)

### Portfolio-Level Constraints

- Maximum concurrent kink trades: 2 (one ETH, one BTC — not both on same pool)
- Maximum total capital allocated to this strategy: 15% of portfolio
- No position sizing increase until 20 live paper trades completed

---

## Backtest Methodology

### Step 1 — Data Collection

**Primary data source:** Aave V3 subgraph (The Graph, free)
```
Query: reserves historical data → utilizationRate, variableBorrowRate, totalVariableDebt
Granularity: hourly (subgraph limitation) — upgrade to block-level via Ethereum archive node for final backtest
Date range: Aave V3 Ethereum launch (January 2023) to present
```

**Secondary data source:** Dune Analytics
```
Dashboard: "Aave V3 Utilization History" (public, multiple community dashboards exist)
Supplement with: aave_v3.borrow_events, aave_v3.repay_events tables for flow confirmation
```

**Price data:** ETH/BTC hourly OHLCV from Hyperliquid historical API or Binance public API (free, complete)

**Funding rate data:** Hyperliquid historical funding rates (downloadable via API, free)

### Step 2 — Event Identification

Write a script (Python) that:
1. Loads hourly utilization data for USDC pool
2. Identifies all hours where `U_t > 0.80` AND `U_{t-1} ≤ 0.80` (kink crossing events)
3. Tags each event with: timestamp, peak utilization reached, duration above kink, total borrow volume at crossing
4. Filters events by pool size filter ($50M minimum borrows)

**Expected event count:** Hypothesis — 15–40 events since January 2023 (needs verification). If fewer than 10 events found, strategy lacks sufficient sample size for statistical inference.

### Step 3 — Simulated Trade Execution

For each identified event:
1. **Entry:** Short ETH-PERP at the hourly close price of the kink-crossing candle + 0.15% slippage
2. **Exit:** Apply exit rules in priority order using subsequent hourly candles
3. **Record:** Entry price, exit price, exit reason, hold duration, P&L in %, funding paid/received
4. **Apply:** 0.05% round-trip trading fee (Hyperliquid taker fee estimate)

### Step 4 — Analysis Metrics

Report the following for the full event set:

| Metric | Target (Go-Live Threshold) |
|--------|---------------------------|
| Win rate | ≥ 55% |
| Average win / Average loss ratio | ≥ 1.5 |
| Median hold time | < 24h |
| Maximum drawdown (single trade) | < 5% |
| Sharpe ratio (annualized, if enough events) | ≥ 1.0 |
| % of trades exited via stop-loss | < 30% |
| % of trades where utilization resolved within 48h | Report as-is |

### Step 5 — Robustness Checks

- **Sensitivity test:** Vary entry threshold from 1.5x rate increase to 2.0x and 3.0x — does win rate improve with stricter entry?
- **Pool size sensitivity:** Test $25M, $50M, $100M minimum pool size filters
- **Collateral share sensitivity:** Test 30%, 40%, 50% ETH collateral share thresholds
- **Slippage stress test:** Re-run with 0.30% slippage (2x base assumption) — does strategy remain profitable?
- **Funding rate impact:** Subtract actual historical funding costs from each trade P&L

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. **Minimum 15 backtested events** identified and simulated (if fewer exist, wait for more data or expand to Arbitrum pool)
2. **Win rate ≥ 55%** across backtested events
3. **Average win/loss ratio ≥ 1.5** (positive expectancy confirmed)
4. **Stop-loss triggered in < 30% of trades** (signal quality check)
5. **Median hold time < 48h** (confirms the mechanism resolves within the trade window)
6. **Funding rate cost < 20% of gross P&L** on average (carry cost manageable)
7. **Monitoring infrastructure live:** Automated alert fires within 5 minutes of kink crossing (Dune alert, custom Python script, or Aave subgraph webhook)

Paper trade for minimum **30 days or 5 live events** (whichever is longer) before allocating real capital.

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

1. **Backtest win rate < 45%** — signal has no edge, do not proceed
2. **3 consecutive stop-losses hit** during paper trading — mechanism may have changed (e.g., Aave parameter update, new liquidity provider behavior)
3. **Aave governance changes the kink parameters** (U_optimal, Slope2) — recalibrate entire backtest from scratch before re-entering
4. **Monitoring latency > 15 minutes** — if we cannot detect the kink crossing within 15 minutes, the entry edge is lost (repayments may already be underway)
5. **Hyperliquid ETH-PERP or BTC-PERP average daily volume drops below $200M** — insufficient liquidity for clean execution at target position sizes
6. **Strategy correlation > 0.7 with token unlock shorts** — if both strategies fire simultaneously and are correlated, portfolio concentration risk is unacceptable

---

## Risks

### Risk 1: Liquidity Provider Response (Primary Risk)
**Description:** New liquidity providers can deposit into the Aave pool within the same block as the kink crossing, immediately resolving the utilization spike before any borrower repays. This would eliminate the flow event entirely.
**Mitigation:** The 3-block confirmation window in entry rules filters single-block spikes. However, fast LP response remains the primary reason this scores 6 not 8.
**Monitoring:** Track LP deposit events in the same pool during the trade window; if LP inflows exceed borrow excess, exit immediately.

### Risk 2: Rate-Tolerant Borrowers
**Description:** Institutional borrowers or protocol treasuries may be willing to pay 30–60% APR for short periods if their strategy generates higher returns (e.g., recursive yield farming at 80% APY). These borrowers do not repay, and no flow event occurs.
**Mitigation:** Collateral composition check (Step 3 of entry rules) — if collateral is dominated by yield-bearing assets (wstETH, rETH), borrowers are more likely to be yield farmers who are rate-tolerant. Skip these events.
**Monitoring:** Track repay event volume on-chain during the trade window; if repay volume is < 5% of excess borrows within 4 hours, exit via time stop.

### Risk 3: Aave Parameter Governance Changes
**Description:** Aave governance can vote to change U_optimal, Slope1, Slope2, or reserve factor at any time. A change to Slope2 (e.g., reducing from 75% to 40%) would materially weaken the economic incentive to repay.
**Mitigation:** Subscribe to Aave governance forum RSS feed and Snapshot voting alerts. Pause strategy during any active governance vote that touches interest rate parameters.
**Monitoring:** Check Aave governance portal weekly; maintain a parameter change log.

### Risk 4: Cross-Protocol Arbitrage Absorbs Flow
**Description:** Other lending protocols (Compound, Morpho, Spark) may offer lower rates, allowing borrowers to migrate their position rather than repay. Migration = repay on Aave + borrow on Compound — the Aave utilization resolves, but no collateral selling occurs.
**Mitigation:** Monitor Compound/Morpho borrow rates at entry. If competing protocol rates are within 5% of Aave's pre-kink rate, the migration path is open and the collateral-selling thesis is weakened. Reduce position size by 50% in this scenario.

### Risk 5: Macro Confound
**Description:** A simultaneous macro event (Fed announcement, major hack, exchange insolvency) can cause ETH/BTC to move 5–10% independent of the Aave flow, triggering the stop-loss on a trade that would otherwise have been profitable.
**Mitigation:** The 2.5% stop-loss is the primary protection. Do not override the stop-loss based on conviction about the Aave mechanism — macro moves are faster than Aave repayment flows.

### Risk 6: Hyperliquid Execution Risk
**Description:** Hyperliquid perp prices may not track spot prices during high-volatility periods, creating basis risk. Additionally, Hyperliquid is a relatively new exchange with smart contract risk.
**Mitigation:** Monitor ETH-PERP basis vs. Binance spot at entry; abort if basis > 0.3%. Maintain no more than 15% of total portfolio on Hyperliquid at any time.

---

## Data Sources

| Data | Source | Access | Cost | Latency |
|------|---------|--------|------|---------|
| Aave V3 utilization (historical) | The Graph — Aave V3 subgraph | Public API | Free | ~1 hour lag |
| Aave V3 utilization (real-time) | Aave Pool contract `getReserveData()` | Ethereum RPC (Alchemy free tier) | Free (limited) | ~12 seconds |
| Borrow/repay event logs | Dune Analytics — `aave_v3.borrow_events` | Public | Free (rate-limited) | ~1 hour lag |
| Collateral composition | Aave V3 subgraph — `userReserves` | Public API | Free | ~1 hour lag |
| ETH/BTC price history | Hyperliquid API or Binance public API | Public | Free | Real-time |
| Funding rate history | Hyperliquid historical API | Public | Free | Real-time |
| Competing protocol rates | Compound subgraph, Morpho API | Public | Free | ~1 hour lag |
| Governance alerts | Aave governance forum RSS, Snapshot API | Public | Free | Manual check |

**Infrastructure requirement:** Python script polling Ethereum RPC every 60 seconds for `getReserveData()` on USDC, USDT, DAI pools. Alert fires to Telegram/Discord when kink crossing confirmed. Estimated setup time: 4–8 hours for a competent Python developer.

---

## Open Questions for Backtest Phase

1. **How many kink crossing events occurred on Aave V3 Ethereum USDC since January 2023?** This determines whether the strategy has sufficient sample size.
2. **What is the typical duration above the kink?** If most events resolve in < 4 hours, the 48-hour time stop is too loose; if most take > 48 hours, the time stop is too tight.
3. **What fraction of kink events are resolved by LP inflows vs. borrower repayments?** This is the primary determinant of whether collateral selling actually occurs.
4. **Is there a detectable price impact on ETH/BTC during confirmed repayment-driven kink resolutions?** If the price impact is < 0.5%, the strategy cannot overcome trading costs.
5. **Do kink events cluster around specific market conditions** (e.g., high funding rates, low spot volatility, protocol launches)? If so, a pre-filter can improve signal quality before the kink actually crosses.

---

## Next Steps

| Step | Action | Owner | Deadline |
|------|--------|-------|----------|
| 1 | Pull Aave V3 USDC utilization history from subgraph, identify all kink crossing events | Researcher | Week 1 |
| 2 | Classify each event: LP-resolved vs. repayment-resolved (check repay event volume) | Researcher | Week 1 |
| 3 | Simulate trades on repayment-resolved events only; compute metrics table | Researcher | Week 2 |
| 4 | Run robustness checks (sensitivity tests) | Researcher | Week 2 |
| 5 | Build monitoring script (Python + Ethereum RPC + Telegram alert) | Engineer | Week 2 |
| 6 | Review backtest results against go-live criteria | Strategy committee | Week 3 |
| 7 | Begin paper trading if criteria met | Trader | Week 3+ |
