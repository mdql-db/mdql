---
title: "Cascading Liquidation Chain — Second-Order Collateral Asset Short"
status: HYPOTHESIS
mechanism: 6
implementation: 4
safety: 4
frequency: 5
composite: 480
categories:
  - liquidation
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large DeFi liquidation occurs in collateral asset Y, the liquidator receives Y and sells it on-market, depressing Y's spot price. This price drop mechanically reduces the health factor of *other* positions across *other* protocols that also use Y as collateral. If the price gap between current Y spot and the next liquidation cluster is small (≤3%), the first liquidation event creates sufficient price pressure to trigger a second-order cascade. The cascade is not probabilistic — it is a deterministic smart contract execution once the price threshold is crossed. Our edge is: (a) the threshold prices are publicly readable on-chain before the cascade begins, and (b) we can enter a short on Hyperliquid perps in the seconds after the first liquidation is confirmed, before the cascade executes.

**Causal chain:**
1. Protocol A liquidates Position X (collateral = token Y, notional > $1M)
2. Liquidator bot receives Y and sells it → Y spot price drops 1–4%
3. On-chain position map shows Position Z in Protocol B has a liquidation threshold at current price − ≤3%
4. Y price drop from step 2 closes or crosses that gap
5. Protocol B's health factor check triggers → Position Z is liquidated
6. Z's liquidation sells more Y → further price depression
7. Repeat until no more positions exist within the cascade band

We short Y on Hyperliquid perps at step 2/3, targeting the cascade trigger price as the exit.

---

## Structural Mechanism — WHY This MUST Happen

This is not a tendency — it is a protocol rule. The mechanism is contractually enforced:

**Health factor formula (Aave v3 example):**
```
HF = Σ(collateral_i × price_i × liquidationThreshold_i) / totalDebt
```
When `HF < 1.0`, liquidation is permissionless and economically incentivised (liquidation bonus of 5–15% depending on asset). Any wallet holding the liquidation bonus token has a direct financial incentive to execute. The liquidation is not optional — it is a race condition where the first caller wins the bonus.

**Why the cascade is forced, not probabilistic:**
- Liquidation thresholds are immutable contract parameters (or governance-locked with timelocks)
- Health factor recalculates on every price oracle update (Chainlink heartbeat: 1% deviation or 1-hour heartbeat, whichever comes first)
- Liquidation bots run 24/7 and respond within seconds of oracle updates
- The only escape for a borrower is to add collateral or repay debt *before* the oracle updates — a narrow window

**Why the second-order effect is predictable:**
- All position data is public on-chain (no private order books)
- TheGraph subgraphs index every open borrow position with current collateral value, debt, and health factor
- The exact price at which each position becomes liquidatable is calculable: `liquidation_price = totalDebt / (collateral_amount × liquidationThreshold)`
- This means the "liquidation depth map" — a histogram of notional value at each price level — can be computed in real time

**The gap between first liquidation and cascade trigger is the tradeable window.** Once Y's price crosses the next cluster threshold, liquidation bots execute within 1–3 blocks (not seconds — we don't need to be faster than the bots, we need to be faster than the *price impact* of the cascade).

---

## Entry/Exit Rules

### Pre-computation (continuous background process)
- Maintain a live liquidation depth map for each supported collateral asset (ETH, WBTC, stETH, cbETH, LINK, UNI, ARB, OP — assets with Hyperliquid perp liquidity)
- For each asset, compute: `next_cluster_price`, `next_cluster_notional`, `current_spot`
- `gap_pct = (current_spot - next_cluster_price) / current_spot × 100`
- Update every block (or every 12 seconds for Ethereum mainnet)

### Entry Trigger (all conditions must be met simultaneously)
| Condition | Threshold | Rationale |
|---|---|---|
| Liquidation event confirmed | Collateral seized > $1M USD | Small liquidations don't move price enough |
| Asset has Hyperliquid perp | Must exist | Execution venue |
| Gap to next cluster | ≤ 3% below current spot | Price impact of first liquidation likely closes this |
| Next cluster notional | ≥ $500K | Cascade must be large enough to sustain price pressure |
| Hyperliquid funding rate | < +0.10% per 8h | Avoid paying excessive funding on short |
| Time since last cascade in this asset | > 4 hours | Avoid re-entering into exhausted liquidation landscape |

**Entry mechanics:**
- Enter short on Hyperliquid perp for asset Y
- Entry price: market order, immediately after liquidation event confirmed on-chain (target: within 30 seconds of liquidation tx)
- Do NOT wait for price to start moving — enter on the structural signal, not the price signal

### Exit Rules (first condition hit)
| Exit Condition | Action |
|---|---|
| Cascade confirmed: second-order liquidation event > $200K in Protocol B | Close 75% of position at market; trail stop on remainder |
| Price reaches `next_cluster_price` | Close 50% of position |
| Price recovers 2.0% above entry price | Full stop-loss exit |
| Position held > 4 hours without cascade trigger | Time-based exit at market |
| Funding rate exceeds +0.15% per 8h | Exit regardless of P&L |

**Partial exit logic:** On cascade confirmation, close 75% immediately (capture the mechanical move), hold 25% for potential further cascade tiers (check if a third cluster exists within 5% of cascade trigger price).

---

## Position Sizing

**Base sizing formula:**
```
position_notional = min(
    account_equity × 0.05,           # max 5% of equity per trade
    next_cluster_notional × 0.10,    # max 10% of the cascade notional
    $50,000                           # hard cap per trade
)
```

**Leverage:** 3–5x on Hyperliquid perps. Do not exceed 5x — cascade may not trigger, and a 2% stop-loss at 5x = 10% equity loss on a single trade.

**Scaling by gap size:**
- Gap 0–1%: Full position size (cascade almost certain to trigger)
- Gap 1–2%: 75% of base size
- Gap 2–3%: 50% of base size
- Gap > 3%: No trade

**Concentration limit:** Maximum 2 simultaneous positions across different assets. Cascades often correlate (risk-off events hit multiple collateral assets simultaneously).

---

## Backtest Methodology

### Data Sources
| Data | Source | URL/Endpoint |
|---|---|---|
| Aave v3 liquidation events | TheGraph | `https://thegraph.com/explorer/subgraphs/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL32` |
| Compound v3 liquidation events | TheGraph | `https://thegraph.com/explorer/subgraphs/AwoxEZbiWLvv6e3QdvdMZw4WDURdGbsvd67yCeej7e9` |
| Morpho Blue positions | Morpho API | `https://api.morpho.org/graphql` |
| Historical position health factors | Dune Analytics | Dashboards: `dune.com/queries/1575443` (Aave liquidations), `dune.com/queries/2390576` (Compound) |
| ETH/BTC/LINK spot prices | Chainlink historical | `https://data.chain.link/` or CoinGecko API |
| Hyperliquid perp price history | Hyperliquid API | `https://api.hyperliquid.xyz/info` (candles endpoint) |
| Hyperliquid funding rate history | Hyperliquid API | Same endpoint, `fundingHistory` method |

### Backtest Period
- **Primary:** January 2022 – December 2024 (covers LUNA crash, 3AC, FTX, March 2023 banking crisis, multiple ETH corrections)
- **Stress test:** May 2022 (LUNA cascade), November 2022 (FTX), August 2023 (ETH -10% in 24h)
- **Quiet market control:** Q4 2023 (low volatility, few liquidations) — expect low trade frequency, verify no false positives

### Backtest Construction Steps

1. **Reconstruct liquidation depth map** at each historical block using Aave/Compound/Morpho subgraph data. For each block, compute `gap_pct` and `next_cluster_notional` for each supported asset.

2. **Identify entry signals:** Filter for blocks where a liquidation event > $1M occurred AND `gap_pct ≤ 3%` AND `next_cluster_notional ≥ $500K`.

3. **Simulate entry:** Use Hyperliquid perp OHLCV data. Enter at the open of the next 1-minute candle after the liquidation block timestamp. Apply 0.05% slippage (Hyperliquid taker fee is 0.035%; add 0.015% for market impact).

4. **Simulate exit:** Apply exit rules in priority order using subsequent candle data. Record: exit reason, hold duration, P&L.

5. **Funding cost:** Subtract actual historical funding rates from Hyperliquid for the hold period.

6. **Cascade confirmation:** Check whether a second-order liquidation event > $200K occurred in the same asset within 4 hours of entry. This is the "cascade confirmed" exit trigger.

### Metrics to Compute
| Metric | Target | Kill threshold |
|---|---|---|
| Win rate | > 55% | < 45% |
| Average win / average loss | > 1.5 | < 1.0 |
| Sharpe ratio (annualised) | > 1.5 | < 0.8 |
| Max drawdown | < 20% | > 35% |
| Trade frequency | 2–15 per month | < 1/month (not worth infrastructure) |
| % trades where cascade confirmed | > 40% | < 25% |
| Average hold time | < 2 hours | N/A |

### Baseline Comparison
Compare against: (a) random short entry on same assets at same timestamps (no liquidation signal), (b) simple "short after any $1M+ liquidation regardless of gap" strategy. The structural signal must outperform both baselines on Sharpe and win rate.

---

## Go-Live Criteria (Paper Trading Gate)

All of the following must be satisfied before paper trading:

1. **Win rate ≥ 55%** across at least 50 historical trades
2. **Average win/loss ratio ≥ 1.5** (asymmetric payoff confirmed)
3. **Cascade confirmation rate ≥ 40%** — if cascades rarely trigger, the structural thesis is wrong
4. **Sharpe ≥ 1.5** on out-of-sample period (2024 data, held out from parameter tuning)
5. **No single event accounts for > 40% of total P&L** — strategy must not be a one-event wonder (e.g., LUNA crash)
6. **Infrastructure test:** Liquidation detection latency < 30 seconds from on-chain confirmation to Hyperliquid order submission, verified on testnet

Paper trading period: minimum 60 days or 20 live signals, whichever comes later.

---

## Kill Criteria

Abandon the strategy (paper or live) if any of the following occur:

| Condition | Action |
|---|---|
| 10 consecutive losses in paper trading | Halt, review structural thesis |
| Cascade confirmation rate drops below 20% over 30-trade rolling window | Protocol landscape has changed; rebuild position map |
| Average hold time exceeds 6 hours (cascades not triggering quickly) | Thesis broken — cascades are being absorbed, not amplified |
| Aave/Compound governance changes liquidation thresholds or introduces circuit breakers | Re-evaluate entire mechanism |
| Hyperliquid removes perp for a key collateral asset | Reduce universe, re-evaluate |
| Infrastructure latency exceeds 60 seconds consistently | Edge is gone; faster bots are front-running entry |
| Live trading Sharpe < 0.5 over 90-day rolling window | Strategy has decayed |

---

## Risks — Honest Assessment

### Critical Risks

**1. Speed disadvantage on entry (HIGH risk)**
Liquidation bots and MEV searchers have the same on-chain data and are faster. By the time we confirm the liquidation and submit a Hyperliquid order, the price may have already moved through the cascade trigger. Mitigation: we are not competing with liquidation bots — we are trading the *perp*, not the spot. Perp price may lag spot by seconds, giving a brief entry window. This must be validated empirically.

**2. Position top-up before cascade (MEDIUM risk)**
Borrowers monitoring their positions can add collateral or repay debt within the window between first liquidation and cascade trigger. Large sophisticated borrowers (DAOs, funds) often have automated health factor management. This reduces cascade probability for large, well-monitored positions. Mitigation: weight cascade probability by position age and wallet type (EOA vs. contract).

**3. Oracle latency and manipulation (MEDIUM risk)**
Chainlink oracles update on 1% deviation or 1-hour heartbeat. In fast-moving markets, spot price may cross the liquidation threshold before the oracle updates, meaning the cascade is delayed by up to 1 hour. Our short may be held longer than expected, accumulating funding costs. Mitigation: time-based exit at 4 hours caps this exposure.

**4. Liquidity exhaustion (MEDIUM risk)**
If the cascade is large, Hyperliquid perp liquidity may be insufficient to exit cleanly. For assets with < $5M daily perp volume on Hyperliquid, position size must be capped at $10K notional. Check Hyperliquid volume data before including any asset in the universe.

**5. Correlated cascade events (LOW-MEDIUM risk)**
During systemic risk events (exchange collapses, stablecoin depegs), cascades occur across all assets simultaneously. Our 2-position concentration limit helps, but a systemic event could trigger both positions at once in the wrong direction (if the cascade is so large it triggers a market-wide recovery bounce after initial drop). Mitigation: add a market-wide volatility filter — if BTC 1-hour realised vol > 5%, reduce position size by 50%.

**6. Regulatory/protocol risk (LOW risk)**
Aave, Compound, and Morpho governance can change liquidation parameters. Monitor governance forums (Tally, Snapshot) for proposals affecting liquidation thresholds or introducing pause mechanisms.

**7. "Our job is to help push it there" — market manipulation concern**
The original proposal noted we are "helping push" the price to the cascade trigger. To be explicit: we are NOT attempting to manipulate prices. We are taking a directional position based on a structural prediction. We do not have the capital to move prices on major assets. This distinction matters legally and practically.

---

## Data Sources — Complete Reference

| Source | Data | URL | Update frequency |
|---|---|---|---|
| TheGraph — Aave v3 Ethereum | Open positions, health factors, liquidation events | `https://gateway.thegraph.com/api/[key]/subgraphs/id/Cd2gEDVeqnjBn1hSeqFMitw8Q1iiyV9FYUZkLNRcL32` | Per block |
| TheGraph — Compound v3 | Same | `https://gateway.thegraph.com/api/[key]/subgraphs/id/AwoxEZbiWLvv6e3QdvdMZw4WDURdGbsvd67yCeej7e9` | Per block |
| Morpho Blue API | Positions, markets | `https://api.morpho.org/graphql` | Per block |
| Euler v2 | Positions | `https://app.euler.finance/api/` | Per block |
| Dune Analytics | Historical liquidation logs | `https://dune.com/queries/1575443` (Aave), `https://dune.com/queries/2390576` (Compound) | On-demand query |
| Chainlink Data Feeds | Oracle prices, historical | `https://data.chain.link/` | Per heartbeat |
| Hyperliquid REST API | Perp candles, funding history, order submission | `https://api.hyperliquid.xyz/info` | Real-time |
| Alchemy / Infura | Raw on-chain event logs (fallback) | `https://www.alchemy.com/` | Per block |
| CoinGecko API | Spot price history for backtesting | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` | 1-minute granularity (Pro) |
| Parsec Finance | Real-time DeFi liquidation monitoring (UI) | `https://parsec.finance/` | Real-time |
| Chaos Labs Risk Dashboard | Aave/Compound position concentration | `https://community.chaoslabs.xyz/aave/risk/overview` | Daily |

---

## Implementation Notes

**Minimum viable infrastructure:**
- WebSocket connection to Ethereum node (Alchemy/Infura) listening for `LiquidationCall` events on Aave, `AbsorbCollateral` on Compound, liquidation events on Morpho
- Background process rebuilding liquidation depth map every 60 seconds from subgraph queries
- On liquidation event: compute gap, check all entry conditions, submit Hyperliquid order if conditions met
- Separate process monitoring open positions for exit conditions

**Recommended stack:** Python (web3.py for on-chain events, httpx for subgraph queries, ccxt or Hyperliquid SDK for order execution). Single-threaded async event loop is sufficient — this is not HFT.

**Estimated infrastructure cost:** ~$200/month (Alchemy Growth plan for WebSocket + TheGraph hosted service queries).
