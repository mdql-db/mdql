---
title: "Lending Protocol Daily Interest Accrual Liquidation Threshold Creep"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 4
composite: 500
categories:
  - liquidation
  - lending
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Aave v3 smart contracts accrue interest every block via a deterministic mathematical formula (`LinearInterestRate` or `CompoundedInterestRate` depending on asset). Borrowers whose health factor sits between 1.01 and 1.08 will be pushed below 1.0 purely by time-elapsed interest accumulation — requiring zero additional price movement. This creates a calculable, forward-looking queue of forced collateral liquidations. When the aggregate notional of this queue exceeds $5M and time-to-liquidation is under 24 hours at flat price, a statistically meaningful forced sell event is scheduled. Shorting the collateral asset on Hyperliquid perpetuals before this event fires captures the price impact of the liquidation discount (5–8% on-chain) bleeding into spot and perp markets.

The edge is **not** that liquidations cause cascades (that is a pattern claim). The edge is that the **timing and minimum size of the forced sell are mathematically determinable in advance** from public on-chain state.

---

## Structural Mechanism

### Why This Must Happen

Aave v3 accrues interest via the `updateState()` function called on every interaction with the pool. The health factor formula is:

```
Health Factor = Σ(collateral_i × liquidation_threshold_i) / total_debt_in_base_currency
```

Debt grows continuously. Collateral value is static if price is flat. Therefore:

```
ΔHF per day ≈ -HF × borrow_APR / 365
```

For a position at HF = 1.04 borrowing USDC at 8% APR:

```
Time to HF = 1.0 ≈ (0.04 / (1.04 × 0.08/365)) ≈ 70 hours
```

This is **not probabilistic** — it is arithmetic. The only escape routes for the borrower are:
1. Add collateral (requires active management and gas cost)
2. Repay debt (requires active management and gas cost)
3. Price of collateral rises enough to restore HF above threshold

Escape routes 1 and 2 require the borrower to act. Many borrowers are inactive (wallets dormant, lost keys, set-and-forget DeFi positions). The fraction of near-threshold positions that self-rescue vs. get liquidated is an empirical question — this is the primary uncertainty in the strategy.

### Why the Market Doesn't Fully Price This

1. **Monitoring cost**: Tracking thousands of individual positions across Aave v3 deployments (Ethereum, Arbitrum, Optimism, Polygon, Base) requires infrastructure most traders don't maintain.
2. **Timing uncertainty**: Even if you know liquidation is coming, you don't know if it fires in 4 hours or 22 hours — bots execute the moment HF < 1.0, which depends on block-by-block accrual.
3. **Fragmentation**: No single dashboard aggregates cross-chain near-threshold queues with dollar-notional weighting in real time.
4. **Small individual positions**: Most individual positions are small; the edge only appears when you aggregate the cluster.

---

## Universe & Scope

**Protocols monitored:** Aave v3 on Ethereum mainnet (primary), Arbitrum, Optimism, Base  
**Collateral assets traded:** ETH and WBTC only (liquid perp markets on Hyperliquid; sufficient depth to absorb our position)  
**Excluded:** stablecoin-collateralized positions (liquidation sells stablecoins, no perp trade available), long-tail collateral assets (illiquid perps)  
**Minimum cluster size:** $5M aggregate notional in the near-threshold queue for a single collateral asset  
**Time-to-liquidation window:** < 24 hours at flat price  
**Health factor range monitored:** 1.01 – 1.08

---

## Entry Rules

### Step 1 — Build the Queue

Every 30 minutes, query Aave v3 GraphQL API for all open positions:

```graphql
{
  users(where: {borrowedReservesCount_gt: 0}) {
    reserves {
      currentATokenBalance
      currentVariableDebt
      reserve {
        symbol
        liquidationThreshold
        variableBorrowRate
        priceInUSD
      }
    }
  }
}
```

Compute health factor and time-to-liquidation for each position assuming flat price.

### Step 2 — Cluster Aggregation

Group positions by collateral asset. Sum notional of all positions with:
- HF between 1.01 and 1.08
- Time-to-liquidation < 24 hours

### Step 3 — Entry Signal

**All three conditions must be true simultaneously:**

| Condition | Threshold |
|-----------|-----------|
| Cluster notional (ETH or WBTC collateral) | ≥ $5M |
| Weighted average time-to-liquidation | < 24 hours |
| Hyperliquid perp funding rate for asset | Not strongly negative (< -0.05% per 8h) — avoids paying excessive funding against the trade |

**Entry execution:** Market order on Hyperliquid perp for the collateral asset (ETH-PERP or BTC-PERP), SHORT direction. Execute within 5 minutes of signal confirmation to avoid front-running by other monitors.

### Step 4 — Entry Timing Refinement

Do not enter if:
- A major macro event (FOMC, CPI) is scheduled within 4 hours — price rally risk is elevated
- The asset has moved +3% in the last 2 hours — potential rescue rally already underway
- Open interest on Hyperliquid has dropped >15% in the last hour — sign of broad deleveraging already in progress

---

## Exit Rules

### Primary Exit — Liquidation Confirmed

Monitor Aave v3 `LiquidationCall` events on-chain. When cumulative liquidated notional from the target cluster reaches 60% of the original cluster size, close the short position at market.

**Rationale:** The forced sell has occurred; remaining positions may self-rescue or the market may rebound.

### Secondary Exit — Time Stop

If no liquidation events fire within 36 hours of entry, close the position regardless. This handles the case where price rallied enough to rescue positions without triggering liquidations.

### Tertiary Exit — Price Stop

If the collateral asset rallies 2.5% from entry price, close the position. A 2.5% rally likely rescues most near-threshold positions (recalculate HF rescue threshold per cluster to confirm). This is a hard risk stop, not a prediction.

### Profit Target

No fixed profit target — exit is event-driven (liquidation fires) or time/price stopped. Do not use limit orders to exit; use market orders when exit conditions are met to avoid partial fills during volatile liquidation windows.

---

## Position Sizing

```
Position size = min(cluster_notional × 0.02, max_position_cap)
```

**Parameters:**
- `cluster_notional`: Total USD value of near-threshold positions in the queue
- `0.02`: 2% of cluster notional — sized to be meaningful but not large enough to move the perp market
- `max_position_cap`: $500K notional per trade (hard cap regardless of cluster size)
- `account_risk_cap`: Never exceed 5% of total trading account in a single position

**Example:** $20M cluster → 2% = $400K position. $50M cluster → capped at $500K.

**Funding cost budget:** At 0.01% per 8h funding rate, a 36-hour hold costs ~0.045% in funding. This is acceptable. If funding exceeds 0.03% per 8h at entry, reduce position size by 50%.

---

## Backtest Methodology

### Data Sources

| Data | Source | Cost |
|------|--------|------|
| Aave v3 historical positions | The Graph (Aave subgraph) | Free |
| Historical health factors | Aave v3 subgraph `UserReserve` entity | Free |
| ETH/BTC price history | Hyperliquid API, Binance API | Free |
| Historical liquidation events | Aave v3 `LiquidationCall` event logs via Alchemy/Infura | Free tier sufficient |
| Historical borrow rates | Aave v3 subgraph `ReserveParamsHistoryItem` | Free |

### Backtest Procedure

**Step 1 — Reconstruct historical queues**  
For each day from Aave v3 launch (March 2022) to present, reconstruct the near-threshold queue using historical position snapshots and borrow rates. The Graph stores historical state; query at 30-minute intervals.

**Step 2 — Identify signal days**  
Flag all timestamps where cluster notional ≥ $5M and weighted TTL < 24 hours.

**Step 3 — Simulate trades**  
For each signal, record:
- Entry price (ETH or BTC spot at signal time)
- Whether liquidations fired within 36 hours (check `LiquidationCall` logs)
- Exit price (at liquidation confirmation or time/price stop)
- Funding costs paid (from historical funding rate data)
- Slippage estimate: assume 3bps for entry and exit on Hyperliquid ETH-PERP

**Step 4 — Measure outcomes**  
Primary metrics:
- Win rate (liquidation fired before stop-loss)
- Average P&L per trade (net of funding and slippage)
- Sharpe ratio
- Maximum drawdown
- False positive rate (signal fired, no liquidation, price rallied)

**Step 5 — Sensitivity analysis**  
Re-run backtest varying:
- Cluster threshold: $2M, $5M, $10M
- TTL window: 12h, 24h, 48h
- Health factor range: 1.01–1.05, 1.01–1.08, 1.01–1.12
- Stop-loss: 1.5%, 2.5%, 4%

**Known backtest limitation:** The Graph may have gaps in historical position data. Cross-validate liquidation events against raw on-chain logs to ensure completeness. Do not trust subgraph data alone for P&L calculation.

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

| Criterion | Threshold |
|-----------|-----------|
| Backtest win rate | ≥ 55% |
| Backtest Sharpe (annualised) | ≥ 1.2 |
| Minimum backtest trades | ≥ 40 independent signals |
| Paper trade win rate (30-day) | ≥ 50% (lower bar due to smaller sample) |
| Paper trade max drawdown | < 8% of paper account |
| Monitoring infrastructure uptime | ≥ 99% (alerts if data feed drops) |
| Funding rate regime | Average funding < 0.02% per 8h over paper trade period |

**Paper trade period:** Minimum 30 days, minimum 8 live signals before go-live decision.

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

| Kill Trigger | Action |
|--------------|--------|
| 5 consecutive losing trades | Pause, review whether rescue-rally frequency has increased |
| Live drawdown exceeds 12% of allocated capital | Full stop, mandatory review |
| Aave v3 governance changes liquidation threshold mechanics | Full stop until re-analysis complete |
| Hyperliquid ETH-PERP or BTC-PERP liquidity drops below $10M daily volume | Suspend (slippage makes strategy uneconomical) |
| Monitoring infrastructure gap > 2 hours undetected | Suspend until infrastructure hardened |
| Funding rate persistently > 0.04% per 8h for > 72 hours | Suspend (carry cost destroys edge) |

---

## Risks

### Risk 1 — Price Rescue (Primary Risk, HIGH)
A price rally of 2–5% rescues near-threshold positions before liquidation fires. This is the most common failure mode. **Mitigation:** 2.5% hard stop-loss. Do not hold through macro events. Avoid entry when asset is already in uptrend.

### Risk 2 — Borrower Self-Rescue (MEDIUM)
Active borrowers add collateral or repay debt before liquidation. This is rational behavior but requires gas and attention. **Mitigation:** Empirically measure self-rescue rate in backtest. If > 40% of near-threshold positions self-rescue, reduce position size or raise cluster threshold.

### Risk 3 — Liquidation Bot Front-Running (LOW-MEDIUM)
Liquidation bots execute the moment HF < 1.0, often within the same block. The collateral sell happens atomically. The price impact may be absorbed by MEV searchers who sandwich the liquidation. **Mitigation:** We are not competing with bots — we are positioned before the event. The perp market should reflect the forced sell pressure regardless of who executes the liquidation.

### Risk 4 — Funding Rate Bleed (LOW-MEDIUM)
If ETH or BTC perps trade at persistent premium (positive funding), shorts pay funding continuously. A 36-hour hold at 0.03% per 8h costs ~0.135%. **Mitigation:** Funding rate filter at entry. Kill criterion for persistent high funding.

### Risk 5 — Aave Protocol Risk (LOW)
Smart contract exploit or governance emergency pause could freeze liquidations, trapping our short while the market reacts unpredictably. **Mitigation:** Monitor Aave governance forums and security alerts. Kill strategy immediately on any protocol incident.

### Risk 6 — Data Lag / Subgraph Staleness (MEDIUM)
The Graph subgraph may lag real-time chain state by several minutes. A position could cross HF = 1.0 and be liquidated before our monitoring detects it. **Mitigation:** Cross-validate subgraph data with direct RPC calls to `getUserAccountData()` on Aave's Pool contract for the largest positions. Alert if subgraph lag > 5 minutes.

### Risk 7 — Cluster Fragmentation Across Chains (LOW)
A $5M cluster on Ethereum mainnet has different market impact than $5M fragmented across Arbitrum, Optimism, and Base. Cross-chain liquidations may not aggregate into a single price impact event. **Mitigation:** In initial deployment, count only Ethereum mainnet clusters. Add L2 clusters only after validating their price impact in backtest.

### Risk 8 — Reflexivity / Strategy Crowding (LOW initially)
If this strategy becomes widely known, monitoring infrastructure proliferates, and the market pre-prices the liquidation queue, the edge compresses. **Mitigation:** Monitor whether entry-to-liquidation price moves shrink over time. If average P&L per trade drops below 0.3% net, reassess.

---

## Data Sources

| Source | URL / Method | Data | Latency |
|--------|-------------|------|---------|
| Aave v3 subgraph (Ethereum) | `https://api.thegraph.com/subgraphs/name/aave/protocol-v3` | Position health factors, borrow rates, liquidation events | ~2 min |
| Aave v3 Pool contract | `getUserAccountData(address)` via Alchemy RPC | Real-time health factor for specific addresses | ~1 block |
| Aave v3 event logs | `LiquidationCall` event via Alchemy/Infura | Liquidation confirmation | ~1 block |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | Perp prices, funding rates, OI | Real-time |
| Binance API | REST API | ETH/BTC spot price reference | Real-time |
| DeFi Llama | `https://api.llama.fi/` | Protocol TVL cross-check | ~1 hour |

---

## Implementation Checklist

- [ ] Build position scanner: query Aave v3 subgraph every 30 minutes, compute HF and TTL for all positions
- [ ] Build cluster aggregator: group by collateral asset, sum notional for HF 1.01–1.08 and TTL < 24h
- [ ] Build signal alerting: Telegram/Slack alert when cluster ≥ $5M
- [ ] Build liquidation monitor: watch `LiquidationCall` events in real time, match to tracked cluster
- [ ] Build Hyperliquid order executor: market short entry and exit with size calculation
- [ ] Build funding rate checker: query Hyperliquid funding before every entry
- [ ] Set up subgraph lag monitor: alert if subgraph timestamp > 5 minutes behind chain head
- [ ] Run historical backtest: reconstruct queues from March 2022 to present
- [ ] Run 30-day paper trade with live infrastructure
- [ ] Review go-live criteria checklist before deploying real capital

---

## Open Questions for Backtest Phase

1. What fraction of positions in HF 1.01–1.08 self-rescue (add collateral/repay) vs. get liquidated? This determines the true signal quality.
2. What is the average ETH/BTC price move between signal detection and liquidation event? This determines realistic P&L per trade.
3. Does cluster size correlate with price impact magnitude, or do liquidation bots absorb the sell without market impact?
4. Are there day-of-week or time-of-day patterns in liquidation clustering (e.g., weekend low-liquidity periods)?
5. How often does a price rally rescue positions after our entry but before our stop-loss triggers? (Measures stop-loss calibration accuracy.)

---

*This document is a hypothesis specification. No backtest has been run. No live trading has occurred. All parameters are initial estimates subject to revision after empirical testing.*
