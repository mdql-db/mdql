---
title: "Liquid Staking Discount Convergence"
status: HYPOTHESIS
mechanism: 9
implementation: 5
safety: 8
frequency: 3
composite: 1080
categories:
  - lst-staking
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When liquid staking tokens (LSTs) trade below their on-chain redemption NAV, a contractually guaranteed arbitrage exists. Buying the discounted LST and submitting a withdrawal request locks in the spread — convergence to NAV is enforced by the smart contract, not predicted from price behaviour. The delta-neutral version (buy LST + short underlying on perps) removes directional exposure, leaving only the redemption spread as P&L.

---

## Why it's an edge

This is **pure arbitrage against a smart contract guarantee**, not a market tendency trade.

Every major LST has a provable redemption rate:
- stETH: 1 stETH redeems for exactly `stETH.getPooledEthByShares(1e18) / 1e18` ETH — a number that only ever increases (staking rewards accumulate)
- rETH: 1 rETH redeems for exactly `rocketTokenRETH.getExchangeRate()` ETH
- mSOL / jitoSOL: equivalent on-chain exchange rate functions on Solana

When market price falls below redemption NAV, the discount is not an opinion — it is a measurable deviation from a contract-enforced value. The mechanism that closes the gap is not "other traders will notice" but "anyone can submit a withdrawal request and receive NAV in 3–14 days."

**The closest traditional finance analogue:** Closed-end fund NAV arbitrage. When a closed-end fund trades at a discount to its underlying holdings, the discount bounds: rational investors buy the fund and redeem the assets. In LST arb, the "redemption" is literally the withdrawal queue. The edge is equally structural.

**Why does the discount exist at all?**
- Retail panic sellers don't track NAV; they sell at market
- The 3–14 day waiting period imposes a cost that not all sellers can absorb
- Some forced sellers (leveraged positions, collateral calls) have no choice
- Automated arb bots are present but finite in capital; large panic events overwhelm them temporarily

**The edge is specific to the post-withdrawal-enabled era.** Before Ethereum's Shapella upgrade (April 2023), there was no on-chain redemption mechanism for ETH LSTs — convergence was market-dependent. Post-Shapella, convergence is contractually enforced.

**Why Zunid can execute this:**
- Discount events are detectable via price feed and on-chain exchange rate queries
- Withdrawal queue length is readable on-chain before committing capital
- Position can be monitored daily; no sub-second execution required
- Autonomous monitoring across multiple LSTs 24/7 is exactly Zunid's capability profile

---

## Backtest Methodology

### Scope

**Time range:** April 2023 (Shapella upgrade) to present — the structural trade does not exist before this date.

**Assets:**
- stETH/ETH (primary — highest liquidity, longest post-Shapella history)
- rETH/ETH (secondary — smaller liquidity, different discount profile)
- cbETH/ETH (tertiary — institutional backing changes discount dynamics)
- mSOL/SOL, jitoSOL/SOL (Solana track — separate backtest due to different withdrawal mechanics)

### Data Required

| Data | Source | Granularity |
|------|--------|-------------|
| stETH/ETH spot ratio | Curve stETH/ETH pool — Dune Analytics query or The Graph | Hourly |
| rETH/ETH spot ratio | Rocket Pool subgraph via The Graph | Hourly |
| cbETH/ETH spot ratio | Coinbase Exchange or Dune | Hourly |
| On-chain redemption rate | Ethereum archive node or Dune (`stETH.getPooledEthByShares`) | Daily |
| ETH withdrawal queue length | Ethereum beacon chain API (`/eth/v1/beacon/states/head/validators`) | Daily |
| ETH perp funding rates | Hyperliquid historical funding; Binance historical funding | 8-hourly |
| ETH spot price | Binance `/api/v3/klines` | Hourly |
| Gas costs (Gwei) | Etherscan gas tracker historical export | Hourly |
| Estimated unstaking gas cost | Fixed: ~500k gas for Lido withdrawal request + claim | Per transaction |

### Backtest Logic

**Step 1 — Signal detection:**
```
discount = (on_chain_redemption_rate - lst_market_price) / on_chain_redemption_rate

if discount > threshold AND withdrawal_queue_days < 14:
    signal = ENTER
```

**Step 2 — P&L calculation per event:**
```
gross_spread = discount at entry
carrying_cost = (avg_funding_rate * 8h_periods_held)  # on short ETH leg
gas_cost_pct = (unstake_gas_cost_in_ETH) / (position_size_in_ETH)
net_spread = gross_spread - carrying_cost - gas_cost_pct - slippage_estimate

if net_spread > 0: profitable trade
```

**Step 3 — Exit timing:**
- Primary: withdrawal processed, receive ETH at NAV, close perp short
- Secondary: if market price recovers to within 0.1% of NAV before queue completes, exit at market (faster realisation)

### Metrics to Measure

| Metric | Target | Kill Level |
|--------|--------|------------|
| Number of qualifying events (post-Shapella) | Establish base rate | < 5 events = insufficient sample |
| Mean gross spread at entry | — | — |
| Mean net spread after costs | > 0.3% | < 0% |
| Win rate (net positive trades) | > 80% | < 60% |
| Mean holding period (days) | — | — |
| Annualised yield on deployed capital | > 20% | < 5% |
| Max carrying cost eaten by funding | — | > 50% of gross spread |

### Baseline Comparison

Compare against:
1. **Null: buy LST and hold** — measures whether timing on discount events adds value vs. passive LST holding (which earns staking yield regardless)
2. **Random entry** — enter at random dates regardless of discount; measures whether the discount trigger adds any edge above the baseline LST yield
3. **Pure ETH hold** — measures total return relative to just holding ETH

### Known Historical Events to Validate Against

| Date | Asset | Event | Expected Signal |
|------|-------|-------|-----------------|
| June 2022 | stETH | 3AC/Celsius crisis — stETH at 0.935 ETH | Pre-Shapella: no arb, but test market convergence speed |
| March 2023 | stETH | SVB banking crisis — brief discount | Pre-Shapella, but close to withdrawal announcement |
| April–May 2023 | stETH | Post-Shapella test period | First genuine arb windows; verify queue length data |
| August 2023 | rETH | Sporadic discount windows | Small but clean post-Shapella arb events |
| October 2023 | cbETH | Coinbase institutional discount | Different dynamics — institutional redeemers |

**The June 2022 event should NOT appear as a profitable arb** (no withdrawal mechanism) — if the backtest shows profit from that period, the logic is wrong.

---

## Entry Rules


### Pre-Entry Checklist (ALL must pass)

1. **Discount confirmed:** `(redemption_rate - market_price) / redemption_rate > threshold`
   - Lido stETH: threshold = 0.50%
   - Rocket Pool rETH: threshold = 0.70%
   - cbETH: threshold = 0.60%
   - mSOL / jitoSOL: threshold = 0.80%

2. **Withdrawal queue operational:** Query `WithdrawalQueueERC721.getLastCheckpointIndex()` — must return current; if reverts or governance pause detected, abort.

3. **Queue wait time acceptable:** Query current queue depth and estimate wait in days. Only enter if estimated wait ≤ 14 days (Ethereum) or ≤ 5 days (Solana).

4. **Carrying cost check:** Calculate `expected_funding_cost = avg_8h_funding_last_7d * (estimated_wait_days * 3)`. Only enter if `gross_spread - carrying_cost - gas_estimate > 0.3%`.

5. **No active governance crisis:** No active Snapshot/Tally vote to halt withdrawals; no unresolved slashing incident > 0.1% of total staked (check Rated.network or beaconcha.in).

6. **Liquidity check:** Verify the LST/ETH Curve pool or relevant DEX has sufficient depth to execute position within 0.2% slippage.

### Entry

| Action | Detail |
|--------|--------|
| Buy LST | Swap ETH → LST via Curve (stETH), Uniswap (rETH), or CEX (cbETH). Target market price, not redemption rate. |
| Short underlying | Short ETH perp on Hyperliquid in equivalent notional to LST position. This hedges ETH price exposure. |
| Submit withdrawal | Immediately after LST purchase, submit withdrawal request on the LST protocol's UI or directly via contract call. This locks in the NAV. |
| Record entry | Log entry price, redemption rate, discount at entry, queue length, expected wait, estimated gas. |

## Exit Rules

### Exit

**Primary exit (arb completion):**
- Withdrawal request processes → receive ETH at NAV
- Close Hyperliquid ETH short at market
- Net P&L = redemption spread minus cumulative funding on short, minus gas, minus entry slippage

**Secondary exit (market convergence before queue completes):**
- If LST market price recovers to within 0.10% of NAV before withdrawal processes:
  - Sell LST at market (faster cash and no gas for redemption)
  - Withdraw queued request if cancellable (Lido allows cancellation)
  - Close perp short
  - This exit captures the same spread with less capital lockup

**Hard stop — exit immediately if:**
- A slashing event is confirmed that impairs > 0.5% of total staked (LST NAV itself is impaired)
- Protocol governance votes to pause withdrawals
- Queue wait extends beyond 30 days and funding cost is accumulating

### No Stop Loss on Price

This trade has **no price stop loss** on the LST leg because the convergence is contractual, not market-driven. A LST price drop in isolation is not a reason to exit — it makes the trade more profitable. The only exits are arb completion, market convergence, or hard-stop protocol failure events.

---

## Position Sizing

### Constraints

1. **Gas efficiency floor:** Minimum position size $5,000 notional (gas cost for Lido withdrawal ~$15–40 at typical gas prices; this must be < 0.5% of position)
2. **Queue capacity ceiling:** Do not enter if position would represent > 5% of the visible queue depth (avoid exacerbating queue delays for yourself)
3. **Protocol concentration limit:** Maximum 30% of total Zunid capital in any single LST protocol
4. **Perp margin:** Maintain 3x buffer above liquidation price on the short ETH leg

### Suggested Sizing Formula

```
max_position = min(
    total_capital * 0.30,                    # protocol concentration cap
    available_perp_liquidity * 0.10,         # perp market impact
    queue_depth_ETH_equivalent * 0.05        # queue capacity limit
)

actual_position = max_position * (net_spread / 1.0%)  # scale with attractiveness
# At 1% net spread: full size; at 0.3% net spread: 30% of max
```

**Initial paper trading size:** $500 notional per trade (small enough to validate mechanics; large enough to test gas economics on testnet or small live position)

---

## Go-Live Criteria

Deploy real capital when:

1. At least 3 complete arb cycles closed (withdrawal processed, not just market convergence exit)
2. Net P&L positive after all costs (fees, gas, funding) across all paper trades
3. Withdrawal queue mechanics verified on-chain (successful test withdrawal on a small position if Ethereum network allows)
4. Pre-entry checklist pipeline automated and tested — no manual steps at entry
5. Hard-stop monitoring (slashing alerts, governance pause alerts) operational before going live
6. Founder approves wallet setup and has reviewed one complete trade cycle end-to-end

---

## Kill Criteria

| Trigger | Action |
|---------|--------|
| After 5 paper trades: net P&L negative after all costs | Kill or redesign (re-examine threshold settings and carrying cost assumptions) |
| After 10 events tracked: fewer than 3 qualifying trades per year | Reclassify as opportunistic (monitor but don't allocate dedicated capital) |
| Mean net spread < 0.2% across closed trades | Kill (costs are too high relative to available spread) |
| Any confirmed LST protocol exploit > $10M | Pause all positions in that protocol; reassess entire strategy |
| Ethereum withdrawal queue consistently > 20 days | Suspend entries until queue normalises |
| stETH/ETH perp becomes widely available → spread compresses permanently below 0.2% | Kill (arb is competed away) |

---

## Risks

### 1. Protocol Insolvency or Smart Contract Exploit
**What happens:** A Lido validator gets slashed at scale, or a smart contract exploit impairs the stETH backing. Redemption NAV falls below entry price. The contract guarantee no longer applies because the backing itself is impaired.

**Likelihood:** Low for Tier 1 (Lido has ~$30B TVL and strong audits); non-trivial for Tier 2.

**Mitigation:**
- Tier 1 only for initial deployment (stETH, rETH)
- Monitor Rated.network daily for validator performance; exit if slashing rate > 3x baseline
- Never deploy more than 30% of capital per protocol
- Consider this the primary unhedgeable risk — size accordingly

---

### 2. Withdrawal Queue Overflow
**What happens:** Entry is made at a 0.8% discount with a 7-day estimated queue. During the trade, panic selling by others floods the queue, extending wait to 45+ days. Cumulative perp funding erodes the spread to zero or negative.

**Likelihood:** Medium — this is exactly what happens during broad panic events, which are also when discounts are largest.

**Mitigation:**
- Check queue length before entry; only enter if estimated wait ≤ 14 days
- Calculate funding carrying cost at entry using current 7-day average funding rate
- Use the secondary exit (sell at market convergence) if the discount closes before queue processes
- Set a 30-day maximum carrying period as a hard review trigger

---

### 3. Depeg Reflects Real Impairment, Not Panic
**What happens:** The discount has widened because an informed party discovered a real problem with the protocol — not because of indiscriminate selling. The discount is a correct signal, not an opportunity.

**Likelihood:** Hard to assess; this is the "unknown unknown" risk.

**Mitigation:**
- Read the reason for the discount before entering — this requires human or LLM-readable event monitoring (CryptoPanic, protocol Discord, Governance forums)
- Never enter a discount event without checking the reason
- If the cause is ambiguous or unknown, do not enter — wait until the cause is identifiable
- This is the step most vulnerable to automation failure; requires deliberate human review rule for large positions

---

### 4. Perp Funding Rate Destroys Spread
**What happens:** During market panic, ETH perp funding can swing strongly negative (shorts pay longs). On a 10-day hold, a funding rate of -0.15% per 8 hours = -4.5% total — more than the typical discount.

**Likelihood:** High during panic events, which is exactly when discounts trigger.

**Mitigation:**
- Always compute expected carrying cost before entry using recent funding rates
- Only enter if `gross_spread - expected_funding_cost - gas > 0.3%`
- Monitor funding daily; if funding deteriorates sharply after entry, consider accelerating to market exit
- Note: Negative funding on ETH shorts means longs pay shorts — this actually favours the trade in some regimes

---

### 5. No Clean Perp Available for the Hedge
**What happens:** Hyperliquid does not list stETH perps. The only available hedge is an ETH perp, which hed

## Data Sources

TBD
