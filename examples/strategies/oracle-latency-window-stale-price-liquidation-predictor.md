---
title: "Oracle Latency Window — Stale Price Liquidation Predictor"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 5
composite: 750
categories:
  - liquidation
  - lending
  - defi-protocol
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Chainlink oracle prices lag spot by more than 1%, a computable set of DeFi lending positions (primarily Aave v2/v3 on Ethereum) are in a "shadow liquidation zone" — they are economically insolvent at current spot prices but not yet liquidatable because the protocol's internal price hasn't updated. When the oracle next updates (either via heartbeat or deviation trigger), these positions become liquidatable simultaneously, creating a concentrated, predictable collateral dump within a narrow time window. This dump causes a short-duration price overshoot below fair value, followed by mean reversion. The edge is not speed — it is *knowing in advance which oracle update will trigger a material liquidation batch*, and sizing a short position before that update fires.

The structural guarantee: Chainlink's deviation threshold (0.5% for ETH/USD) and heartbeat (1 hour) are immutable contract parameters. The oracle MUST update when either condition is met. The liquidation math on Aave is deterministic and public. The set of positions that will be liquidated is computable from on-chain state before the oracle fires. This is not a pattern — it is a mechanical sequence with a known trigger.

---

## Structural Mechanism

### The Oracle Update Cycle

Chainlink ETH/USD (and most major asset feeds) on Ethereum mainnet operate under two update conditions, both encoded in the aggregator contract:

1. **Heartbeat:** The feed updates unconditionally every ~3,600 seconds (1 hour), regardless of price movement.
2. **Deviation threshold:** The feed updates immediately if the off-chain Chainlink node network detects that the current answer deviates from the last on-chain answer by ≥0.5%.

The last on-chain update timestamp and price are readable from `latestRoundData()` on the aggregator contract at zero cost via any Ethereum RPC node.

### The Shadow Liquidation Zone

Aave v2/v3 computes Health Factor (HF) as:

```
HF = Σ(collateral_i × liquidation_threshold_i × oracle_price_i) / total_debt_USD
```

A position becomes liquidatable when HF < 1.0. Because `oracle_price_i` is sourced from Chainlink, a position's HF is computed against the *last on-chain Chainlink price*, not live spot. During a rapid spot price decline:

- Spot price falls (visible on Binance/Coinbase in real time)
- Chainlink on-chain price has not yet updated (deviation < 0.5% from last update, or heartbeat not yet elapsed)
- Positions that would have HF < 1.0 at spot price have HF > 1.0 at Chainlink price
- These positions are **economically insolvent but protocol-protected** — they cannot be liquidated yet

This shadow zone is computable: pull all open Aave positions from TheGraph, apply current spot price to each position's collateral, identify all positions where HF(spot) < 1.0 but HF(chainlink) ≥ 1.0. The aggregate USD value of collateral in this zone is the **Pending Liquidation Volume (PLV)**.

### The Trigger and Dump

When the oracle updates (deviation threshold breached or heartbeat fires), all shadow-zone positions simultaneously become liquidatable. Liquidation bots (MEV searchers) race to liquidate these positions, receiving a liquidation bonus (5-8% on Aave). The liquidated collateral (ETH, wBTC, etc.) is sold into spot/perp markets to repay debt. This creates a concentrated, non-discretionary sell flow in a window of approximately 1-5 minutes post-oracle-update.

The sell flow is:
- **Non-discretionary:** Liquidation bots are profit-maximizing and will liquidate every eligible position immediately
- **Concentrated:** All positions become eligible at the same oracle update tick
- **Computable in advance:** The exact positions and collateral amounts are on-chain before the oracle fires

Post-liquidation, the forced sell pressure dissipates and price reverts toward fair value. The reversion trade (long) is the higher-probability leg because the dump is mechanical and temporary.

### Why This Is Not Pure HFT

The pre-oracle short entry does not require being first to liquidate. It requires:
1. Identifying that a material PLV exists (computable minutes to hours in advance)
2. Entering a short on Hyperliquid ETH-PERP before the oracle fires
3. Exiting the short and entering a long within minutes of the oracle update

The entry window is not milliseconds — it is the time between "deviation is approaching 0.5%" and "deviation threshold is breached." During a 5% spot drop, this window may be 2-10 minutes as price slides through the threshold. The exit timing is tighter (1-5 min post-update) but does not require on-chain execution — it requires closing a Hyperliquid perp position, which is a CEX-like operation.

---

## Entry Rules

### Pre-Conditions (all must be true)

1. **Oracle lag delta ≥ 0.8%:** `(chainlink_price - spot_price) / spot_price ≥ 0.008` (oracle is lagging a falling market). Monitor every 30 seconds via RPC call to `latestRoundData()` on the ETH/USD Chainlink aggregator (`0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`).

2. **Pending Liquidation Volume ≥ $5M USD:** Compute PLV from Aave v3 position data (TheGraph query, refreshed every 5 minutes). Below $5M, the collateral dump is insufficient to move ETH price materially on Hyperliquid.

3. **Spot price is still falling or flat:** Confirm via 30-second Binance ETH/USDT OHLCV that the move is not already reversing. A reverting spot price means the oracle may catch up without triggering a large liquidation batch.

4. **Time to next heartbeat ≥ 5 minutes:** If the heartbeat is imminent (< 5 min), the oracle will update regardless of deviation — this is still a valid trigger, but the entry window is compressed. Compute as `3600 - (current_timestamp - last_update_timestamp)`.

5. **Hyperliquid ETH-PERP funding rate is not extreme positive (< +0.05% per 8h):** Extreme positive funding means longs are paying shorts heavily, which may indicate the market is already positioned short and the dump is partially priced.

### Entry Signal

When all pre-conditions are met:

- **Instrument:** ETH-PERP on Hyperliquid (or BTC-PERP if the oracle lag is on BTC/USD feed)
- **Direction:** SHORT
- **Entry price:** Market order at current Hyperliquid mid-price
- **Entry timing:** Enter as soon as oracle delta crosses 0.8% threshold, not when it crosses 0.5% (give yourself buffer before the oracle fires)

### Entry Size

See Position Sizing section.

---

## Exit Rules

### Short Exit (Post-Oracle Update)

- **Primary trigger:** Detect oracle update via RPC polling (`latestRoundData()` returns a new `updatedAt` timestamp). Exit short within 60 seconds of confirmed oracle update.
- **Secondary trigger (stop):** If oracle has not updated within 45 minutes of entry and spot price has recovered to within 0.3% of Chainlink price (delta collapsed without oracle update), exit short immediately — the liquidation batch will be small or zero.
- **Hard stop-loss:** Exit short if ETH-PERP price rises 1.5% above entry price (oracle lag trade is invalidated if price is rising).

### Long Entry (Reversion Trade)

- **Trigger:** Exit short AND confirm via TheGraph that liquidation volume has fired (query Aave liquidation events, or proxy: observe a 0.5-2% price spike down on Hyperliquid within 5 minutes of oracle update).
- **Entry:** Market long immediately after short exit, same size.
- **Hold period:** 5-20 minutes maximum.
- **Exit:** Take profit at 0.5% gain from long entry, OR exit at 15-minute mark regardless of P&L.
- **Long stop-loss:** Exit if price falls 0.8% below long entry (liquidation dump is continuing, not reverting).

### Full Trade Sequence Summary

```
[Pre-conditions met]
    → Enter SHORT on ETH-PERP
    → Monitor oracle via RPC every 30s
    
[Oracle updates]
    → Exit SHORT within 60s
    → Confirm liquidation dump occurred (price dip visible)
    → Enter LONG
    → Hold 5-20 min, TP at +0.5%, SL at -0.8%
    
[No oracle update in 45 min OR delta collapses]
    → Exit SHORT, no long trade
```

---

## Position Sizing

### Base Size Formula

```
position_size_USD = min(
    account_equity × 0.05,          # max 5% of account per trade
    PLV_USD × 0.10,                 # max 10% of estimated liquidation volume
    50,000                          # hard cap $50k until strategy is validated
)
```

**Rationale for PLV scaling:** The price impact of the liquidation dump is proportional to PLV relative to ETH market depth. Sizing at 10% of PLV ensures the strategy is not larger than the edge it is trading against.

### Leverage

- Use 2-3x leverage maximum on Hyperliquid.
- The edge is timing, not leverage. High leverage increases liquidation risk during the entry window if the oracle fires faster than expected.

### Scaling Rules

- Do not run more than one open position in this strategy simultaneously.
- After 10 live trades, review Sharpe and win rate. If win rate > 55% and average R > 0.8, increase hard cap to $150k.

---

## Backtest Methodology

### Data Requirements

| Dataset | Source | Cost | Notes |
|---|---|---|---|
| Chainlink ETH/USD historical round data | Ethereum archive node or Chainlink API | Free | Pull `AnswerUpdated` events from aggregator contract |
| Aave v3 position snapshots | TheGraph (Aave subgraph) | Free | Historical HF per position, updated per block |
| Binance ETH/USDT 1-second OHLCV | Binance API | Free | For spot price during oracle lag windows |
| Hyperliquid ETH-PERP 1-minute OHLCV | Hyperliquid API | Free | For trade execution simulation |
| Aave liquidation events | TheGraph or Dune Analytics | Free | Confirm liquidation volume post-oracle-update |

### Reconstruction Steps

**Step 1 — Build oracle lag time series:**
For each Chainlink round, record `(round_id, answer, updatedAt)`. Interpolate Binance spot price at each `updatedAt` timestamp. Compute `delta = (chainlink_answer - binance_spot) / binance_spot` for every 30-second interval between rounds.

**Step 2 — Identify candidate events:**
Flag all intervals where `delta ≥ 0.8%` and `delta` is increasing (spot falling away from oracle). These are candidate entry windows.

**Step 3 — Compute PLV for each candidate:**
For each candidate event timestamp, query Aave v3 subgraph for all open positions. Apply spot price to each position's collateral. Count positions where `HF(spot) < 1.0` and `HF(chainlink) ≥ 1.0`. Sum collateral USD value = PLV. Filter to events where PLV ≥ $5M.

**Step 4 — Simulate trade execution:**
- Entry: Hyperliquid ETH-PERP price at the 30-second bar when delta first crosses 0.8%
- Short exit: Hyperliquid price at the 1-minute bar immediately following the oracle update timestamp
- Long entry: Same bar as short exit
- Long exit: First bar where price is +0.5% above long entry, or 20-minute bar, whichever comes first
- Apply 0.05% round-trip slippage per leg (conservative for Hyperliquid)

**Step 5 — Validate liquidation dump:**
For each event, check Aave liquidation event logs in the 10-minute window post-oracle-update. Confirm that PLV > $5M events actually produced liquidation volume. This validates the PLV computation methodology.

**Step 6 — Segment analysis:**
Separate results by:
- PLV bucket ($5-20M, $20-50M, $50M+)
- Oracle update trigger type (deviation vs. heartbeat)
- Time of day (UTC) — liquidation bot activity may vary
- Market regime (trending vs. ranging, measured by 24h ATR)

### Backtest Period

- Primary: January 2022 – December 2024 (covers multiple high-volatility regimes including LUNA crash, FTX collapse, 2022 bear market)
- Out-of-sample validation: January 2025 – present

### Minimum Sample Size for Validity

- Require ≥ 30 qualifying events (PLV ≥ $5M, delta ≥ 0.8%) before drawing conclusions
- If fewer than 30 events exist in the backtest period, the strategy is too infrequent to validate statistically — downgrade to "monitor only"

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

1. **Backtest win rate ≥ 50%** on the combined short + reversion trade across ≥ 30 events
2. **Backtest average R ≥ 1.2** (average winner / average loser ratio)
3. **PLV correlation confirmed:** Events with PLV > $20M must show statistically larger price dips post-oracle-update than events with PLV $5-20M (Welch's t-test, p < 0.10)
4. **Paper trade ≥ 10 events** with the live bot before committing real capital, confirming execution latency is within acceptable bounds (short entry to oracle update gap ≥ 3 minutes on average)
5. **Bot infrastructure validated:** RPC polling confirmed reliable at 30-second intervals with < 5-second latency on oracle update detection

---

## Kill Criteria

Immediately suspend the strategy if any of the following occur:

1. **5 consecutive losing trades** on the combined short + reversion sequence
2. **Realized win rate drops below 40%** over any rolling 20-trade window
3. **Oracle update detection latency exceeds 2 minutes** (bot infrastructure failure — the edge requires detecting the update quickly)
4. **Aave governance changes liquidation mechanics** or Chainlink changes the ETH/USD deviation threshold — re-evaluate structural mechanism before resuming
5. **PLV computation is found to be systematically wrong** (e.g., TheGraph data lag causes PLV to be overstated) — suspend until data pipeline is corrected
6. **Hyperliquid ETH-PERP market depth drops below $2M** at the time of entry — insufficient liquidity to exit cleanly

---

## Risks

### Execution Risk (HIGH)
The short entry window between "delta crosses 0.8%" and "oracle fires" may be shorter than expected during fast-moving markets. A 3% spot drop in 60 seconds could breach the 0.5% deviation threshold before the monitoring bot detects the 0.8% delta. **Mitigation:** Lower the entry trigger to 0.6% delta to give more buffer, accepting a slightly lower expected dump magnitude.

### Liquidation Bot Competition (MEDIUM)
Professional MEV searchers are already positioned to liquidate Aave positions the moment the oracle updates. The question is whether their *liquidation activity* creates a price impact that is tradeable from a perp position. The strategy does not compete with liquidation bots — it trades the *consequence* of their activity. However, if liquidation bots have become sophisticated enough to hedge their liquidation exposure (buying perp shorts before the oracle fires), they may be front-running the same trade. **Mitigation:** Monitor whether the price dump post-oracle-update has been shrinking over time (evidence of increased competition).

### Oracle Mechanism Changes (MEDIUM)
Chainlink has migrated some feeds to different update mechanisms (e.g., Low Latency Oracle for some DeFi protocols). If Aave migrates to a faster oracle (e.g., Pyth, Redstone, or Chainlink's Data Streams), the lag window disappears entirely. **Mitigation:** Monitor Aave governance proposals and Chainlink feed configuration changes. This is a strategy with a finite lifespan tied to oracle architecture.

### TheGraph Data Lag (MEDIUM)
TheGraph subgraph indexing can lag the chain by 1-5 minutes during high network congestion — precisely when this strategy is most active. PLV computed from a lagged subgraph may be inaccurate. **Mitigation:** Cross-validate PLV estimates using a direct RPC scan of Aave's `getUserAccountData()` for the top 100 positions by collateral value (covers ~80% of PLV with manageable RPC calls).

### Reversion Trade Failure (LOW-MEDIUM)
The long reversion trade assumes the post-liquidation dump is temporary. In a genuine market crash (not a liquidation-driven overshoot), the dump continues and the long loses. **Mitigation:** The 0.8% stop-loss on the long leg limits downside. Additionally, only enter the long if the price dip post-oracle-update is ≥ 0.3% (confirming a dump occurred) and the broader market (BTC) is not simultaneously crashing.

### Regulatory / Protocol Risk (LOW)
Aave governance could change liquidation thresholds, bonuses, or collateral factors, altering the PLV computation. **Mitigation:** Re-validate PLV formula after any Aave governance parameter change.

### Funding Rate Drag (LOW)
Holding a short on Hyperliquid ETH-PERP during a falling market typically means receiving positive funding (longs pay shorts). This is favorable. However, if the market is already heavily short (negative funding), the short position pays funding. **Mitigation:** Pre-condition 5 (funding rate check) filters out the worst cases.

---

## Data Sources

| Source | URL / Method | Data Type | Refresh Rate |
|---|---|---|---|
| Chainlink ETH/USD Aggregator | `eth_call` to `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`, `latestRoundData()` | Oracle price + timestamp | Every 30s (live) |
| Chainlink historical rounds | Ethereum archive node, filter `AnswerUpdated` events | Historical oracle prices | One-time pull |
| Aave v3 positions | TheGraph: `https://thegraph.com/hosted-service/subgraph/aave/protocol-v3` | Position HF, collateral, debt | Every 5 min |
| Aave liquidation events | TheGraph or Dune: `aave_v3_ethereum.LiquidationCall` | Liquidation volume | Post-event validation |
| Binance ETH/USDT spot | `https://api.binance.com/api/v3/klines` | 1s/1m OHLCV | Every 30s (live) |
| Hyperliquid ETH-PERP | Hyperliquid API / WebSocket | Perp price, funding rate | Real-time |
| Ethereum RPC (archive) | Alchemy, Infura, or self-hosted | Block data, contract calls | On-demand |

---

## Open Questions for Backtest Phase

1. **What is the actual distribution of entry-to-oracle-update time?** If the median window is < 2 minutes, the strategy is operationally difficult without automation. If it is > 5 minutes, it is comfortably executable.

2. **Does PLV ≥ $5M reliably produce a measurable price dip?** The hypothesis requires a causal link between PLV magnitude and post-oracle price impact. This must be empirically confirmed, not assumed.

3. **Is the reversion trade the higher-value leg, or is the short the higher-value leg?** Intuition says the reversion is cleaner (less competition, more predictable), but the backtest will determine which leg drives returns.

4. **How often does the oracle update via heartbeat vs. deviation threshold?** Heartbeat updates are predictable by timestamp but may not coincide with large PLV events. Deviation updates are the primary mechanism for this strategy.

5. **Has the edge degraded since 2022?** MEV bot sophistication has increased significantly. The 2022-2023 data may show a larger edge than 2024-2025. The out-of-sample validation period is critical.

---

*This document is a hypothesis specification. No backtest has been run. All claims about mechanism are based on publicly documented protocol behavior. All claims about edge magnitude are unvalidated.*
