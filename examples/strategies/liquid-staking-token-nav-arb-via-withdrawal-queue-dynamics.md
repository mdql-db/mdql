---
title: "LST Discount-to-NAV Mechanical Convergence"
status: HYPOTHESIS
mechanism: 7
implementation: 5
safety: 7
frequency: 3
composite: 735
categories:
  - lst-staking
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Liquid staking tokens (stETH, rETH) are contractually redeemable for ETH at a published on-chain exchange rate. When secondary market prices fall below that rate, the discount has a **calculable maximum** determined by withdrawal queue wait time and staking yield. When observed discount exceeds this maximum rational discount, patient capital can buy LST, short ETH perps, and wait for mechanical convergence — collecting both the discount and staking yield while delta-neutral.

---

## Why It's an Edge

The edge is structural, not statistical. Three facts combine to create it:

**Fact 1 — NAV is contractually exact, not estimated.**
The stETH/ETH exchange rate is published by `Lido.getPooledEthByShares()`. The rETH/ETH rate by `RocketPoolToken.getExchangeRate()`. These are the protocol's own accounting, updated every block. The NAV is not an opinion.

**Fact 2 — Redemption is guaranteed (conditional on protocol solvency).**
Any holder can submit a withdrawal request and receive ETH at the published NAV. This is a hard contractual claim. The only reason to accept less on the secondary market is unwillingness to wait in the withdrawal queue.

**Fact 3 — The rational maximum discount is calculable.**
A seller should accept at most `(queue_wait_days × daily_staking_yield)` less than NAV, because that is what they sacrifice by waiting. Add a liquidity/gas premium (~0.5%) and you have a **hard ceiling on rational discount**. Discounts beyond this ceiling are irrational — caused by panic, forced liquidation, or information asymmetry — and are mechanically self-correcting.

The causal mechanism is identical in structure to token unlock shorts: a contractually guaranteed future event (queue clearance → NAV redemption) disciplines current market price. The only uncertainty is timing (queue length) and the tail risk that the protocol itself is insolvent — in which case the NAV itself is wrong.

This is **not** a bet that stETH will recover after a depeg. It is a bet that a measurable mispricing relative to a contractual entitlement will close — the same logic as buying a bond at a discount to par when maturity is guaranteed.

---

## Proposed Backtest Methodology

### Dataset

| Data series | Source | Resolution | Period |
|---|---|---|---|
| stETH/ETH price ratio | Curve stETH/ETH pool (The Graph / Dune) | Daily close | 2022-01-01 to present |
| stETH NAV (protocol exchange rate) | Lido `getPooledEthByShares()` via Dune | Daily | 2022-01-01 to present |
| Lido withdrawal queue length (days) | Lido API + Dune (`lido_ethereum.withdrawals`) | Daily | 2023-04-12 (Shapella) to present |
| ETH perpetual funding rate | Binance/Hyperliquid API | 8-hourly | 2022-01-01 to present |
| Curve swap fee | Fixed 0.04% (historical Curve stETH pool fee) | Constant | — |
| Ethereum gas cost | Etherscan gas tracker history | Daily avg gwei | 2022-01-01 to present |

**Note on pre-Shapella data:** Before April 2023, ETH withdrawals were not possible. All pre-Shapella discounts had a theoretically infinite queue — there was no withdrawal mechanism. Pre-Shapella discounts are a **different regime** (discounts were a pure secondary market phenomenon with no convergence mechanism). The structural trade described here only applies **post-Shapella (April 2023 onwards)**. Pre-Shapella data should be catalogued separately to understand what drove discounts but must not be used to validate the convergence mechanism.

### Discount Calculation

At each daily close:

```
observed_discount = 1 - (stETH_curve_price / stETH_NAV)
queue_days        = lido_queue_length_ETH / daily_processing_rate_ETH
daily_yield       = 0.0108%  # ~3.95% APR / 365
rational_max_disc = (queue_days × daily_yield) + 0.005  # 0.5% gas/liquidity buffer
excess_discount   = observed_discount - rational_max_disc
```

Entry signal fires when `excess_discount > 0`.

### Trade Simulation

For each entry signal:

1. **Day 0:** Buy stETH on Curve (pay 0.04% swap fee + gas). Short equivalent ETH notional on perps (pay taker fee 0.045%).
2. **Daily:** Accrue staking yield on stETH position. Accrue funding rate cost/revenue on ETH short.
3. **Exit:** First of: (a) stETH/ETH ratio ≥ 0.9950, (b) queue_days < 2, (c) day 30, (d) discount widens to 3% (stop loss).
4. **Close:** Sell stETH on Curve. Close ETH short.

### P&L Components

```
gross_pnl    = (exit_NAV_ratio - entry_NAV_ratio) × notional
staking_carry = daily_yield × hold_days × notional
funding_cost  = sum(8h_funding_rate) × notional  [sign depends on rate direction]
fees          = 0.04% (Curve in) + 0.04% (Curve out) + 0.09% (perp round trip) + gas_USD
net_pnl       = gross_pnl + staking_carry - funding_cost - fees
```

### Baseline

Compare each entry event against:

- **Hold ETH baseline:** What would holding equivalent ETH have returned over same hold period (measures whether delta hedge is working)
- **Random entry baseline:** Enter on random dates (no discount signal) with same exit rules — measures whether the discount threshold adds value vs. noise
- **Full-queue-wait baseline:** Enter withdrawal queue directly instead of perp hedge — measures whether the perp hedge improves or hurts risk-adjusted return

### Metrics to Report

| Metric | Target |
|---|---|
| Number of qualifying entry events (post-Shapella) | Report actual count |
| Mean net P&L per trade (%) | > 0.5% net of all costs |
| Median hold time to exit (days) | < 20 days |
| Win rate | > 70% |
| Max drawdown on open position | < 3% (before stop) |
| Worst single trade net P&L | Document explicitly |
| Correlation to ETH spot returns | < 0.2 (tests delta neutrality) |
| Funding cost as % of gross P&L | Document to assess bleed risk |

### Key Historical Events to Stress-Test

| Event | Date | Peak Discount | Why Important |
|---|---|---|---|
| Celsius / 3AC crisis | Jun 2022 | ~8% | Pre-Shapella — no queue mechanism. Document but exclude from convergence backtest. |
| FTX collapse | Nov 2022 | ~6.5% | Pre-Shapella. Same exclusion. |
| stETH mini-depeg | May 2023 | ~0.7% | Post-Shapella. First real test of the mechanism. |
| Restaking narrative shift | 2024 various | ~0.2–0.5% | Small but frequent. Tests whether threshold is too tight. |

The 2022 events are the most important to **understand** even if excluded: a live protocol stress event would look similar. Studying how the discount evolved and whether it would have triggered the stop-loss informs the tail-risk model.

---

## Entry Rules

### Preconditions (all must be true before monitoring for signal)

1. Lido withdrawal queue is actively processing (check Lido API `withdrawalQueue.isPaused()` → must be `false`)
2. No active security incident or governance emergency on Lido (check Lido Status page / governance forum — manual check initially, automate later via RSS)
3. Net carry is positive: `(queue_days × daily_yield) + expected_discount_convergence > (expected_funding_cost + estimated_fees)`
4. Curve pool depth > $50M (ensures exit liquidity for position sizes up to $5,000)

### Entry Signal

```
entry = True
  if observed_discount > rational_max_discount
  and excess_discount > 0.10%  # minimum excess to cover any estimation error
  and preconditions all satisfied
```

### Position Construction

| Leg | Action | Venue | Size |
|---|---|---|---|
| Long LST | Buy stETH with ETH | Curve stETH/ETH pool | $X notional |
| Short ETH | Short ETH-PERP | Hyperliquid | $X notional |

Maintain delta neutrality: rebalance if ETH price moves >5% (stETH/ETH quantity gets mismatched).

---

## Exit Rules

Exit on **first** of the following conditions:

| Condition | Action | Rationale |
|---|---|---|
| stETH/ETH ≥ 0.9950 | Full exit both legs | Convergence target reached |
| Withdrawal queue < 2 days | Full exit both legs | Impatience premium gone; risk/reward no longer favourable |
| 30 calendar days elapsed | Full exit both legs | Maximum hold; something structural may be wrong |
| Discount widens to > 3.0% | Stop loss — full exit | Signals non-queue reason (protocol risk); cut before worse |
| Protocol incident detected | Emergency exit | Override all other conditions |

---

## Position Sizing

### Paper trading phase
- **$500 notional per trade** (smaller than token unlock shorts given higher operational complexity)
- Maximum 1 open trade at a time during paper phase

### Live trading phase (if paper validates)
- Base size: **$1,000–$2,000 per trade**
- Scale with excess discount: `size = base × min(excess_discount / 0.5%, 2.0)` — larger size when mispricing is larger
- Hard cap: **$5,000 notional** until 10+ live trades complete
- Maximum portfolio allocation to this strategy: **20% of deployed capital** (illiquidity premium: stETH exit requires Curve swap, not instant)
- Never exceed 0.5% of Curve pool depth to avoid meaningful price impact on exit

### Leverage
- The delta-neutral structure uses no net directional leverage
- ETH short on Hyperliquid: use **1x–2x only** (no amplification needed; the edge is the discount, not leverage)

---

## Go-Live Criteria

Deploy real capital when ALL of the following are met:

1. Backtest shows ≥ 3 post-Shapella qualifying entry events with positive net P&L
2. Mean net P&L per backtest trade > 0.5% after all costs
3. At least 2 **paper trades closed** with net P&L positive
4. Funding cost model validated: actual ETH perp funding costs during paper trades match the pre-entry estimate within 50%
5. Operational checklist confirmed:
   - Automated stETH/ETH ratio monitor running (checked every 4 hours)
   - Withdrawal queue API integration tested
   - Curve swap execution tested (paper)
   - Hyperliquid ETH short execution tested (paper)
   - Stop-loss alert system tested
6. Founder approves: Ethereum mainnet wallet for Curve swaps, USDC/ETH on mainnet, Hyperliquid account

---

## Kill Criteria

| Trigger | Action |
|---|---|
| Backtest shows < 3 qualifying events post-Shapella | Kill — insufficient frequency to justify infrastructure build |
| Backtest mean net P&L < 0.3% after all costs | Kill — fees and funding eat the edge |
| After 3 paper trades: net P&L negative | Kill or redesign — entry threshold may need widening |
| After 5 paper trades: Sharpe < 0.5 annualised | Kill — risk-adjusted return insufficient |
| Lido implements fee change that makes withdrawals costless and instant | Kill — edge mechanism no longer applies |
| Any trade triggers stop-loss (>3% discount widening) during paper phase | Do not kill automatically, but investigate cause before next entry |
| Two consecutive stop-losses during live phase | Halt and investigate |

---

## Risks

### Tier 1 — Strategy-killing risks

| Risk | Description | Mitigation |
|---|---|---|
| Protocol insolvency / smart contract exploit | Actual destruction of NAV — stETH backs <1 ETH in reality. This is the only scenario where the convergence mechanism breaks. | Hard stop at 3% discount widening (pre-exploit, discount widens fast). Never enter during active security incident. Cap allocation to 20% of portfolio. |
| Governance attack (malicious upgrade) | Lido governance could theoretically pass an upgrade that changes withdrawal terms. | Monitor Lido governance forum. Lido uses 48h timelock on upgrades — protocol change cannot happen without warning. |

### Tier 2 — Profitability risks

| Risk | Description | Mitigation |
|---|---|---|
| ETH perp funding costs | If ETH perp funding runs persistently positive (longs pay shorts), the short ETH leg bleeds carry. | Only enter when net carry positive. Check 30-day trailing funding as proxy. Exit if funding flips and erodes >50% of expected convergence P&L. |
| Gas costs on Ethereum mainnet | Mainnet swaps cost $10–50 in gas depending on conditions. | Include gas in entry threshold calculation. Only enter when discount is large enough that gas is <20% of expected P&L. Consider L2 wrappers (wstETH on Arbitrum) as alternative venue. |
| Slow queue processing | Queue clears slower than modelled; carry still accrues but capital is locked longer. | 30-day maximum hold enforced regardless. Adjust model if queue processing rates change. |
| stETH secondary market illiquidity | Large exit during stress could move Curve pool against us. | Hard cap at 0.5% of pool depth. Monitor pool depth before entry. |

### Tier 3 — Operational risks

| Risk | Description | Mitigation |
|---|---|---|
| Curve/Lido UI downtime | Cannot exit cleanly during incident. | Use direct contract calls, not UI. Have backup RPC endpoints. |
| Hyperliquid outage | Cannot close ETH short. | Monitor via separate status check. Have manual close procedure documented. |
| Delta drift | ETH price moves >5% making position no longer delta-neutral. | Automated rebalance trigger at 5% ETH move. |

---

## Data Sources

| Data | Source | Access Method | Cost |
|---|---|---|---|
| stETH/ETH spot ratio (Curve) | Dune Analytics query or The Graph (Curve subgraph) | REST API / Dune API | Free |
| stETH NAV (protocol rate) | Lido contract `getPooledEthByShares(1e18)` | Ethereum RPC call (Alchemy/Infura free tier) | Free |
| rETH exchange rate | Rocket Pool contract `getExchangeRate()` | Ethereum RPC call | Free |
| Lido withdrawal queue state | Lido API `https://stake.lido.fi/api/withdrawal-queue-info` | REST GET | Free |
| Lido queue processing history | Dune: `lido_ethereum.withdrawals` table | Dune API | Free (rate-limited) |
| ETH perp funding rate | Hyperliquid `/info` with `fundingHistory` or Binance `/fapi/v1/fundingRate` | REST API | Free |
| Ethereum gas price | Etherscan API `gastracker` | REST API | Free (rate-limited) |
| Lido incident/governance alerts | Lido governance forum RSS + Lido Twitter | RSS/scrape | Free |

---

## Implementation Notes

### Monitoring loop (runs every 4 hours)

```python
# Pseudocode — not production
steth_nav    = lido_contract.getPooledEthByShares(1e18) / 1e18
steth_price  = curve_pool.get_dy(0, 1, 1e18) / 1e18  # ETH received for 1 stETH
discount     = 1 - (steth_price / steth_nav)

queue_info   = requests.get(LIDO_QUEUE
