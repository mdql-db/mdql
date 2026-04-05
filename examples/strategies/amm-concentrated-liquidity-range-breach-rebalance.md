---
title: "AMM Concentrated Liquidity Range Breach Rebalance"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 5
composite: 625
categories:
  - defi-protocol
  - liquidation
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When price crosses the boundary tick of a large concentrated liquidity position on Uniswap v3, the smart contract mechanically converts that LP's entire holdings to a single asset. The LP now holds 100% of the depreciating asset with zero fee revenue. This creates a structurally predictable rebalance event: the LP must either withdraw and re-range, or accept permanent single-asset exposure. The withdrawal and re-ranging process involves selling the now-dominant asset back into the market, generating directional pressure that is detectable on-chain before it appears in perp prices. We trade the perp in the direction of the anticipated LP unwind, entering after tick breach confirmation and exiting when on-chain data confirms the LP position has been reset or removed.

The edge is **not** that LPs always rebalance immediately. The edge is that the *trigger* (tick breach) is mechanically guaranteed by the smart contract, the *incentive* to rebalance is structurally overwhelming (zero fee revenue, increasing impermanent loss), and the *action* (selling the dominant asset) is observable on-chain before it is fully priced into perps.

---

## Structural Mechanism

### Why the contract enforces the outcome

Uniswap v3 positions are defined by a lower tick `tL` and upper tick `tU`. The invariant curve `x * y = k` is only active within this range. When spot price `P` falls below `tL`, the position converts entirely to the base token (e.g., ETH in an ETH/USDC pair). When `P` rises above `tU`, the position converts entirely to the quote token (USDC). This is not a choice — it is enforced by the `swap()` function math. No LP action is required for conversion; it happens atomically as price crosses the tick.

### Why large LPs must respond

1. **Zero fee revenue:** Out-of-range positions earn no fees. For a position earning 0.3% fees on $10M TVL at 5× daily volume turnover, going out of range costs ~$15,000/day in foregone fees.
2. **Increasing directional loss:** A position that converted to 100% ETH at $2,000 and watches ETH fall to $1,800 has a 10% unhedged loss with no fee offset.
3. **Capital efficiency pressure:** Institutional LPs and market makers running concentrated liquidity strategies have internal risk limits on single-asset exposure. Breach of tick = breach of their internal hedge ratio.

### Why this creates tradeable flow

Large LPs withdrawing and re-ranging must execute two transactions:
- `decreaseLiquidity()` → removes position, receives single asset
- Optionally: swap back to 50/50 before `mint()` on new range

The swap-back is the tradeable event. A $5M LP position that converted to 100% ETH must sell ~$2.5M of ETH to re-establish a balanced position. This sell pressure is:
- **Predictable in direction** (sell the asset the LP is now overweight)
- **Predictable in timing** (within hours of tick breach, driven by fee-loss urgency)
- **Observable on-chain** before execution (position still shows as out-of-range in subgraph)

### Why perps lag

Perp traders watch order books and funding rates, not Uniswap v3 subgraph data. The LP rebalance flow hits the spot market first (via DEX), then propagates to perps via arbitrageurs. This creates a 5–30 minute window where the on-chain signal precedes the perp price move.

---

## Universe

**Pairs:** ETH/USDC (0.05% and 0.3% fee tiers), BTC/USDC (0.3% fee tier), ARB/USDC, OP/USDC on Uniswap v3 mainnet and Arbitrum. Start with ETH/USDC only for backtest — highest TVL, most liquid perp, tightest spreads.

**LP size threshold:** Only track positions with TVL > $500,000 at time of monitoring. Below this, rebalance flow is too small to move perp prices meaningfully.

**Perp venue:** Hyperliquid ETH-PERP or Binance ETH-USDT-PERP. Hyperliquid preferred for lower fees and no KYC friction.

---

## Entry Rules

### Step 1 — Identify candidate positions (continuous monitoring)

Query Uniswap v3 subgraph every 5 minutes:

```
{
  positions(
    where: {liquidity_gt: "0", pool: "<ETH_USDC_POOL_ADDRESS>"}
    orderBy: depositedToken0
    orderDirection: desc
    first: 50
  ) {
    id
    tickLower { tickIdx }
    tickUpper { tickIdx }
    liquidity
    depositedToken0
    depositedToken1
    owner
  }
}
```

Convert ticks to prices: `price = 1.0001^tick`. Flag any position where current spot price is within 0.5% of `tickLower` or `tickUpper`.

### Step 2 — Breach confirmation

**Entry trigger:** Spot price (Uniswap v3 pool `sqrtPriceX96`) crosses the tick boundary of a qualifying position (TVL > $500K). Confirm breach on two consecutive 5-minute subgraph polls to avoid false triggers from single-block price spikes.

**Direction logic:**
- Price crosses **below** `tickLower` → LP now holds 100% ETH → LP will sell ETH to rebalance → **SHORT ETH-PERP**
- Price crosses **above** `tickUpper` → LP now holds 100% USDC → LP will buy ETH to rebalance → **LONG ETH-PERP**

### Step 3 — Entry execution

Enter market order on Hyperliquid ETH-PERP within 2 minutes of breach confirmation. Do not use limit orders — the edge is time-sensitive and slippage on a $50K position in ETH-PERP is negligible (<0.05%).

**Entry size:** See Position Sizing section.

**Do not enter if:**
- Funding rate is >0.05% per 8h in the direction of trade (paying excessive carry)
- Spot-perp basis >0.3% (market already pricing in the move)
- Position TVL < $500K at time of breach (rebalance flow too small)
- More than 3 active trades open simultaneously (concentration risk)

---

## Exit Rules

### Primary exit — On-chain confirmation

Monitor the breached LP position's NFT ID every 5 minutes. Exit when **any** of the following occur:

1. `liquidity` field drops to 0 (LP withdrew position) — exit immediately
2. `tickLower` or `tickUpper` changes (LP re-ranged) — exit immediately
3. Position TVL drops by >80% (partial withdrawal indicating rebalance in progress) — exit immediately

### Secondary exit — Time stop

If none of the above triggers fire within **4 hours** of entry, exit at market. Rationale: if the LP has not rebalanced in 4 hours, either (a) they are a passive LP who will not rebalance, or (b) price has moved back in-range, eliminating the thesis.

### Hard stop-loss

Exit immediately if position moves **1.5%** against entry price. This is a structural trade, not a directional bet — if price moves 1.5% against us before the LP rebalances, the thesis is broken or the LP is too small to matter.

### Take-profit

No fixed take-profit. Hold until on-chain exit trigger fires. The trade duration is defined by LP behavior, not price targets. Historical LP rebalance times (hypothesis: 30 minutes to 4 hours) will be validated in backtest.

---

## Position Sizing

**Base size:** 2% of total portfolio per trade.

**Scaling by LP TVL:**
- LP TVL $500K–$2M: 1% of portfolio
- LP TVL $2M–$10M: 2% of portfolio
- LP TVL >$10M: 3% of portfolio (capped — larger positions have more market impact but also more rebalance flow)

**Leverage:** 3× on Hyperliquid. Rationale: trade duration is short (minutes to hours), stop-loss is tight (1.5%), and structural trades should not require high leverage to be profitable. 3× gives meaningful return on a 0.5–1% price move without excessive liquidation risk.

**Maximum concurrent exposure:** 6% of portfolio (3 trades × 2% average). Do not open new positions if at maximum.

---

## Backtest Methodology

### Data requirements

| Dataset | Source | Cost | Notes |
|---|---|---|---|
| Uniswap v3 position history | The Graph (free tier) | Free | Query historical `positions` and `ticks` |
| Uniswap v3 pool swap events | The Graph or raw Ethereum logs | Free | Reconstruct price at each block |
| ETH/USDC spot price (minute-level) | Uniswap v3 subgraph or Dune Analytics | Free | Cross-reference with perp |
| ETH-PERP OHLCV (minute-level) | Hyperliquid API or Binance API | Free | Entry/exit price simulation |
| Historical funding rates | Hyperliquid API or Coinglass | Free | Carry cost calculation |

### Reconstruction approach

1. **Identify all historical tick breaches** for top-50 ETH/USDC positions by TVL, January 2023 – present. Use Ethereum block data via The Graph to find the exact block where `sqrtPriceX96` crossed a qualifying position's tick boundary.

2. **Record LP response time:** For each breach event, find the block where the LP's `liquidity` changed (withdrew or re-ranged). This gives the empirical distribution of LP response times — the core unknown in this strategy.

3. **Simulate trade:** Enter ETH-PERP at the 2-block-confirmed breach price. Exit at the block corresponding to LP response (or 4-hour time stop). Apply 0.05% entry + 0.05% exit fee (Hyperliquid taker). Apply funding rate accrual for hold duration.

4. **Filter by TVL threshold:** Run backtest at $500K, $1M, $2M, $5M thresholds to find optimal minimum position size.

5. **Measure:** Win rate, average P&L per trade, Sharpe ratio, maximum drawdown, average hold time, and — critically — the correlation between LP TVL and price impact.

### Key hypotheses to test in backtest

- H1: LP response time is <4 hours in >70% of breach events for positions >$1M TVL
- H2: ETH-PERP moves in the predicted direction within 30 minutes of breach in >55% of cases
- H3: Average trade P&L is positive after fees at 3× leverage
- H4: Larger LP positions (>$5M) produce larger and faster price moves

### Backtest period

Primary: January 2023 – December 2024 (covers multiple volatility regimes, bull and bear).
Out-of-sample validation: January 2025 – present.

---

## Go-Live Criteria

All of the following must be satisfied before live trading:

1. **Backtest Sharpe ratio ≥ 1.5** on primary period, calculated on daily P&L
2. **Win rate ≥ 52%** (edge must be positive, not just large winners)
3. **Average hold time ≤ 3 hours** (confirms LP response is fast enough to be tradeable)
4. **H1 confirmed:** LP response <4 hours in >70% of qualifying breach events
5. **Out-of-sample P&L positive** on January 2025 – present period
6. **Paper trade for 30 days** with at least 20 qualifying events observed, P&L positive after simulated fees
7. **Monitoring infrastructure live:** Subgraph polling every 5 minutes, automated breach alerts, on-chain exit triggers firing correctly in paper trade

---

## Kill Criteria

Immediately halt live trading if any of the following occur:

1. **3 consecutive losses** at full stop-loss (1.5% each) — suggests regime change or strategy leak
2. **Rolling 30-day Sharpe < 0** — strategy is not working in current regime
3. **Average LP response time increases to >6 hours** on rolling 20-trade basis — LPs have changed behavior (possible: more passive LPs, more automated re-ranging bots reducing our edge)
4. **On-chain monitoring latency >15 minutes** — infrastructure failure invalidates the timing edge
5. **Uniswap v3 TVL in ETH/USDC drops below $50M** — insufficient LP activity to generate qualifying events

---

## Risks

### Risk 1 — LP does not rebalance (passive LP)
**Description:** Many retail LPs set ranges and forget them. A $600K position that goes out of range may belong to a passive holder who will not rebalance for days or weeks.
**Mitigation:** TVL threshold ($500K minimum) biases toward institutional/active LPs. Backtest will reveal what % of qualifying positions are passive — if >40%, raise threshold to $2M.
**Residual risk:** Medium. Cannot fully distinguish active from passive LPs without wallet-level behavioral analysis.

### Risk 2 — Automated re-ranging bots eliminate the lag
**Description:** Protocols like Arrakis Finance, Gamma Strategies, and Revert Finance run automated LP management vaults that re-range within minutes of a tick breach. If the LP is managed by one of these vaults, the rebalance happens before we can enter.
**Mitigation:** Identify vault contract addresses (Arrakis, Gamma, etc.) and exclude them from the watchlist — their rebalances are too fast to trade. Only track EOA (externally owned account) LPs and unmanaged contracts.
**Residual risk:** Medium. Vault TVL is growing; the addressable universe may shrink over time.

### Risk 3 — Rebalance flow too small to move perp price
**Description:** A $1M LP rebalancing $500K of ETH into the spot market may not move ETH-PERP at all given its multi-billion dollar daily volume.
**Mitigation:** Backtest will directly measure price impact by LP TVL. Hypothesis is that the signal works through *information* (on-chain observers front-running the flow) not just the flow itself. If backtest shows no price impact below $5M TVL, raise threshold accordingly.
**Residual risk:** High for small positions. This is the most likely reason the strategy underperforms.

### Risk 4 — Subgraph data lag
**Description:** The Graph's hosted service can lag 5–15 minutes behind chain tip during congestion. If subgraph data is stale, we may enter after the LP has already rebalanced.
**Mitigation:** Cross-reference subgraph data with direct RPC calls to the Uniswap v3 pool contract (`slot0()` for current price, `positions()` for LP state). Use subgraph for discovery, RPC for confirmation.
**Residual risk:** Low with dual-source monitoring, but requires engineering effort.

### Risk 5 — Adverse selection on breach events
**Description:** Price may breach a tick because of a large directional trade that continues moving against us. We short ETH after a downward tick breach, but if the breach was caused by a large sell order that continues, we are trading with the flow — but if it reverses, we are caught.
**Mitigation:** The 1.5% hard stop-loss limits this. Additionally, filter out breach events where the 5-minute price move preceding the breach is >1% (suggests momentum that may reverse sharply).
**Residual risk:** Medium. Structural trades can still be caught in momentum reversals.

### Risk 6 — Strategy crowding
**Description:** If multiple quant funds discover this signal, the edge compresses as everyone front-runs the same LP rebalances.
**Mitigation:** Monitor the time between tick breach and observable price move. If this lag compresses from 20 minutes to <5 minutes on rolling basis, the edge is being competed away. Kill criterion #3 captures this indirectly.
**Residual risk:** Low currently (niche, requires on-chain data literacy), but increases as on-chain analytics tools become mainstream.

---

## Data Sources

| Source | URL | Data | Latency | Cost |
|---|---|---|---|---|
| Uniswap v3 Subgraph (Ethereum) | thegraph.com/hosted-service | Position ticks, liquidity, TVL | 5–15 min | Free |
| Uniswap v3 Subgraph (Arbitrum) | thegraph.com/hosted-service | Same, Arbitrum deployment | 5–15 min | Free |
| Dune Analytics | dune.com | Historical LP events, tick crossings | Batch | Free/paid |
| Ethereum RPC (Alchemy/Infura) | alchemy.com | Real-time `slot0()`, `positions()` | <1 min | Free tier |
| Hyperliquid API | api.hyperliquid.xyz | ETH-PERP OHLCV, funding rates | Real-time | Free |
| Binance API | api.binance.com | ETH-USDT-PERP backup | Real-time | Free |
| Coinglass | coinglass.com | Historical funding rates | Daily | Free |
| Arrakis/Gamma contract addresses | GitHub / Etherscan | Vault exclusion list | Static | Free |

---

## Implementation Checklist

- [ ] Build subgraph polling script (Python, 5-minute intervals, top-50 ETH/USDC positions by TVL)
- [ ] Build tick-to-price converter and breach detection logic
- [ ] Build Ethereum RPC confirmation layer (cross-check subgraph with `slot0()`)
- [ ] Compile exclusion list of automated vault contract addresses (Arrakis, Gamma, Revert, etc.)
- [ ] Pull historical breach events via Dune Analytics (January 2023 – present)
- [ ] Pull historical LP response times (block of breach → block of `liquidity` change)
- [ ] Pull ETH-PERP minute OHLCV from Hyperliquid API for same period
- [ ] Run backtest simulation with fee and funding rate deduction
- [ ] Validate H1–H4 hypotheses
- [ ] If go-live criteria met: build paper trade execution layer with Hyperliquid testnet
- [ ] Run 30-day paper trade, log all events and P&L
- [ ] Review kill criteria thresholds before live capital deployment

---

## Open Questions for Researcher Review

1. **What is the empirical distribution of LP response times?** This is the single most important unknown. If median response time is >6 hours, the strategy is not viable.
2. **What fraction of top-50 ETH/USDC positions by TVL are managed by automated vaults?** If >60%, the addressable universe is too small.
3. **Is the price impact from LP rebalancing detectable in perp data, or does it only show in spot?** If perps don't move, we need to trade spot instead — possible but more complex.
4. **Does the signal work on Arbitrum Uniswap v3 as well as mainnet?** Arbitrum has lower gas costs, which may mean faster LP response times and a tighter trading window.
5. **Is there a size threshold below which LP rebalances produce zero measurable perp price impact?** Backtest must answer this before live trading.
