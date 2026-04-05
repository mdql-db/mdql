---
title: "Beefy/Yearn Vault Harvest Frontrun — Compounding Spike Arbitrage"
status: HYPOTHESIS
mechanism: 4
implementation: 3
safety: 4
frequency: 7
composite: 336
categories:
  - defi-protocol
  - liquidation
created: "2025-07-11"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Auto-compounding vaults (Beefy Finance, Yearn V3, Harvest Finance) accumulate reward tokens continuously, then periodically execute a "harvest" transaction that sells those rewards into an AMM pool and re-deposits the proceeds. This sell is:

1. **Programmatic** — executed by a keeper bot, not a human
2. **Predictable** — harvest frequency is either time-based (every N hours) or threshold-based (when accrued rewards exceed X USD)
3. **On-chain visible** — full history of past harvests is queryable, allowing estimation of the next harvest window

**Causal chain:**

> Vault accumulates reward tokens → keeper triggers harvest → harvest tx sells reward token into AMM (e.g. CAKE→BNB on PancakeSwap) → AMM price of reward token dips momentarily → vault re-deposits → price recovers as arbitrageurs restore AMM balance

**Tradeable edge:** Enter a short on the reward token (via perp or spot borrow) immediately before the expected harvest sell. The harvest sell creates a mechanical, size-knowable price impact. Exit the short within 1–3 blocks of harvest confirmation.

**Secondary edge (LP capture):** Position as a just-in-time liquidity provider in the specific AMM pool the vault routes through, capturing the harvest swap fees and slippage spread without directional exposure.

---

## Structural Mechanism — WHY This Must Happen

This is **not** a "tends to happen" pattern. The mechanism is contractually enforced by vault smart contract logic:

1. **Reward accumulation is continuous.** Staking contracts (e.g. MasterChef, Gauge contracts) distribute reward tokens per block. The vault's balance of claimable rewards grows deterministically.

2. **Harvest is triggered by a keeper.** Beefy's keeper network calls `harvest()` on a schedule (typically every 12–24 hours per vault, sometimes shorter on high-APY vaults). The `harvest()` function is public — anyone can call it, but Beefy's keeper does so on a predictable cadence.

3. **The sell path is hardcoded.** Each vault's strategy contract specifies the exact swap route (e.g. CAKE → WBNB → BUSD → deposit token). This is immutable or upgradeable only via timelock. The AMM pool receiving the sell is known in advance.

4. **Sell size is estimable.** Claimable rewards = (vault's staked balance) × (reward rate per block) × (blocks since last harvest). All inputs are on-chain readable. Estimation error is low for time-based harvesters.

5. **Price impact is non-zero.** For smaller-cap reward tokens (CAKE on BSC, BIFI, smaller L2 farm tokens) with thinner AMM liquidity, a $10K–$100K harvest sell creates measurable price impact (0.1%–2% depending on pool depth).

**Why MEV hasn't fully captured this:** Sandwich bots operate in the mempool on the same block. This strategy operates *before* the harvest tx is submitted — it's a positional trade, not a same-block sandwich. The edge is in the *anticipation window* (minutes to hours before harvest), not the mempool. This is outside the MEV bot's operational domain.

---

## Entry / Exit Rules

### Vault Selection Criteria (pre-trade filter)
- Vault must have ≥ 30 historical harvests (sufficient for frequency estimation)
- Reward token must have a liquid perp market OR borrowable spot (for shorting)
- Reward token AMM pool depth < $5M (larger pools absorb harvest with negligible impact)
- Harvest sell size must be ≥ 0.5% of AMM pool depth at time of entry (minimum impact threshold)
- Vault must be on a chain with block-level data accessible (BSC, Arbitrum, Optimism, Base)

### Harvest Timing Estimation
- Pull last 30 harvest tx timestamps for the target vault
- Compute median inter-harvest interval (e.g. 21,600 seconds = 6 hours)
- Compute standard deviation of interval
- Define entry window: `[last_harvest_time + median_interval - 1σ, last_harvest_time + median_interval + 0.5σ]`
- If threshold-triggered: monitor vault's `pendingRewards()` view function; enter when pending rewards reach 90% of the historical average harvest size

### Entry
- **Instrument:** Perpetual short on reward token (Hyperliquid if listed; else borrow spot on lending protocol)
- **Entry timing:** Enter short at the start of the estimated harvest window
- **Entry size:** See Position Sizing section
- **Entry condition:** Confirm no harvest tx has occurred in the last `0.5 × median_interval` (avoid entering after a recent harvest)

### Exit
- **Primary exit:** Close short within 3 blocks of harvest tx confirmation (monitor target vault address for `Harvest` event)
- **Stop-loss exit:** If harvest does not occur within `last_harvest_time + median_interval + 2σ`, exit the short (harvest delayed or keeper failure)
- **Hard stop:** If reward token price moves +3% against position before harvest, exit immediately

### Execution
- Requires a monitoring bot that:
  - Watches vault contract for `Harvest` event in real time
  - Submits close order on perp exchange within 1–2 seconds of event detection
  - This is **not** HFT — 1–2 second latency is acceptable; harvest price impact persists for multiple blocks

---

## Position Sizing

**Base sizing formula:**

```
position_size = min(
    estimated_harvest_sell_USD × impact_multiplier,
    max_position_cap
)
```

Where:
- `estimated_harvest_sell_USD` = `pendingRewards × reward_token_price`
- `impact_multiplier` = 0.5 (size the short at 50% of estimated harvest sell to avoid being the dominant flow)
- `max_position_cap` = $5,000 per trade (hard cap during backtest/paper trade phase)

**Rationale:** The harvest sell is the "guaranteed" flow. Sizing at 50% of that flow means the strategy is not larger than the event it's trading. Exceeding 100% of harvest size means the strategy itself becomes the price-moving event.

**Portfolio allocation:** No more than 20% of total strategy capital in any single vault harvest position. Multiple vaults can run simultaneously if they harvest at different times.

**Leverage:** 2–3× maximum on perp. The edge is small (0.1%–1% price move); leverage amplifies but also amplifies stop-loss risk.

---

## Backtest Methodology

### Data Required

| Data Type | Source | Endpoint/Method |
|---|---|---|
| Vault harvest tx history | The Graph (Beefy subgraph) | `https://api.thegraph.com/subgraphs/name/beefyfinance/beefy-bsc` — query `HarvestCalls` |
| Reward token OHLCV (1-min) | Binance/Gate.io historical API | REST: `GET /api/v3/klines?symbol=CAKEUSDT&interval=1m` |
| AMM pool depth at harvest time | BSC archive node or Dune Analytics | `pancakeswap_v2.pool_stats` on Dune |
| Pending rewards at harvest | BSC archive node RPC | `eth_call` to `pendingCake()` at block N-1 before harvest |
| Block timestamps | BSC RPC or Etherscan API | `eth_getBlockByNumber` |

### Backtest Period
- **Start:** January 2022 (Beefy BSC vaults mature, sufficient harvest history)
- **End:** December 2024
- **Minimum vaults:** 5 vaults with different reward tokens (CAKE, BIFI, and 3 smaller farm tokens)

### Backtest Steps

1. **Extract harvest events:** For each target vault, pull all `Harvest` tx hashes, block numbers, and timestamps. Compute reward token amount sold per harvest from tx input data or emitted events.

2. **Simulate entry signal:** For each harvest, compute what the entry window would have been using only data available *before* that harvest (rolling median/std of prior 30 harvests). Record the entry timestamp.

3. **Measure price impact:** For each harvest, record reward token price at:
   - Entry timestamp (T_entry)
   - Harvest block (T_harvest)
   - T_harvest + 3 blocks (T_exit)
   - T_harvest + 10 blocks (T_decay, for reference)

4. **Compute trade P&L:**
   - `raw_return = (price_at_entry - price_at_exit) / price_at_entry`
   - `net_return = raw_return - estimated_fees` (perp funding + taker fee ~0.05% each side)

5. **Baseline comparison:** Compare against a naive strategy of shorting the reward token at a random time each day and holding for the same duration. This controls for any general downward drift in farm token prices.

6. **Stratify by:**
   - Harvest sell size as % of pool depth (expect stronger edge when >1%)
   - Time of day (keeper bots may have patterns)
   - Vault age (newer vaults may have less predictable cadence)

### Key Metrics to Report
- Win rate (% of harvests where short was profitable)
- Average net return per trade
- Sharpe ratio of trade returns
- Maximum adverse excursion (worst case before harvest)
- Timing accuracy: % of harvests that occurred within the predicted window
- Edge decay over time (is the edge shrinking year-over-year as MEV matures?)

---

## Go-Live Criteria

The backtest must show **all** of the following before paper trading begins:

1. **Win rate ≥ 58%** on harvests where sell size ≥ 0.5% of pool depth
2. **Average net return per trade ≥ 0.15%** after fees (2× taker fee + funding)
3. **Timing accuracy ≥ 70%** — harvest occurs within the predicted window on ≥ 70% of events
4. **Sharpe ratio ≥ 1.0** on trade-level returns (not annualised — per-trade distribution)
5. **Edge present in ≥ 3 of 5 tested vaults** (not a single-vault artifact)
6. **No evidence of edge collapse post-2023** — the strategy must show comparable performance in 2023–2024 vs 2022 (MEV maturation check)

---

## Kill Criteria

Abandon the strategy if any of the following occur:

### During Backtest
- Win rate < 52% across all tested vaults
- Average net return < 0.05% (insufficient to cover execution slippage and ops overhead)
- Timing accuracy < 55% (harvest cadence too irregular to predict)
- Edge is entirely explained by baseline drift (farm tokens declining regardless of harvest)

### During Paper Trading (first 60 days)
- Fewer than 20 paper trades executed (insufficient sample)
- Realised win rate < 50% on paper trades
- More than 3 instances of harvest occurring outside the predicted window by >2σ (keeper behaviour has changed)
- Reward token perp delisted or borrow market dries up on primary instruments

### Ongoing
- Beefy/Yearn migrates to a randomised harvest schedule (protocol change that destroys timing predictability)
- MEV bots begin operating in the *anticipation window* (detectable if price starts moving against position before harvest tx is submitted)
- Harvest sell sizes drop below $5K consistently (protocol TVL decline makes edge too small)

---

## Risks — Honest Assessment

### High Severity

**MEV encroachment:** The primary risk is that sophisticated actors begin monitoring the same signals and front-run the front-runner. If bots start shorting reward tokens in the anticipation window, the edge compresses to zero. This is detectable (price starts moving before harvest without a harvest tx) but not preventable.

**Keeper irregularity:** Beefy's keeper network has experienced outages and delays. If a keeper fails to harvest on schedule, the position sits open past the stop-loss window and must be exited at a loss. Historical keeper reliability should be measured in backtest.

**Harvest size estimation error:** If the vault's reward rate changes (e.g. farm emissions cut, TVL spike), the estimated harvest size will be wrong. A smaller-than-expected harvest may produce no measurable price impact, resulting in a losing trade.

### Medium Severity

**Liquidity on perp side:** Smaller reward tokens (the ones with sufficient AMM impact) may have thin or non-existent perp markets. Spot borrow is an alternative but introduces borrow rate risk and recall risk.

**Chain-level congestion:** On BSC during high-activity periods, the 1–2 second exit window may be insufficient. The harvest price impact may recover before the close order is filled.

**Correlated positions:** Running multiple vault harvests simultaneously means positions are often in the same reward tokens (e.g. multiple CAKE vaults). This creates unintended concentration.

### Low Severity

**Protocol upgrade risk:** Vault strategy contracts can be upgraded via timelock. A new strategy may change the swap route, making the pre-identified AMM pool irrelevant. Monitor vault upgrade events.

**Tax/reporting complexity:** High-frequency short positions across multiple tokens create significant tax lot complexity. Not a trading risk but an operational one.

### Honest Overall Assessment

This strategy has a real mechanical basis but sits at the **lower end of structural edges**. The harvest sell is guaranteed; the price impact is not. On large-cap reward tokens (CAKE with $50M+ pool depth), the edge is likely negligible. The viable universe is narrow: small-to-mid cap reward tokens, thin AMM pools, chains where MEV infrastructure is less mature. The strategy requires ongoing maintenance (monitoring keeper behaviour, vault upgrades, pool depth changes) for what may be small per-trade returns. It is worth backtesting to quantify the edge precisely, but the prior expectation is that it is a **niche, operationally intensive strategy with a small but real edge** rather than a scalable core strategy.

---

## Data Sources

| Source | URL | Notes |
|---|---|---|
| Beefy Finance Subgraph (BSC) | `https://api.thegraph.com/subgraphs/name/beefyfinance/beefy-bsc` | Harvest events, vault addresses |
| Beefy API (vault list) | `https://api.beefy.finance/vaults` | Vault metadata, strategy addresses |
| Yearn V3 Subgraph | `https://api.thegraph.com/subgraphs/name/yearn/yearn-vaults-v3` | Harvest events for Yearn vaults |
| Dune Analytics | `https://dune.com` — query `pancakeswap_v2.trades` | AMM pool depth, swap history |
| BSC Archive Node | QuickNode or Ankr BSC endpoint | `eth_call` for `pendingCake()`, block data |
| Binance Historical Klines | `https://api.binance.com/api/v3/klines` | 1-min OHLCV for CAKE, BNB, etc. |
| Etherscan BSC API | `https://api.bscscan.com/api` | Tx history, internal tx traces |
| Hyperliquid Perp Data | `https://app.hyperliquid.xyz/trade` + REST API | Perp availability and funding rates for reward tokens |
| DefiLlama TVL API | `https://api.llama.fi/protocol/beefy` | Vault TVL over time (for harvest size estimation) |
