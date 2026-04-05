---
title: "Lending Protocol Utilization Ceiling Squeeze — Repo Rate Analogue"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 5
frequency: 4
composite: 600
categories:
  - lending
  - defi-protocol
  - liquidation
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a major DeFi lending pool (Aave V3, Morpho, Compound V3) crosses above its interest rate kink threshold (~80–92% utilization depending on the pool), the smart contract rate model **contractually** steps borrow rates from ~5–15% APY to 50–150% APY. This creates two forced flows within a 1–24 hour window:

1. **Forced unwind (sell pressure):** Leveraged borrowers whose position economics no longer work at the new rate must repay loans by selling collateral. This is not discretionary — at 100% APY borrow cost, a 2x leveraged ETH long loses money even in a flat market. The longer they wait, the more capital they destroy.
2. **Supply rush (rate compression):** Yield-seeking capital deposits into the pool chasing the temporarily elevated deposit APY, compressing utilization back below the kink.

**Causal chain:**
```
Utilization > kink threshold
    → borrow rate steps to 50–150% APY (contractually forced by rate model)
    → marginal leveraged borrowers face negative carry
    → forced collateral sales within hours (sell pressure on collateral asset)
    → simultaneously, deposit inflows chase elevated deposit APY
    → utilization compresses back below kink
    → borrow rate normalizes
```

**Tradeable signal:** Short the primary collateral asset on Hyperliquid perps at kink crossing. Exit when utilization retreats or 24h elapses.

The edge is **not** that prices always fall — it is that a specific, identifiable cohort of market participants faces a mechanically-imposed cost that makes their current position economically irrational to hold. The sell pressure is probabilistic in magnitude but directionally predictable.

---

## Structural Mechanism — Why This Must Happen

### The Rate Kink Is a Smart Contract Formula, Not a Tendency

Aave V3 uses a two-slope interest rate model (see `DefaultReserveInterestRateStrategy.sol`):

```
If utilization (U) <= U_optimal:
    borrow_rate = base_rate + (U / U_optimal) * slope1

If utilization (U) > U_optimal:
    borrow_rate = base_rate + slope1 + ((U - U_optimal) / (1 - U_optimal)) * slope2
```

For ETH on Aave V3 Ethereum mainnet (as of 2024):
- `U_optimal` = 80%
- `slope1` ≈ 3.8% APY
- `slope2` ≈ 80% APY

At 90% utilization, borrow rate ≈ **43% APY**. At 95%, ≈ **83% APY**. These are not estimates — they are the output of a deterministic formula executed on every block.

### Why Borrowers Must Act

A borrower using ETH as collateral to borrow USDC (to buy more ETH — a leveraged long) faces this arithmetic at 90% utilization:

- ETH must appreciate at >43% APY just to break even on borrow cost
- At 95% utilization, ETH must appreciate at >83% APY
- Most leveraged longs are not sized to absorb this; they unwind or get liquidated

The unwind is not guaranteed to happen *immediately* — borrowers with large buffers may wait. But the economic pressure is real and quantifiable, and it accumulates with every block.

### Why Supply Rushes In

Deposit APY = Borrow APY × Utilization. At 90% utilization and 43% borrow APY, deposit APY ≈ **38.7% APY**. This is visible on-chain and on Aave's UI in real time. Yield aggregators (Yearn, Morpho optimizers, idle capital in EOA wallets) respond within hours. This supply inflow is the natural equilibrating force — and it caps the duration of the high-rate window.

---

## Entry Rules


### Universe

Target pools only. Minimum criteria:
- Pool TVL > $50M (sufficient collateral to create meaningful sell pressure)
- Collateral asset must have a liquid Hyperliquid perp (ETH, wBTC, LINK, AAVE, etc.)
- Protocols: Aave V3 (Ethereum mainnet, Arbitrum, Base), Morpho Blue, Compound V3

Priority pools to monitor (as of 2025):
| Pool | Protocol | Chain | Kink Threshold | Collateral Perp |
|------|----------|-------|----------------|-----------------|
| USDC/ETH | Aave V3 | Ethereum | 80% | ETH-PERP |
| USDC/wBTC | Aave V3 | Ethereum | 80% | BTC-PERP |
| USDC/ETH | Morpho Blue | Ethereum | 92% | ETH-PERP |
| USDC/ETH | Compound V3 | Arbitrum | 80% | ETH-PERP |

### Entry Signal

**Trigger:** Pool utilization crosses **above kink threshold + 2% buffer** (e.g., >82% for an 80%-kink pool) on two consecutive 15-minute observations (prevents false triggers from transient spikes).

**Confirmation filters (all must pass):**
1. Utilization has been **below** kink for at least 4 hours prior (avoids entering mid-squeeze when pressure is already partially resolved)
2. Current borrow rate on the pool is **>2× the 7-day average borrow rate** for that pool
3. No active governance proposal to change rate parameters in the next 48h (check Aave governance forum / Snapshot)
4. Hyperliquid funding rate on the collateral perp is **not already negative >0.05% per 8h** (avoids piling into already-crowded shorts)

**Entry execution:**
- Open short on Hyperliquid perp of the primary collateral asset
- Use market order or limit order within 0.1% of mid (not time-sensitive enough to require aggressive fills)
- Record entry utilization, entry borrow rate, entry timestamp

## Exit Rules

### Exit Signal

**Exit triggers (first to occur):**
1. Pool utilization drops back **below kink threshold − 3%** (e.g., <77% for 80%-kink pool) — primary exit
2. **24 hours elapsed** since entry — time stop
3. **Stop-loss:** Collateral asset price moves +3% adverse from entry price
4. **Profit target:** Collateral asset price moves −5% from entry price (take partial profits at −3%)

**Exit execution:** Market order on Hyperliquid perp.

---

## Position Sizing

- **Base size:** 0.5% NAV per event
- **Scale up to 1% NAV** if: utilization >95% (extreme squeeze), AND pool TVL >$200M, AND confirmation filters all pass with margin
- **Maximum concurrent exposure:** 2% NAV across all open squeeze positions (events can cluster in bull markets)
- **No leverage beyond 3× on the perp** — the edge is directional, not a basis trade requiring high leverage

**Rationale for small sizing:** The sell pressure mechanism is probabilistic. This is not a guaranteed convergence trade. Small size allows many repetitions to build statistical evidence without ruin risk.

---

## Backtest Methodology

### Data Sources

| Data | Source | URL / Endpoint |
|------|--------|----------------|
| Aave V3 utilization history | The Graph (Aave subgraph) | `https://thegraph.com/explorer/subgraphs/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL57s` |
| Aave V3 rate parameters | Aave GitHub / on-chain | `ReserveData` struct via `getReserveData()` on `PoolDataProvider` |
| Morpho Blue utilization | Morpho subgraph | `https://api.thegraph.com/subgraphs/name/morpho-labs/morpho-blue` |
| Compound V3 utilization | Compound API | `https://api.compound.finance/api/v2/market_history` |
| ETH/BTC OHLCV | Hyperliquid API | `https://api.hyperliquid.xyz/info` (candleSnapshot endpoint) |
| Hyperliquid funding rates | Hyperliquid API | `https://api.hyperliquid.xyz/info` (fundingHistory endpoint) |

### Backtest Period

- **Primary:** January 2023 – December 2024 (covers multiple market regimes: bear, recovery, bull)
- **Minimum events required for statistical validity:** 30 kink crossings across all pools
- **Note:** Aave V3 launched on Ethereum mainnet in January 2022; sufficient history exists

### Reconstruction Steps

1. Pull hourly `liquidityRate`, `variableBorrowRate`, `availableLiquidity`, `totalVariableDebt` from Aave V3 subgraph for each target pool
2. Compute utilization = `totalVariableDebt / (totalVariableDebt + availableLiquidity)` at each hourly snapshot
3. Identify all kink-crossing events (utilization crosses threshold on 2 consecutive observations, preceded by 4h below threshold)
4. For each event: record entry timestamp, entry price of collateral asset, utilization at entry
5. Simulate exit at first of: utilization <kink−3%, 24h elapsed, +3% stop, −5% target
6. Compute P&L per trade (include Hyperliquid taker fee of 0.035% per side)

### Metrics to Compute

| Metric | Minimum Acceptable | Target |
|--------|-------------------|--------|
| Win rate | >45% | >55% |
| Average win / average loss ratio | >1.5 | >2.0 |
| Expectancy per trade (% NAV) | >0.05% | >0.15% |
| Max drawdown (strategy-level) | <3% NAV | <2% NAV |
| Sharpe ratio (annualized) | >0.8 | >1.5 |
| Number of events per year | >15 | >30 |

### Baseline Comparison

Compare against:
1. **Random short entry:** Short ETH at random times, same duration/stop/target parameters — tests whether the utilization signal adds value over noise
2. **Always-short:** Constant short ETH position over the same period — tests whether the signal beats passive directional bias
3. **Funding rate short:** Short ETH only when funding rate >0.05% per 8h — tests whether utilization signal is independent of the crowded-long signal

### Segmentation Analysis

Break results down by:
- Bull vs. bear market regime (ETH price trend over prior 30 days)
- Pool size (>$200M vs. $50–200M TVL)
- Utilization level at entry (82–90% vs. >90%)
- Time-to-resolution (did utilization resolve in <6h, 6–12h, 12–24h?)

The hypothesis predicts the strategy performs **worse** in strong bull markets (borrowers absorb high rates rather than unwind) and **better** in sideways/bear markets. If the backtest shows the opposite, the causal story is wrong.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **≥30 events** identified in backtest period across all pools
2. **Expectancy >0.05% NAV per trade** after fees
3. **Win rate >45%** (below this, variance makes the strategy unrunnable psychologically)
4. **Sharpe >0.8** on strategy-level equity curve
5. **Baseline comparison:** Strategy expectancy must exceed random-entry baseline by >0.03% NAV per trade (confirms signal adds value)
6. **Regime analysis:** Strategy must show positive expectancy in at least 2 of 3 market regimes tested (bull, bear, sideways) — if it only works in one regime, it's a regime bet, not a structural edge
7. **No single event accounts for >30% of total strategy P&L** (concentration risk check)

---

## Kill Criteria

Abandon the strategy (in backtest or live) if:

1. **Backtest:** Fewer than 20 events found in 2-year period — insufficient frequency to be worth the operational overhead
2. **Backtest:** Expectancy is negative or indistinguishable from random entry baseline
3. **Backtest:** Strategy only works in bear markets — this means it's a disguised directional bet on ETH, not a structural edge
4. **Live (paper trade):** After 20 paper trades, realized expectancy is <0 and lower than backtest by >0.1% NAV per trade (suggests regime change or strategy decay)
5. **Structural change:** Aave governance migrates to a dynamic rate model that removes the kink (monitor governance forums; this has been discussed)
6. **Crowding signal:** If a public strategy writeup or Dune dashboard tracking this exact signal gains >500 followers, assume the edge is being competed away — reduce size or pause

---

## Risks

### Primary Risks (Honest Assessment)

**1. Bull market suppression (HIGH probability, HIGH impact)**
In strong bull markets, leveraged borrowers are making money faster than the borrow rate costs them. A borrower paying 80% APY borrow cost is still profitable if ETH is up 5% in a week. These borrowers do NOT unwind. The strategy's core mechanism fails in the exact market condition where shorting ETH is most dangerous. This is the most serious risk.

*Mitigation:* The regime segmentation in backtest will quantify this. If bull-market events show negative expectancy, add a filter: only trade when ETH 30-day return is <+15%.

**2. Supply rush resolves utilization before sell pressure materializes (MEDIUM probability, HIGH impact)**
Yield aggregators and large depositors can resolve a utilization spike within 1–3 hours by depositing supply. If the rate normalizes before borrowers unwind, there is no sell pressure and the short loses to the stop.

*Mitigation:* The 24h time stop limits damage. Monitor deposit inflows in real time; if a single large deposit resolves utilization within 1 hour of entry, exit early.

**3. Governance parameter changes (LOW probability, HIGH impact)**
Aave governance can change `U_optimal`, `slope1`, or `slope2` via on-chain vote. A change that raises `U_optimal` from 80% to 90% would invalidate historical backtest data for that pool.

*Mitigation:* Check Aave governance (https://governance.aave.com) and Snapshot before each trade. Maintain a log of rate parameter changes and exclude affected periods from backtest.

**4. Liquidation cascade confounds signal (MEDIUM probability, MEDIUM impact)**
High utilization often coincides with high volatility. If the collateral asset is falling *because of a liquidation cascade*, the utilization spike is a lagging indicator of sell pressure already in progress — the short entry is late and chasing.

*Mitigation:* Add filter: only enter if collateral asset price has moved <2% in the 2 hours prior to utilization crossing the kink. If price is already moving, the cascade is underway and the edge is gone.

**5. Cross-protocol capital migration (LOW probability, MEDIUM impact)**
Sophisticated borrowers may migrate positions to Morpho or Compound when Aave rates spike, rather than unwinding. This reduces sell pressure without reducing utilization on the target pool.

*Mitigation:* Monitor utilization across all major protocols simultaneously. If Morpho utilization is also elevated, the migration escape valve is closed and the signal is stronger.

**6. Operational complexity (CERTAIN, LOW impact)**
This strategy requires real-time on-chain monitoring, subgraph queries, and cross-referencing governance forums. It is not a set-and-forget system. Subgraph indexing can lag by 15–30 minutes during high network load — exactly when this signal fires.

*Mitigation:* Use direct RPC calls to `getReserveData()` on Aave's `PoolDataProvider` contract for real-time utilization, not subgraph, for live trading. Subgraph is fine for backtesting.

---

## Data Sources

| Resource | URL | Notes |
|----------|-----|-------|
| Aave V3 subgraph (Ethereum) | `https://thegraph.com/explorer/subgraphs/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL57s` | Free, hourly granularity sufficient |
| Aave PoolDataProvider ABI | `https://github.com/aave/aave-v3-core/blob/master/contracts/misc/AaveProtocolDataProvider.sol` | For live RPC queries |
| Aave V3 deployed contracts | `https://docs.aave.com/developers/deployed-contracts/v3-mainnet` | Contract addresses by chain |
| Morpho Blue subgraph | `https://api.thegraph.com/subgraphs/name/morpho-labs/morpho-blue` | |
| Compound V3 market history | `https://api.compound.finance/api/v2/market_history` | |
| Aave governance forum | `https://governance.aave.com` | Monitor for rate parameter proposals |
| Aave Snapshot | `https://snapshot.org/#/aave.eth` | Active governance votes |
| Hyperliquid candles API | `https://api.hyperliquid.xyz/info` | POST `{"type":"candleSnapshot","req":{"coin":"ETH","interval":"15m",...}}` |
| Hyperliquid funding history | `https://api.hyperliquid.xyz/info` | POST `{"type":"fundingHistory","coin":"ETH",...}` |
| Dune Analytics (Aave utilization) | `https://dune.com/queries` | Search "Aave utilization" for community dashboards; useful for sanity-checking subgraph data |

---

## Open Questions for Researcher

Before building the backtest, answer these:

1. **How many kink-crossing events actually occurred on Aave V3 ETH mainnet in 2023–2024?** Pull the subgraph and count. If fewer than 20, the strategy is too infrequent to be worth building.
2. **What is the typical duration of above-kink utilization?** If most events resolve in <2 hours, the 15-minute polling interval may miss the entry window entirely.
3. **Is there an existing Dune dashboard tracking this?** If yes, check whether it shows any price correlation. If someone has already published this analysis, the edge may be partially competed away.
4. **Does Morpho Blue's oracle-based liquidation system interact with utilization spikes differently than Aave?** Morpho's architecture is meaningfully different — validate that the same causal chain applies before including it in the universe.
