---
title: "Liquidation Cluster Oracle Lag Short"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 3
composite: 375
categories:
  - liquidation
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When ETH or BTC spot price crosses a dense liquidation threshold cluster on a major lending protocol (Aave v3, Compound v3, Morpho), but the Chainlink oracle price feed has not yet updated to reflect that crossing, a measurable window exists (typically 2–20 minutes) during which on-chain liquidations cannot fire. Once the oracle updates, forced selling from liquidation bots creates directional price pressure. Shorting the perp during the lag window — after market price has crossed the cluster but before the oracle catches up — positions us ahead of that forced selling.

**Causal chain (step by step):**

1. Users deposit collateral in round-number denominations (10 ETH, 1 BTC) and borrow round-number amounts (5,000 USDC, 20,000 USDC), causing liquidation thresholds to cluster at round-number price levels.
2. Chainlink price feeds update on a heartbeat schedule (every 3,600 seconds for ETH/USD on mainnet) OR when price deviation exceeds 0.5%, whichever comes first.
3. If price moves through a cluster level but the cumulative move since the last oracle update is <0.5%, the heartbeat governs — the oracle may lag by up to the remaining heartbeat window.
4. During this lag, liquidation contracts read the stale oracle price and cannot trigger, even though market price has already crossed the threshold.
5. When the oracle updates, liquidation bots (which monitor oracle updates in the same block) fire simultaneously, creating a burst of forced sell orders on spot/perp markets.
6. Shorting the perp during step 3–4 captures the price impact of step 5.

**What this is NOT:** This is not a prediction that price will fall because of chart patterns. The forced selling is mechanically guaranteed once the oracle updates — the only uncertainty is the magnitude of the price impact.

---

## Structural Mechanism

### Why oracle lag exists (and is measurable)

Chainlink ETH/USD on Ethereum mainnet uses a **1-hour heartbeat + 0.5% deviation threshold** (verifiable at `feeds.chain.link/eth-usd`). The last update timestamp is stored on-chain in the `latestRoundData()` function of the aggregator contract. This is public, real-time, and deterministic.

- If price has moved <0.5% since the last update, the next update is guaranteed to occur no later than `lastUpdateTimestamp + 3600 seconds`.
- The lag window is therefore: `3600 - (currentTime - lastUpdateTimestamp)` seconds, bounded by the 0.5% deviation trigger.

### Why liquidation clusters exist at round numbers

This is the **unproven assumption** in this strategy. The hypothesis is:

- Users set collateral deposits in round numbers (cognitive ease, UI defaults).
- Borrowing amounts are also round numbers (borrow exactly 10,000 USDC, not 9,847 USDC).
- Liquidation threshold = `collateral_value * LTV_ratio`. If collateral is a round ETH amount and borrow is a round USDC amount, the break-even price is `borrow_amount / (collateral_ETH * LTV)`, which often resolves to a round or near-round number.
- Example: 10 ETH collateral, 15,000 USDC borrowed, 80% LTV → liquidation at $15,000 / (10 × 0.80) = $1,875. Not perfectly round, but the *distribution* of these values should show peaks near $100 increments.

**This must be validated empirically before proceeding.** See Backtest Methodology.

### Why the forced selling is real

Aave v3 liquidation mechanics: when `healthFactor < 1`, any address can call `liquidationCall()` and receive a liquidation bonus (currently 5% for ETH). This creates a competitive market of liquidation bots that monitor oracle updates in the mempool and submit liquidation transactions in the same block as the oracle update. The selling pressure is real and has been documented in academic literature (e.g., "Liquidations: DeFi on a Knife-Edge," Qin et al., 2021).

---

## Entry Rules


### Pre-trade setup (run once daily, update continuously)

1. Pull all open borrowing positions on Aave v3 Ethereum, Compound v3, and Morpho Blue for ETH and WBTC collateral using TheGraph or direct RPC calls.
2. For each position, compute the liquidation price: `liquidation_price = debt_value / (collateral_amount * liquidation_threshold_ratio)`.
3. Aggregate into a liquidation density histogram with $10 price buckets.
4. Identify "cluster levels": any $10 bucket containing >$5M notional liquidatable value (threshold TBD from data).
5. Rank clusters by notional value. Focus on top 5 clusters within 10% of current market price.

### Entry trigger (all conditions must be met simultaneously)

| Condition | Specification |
|-----------|---------------|
| **C1: Price crossing** | Hyperliquid ETH-PERP or BTC-PERP mid-price has crossed a cluster level by ≥0.3% to the downside in the last 60 seconds |
| **C2: Oracle lag confirmed** | Chainlink `latestRoundData().updatedAt` timestamp is >20 minutes ago AND current price deviation from oracle price is <0.5% (i.e., deviation trigger has NOT been hit) |
| **C3: Cluster density** | The cluster being crossed contains ≥$10M notional liquidatable value (validate this threshold in backtest) |
| **C4: No recent oracle update** | No Chainlink update in the last 5 minutes (prevents entering after an update has already fired) |
| **C5: Perp funding neutral** | Funding rate on Hyperliquid is not >+0.05% per 8h (avoid paying excessive funding while waiting) |

**Entry:** Market short on Hyperliquid ETH-PERP or BTC-PERP at mid-price immediately upon all conditions met.

## Exit Rules

### Exit rules (first condition hit)

| Exit trigger | Action |
|--------------|--------|
| **E1: Oracle update confirmed** | Chainlink `updatedAt` changes to current block → hold for 3 minutes post-update to capture liquidation impact, then close at market |
| **E2: Stop-loss** | Perp price moves 1.0% against position (upward) from entry → close at market |
| **E3: Time stop** | Position open >45 minutes without oracle update → close at market (oracle update is overdue; something is wrong) |
| **E4: Deviation trigger fires early** | Price moves ≥0.5% from oracle price before heartbeat → oracle will update immediately; treat as E1 |

**Note on E1 timing:** The 3-minute hold after oracle update is a hypothesis. Liquidation bots fire in the same block as the oracle update, but the *market impact* of their selling may take 1–5 minutes to fully propagate. This window needs calibration in the backtest.

---

## Position Sizing

- **Base size:** 0.5% of portfolio per trade.
- **Rationale:** This is a high-frequency, short-duration trade with a narrow stop (1%). Expected win rate and magnitude are unknown pre-backtest. Small size limits ruin risk during hypothesis validation.
- **Maximum concurrent positions:** 1 (ETH or BTC, not both simultaneously — correlation risk).
- **Scale-up rule:** After 50 live paper trades with Sharpe >1.5, increase to 1% of portfolio.
- **No leverage beyond 3x** until backtest confirms edge. Oracle lag windows are short; slippage and fees can easily consume the edge at higher leverage.

---

## Backtest Methodology

### Phase 1: Validate the clustering hypothesis

**Goal:** Confirm that liquidation thresholds are non-uniformly distributed and cluster near round numbers.

**Data:**
- Aave v3 Ethereum: TheGraph subgraph `https://api.thegraph.com/subgraphs/name/aave/protocol-v3` — query `borrows` and `collateralDeposits` with timestamps.
- Morpho Blue: `https://api.morpho.org/graphql` — query open positions.
- Compound v3: `https://api.thegraph.com/subgraphs/name/graphprotocol/compound-v3` — query `positions`.

**Method:**
1. Pull all open positions as of a historical date (e.g., 2024-01-01).
2. Compute liquidation price for each position.
3. Plot histogram with $10 buckets.
4. Run a chi-squared test against a uniform distribution. If p < 0.05, clustering is real.
5. Specifically test: are there excess positions with liquidation prices at multiples of $100 vs. multiples of $50 vs. arbitrary prices?

**Pass criterion:** Statistically significant clustering (p < 0.05) with at least 3 identifiable clusters containing >$5M notional within 15% of any historical ETH price.

### Phase 2: Validate oracle lag windows

**Data:**
- Chainlink ETH/USD historical round data: query `AggregatorV3Interface` at `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` via Ethereum archive node or Dune Analytics.
- Dune query: `https://dune.com/queries/` — search "Chainlink ETH/USD updates" for pre-built queries.
- Hyperliquid historical OHLCV: `https://api.hyperliquid.xyz/info` endpoint `candleSnapshot`.

**Method:**
1. Pull all Chainlink ETH/USD update timestamps for 2023-01-01 to 2024-12-31.
2. Compute inter-update intervals. Identify all windows where gap >20 minutes AND price moved >0.3% during the window.
3. For each such window, check if price crossed a cluster level (from Phase 1) during the lag.
4. Count: how many tradeable events per month? (Frequency estimate.)

**Pass criterion:** ≥10 tradeable events per month historically. Fewer than that makes the strategy impractical.

### Phase 3: Measure price impact post-oracle-update

**Goal:** Quantify the actual price move on Hyperliquid perp in the 1–10 minutes following a Chainlink oracle update that crosses a cluster level.

**Method:**
1. For each oracle update event identified in Phase 2 that crossed a cluster level:
   - Record Hyperliquid ETH-PERP price at T=0 (oracle update block timestamp).
   - Record prices at T+1min, T+3min, T+5min, T+10min.
   - Compute return from T-5min (entry proxy) to each exit point.
2. Separate into: (a) events where cluster had >$10M notional, (b) events where cluster had $5–10M notional, (c) events with <$5M.
3. Compute median return, win rate, and Sharpe for each group.

**Baseline:** Compare against random 5-minute short entries during the same periods (same time of day, same market conditions). The strategy must outperform the baseline by a statistically significant margin.

**Key metrics:**
- Win rate (target: >55%)
- Median return per trade (target: >0.15% after fees)
- Sharpe ratio (target: >1.0 annualized)
- Maximum drawdown (kill if >5% of portfolio in backtest)
- Average holding time
- Number of trades (must be >30 for statistical validity)

**Fees assumption:** Hyperliquid taker fee = 0.035% per side = 0.07% round trip. This is the minimum hurdle per trade.

---

## Go-Live Criteria

All three must be satisfied before paper trading:

1. **Clustering confirmed:** Phase 1 chi-squared test passes (p < 0.05) with ≥3 identifiable clusters.
2. **Frequency sufficient:** ≥10 tradeable events per month in Phase 2 historical data.
3. **Edge confirmed:** Phase 3 shows median return >0.15% per trade after fees, win rate >55%, Sharpe >1.0, on ≥30 historical events, with statistically significant outperformance vs. baseline (p < 0.10).

Paper trade for minimum 60 days / 30 trades before any real capital deployment.

---

## Kill Criteria

Abandon the strategy (backtest or live) if any of the following occur:

| Condition | Action |
|-----------|--------|
| Phase 1 clustering test fails (p > 0.10) | Kill immediately — foundational assumption is wrong |
| Phase 2 yields <5 events/month historically | Kill — not enough frequency to be worth the infrastructure |
| Phase 3 median return <0.10% after fees | Kill — edge too thin to survive real-world slippage |
| Live paper trading: 20 consecutive losses | Kill — regime change or model failure |
| Live paper trading: drawdown >3% of portfolio | Kill — risk model is wrong |
| Chainlink announces migration to push-based feeds (no heartbeat) | Kill — structural mechanism disappears |
| Aave/Morpho migrate to Chainlink Low Latency feeds | Kill — oracle lag window collapses to seconds, untradeable |

---

## Risks

### Risk 1: Deviation trigger fires before heartbeat (HIGH probability)
If price moves ≥0.5% from the last oracle price, Chainlink updates immediately — no lag window. In volatile markets, this is the common case. The strategy only works when price drifts slowly through a cluster level. **Mitigation:** Condition C2 explicitly checks that deviation is <0.5%. But this means the strategy is only active in low-volatility drift conditions, which may be rare.

### Risk 2: Liquidation bots front-run the oracle update (HIGH probability)
Sophisticated liquidation bots monitor the Chainlink aggregator contract and may submit transactions in the same block as the oracle update, or even use flashbots bundles to atomically update + liquidate. If liquidation selling is fully absorbed in a single block, the perp price impact may be too brief to capture. **Mitigation:** This is a fundamental risk. Phase 3 backtest will reveal whether the price impact persists long enough to trade.

### Risk 3: Round-number clustering is weaker than assumed (MEDIUM probability)
Users may not actually cluster at round numbers as strongly as hypothesized. Institutional borrowers, smart contract vaults, and leveraged yield strategies may have arbitrary liquidation prices. **Mitigation:** Phase 1 empirically tests this. If clustering is weak, kill the strategy.

### Risk 4: Cluster data is stale (MEDIUM probability)
Positions change continuously. A cluster that existed when the map was built may have been partially liquidated or repaid by the time price approaches it. **Mitigation:** Rebuild the cluster map every 30 minutes using live TheGraph queries. Flag clusters where notional has decreased >20% since last update.

### Risk 5: Hyperliquid perp doesn't reflect spot liquidation pressure (LOW-MEDIUM probability)
Liquidations on Aave sell the collateral on Ethereum mainnet (Uniswap, 1inch), not on Hyperliquid. The price impact must propagate from Ethereum DEX markets to Hyperliquid perp via arbitrageurs. This propagation is fast (seconds to minutes) but not instantaneous. **Mitigation:** Phase 3 measures this empirically. If propagation is too slow or too noisy, consider trading Uniswap directly instead (though this adds complexity).

### Risk 6: Infrastructure latency (LOW probability given non-HFT constraint)
The oracle lag window is 2–20 minutes. This is not an HFT strategy — a 30-second execution delay is acceptable. Standard RPC node + Hyperliquid API is sufficient.

---

## Data Sources

| Data | Source | Endpoint/URL |
|------|---------|--------------|
| Chainlink ETH/USD oracle updates (live) | Ethereum RPC | Contract: `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`, function: `latestRoundData()` |
| Chainlink historical round data | Dune Analytics | `https://dune.com/` — query `chainlink.price_feeds` table |
| Aave v3 positions (live + historical) | TheGraph | `https://api.thegraph.com/subgraphs/name/aave/protocol-v3` |
| Morpho Blue positions | Morpho API | `https://api.morpho.org/graphql` |
| Compound v3 positions | TheGraph | `https://api.thegraph.com/subgraphs/name/graphprotocol/compound-v3` |
| Hyperliquid perp OHLCV (historical) | Hyperliquid API | `https://api.hyperliquid.xyz/info` → `candleSnapshot` |
| Hyperliquid perp live feed | Hyperliquid WebSocket | `wss://api.hyperliquid.xyz/ws` → subscribe `trades` |
| Aave liquidation events (historical) | TheGraph | Query `LiquidationCall` events on Aave v3 subgraph |
| ETH spot price (reference) | Binance REST | `https://api.binance.com/api/v3/klines?symbol=ETHUSDT` |

**Recommended archive node provider for Chainlink historical data:** Alchemy or Infura with archive access, or use Dune Analytics pre-indexed data to avoid archive node costs during backtest phase.

---

## Implementation Notes

### Cluster map update logic (pseudocode)
```
every 30 minutes:
    positions = fetch_all_positions(aave, morpho, compound)
    for each position:
        liq_price = position.debt_usd / (position.collateral_eth * position.liq_threshold)
        bucket = round(liq_price / 10) * 10  # $10 buckets
        cluster_map[bucket] += position.collateral_usd
    
    # Flag top clusters within 10% of current price
    current_price = get_hyperliquid_mid()
    active_clusters = [b for b in cluster_map 
                       if abs(b - current_price) / current_price < 0.10
                       and cluster_map[b] > MIN_CLUSTER_SIZE]
```

### Oracle lag monitor (pseudocode)
```
every 60 seconds:
    round_data = chainlink.latestRoundData()
    oracle_price = round_data.answer / 1e8
    oracle_age = current_time - round_data.updatedAt
    market_price = get_hyperliquid_mid()
    deviation = abs(market_price - oracle_price) / oracle_price
    
    if oracle_age > 1200:  # 20 minutes
        if deviation < 0.005:  # <0.5%, deviation trigger not hit
            check_cluster_crossing(market_price, oracle_price, active_clusters)
```

---

*This document is a hypothesis specification. No backtest has been run. All thresholds (cluster size, oracle age, deviation limits) are initial estimates requiring empirical calibration. Do not trade real capital until all go-live criteria are met.*
