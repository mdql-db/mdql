---
title: "Ethereum Staking Reward Distribution — Daily Consensus Layer Credit"
status: KILLED
mechanism: 7
implementation: 3
safety: 7
frequency: 8
composite: 1176
categories:
  - lst-staking
  - defi-protocol
  - basis-trade
created: "2026-04-04"
pipeline_stage: "Killed at analysis (step 3 of 9)"
killed: "2026-04-05"
kill_reason: "Fee-negative at any retail scale. Daily rebase is ~0.0096% but Curve fees alone are 0.04% — edge is 4x smaller than the fee. Strategy doc admits this. Also I=3 (needs on-chain Curve + Lido oracle interaction)."
---

## Hypothesis

The Lido daily oracle report (~12:00 UTC) triggers a deterministic, on-chain rebase of stETH balances by approximately 0.0096%/day (3.5% APR ÷ 365). In the minutes immediately preceding this report, stETH/ETH on Curve trades at a marginal discount because the day's yield has not yet been credited. Immediately post-rebase, the NAV of stETH increases by exactly the accrued yield. If the Curve pool price does not fully pre-price this rebase, a long stETH / short ETH position opened 5–15 minutes before the oracle submission and closed 5–15 minutes after should capture the rebase increment minus execution costs.

**Honest prior:** This edge is structurally real but almost certainly fee-negative at any scale below ~$10M notional. The specification exists to (a) confirm or deny this via backtest, and (b) identify whether a scaled or modified version is viable.

---

## Structural Mechanism

### Why the rebase is guaranteed

Lido's oracle system works as follows:

1. A quorum of Lido-designated oracle nodes monitors the Beacon Chain consensus layer.
2. Once per day, these oracles submit an `AccountingOracle` report to the Lido smart contract on Ethereum mainnet.
3. The report contains the total ETH held across all Lido validators (principal + accumulated rewards).
4. The Lido contract recalculates the stETH/ETH exchange rate and rebases all stETH balances upward by the day's accrued yield.
5. This is enforced by smart contract logic — it is not discretionary. The rebase magnitude is deterministic given the Beacon Chain state.

**The rebase amount is not a prediction. It is a calculation from on-chain validator data that any node can verify before the oracle submits.**

### Why a pricing gap might exist

- Curve's stETH/ETH pool prices stETH continuously based on supply/demand.
- The rebase event is a step-function increase in stETH NAV.
- If the pool does not continuously pre-price the accruing yield (it does not — Curve is an AMM, not a NAV-tracking mechanism), a small discount to post-rebase NAV exists in the minutes before the oracle fires.
- Post-rebase, arbitrageurs should close any remaining gap between Curve spot and the new NAV.

### Why this is NOT a 9/10

The rebase magnitude is ~$0.096 per $1,000 notional. Curve swap fees are 0.04% ($0.40 per $1,000). Gas for a round-trip Curve interaction at 20 gwei is approximately $8–$25 depending on block conditions. The structural mechanism is real; the economics are hostile at retail scale.

---

## Entry Rules


### Pre-conditions (all must be true before entry)

| Condition | Check | Source |
|---|---|---|
| Oracle report not yet submitted for today | Query `AccountingOracle` contract last report timestamp | Etherscan / direct RPC |
| Current time is T-15 to T-5 minutes before expected oracle window | Oracle fires within a predictable ±30 min window around 12:00 UTC | Historical oracle timestamps |
| stETH/ETH Curve pool ratio is ≤ 0.9990 (stETH at discount or parity) | Query Curve pool reserves | Curve subgraph / on-chain |
| ETH gas price < 30 gwei | Check mempool | Any RPC provider |
| No active Lido governance vote or protocol pause | Check Lido DAO Aragon | Lido governance UI |

### Entry

- **Long leg:** Acquire stETH via Curve stETH/ETH pool (swap ETH → stETH).
- **Short leg:** Short equivalent ETH notional on Hyperliquid ETH-PERP to hedge directional ETH exposure.
- **Entry timing:** Execute 10 minutes before the expected oracle submission window opens.
- **Position size:** See sizing section below.

## Exit Rules

### Exit

- **Trigger:** Oracle `AccountingOracle` report confirmed on-chain (monitor `ReportSubmitted` event).
- **Exit window:** Close both legs within 5 minutes of oracle confirmation.
- **Long leg exit:** Swap stETH → ETH on Curve, or hold stETH if gas cost of exit exceeds remaining edge.
- **Short leg exit:** Close Hyperliquid ETH-PERP short simultaneously with long leg exit.
- **Abort rule:** If oracle has not fired within 60 minutes of expected window, exit both legs at market — oracle delay signals potential protocol issue.

### Do not enter if

- Curve pool stETH/ETH ratio is > 1.0000 (stETH at premium — rebase already pre-priced or pool imbalanced).
- Gas price > 30 gwei (fee destruction exceeds edge).
- Lido oracle has already submitted today's report (check timestamp).
- ETH price moved > 2% in the preceding 30 minutes (directional risk overwhelms yield capture).

---

## Position Sizing

### Minimum viable scale

At $0.096 edge per $1,000 notional and $15 round-trip gas cost:

- Break-even notional = $15 / 0.000096 = **$156,250 minimum per trade**
- At $156,250 notional, net edge after gas ≈ $0 (break-even only)
- Curve swap fee (0.04%) on $156,250 = $62.50 — this alone exceeds the edge

**Conclusion from math:** At current Curve fees and gas, this trade is fee-negative at any retail scale. Proceed to backtest only to confirm or find edge in modified versions (see Variants section).

### Theoretical institutional scale

At $10,000,000 notional:
- Gross edge: $960
- Gas: ~$20 (negligible)
- Curve swap fee (0.04%): $4,000 — **still exceeds edge**

**Revised conclusion:** Curve's 0.04% swap fee alone is 4× the daily rebase yield. The trade is structurally fee-negative on Curve regardless of scale. The hypothesis as stated is **likely not viable** via Curve swap execution.

### Modified sizing for variant strategies

See Variants section — the viable version of this trade does not involve swapping on Curve.

---

## Variants Worth Testing

The core mechanism (guaranteed daily rebase) is real. The execution path (Curve swap) is the problem. Three variants may be fee-viable:

### Variant A: Hold stETH, hedge only

- **Mechanism:** Hold a permanent stETH position. Each day, short ETH-PERP on Hyperliquid 10 minutes before oracle, close short 10 minutes after.
- **Edge:** Capture the rebase without paying Curve swap fees. Only cost is Hyperliquid funding + taker fee on the hedge leg.
- **Hyperliquid taker fee:** ~0.035% per side = 0.07% round-trip on the hedge notional.
- **Math:** 0.0096% rebase vs 0.07% hedge cost = still fee-negative, but closer.
- **Verdict:** Still negative, but within 1 order of magnitude. If funding rate is negative (shorts paid), this flips positive.

### Variant B: Exploit negative funding on ETH-PERP around oracle time

- **Mechanism:** If ETH-PERP funding goes negative in the 30-minute window around the oracle (shorts are paid), the short hedge generates positive carry that supplements the rebase yield.
- **Testable question:** Does ETH-PERP funding systematically go negative around 12:00 UTC due to stETH-related hedging flows?
- **Data needed:** Hyperliquid funding rate history at 1-minute resolution around 12:00 UTC daily.
- **Verdict:** Hypothesis — needs data pull.

### Variant C: stETH/ETH basis on a lower-fee venue

- **Mechanism:** If a venue offers stETH/ETH trading with fees < 0.0096%, the swap-based version becomes viable.
- **Current candidates:** Uniswap v3 stETH/ETH 0.01% fee tier (exists, low liquidity), CoW Protocol (MEV-protected, variable fees).
- **Math on Uniswap v3 0.01% tier:** 0.01% fee vs 0.0096% edge = still negative, but only by 4%.
- **Verdict:** Closest to viable. Needs liquidity depth check — if pool is thin, slippage kills it.

---

## Backtest Methodology

### Data required

| Dataset | Source | Format | Cost |
|---|---|---|---|
| Lido oracle submission timestamps (all historical) | Ethereum mainnet `AccountingOracle` contract event logs | Block number + timestamp | Free (RPC or Etherscan) |
| stETH/ETH Curve pool price at 1-minute resolution | Curve subgraph (The Graph) | Price ratio time series | Free |
| ETH-PERP funding rate at 1-minute resolution | Hyperliquid public API | Funding rate % | Free |
| ETH gas price history | Etherscan gas tracker API or Dune Analytics | Gwei time series | Free |
| Curve pool swap fee history | Curve contract events | Fee % | Free |

### Backtest procedure

**Step 1 — Oracle timestamp extraction**
Query all `ReportSubmitted` events from the Lido `AccountingOracle` contract (deployed post-Merge). Extract block timestamps. Calculate the distribution of submission times relative to 12:00 UTC. Measure: mean, standard deviation, and maximum deviation from expected window.

**Step 2 — Price gap measurement**
For each oracle event:
- Record stETH/ETH Curve pool price at T-30, T-15, T-10, T-5, T-1, T+0, T+1, T+5, T+15, T+30 minutes.
- Calculate the price change from T-10 to T+10 around each oracle event.
- Compare this change to the expected rebase magnitude (calculable from Lido's reported yield).
- Measure: how much of the rebase is pre-priced vs post-priced.

**Step 3 — Fee-adjusted P&L simulation**
For each oracle event, simulate:
- Entry at T-10 Curve price + 0.04% fee
- Exit at T+10 Curve price - 0.04% fee
- Gas cost: use actual gas price at entry block × estimated gas units (200,000 for Curve swap)
- Net P&L per trade in basis points

**Step 4 — Funding rate analysis (Variant B)**
For each oracle event:
- Extract Hyperliquid ETH-PERP funding rate in the T-30 to T+30 window.
- Test whether funding is systematically more negative around oracle time.
- Calculate whether negative funding + rebase yield > hedge execution cost.

**Step 5 — Variant C liquidity check**
For each oracle event:
- Query Uniswap v3 stETH/ETH 0.01% pool depth at T-10.
- Estimate slippage for $100K, $500K, $1M, $5M notional.
- Calculate break-even notional where slippage + fee < rebase yield.

### Success criteria for backtest

| Metric | Threshold to proceed |
|---|---|
| Median fee-adjusted P&L per trade | > 0 bps (any variant) |
| Win rate | > 60% |
| Sharpe (annualised) | > 1.5 |
| Maximum drawdown | < 5% of capital |
| Minimum viable notional | < $5M (institutional but not whale-only) |

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. Backtest shows positive median fee-adjusted P&L on at least one variant across ≥ 180 oracle events (approximately 6 months of data).
2. Oracle timing predictability confirmed: ≥ 95% of oracle submissions occur within ±20 minutes of the predicted window.
3. Execution path identified with total round-trip cost < 0.008% (below rebase yield).
4. Hyperliquid hedge leg confirmed: ETH-PERP taker fee + funding cost < 0.003% for the hedge window.
5. Monitoring infrastructure built: automated alert fires when `AccountingOracle` `ReportSubmitted` event is detected on-chain within 30 seconds.

Paper trade for 30 days minimum before live capital deployment.

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

| Trigger | Action |
|---|---|
| Three consecutive trades with fee-adjusted P&L < -0.02% | Stop trading, review oracle timing and fee assumptions |
| Lido migrates to a different oracle architecture | Re-evaluate mechanism from scratch |
| Curve raises stETH/ETH pool fee above 0.05% | Kill Curve-based variants permanently |
| ETH gas consistently > 50 gwei for 2+ weeks | Suspend until gas normalises |
| stETH/ETH Curve pool TVL drops below $500M | Liquidity insufficient for meaningful position size |
| Competing bot activity detected (price moves to post-rebase level before oracle fires) | Mechanism has been arbed away — kill strategy |

---

## Risks

### Risk 1: Oracle timing drift
**Description:** Lido oracle submission time shifts from historical pattern due to network congestion or oracle node issues.
**Probability:** Low (oracle nodes are professional operators with SLAs).
**Impact:** Position held longer than intended, increasing directional ETH exposure.
**Mitigation:** Hard abort rule — exit both legs if oracle has not fired within 60 minutes of expected window.

### Risk 2: Lido protocol pause
**Description:** Lido governance or emergency multisig pauses the protocol, preventing rebase.
**Probability:** Very low (has occurred once historically during Merge preparation).
**Impact:** stETH does not rebase; long leg loses the expected yield increment.
**Mitigation:** Pre-condition check on Lido protocol status before every entry. Monitor Lido governance forum for emergency proposals.

### Risk 3: ETH directional move overwhelms yield
**Description:** ETH price moves 1%+ during the 20-minute trade window, overwhelming the 0.0096% yield edge.
**Probability:** Moderate (ETH is volatile).
**Impact:** Net loss on the combined position despite correct rebase capture.
**Mitigation:** Short ETH-PERP hedge leg is mandatory — not optional. Abort entry if ETH has moved > 2% in the preceding 30 minutes.

### Risk 4: Curve pool imbalance
**Description:** Large stETH seller imbalances the Curve pool before oracle fires, causing stETH to trade at a discount unrelated to the rebase.
**Probability:** Low but non-zero (occurred during 2022 depeg event).
**Impact:** Entry price is worse than expected; exit price may not recover to NAV.
**Mitigation:** Pre-condition check: only enter if stETH/ETH ratio is within 0.10% of parity. Do not enter during periods of elevated stETH discount (> 0.5%).

### Risk 5: Smart contract execution risk
**Description:** Curve swap transaction fails or is front-run, resulting in partial execution.
**Probability:** Low with proper gas pricing.
**Impact:** Unhedged position in either stETH or ETH.
**Mitigation:** Use slippage tolerance of 0.02% on Curve swaps. Ensure hedge leg executes atomically with long leg (or within 30 seconds).

### Risk 6: Mechanism already fully arbed
**Description:** Sophisticated bots already pre-price the rebase perfectly, leaving zero gap.
**Probability:** High — this is the most likely reason this strategy scores 5/10.
**Impact:** No edge exists in practice despite structural mechanism being real.
**Mitigation:** Backtest will reveal this immediately. If price gap at T-10 to T+10 is consistently < 0.001%, the mechanism is fully arbed and the strategy is abandoned before any capital is deployed.

---

## Data Sources

| Data | Source | Access Method | Latency |
|---|---|---|---|
| Lido oracle events | Ethereum mainnet RPC | `eth_getLogs` on `AccountingOracle` contract | Real-time |
| stETH/ETH Curve pool price | Curve Finance subgraph (The Graph) | GraphQL API | ~1 min lag |
| stETH/ETH Curve pool price (faster) | Direct on-chain pool query | `get_dy` on Curve pool contract | Real-time |
| ETH-PERP funding rate | Hyperliquid public REST API | `/info` endpoint, `fundingHistory` | Real-time |
| ETH gas price | Blocknative or Alchemy gas API | REST API | Real-time |
| Lido protocol status | Lido contract `isPaused()` | Direct RPC call | Real-time |
| Historical oracle timestamps | Dune Analytics | Pre-built dashboard or custom query | Historical |
| Uniswap v3 pool depth | Uniswap v3 subgraph | GraphQL API | ~1 min lag |

**Key contract addresses (Ethereum mainnet):**
- Lido `AccountingOracle`: `0x852deD011285fe67063a08005c71a85690503Cee`
- Curve stETH/ETH pool: `0xDC24316b9AE028F1497c275EB9192a3Ea0f67022`
- stETH token: `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84`

---

## Honest Assessment and Next Steps

### What this strategy is

A structurally sound mechanism (guaranteed daily rebase) attached to an economically hostile execution path (Curve swap fees exceed the edge by 4×). The hypothesis is worth testing only because the variants (Variant B: funding rate exploitation, Variant C: lower-fee venue) may be viable even if the base case is not.

### Recommended research priority

1. **Pull Hyperliquid ETH-PERP funding rate data** around 12:00 UTC daily for the past 12 months. If funding is systematically negative in this window, Variant B may be viable without any Curve interaction.
2. **Query Uniswap v3 stETH/ETH 0.01% pool depth** — if liquidity supports $1M+ with < 0.005% slippage, Variant C approaches viability.
3. **Run the Dune query** on oracle timestamps to confirm timing predictability before building any infrastructure.

### What would change the score

- Discovery that ETH-PERP funding is systematically negative around oracle time → score increases to 6/10.
- Discovery that a new low-fee venue (e.g., a new Balancer or Curve pool with < 0.005% fees) offers stETH/ETH liquidity → score increases to 7/10.
- Discovery that the price gap is already zero in backtest data → score decreases to 2/10 and strategy is abandoned.

**Do not deploy capital until backtest is complete. The math as stated suggests this trade is fee-negative in its base form.**
