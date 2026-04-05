---
title: "Aave V3 / Compound V3 Pre-Liquidation Overhang Short"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 5
composite: 625
categories:
  - liquidation
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large lending protocol position (>$500K notional) enters the critical health factor zone (1.00–1.05), the position creates a structural overhang on the collateral asset's price. The causal chain is:

1. **Position enters HF 1.00–1.05** → liquidation is imminent and mechanically inevitable unless the borrower tops up collateral or repays debt
2. **Liquidation bots race to execute** → they must sell the collateral they receive to realise the bonus, creating known future sell pressure
3. **Market participants who can read on-chain data** begin positioning short ahead of the forced sale
4. **Collateral asset experiences net downward pressure** in the blocks/minutes surrounding liquidation
5. **Post-liquidation, the overhang clears** → price mean-reverts as the forced seller (liquidator) has exited

The tradeable signal: **short the collateral asset on Hyperliquid perps when a large position enters HF < 1.05, cover when liquidation is confirmed on-chain.**

This is NOT a bet on "prices tend to fall before liquidations." It is a bet on a mechanically guaranteed forced sale of a known size at a known discount, creating predictable directional flow in a narrow window.

---

## Structural Mechanism — Why This MUST Happen

### The Liquidation Bonus Is Code-Enforced

Aave V3 liquidation logic (`LiquidationLogic.sol`):
- Liquidator repays up to 50% of the borrower's debt
- Liquidator receives collateral worth `debt_repaid × (1 + liquidationBonus)`
- `liquidationBonus` is set per-asset in the protocol's `ReserveConfiguration`: ETH = 5%, WBTC = 5–6.5%, long-tail assets = 10–15%
- This is not negotiable — the contract enforces it at execution

Compound V3 equivalent: `Comet.sol` `absorb()` function transfers collateral to the protocol at a discount, then `buyCollateral()` allows anyone to purchase it at a further discount (5–10% below oracle price).

### The Forced Sale Is Structurally Inevitable

Once HF < 1.0, the position **cannot** avoid liquidation without external intervention (borrower top-up or debt repayment). The liquidation bonus creates a race condition among bots — the first bot to execute captures the spread. The bot **must** then sell the collateral to realise the profit (they are not long-term holders; they are arbitrageurs). This sell pressure is:

- **Sized**: proportional to the position's collateral value
- **Timed**: concentrated in the blocks immediately following HF < 1.0
- **Directional**: always a sell of the collateral asset

### The Pre-Liquidation Overhang

Between HF entering 1.00–1.05 and actual liquidation:
- The position is a known, public, pending forced sale
- Any market participant reading the Aave subgraph or monitoring `getUserAccountData()` can see it
- This creates a game-theoretic short opportunity: if enough participants short, the collateral price drops further, accelerating the liquidation
- The overhang is **not priced in** because most market participants do not monitor on-chain health factors in real time

---

## Entry / Exit Rules

### Signal Generation

**Monitor continuously** (target: <30 second latency):
- Aave V3 on Ethereum mainnet: call `getReservesList()` then `getUserAccountData(user)` for all positions flagged by the subgraph as having `healthFactor` between 1.0 and 1.15
- Filter: `totalDebtBase > 500,000 USD` (using Aave's base currency, 8 decimals)
- Trigger: `healthFactor` crosses below 1.05

### Entry

| Parameter | Value |
|---|---|
| Trigger | HF < 1.05 AND notional debt > $500K |
| Instrument | Hyperliquid perp of the collateral asset (e.g., ETH-PERP, BTC-PERP) |
| Direction | SHORT |
| Entry timing | Market order within 1 block of HF trigger confirmation |
| Max entry slippage | 0.15% (reject if worse) |

**Do not enter if:**
- Position HF has been below 1.05 for >10 minutes without liquidation (bots may have already front-run the signal; overhang may be partially cleared)
- Collateral asset has already moved >1.5% down in the prior 5 minutes (signal may be stale)
- Funding rate on Hyperliquid perp is >0.05% per 8h in the short direction (carry cost too high)

### Exit

| Condition | Action |
|---|---|
| Liquidation confirmed on-chain (`LiquidationCall` event emitted) | Close 100% of position at market within 2 blocks |
| HF recovers above 1.15 (borrower topped up) | Close 100% at market immediately |
| Position held >4 hours without liquidation or recovery | Close 100% — thesis invalidated |
| Mark-to-market loss exceeds 1.5× expected move (see sizing) | Stop loss — close immediately |

---

## Position Sizing

### Base Sizing Formula

```
position_size_USD = min(
    collateral_notional_USD × 0.02,   # 2% of the overhang
    max_position_cap_USD               # hard cap
)
```

**Rationale for 2%:** The liquidation bonus creates ~5% forced sell pressure on the collateral. We expect to capture 20–40% of that move (1–2%). Sizing at 2% of notional means a 1% move in our favour = 50% of the overhang size, which is realistic for large liquid assets.

### Hard Caps

| Account size | Max single position | Max concurrent positions |
|---|---|---|
| $100K | $10K | 3 |
| $500K | $40K | 5 |
| $1M | $75K | 5 |

### Leverage

- Target: 3–5× on Hyperliquid perp
- Never exceed 5× — this is a flow trade, not a conviction trade
- If Hyperliquid margin requirement forces >5×, reduce position size

---

## Backtest Methodology

### Data Sources

| Data | Source | URL / Endpoint |
|---|---|---|
| Aave V3 liquidation events | TheGraph hosted service | `https://api.thegraph.com/subgraphs/name/aave/protocol-v3` — query `LiquidationCall` events |
| Aave V3 health factor history | Aave V3 subgraph `UserReserve` entity + `getReserveData` | Same subgraph, reconstruct HF from `currentATokenBalance`, `currentVariableDebt`, asset prices |
| Compound V3 absorb events | Etherscan event logs | `Comet.sol` `AbsorbCollateral` event, topic `0x...` on mainnet |
| ETH/BTC/altcoin OHLCV (1-min) | Hyperliquid historical data API | `https://api.hyperliquid.xyz/info` — `candleSnapshot` endpoint |
| Aave oracle prices (historical) | Chainlink price feed archives or Dune Analytics | `https://dune.com/queries` — Chainlink `AnswerUpdated` events |

### Reconstruction Approach

1. Pull all `LiquidationCall` events from Aave V3 mainnet deployment (block 16291127 onward, ~Jan 2023)
2. For each liquidation, record: `collateralAsset`, `debtAsset`, `liquidatedCollateralAmount`, `debtToCover`, `block_number`, `timestamp`
3. Filter: `liquidatedCollateralAmount × collateral_price_at_block > $500K`
4. For each qualifying event, reconstruct the HF trajectory in the 60 minutes prior using subgraph data
5. Identify the block where HF first crossed below 1.05
6. Pull 1-minute OHLCV for the collateral asset from Hyperliquid (or Binance as proxy if Hyperliquid data is sparse pre-2023) for the window: `[HF_cross_block - 5min, liquidation_block + 30min]`
7. Simulate entry at HF < 1.05 cross, exit at liquidation confirmation block

### Metrics to Compute

| Metric | Target | Notes |
|---|---|---|
| Win rate | >55% | Each trade = one qualifying liquidation event |
| Median return per trade | >0.3% (gross) | Before funding and fees |
| Mean return per trade | >0.2% | Skew check — outliers shouldn't drive the result |
| Max drawdown (per trade) | <2% | On position notional |
| Sharpe (annualised) | >1.5 | Use daily P&L aggregation |
| Average hold time | <2 hours | Longer = thesis not working |
| % trades exited via stop (not liquidation) | <30% | If >30%, pre-liquidation signal is too noisy |

### Baseline Comparison

- **Null hypothesis**: short the collateral asset at a random time (same asset, same time of day, same duration) — if our signal doesn't beat random shorts, it has no edge
- **Secondary baseline**: short immediately after liquidation confirmation (post-event, not pre-event) — tests whether the pre-event signal adds value over simply reacting

### Stratification

Run separately for:
- ETH collateral vs. BTC collateral vs. altcoin collateral (expect different results)
- Position size buckets: $500K–$2M, $2M–$10M, >$10M
- Market regime: trending down vs. ranging vs. trending up (HF creep happens differently in each)

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Win rate ≥ 55%** across ≥ 50 qualifying events in backtest
2. **Median gross return ≥ 0.3%** per trade (must cover ~0.1% Hyperliquid fees + expected funding)
3. **Backtest Sharpe ≥ 1.5** on daily P&L
4. **% stopped out < 30%** — if the position frequently hits stop before liquidation, the pre-liquidation signal is too noisy
5. **Result holds across ETH and BTC collateral** — if it only works on altcoins, the edge is too thin and liquidity-constrained
6. **No single event drives >25% of total backtest P&L** — concentration check

If criteria are met: move to **paper trading for 30 days** with real-time on-chain monitoring but no real capital.

---

## Kill Criteria

### Kill during backtest

- Win rate < 50% across ≥ 50 events
- Median return < 0.1% (insufficient to cover costs)
- Result is entirely driven by 1–3 large liquidation events (not robust)
- Pre-liquidation signal adds no value over post-liquidation entry (baseline beats us)

### Kill during paper trading

- Paper trading win rate < 50% over ≥ 20 live events
- Average slippage on Hyperliquid entry > 0.2% (signal latency too high)
- >40% of signals result in borrower top-up (HF recovery) rather than liquidation — the signal is triggering too early
- A competing bot is demonstrably front-running our entry (we observe price moving >0.5% in our direction within 1 block of HF trigger, before our order)

### Kill in live trading

- 3 consecutive losing months
- Sharpe drops below 0.8 on rolling 90-day basis
- Aave or Compound changes liquidation bonus parameters materially (re-evaluate from scratch)

---

## Risks — Honest Assessment

### High Severity

| Risk | Description | Mitigation |
|---|---|---|
| **Borrower rescue** | Borrower tops up collateral before liquidation; HF recovers; our short loses | Exit rule: close if HF > 1.15. Accept this as a cost of doing business (~20–30% of signals may resolve this way) |
| **Bot front-run on price** | Liquidation bots also short the perp pre-liquidation; by the time we enter, the move is already in | Check: if >50% of the expected move occurs in the 60 seconds before our entry, signal is too slow. Kill criterion. |
| **Thin Hyperliquid liquidity for altcoin perps** | For non-ETH/BTC collateral, Hyperliquid perp may have insufficient depth for our position size | Restrict to ETH-PERP and BTC-PERP initially; expand only if altcoin perps show >$5M daily volume |

### Medium Severity

| Risk | Description | Mitigation |
|---|---|---|
| **Funding rate carry** | If market is heavily short, funding rate on Hyperliquid perp may erode returns | Entry filter: reject if funding > 0.05%/8h short-side |
| **Oracle lag** | Aave uses Chainlink oracles; if spot price drops but oracle hasn't updated, HF appears higher than reality — liquidation happens suddenly | This actually helps us: sudden liquidation = sharper price impact = better trade. Not a risk, potentially a feature. |
| **Subgraph latency** | TheGraph subgraph may lag by 1–3 blocks | Use direct RPC calls to `getUserAccountData()` for real-time monitoring; subgraph only for historical backtest |
| **Cross-chain fragmentation** | Aave V3 exists on Arbitrum, Optimism, Polygon — liquidations there may not impact ETH mainnet price | Initially restrict to Ethereum mainnet Aave V3 only; largest positions, most liquid collateral |

### Low Severity

| Risk | Description | Mitigation |
|---|---|---|
| **Strategy crowding** | Other quant funds discover same signal | Monitor: if win rate degrades over time, crowding is likely. Kill criterion handles this. |
| **Gas costs** | Irrelevant — we are trading perps, not executing liquidations | N/A |
| **Regulatory** | Perpetual futures regulatory risk on Hyperliquid | Standard operational risk; not strategy-specific |

---

## Data Sources — Specific Endpoints

```
# Aave V3 Subgraph (Ethereum mainnet)
https://api.thegraph.com/subgraphs/name/aave/protocol-v3

# Example GraphQL query — recent large liquidations
{
  liquidationCalls(
    where: { debtAssetPriceUSD_gt: "500000" }
    orderBy: timestamp
    orderDirection: desc
    first: 1000
  ) {
    id
    collateralAsset { symbol }
    debtAsset { symbol }
    liquidatedCollateralAmount
    debtToCover
    timestamp
    blockNumber
    user { id }
  }
}

# Aave V3 getUserAccountData — real-time RPC
# Contract: 0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2 (Aave V3 Pool, Ethereum)
# Function: getUserAccountData(address user) returns (totalCollateralBase, totalDebtBase, 
#           availableBorrowsBase, currentLiquidationThreshold, ltv, healthFactor)

# Compound V3 Comet (USDC market, Ethereum)
# Contract: 0xc3d688B66703497DAA19211EEdff47f25384cdc3
# Event: AbsorbCollateral(address indexed absorber, address indexed borrower, 
#        address indexed asset, uint collateralAbsorbed, uint usdValue)

# Hyperliquid historical candles
POST https://api.hyperliquid.xyz/info
{
  "type": "candleSnapshot",
  "req": {
    "coin": "ETH",
    "interval": "1m",
    "startTime": <unix_ms>,
    "endTime": <unix_ms>
  }
}

# Dune Analytics — pre-built Aave liquidation dashboards
https://dune.com/queries/1174220  # Aave V3 liquidations
https://dune.com/queries/2390576  # Large position health factors

# Etherscan API — LiquidationCall event logs
https://api.etherscan.io/api?module=logs&action=getLogs
  &address=0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2
  &topic0=0xe413a321e8681d831f4dbccbca790d2952b56f977908e45be37335533e005286
  &apikey=<KEY>
```

---

## Implementation Notes

### Monitoring Stack (for paper trading phase)

- **RPC provider**: Alchemy or Infura with WebSocket subscription to `LiquidationCall` events
- **Health factor polling**: Poll `getUserAccountData()` every block (~12s) for all positions flagged by subgraph as HF < 1.15
- **Alert latency target**: < 30 seconds from HF trigger to order submission
- **Order execution**: Hyperliquid Python SDK (`hyperliquid-python-sdk`) — market order with slippage tolerance

### What This Strategy Is NOT

- It is **not** a liquidation bot (we do not execute liquidations)
- It is **not** an HFT strategy (30-second latency is acceptable)
- It is **not** a directional macro bet on ETH/BTC — positions are sized to be in and out within hours, not days

### Open Questions for Backtest Phase

1. What fraction of HF < 1.05 signals result in liquidation vs. recovery? (Determines signal purity)
2. What is the median time between HF < 1.05 and liquidation confirmation? (Determines hold time distribution)
3. Does the pre-liquidation price move scale linearly with position size, or is there a threshold effect?
4. Is the signal stronger during high-volatility regimes (when HF creep is fast) vs. slow-moving markets?
