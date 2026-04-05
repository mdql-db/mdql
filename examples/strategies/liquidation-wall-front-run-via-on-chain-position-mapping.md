---
title: "Liquidation Wall Front-Run via On-Chain Position Mapping"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 5
composite: 750
categories:
  - liquidation
  - defi-protocol
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When the spot price of a collateral asset approaches a large aggregated liquidation wall on a DeFi lending protocol, the subsequent collateral seizure and forced market sale is contractually guaranteed by the protocol's smart contract logic. The sell pressure from liquidation bots dumping seized collateral creates a predictable, front-runnable price impact. A short position entered before price reaches the wall captures this structural sell flow. The edge is NOT that price will reach the wall — that remains a directional bet — but that IF price reaches the wall, the liquidation cascade and its downward price impact are mechanically certain.

---

## Structural Mechanism

### Why liquidations MUST happen

Aave v3 (and equivalent protocols: Compound v3, Spark, Morpho) enforce liquidation via immutable smart contract logic. When a position's Health Factor drops below 1.0, any external address can call `liquidationCall()` and receive a liquidation bonus of 5–15% of the seized collateral value. This bonus is funded by the borrower's collateral. The bonus creates a guaranteed profit for liquidation bots, meaning competitive bots will execute liquidations within 1–3 blocks of HF crossing 1.0. There is no scenario where a large wall is reached and liquidations do not occur — the economic incentive is too large and the competition too fierce.

### Why liquidations create sell pressure

Liquidation bots typically operate one of two strategies:
1. **Atomic arb**: Seize collateral, swap to repay debt asset in same transaction — net effect is collateral sold into spot market.
2. **Hold and sell**: Seize collateral, sell separately — creates delayed but certain sell pressure within minutes.

Both strategies result in the collateral asset being sold. For ETH or BTC collateral, this sale hits CEX spot markets and DEX pools simultaneously, creating measurable downward price impact proportional to the wall size.

### The cascade mechanic

Liquidations reduce collateral value for adjacent positions. A $100m ETH liquidation wall at $2,000 does not exist in isolation — positions with liquidation prices at $1,980, $1,960, etc. become newly at-risk as the first wall is hit. This cascade structure is observable in advance from on-chain data. The cascade is not guaranteed (price may recover), but the structure of the cascade is fully computable before it begins.

### Why this is not fully priced in

Perpetual futures funding rates and order books do not systematically incorporate on-chain liquidation wall data. Most market participants use price charts, not protocol subgraph queries. The information exists publicly but requires non-trivial engineering to aggregate, creating a temporary information asymmetry that is structural (data is always public) rather than speed-based.

---

## Entry Rules

### Prerequisites (all must be true)

1. **Wall size**: Aggregated liquidatable notional within a 3% price band below current spot ≥ $50m for ETH, ≥ $30m for BTC. Compute across Aave v3 mainnet, Aave v3 Arbitrum, Compound v3 mainnet, Spark Protocol.
2. **Directional filter**: 4-hour price change is negative (price is moving toward the wall, not away from it). Use Binance ETHUSDT or BTCUSDT as price reference.
3. **Distance filter**: Current spot price is within 2–5% above the top of the liquidation wall. Entry too early (>5%) means excessive time exposure; entry too late (<2%) means liquidation bots may already be front-running.
4. **Funding rate filter**: Hyperliquid ETH-PERP or BTC-PERP funding rate is not strongly positive (>0.05% per 8h). Strongly positive funding means the market is already heavily short and the edge may be crowded or the bounce risk is elevated.
5. **No recent cascade**: No liquidation cascade of >$20m has occurred in the prior 2 hours on the same asset. Post-cascade, walls are partially cleared and the structural setup is degraded.

### Entry execution

- **Instrument**: Hyperliquid ETH-PERP or BTC-PERP (short)
- **Entry price**: Market order at next available price after all prerequisites confirmed
- **Check frequency**: Run prerequisite scan every 15 minutes using automated script

---

## Exit Rules

### Take profit (TP)

- **TP1**: Close 50% of position when price moves through the center of the liquidation wall (i.e., price has dropped into the wall and liquidations are actively occurring). Target: wall midpoint price.
- **TP2**: Close remaining 50% when price drops an additional 1.5% below wall bottom OR when on-chain liquidation volume for the session exceeds 80% of the estimated wall notional (confirming the wall is cleared).

### Stop loss (SL)

- **Hard stop**: 1.5% adverse move from entry price (price moves up, away from wall). This is a mechanical stop, no discretion.
- **Time stop**: If price has not moved into the wall within 8 hours of entry, close the position at market regardless of P&L. The directional setup has failed and time decay on the thesis increases risk.

### Invalidation exit (before stop is hit)

- Close immediately if: a large buyer (>$10m) appears in the Aave position data (borrower adds collateral, repairing HF) AND the wall notional drops below $30m. The structural setup has been removed.
- Close immediately if: funding rate spikes above 0.08% per 8h (crowding signal — the trade is no longer structural, it is momentum).

---

## Position Sizing

### Base sizing

- Risk 0.5% of total portfolio per trade (this is a 6/10 hypothesis, not a confirmed edge — size conservatively until backtested).
- With a 1.5% hard stop, a 0.5% portfolio risk implies position size = 0.5% / 1.5% = **33% of portfolio in notional exposure**.
- Cap maximum notional at $500k until live performance data exists, regardless of portfolio size.

### Scaling rules

- Do NOT scale up based on wall size alone. A $500m wall does not mean 10x the position — it means the cascade risk is higher but so is the bounce risk from forced buyers (protocols may pause, large holders may defend).
- Scale down to 0.25% portfolio risk if: funding rate is between 0.02–0.05% per 8h (mild crowding), or if the wall is concentrated in a single protocol (single-protocol walls are more vulnerable to governance pause).

### Leverage

- Use 3–5x leverage on Hyperliquid. Higher leverage is not warranted given the directional uncertainty of whether price reaches the wall.

---

## Backtest Methodology

### Data requirements

| Data Source | What to Pull | Format |
|---|---|---|
| Aave v3 Subgraph (TheGraph) | All borrow positions, collateral amounts, debt amounts, asset prices at each block | GraphQL queries, hourly snapshots |
| Compound v3 Subgraph | Same as above | GraphQL queries |
| Binance historical OHLCV | ETH/USDT, BTC/USDT, 15-minute candles | CSV via Binance API |
| Hyperliquid historical funding | ETH-PERP, BTC-PERP 8h funding rates | CSV via Hyperliquid API |
| Etherscan/Alchemy | Historical `LiquidationCall` events with block timestamps and amounts | RPC event logs |

### Reconstruction methodology

1. **Build position snapshots**: For each hour from Jan 2023 to present, reconstruct all open Aave v3 positions using subgraph data. Compute Health Factor for each position using the price at that hour.
2. **Compute liquidation walls**: For each hour, aggregate positions by their liquidation price into $100 price buckets. Record the total notional liquidatable at each price level.
3. **Identify historical setups**: Find all hours where spot price was within 2–5% of a ≥$50m wall AND 4h price change was negative. These are your historical entry signals.
4. **Simulate trades**: For each signal, apply entry/exit rules mechanically. Use Binance 15-minute OHLCV to determine when stops, TPs, and time stops would have been hit.
5. **Validate with actual liquidation events**: Cross-reference simulated entries with actual `LiquidationCall` events from Etherscan. Confirm that large walls did produce large liquidation events when price reached them.
6. **Measure slippage**: Hyperliquid historical order book data (if available) or estimate 0.05% market impact per $100k notional as a conservative assumption.

### Key metrics to compute

- **Win rate**: % of trades where price reached the wall before the stop was hit
- **Average R**: Average profit in units of risk (1R = 1.5% stop distance)
- **Wall hit rate**: Of all setups where price was within 5% of wall, what % actually reached the wall within 8 hours?
- **Cascade confirmation rate**: Of all wall hits, what % produced measurable liquidation volume (>50% of wall notional liquidated)?
- **False positive rate**: How often does a wall ≥$50m dissolve (borrowers add collateral) before price reaches it?

### Minimum backtest standard

- Minimum 30 qualifying setups before drawing conclusions
- Test on ETH and BTC separately — do not aggregate
- Test across at least two distinct market regimes: a sustained downtrend (e.g., May–Nov 2022) and a choppy/ranging market (e.g., Q1 2024)

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. Backtest shows positive expectancy (average R > 0.3) across ≥30 setups on ETH
2. Wall hit rate ≥ 40% (price reaches wall within 8h in at least 4 of 10 setups)
3. Cascade confirmation rate ≥ 70% (when price hits wall, liquidations actually occur at scale)
4. False positive rate (wall dissolves before price arrives) ≤ 30%
5. Paper trade for minimum 4 weeks with ≥5 live setups observed, tracking all entry/exit rules mechanically
6. Paper trade results within 20% of backtest expectancy (confirms no major data-snooping artifact)
7. Automated monitoring pipeline is live and tested: subgraph queries running every 15 minutes, alert system functional, no manual steps required for signal detection

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

1. **Live drawdown**: 3 consecutive losing trades OR total strategy drawdown exceeds 3% of portfolio
2. **Protocol change**: Aave governance votes to change liquidation bonus, liquidation threshold, or introduces a circuit breaker — the structural mechanism has changed
3. **Crowding signal**: A public tool or dashboard (e.g., DefiLlama liquidation heatmap) begins tracking the exact same metric with wide adoption — the information asymmetry is gone
4. **Execution failure**: Hyperliquid experiences downtime or liquidity issues during a live setup — the instrument is unreliable for this strategy
5. **Backtest invalidation**: Post-live data shows wall hit rate <25% over 20+ setups — the directional component is too weak to support the strategy

---

## Risks

### Risk 1: Price does not reach the wall (primary risk, score impact)
The wall is a structural catalyst, not a price magnet. A 2% bounce from a buyer, a positive macro catalyst, or a large borrower adding collateral can invalidate the setup entirely. **Mitigation**: Hard stop at 1.5%, time stop at 8h, position sizing at 0.5% portfolio risk.

### Risk 2: Wall dissolves before price arrives
Borrowers can add collateral, repay debt, or be liquidated by smaller price moves before the main wall is reached. The $50m wall you observed 2 hours ago may be $15m by the time price arrives. **Mitigation**: Re-check wall size every 15 minutes and exit if wall drops below $30m. Build a "wall decay rate" metric into the backtest.

### Risk 3: Liquidation bots front-run more aggressively than expected
If bots begin liquidating positions before HF reaches exactly 1.0 (e.g., via MEV strategies that anticipate price moves), the sell pressure may arrive earlier and smaller than the wall data suggests. **Mitigation**: Track actual liquidation timing relative to HF in backtest — if bots are front-running by >0.5% price, adjust entry trigger accordingly.

### Risk 4: Protocol pause or governance intervention
Aave has an emergency pause mechanism. In extreme market conditions (e.g., oracle failure, governance attack), liquidations can be paused. This would remove the structural guarantee entirely. **Mitigation**: Monitor Aave governance forums and Chaos Labs risk dashboards. Exit immediately if a pause is announced. Accept this as a tail risk that cannot be fully hedged.

### Risk 5: Oracle lag creates phantom walls
Aave uses Chainlink oracles with heartbeat updates (not tick-by-tick). During fast moves, the on-chain price used for HF calculation may lag spot by 30–120 seconds. This means the "wall" may be hit on spot before the protocol recognizes it, creating a brief window where liquidations are delayed. **Mitigation**: Use the Chainlink oracle price (not Binance spot) as the reference price for wall distance calculations. Pull oracle price directly from the Chainlink aggregator contract.

### Risk 6: Cross-protocol contagion is not captured
This strategy monitors Aave v3 and Compound v3 but not all lending protocols. Euler v2, Morpho vaults, and newer protocols may have significant additional walls not captured in the aggregation. **Mitigation**: Expand protocol coverage before go-live. Treat the $50m threshold as a floor — actual liquidatable notional may be higher.

### Risk 7: Hyperliquid-specific execution risk
Hyperliquid is a relatively new venue. Liquidity for large ETH-PERP or BTC-PERP positions may be insufficient during fast market moves (exactly when this strategy is most active). **Mitigation**: Cap position size at $500k notional. Test order book depth before each entry. Use limit orders within 0.05% of mid if possible.

---

## Data Sources

| Source | URL / Access Method | Cost | Latency |
|---|---|---|---|
| Aave v3 Subgraph (mainnet) | `https://thegraph.com/explorer/` — search "Aave V3" | Free (rate limited) | ~5 min lag |
| Aave v3 Subgraph (Arbitrum) | TheGraph hosted service | Free | ~5 min lag |
| Compound v3 Subgraph | TheGraph hosted service | Free | ~5 min lag |
| Spark Protocol Subgraph | TheGraph hosted service | Free | ~5 min lag |
| Chainlink ETH/USD Oracle | Contract: `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` — read `latestRoundData()` | Free (RPC calls) | ~13s (1 block) |
| Binance historical OHLCV | `https://api.binance.com/api/v3/klines` | Free | Real-time |
| Hyperliquid funding rates | `https://api.hyperliquid.xyz/info` | Free | Real-time |
| Etherscan LiquidationCall events | `https://api.etherscan.io/api` — `getLogs` for Aave Pool contract | Free (rate limited) / $25/mo for higher limits | ~1 block |
| Alchemy / Infura RPC | Direct contract reads | Free tier / ~$50/mo for production | ~1 block |
| DefiLlama liquidation data | `https://defillama.com/liquidations` | Free | ~15 min lag |

---

## Implementation Notes

### Minimum viable pipeline (build order)

1. **Week 1**: Write subgraph queries to pull all Aave v3 positions. Validate against DefiLlama liquidation heatmap as a sanity check.
2. **Week 2**: Build position aggregator — compute HF for each position using Chainlink oracle price, bucket by liquidation price, sum notional per bucket.
3. **Week 3**: Build signal detector — run aggregator every 15 minutes, alert when wall ≥$50m is within 2–5% of spot AND 4h trend is negative.
4. **Week 4**: Backtest using historical subgraph snapshots and Binance OHLCV. Compute all metrics listed above.
5. **Week 5–8**: Paper trade. Log every signal, every entry decision, every exit.
6. **Week 9+**: Go-live decision based on go-live criteria above.

### Monitoring dashboard (minimum viable)

- Current liquidation wall map for ETH and BTC (updated every 15 minutes)
- Distance from spot to nearest ≥$50m wall
- Wall notional over time (track decay rate)
- Active trade P&L and distance to stop/TP
- Funding rate for ETH-PERP and BTC-PERP

---

*This document is a hypothesis specification. No backtest has been run. No live performance data exists. All claims about mechanism are based on protocol documentation and smart contract logic, not empirical validation. Do not allocate capital until go-live criteria are satisfied.*
