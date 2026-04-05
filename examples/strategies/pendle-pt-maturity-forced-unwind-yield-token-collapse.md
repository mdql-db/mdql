---
title: "Pendle PT Maturity Forced Unwind — Yield Token Collapse"
status: HYPOTHESIS
mechanism: 9
implementation: 2
safety: 7
frequency: 3
composite: 378
categories:
  - defi-protocol
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Pendle Principal Tokens (PT) are contractually guaranteed to redeem at exactly 1.0 of the underlying asset at maturity. When PT trades below par within 14 days of maturity, the discount represents a risk-free yield with a hard smart-contract expiry date. The convergence is not probabilistic — it is enforced by the `PendleMarket` redemption function. Any PT/par ratio below 1.0 at T-0 is an exploit of the contract itself, which cannot occur. Therefore, buying PT at a discount within the final 14-day window and holding to maturity is structurally equivalent to buying a zero-coupon bond at a discount when the issuer is the smart contract itself.

The secondary play — shorting YT as it approaches zero — is directionally correct but mechanically weaker: YT price decay is guaranteed in direction but not in rate, and thin liquidity makes execution costly.

**Primary trade: PT discount capture. Secondary trade: YT decay short (lower priority, higher friction).**

---

## Structural Mechanism

### PT Convergence (Score: 8/10 — Smart Contract Guarantee)

1. Pendle's `PendleMarket` contract splits a yield-bearing token (e.g., stETH, aUSDC) into PT and YT at issuance.
2. At maturity block `T`, the contract's `redeemPY()` function allows any PT holder to redeem exactly 1 PT for exactly 1 unit of the underlying asset — no oracle, no governance vote, no admin key required.
3. If PT trades at 0.96 par with 10 days to maturity, the annualised yield is approximately `(0.04 / 0.96) × (365 / 10) = 152% APY` — risk-free if the underlying asset holds value.
4. The discount exists because: (a) retail YT buyers who purchased PT as a byproduct need to exit, (b) AMM pricing mechanics create temporary dislocations as YT demand collapses, (c) most participants are passive depositors who do not actively monitor expiry calendars.
5. The AMM's implied yield curve steepens mechanically as maturity approaches because YT time value approaches zero — this is a mathematical identity, not an empirical tendency.

### YT Collapse (Score: 6/10 — Directionally Guaranteed, Rate Uncertain)

1. YT represents the right to receive yield accrued on the underlying from now until maturity.
2. At maturity, YT redeems for exactly the yield accrued — which for fixed-rate pools is zero additional value beyond what has already been distributed.
3. YT's time value decays to zero by definition: a claim on future yield has no future yield after maturity.
4. Holders who have not sold by T-0 receive zero residual value from time-value component — the binary is contractually enforced.
5. The rate of decay is uncertain because it depends on implied yield movements, but the terminal value is not.

---

## Universe Filters

Apply ALL of the following filters before considering a pool eligible:

| Filter | Threshold | Rationale |
|---|---|---|
| Pool TVL | ≥ $2M | Minimum liquidity to enter/exit without excessive slippage |
| PT notional outstanding | ≥ $1M | Ensures meaningful market depth |
| Days to maturity | ≤ 14, ≥ 1 | Window where discount/annualised yield is attractive; exclude final day (redemption gas risk) |
| PT/par discount | ≥ 1% (i.e., PT price ≤ 0.99) | Minimum edge after gas and slippage |
| Underlying asset | Liquid, non-exotic (stETH, USDC, USDT, wBTC, ETH) | Eliminates underlying collapse risk |
| Smart contract audit status | Audited by ≥ 2 firms | Reduces smart contract risk |
| Pool version | Pendle V2 only | V1 has deprecated redemption mechanics |

---

## Entry Rules

### Primary Trade: PT Discount Capture

**Step 1 — Scan**
- Query Pendle API endpoint `GET /v1/markets` daily at 09:00 UTC.
- Filter for pools meeting all universe criteria above.
- Calculate `PT_discount = 1 - PT_price_in_underlying`.
- Calculate `annualised_yield = (PT_discount / PT_price) × (365 / days_to_maturity)`.

**Step 2 — Entry Trigger**
- Enter when `annualised_yield > 20% APY` AND `PT_discount > 1%` AND `days_to_maturity ≤ 14`.
- Do NOT enter if `days_to_maturity = 0` (redemption-day gas race risk).
- Do NOT enter if pool TVL has dropped >30% in the prior 24 hours (liquidity flight signal).

**Step 3 — Execution**
- Buy PT directly on Pendle AMM via `app.pendle.finance` or direct contract interaction.
- Set slippage tolerance ≤ 0.5%; abort if slippage exceeds this (thin pool).
- Record: entry PT price, underlying asset price at entry, entry timestamp, pool address, maturity block.

**Step 4 — Hedge (Optional, for large positions)**
- If position size > $50k notional, hedge underlying asset price risk by shorting equivalent notional of underlying on Hyperliquid perpetuals.
- This converts the trade from "PT discount capture + underlying price exposure" to "pure discount capture."
- Hedge ratio: 1:1 notional. Unwind hedge at maturity simultaneously with PT redemption.

### Secondary Trade: YT Decay Short (Lower Priority)

- Enter short YT position on Pendle AMM when `days_to_maturity ≤ 7` and YT price implies >50% annualised yield (i.e., YT is overpriced relative to remaining time value).
- Maximum allocation: 20% of total strategy capital (high friction, thin liquidity).
- Exit: Close before T-2 days to avoid redemption-day liquidity collapse.

---

## Exit Rules

### Primary Trade

| Scenario | Action |
|---|---|
| Maturity reached (T-0) | Call `redeemPY()` on `PendleMarket` contract; receive 1.0 underlying per PT |
| PT price converges to par before maturity (discount < 0.3%) | Sell PT on AMM if annualised remaining yield < 5% APY; lock in gain early |
| Underlying asset drops >15% from entry | Evaluate hedge adequacy; if unhedged, consider early exit to limit underlying loss |
| Pool TVL drops >50% from entry | Emergency exit via AMM; accept slippage |
| Smart contract exploit detected on Pendle | Emergency exit immediately regardless of loss |

### Secondary Trade (YT Short)

- Hard close at T-2 days regardless of P&L.
- Close earlier if YT price drops >80% from entry (most of the gain is captured).

---

## Position Sizing

- **Maximum per pool:** 2% of total strategy capital, capped at $100k notional per pool.
- **Maximum total strategy exposure:** 10% of total strategy capital across all open PT positions simultaneously.
- **Rationale for small sizing:** Smart contract risk is binary — a Pendle exploit would result in total loss of all PT positions simultaneously. Concentration risk is correlated, not independent.
- **Scaling rule:** If a pool has TVL > $10M and discount > 3%, allow up to 4% of strategy capital (double the base limit).
- **YT short allocation:** Hard cap at 20% of total strategy capital allocated to this strategy.

---

## Backtest Methodology

### Data Requirements

| Data Source | Fields Required | Access |
|---|---|---|
| Pendle API (`api.pendle.finance/core/v1`) | PT price, YT price, pool TVL, maturity date, implied APY | Free, public |
| Pendle subgraph (The Graph) | Historical PT/YT prices, swap events, liquidity events | Free, public |
| Ethereum mainnet RPC | `redeemPY()` call data, block timestamps | Free via Alchemy/Infura |
| Coingecko/CoinMarketCap | Underlying asset price history | Free tier sufficient |

### Backtest Steps

1. **Pull historical Pendle market data** from subgraph for all V2 pools from Pendle V2 launch (June 2023) to present.
2. **Identify all maturity events** where a pool had ≥$1M TVL and ≥1 day remaining.
3. **For each pool, reconstruct PT price series** in the T-14 to T-0 window using subgraph swap data.
4. **Apply entry filter:** Flag all days where `annualised_yield > 20%` and `PT_discount > 1%`.
5. **Simulate entry:** Record entry price, apply 0.3% slippage assumption (conservative for pools >$2M TVL).
6. **Simulate exit:** Record redemption at par (1.0) at maturity block.
7. **Calculate P&L:** `(1.0 - entry_price - slippage - gas_cost_in_underlying) / entry_price`.
8. **Segment results by:** pool size, underlying asset, days-to-maturity at entry, discount magnitude.
9. **Stress test:** Identify any historical instances where PT did NOT converge to par (should be zero for V2; document any anomalies).
10. **Calculate Sharpe, max drawdown, win rate, average hold period.**

### Key Backtest Validation Questions

- Has PT EVER failed to redeem at par on Pendle V2? (Expected answer: No — document if otherwise.)
- What is the distribution of PT discounts in the T-14 window across all historical pools?
- What percentage of pools had sufficient liquidity to enter at <0.5% slippage?
- What is the average annualised yield captured after slippage and gas?
- Does the edge concentrate in specific underlying assets or pool sizes?

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

- [ ] Backtest covers ≥ 20 distinct pool maturity events on Pendle V2.
- [ ] Zero instances of PT failing to redeem at par in historical data.
- [ ] Average net annualised yield after slippage and gas ≥ 15% APY across backtest sample.
- [ ] Win rate ≥ 95% (allowing for early exits at loss due to underlying price moves on unhedged positions).
- [ ] Paper trade ≥ 3 live maturity events with simulated execution before real capital.
- [ ] Smart contract risk assessment completed: confirm no admin keys can pause redemption.
- [ ] Gas cost model validated: confirm `redeemPY()` gas cost < 0.1% of position notional at current gas prices.
- [ ] Legal/compliance review: confirm PT purchase and redemption is permissible in operating jurisdiction.

---

## Kill Criteria

Immediately halt strategy and exit all positions if ANY of the following occur:

| Trigger | Action |
|---|---|
| Any Pendle V2 smart contract exploit reported | Exit all positions immediately; accept any slippage |
| PT fails to redeem at par at maturity on any pool | Full strategy halt; investigate root cause before resuming |
| Pendle governance vote passes that modifies redemption mechanics | Halt new entries; monitor existing positions |
| Average slippage on entry exceeds 1% across 3 consecutive trades | Pause; re-evaluate liquidity filters |
| Underlying asset (e.g., stETH) depegs >5% from ETH | Exit unhedged positions; re-evaluate hedge model |
| Strategy drawdown exceeds 5% of allocated capital | Halt; review position sizing and filter parameters |

---

## Risks

| Risk | Severity | Probability | Mitigation |
|---|---|---|---|
| Pendle V2 smart contract exploit | Critical (total loss) | Low | Position size cap 2% per pool; 10% total cap; audit verification |
| Underlying asset depeg (e.g., stETH/ETH) | High | Low-Medium | Optional Hyperliquid hedge; underlying asset filter |
| Liquidity collapse before maturity | Medium | Low-Medium | TVL drop kill switch; slippage cap on entry |
| Gas cost spike on redemption day | Low | Medium | Pre-calculate gas cost; redeem during low-gas windows |
| Pendle AMM slippage on entry | Medium | Medium | 0.5% slippage cap; pool TVL filter ≥$2M |
| Regulatory action against Pendle protocol | High | Very Low | Monitor; no mitigation beyond position sizing |
| Opportunity cost (capital locked until maturity) | Low | High | Maximum 14-day lock; size accordingly |
| YT short: liquidity too thin to close | Medium | Medium | Hard T-2 exit rule; 20% capital cap on YT shorts |

---

## Data Sources

| Source | URL | Usage | Cost |
|---|---|---|---|
| Pendle REST API | `api.pendle.finance/core/v1/` | Live market scanning, PT prices, maturity dates | Free |
| Pendle Subgraph | `thegraph.com/hosted-service/subgraph/pendle-finance/core` | Historical price reconstruction for backtest | Free |
| Ethereum RPC | Alchemy / Infura free tier | Contract interaction, redemption calls | Free (low volume) |
| Etherscan | `etherscan.io` | Contract verification, audit confirmation | Free |
| Coingecko API | `api.coingecko.com/api/v3` | Underlying asset price history | Free |
| Hyperliquid API | `api.hyperliquid.xyz` | Hedge execution for underlying price risk | Free |
| DeFiLlama | `defillama.com/protocol/pendle` | TVL history, pool-level data | Free |

---

## Implementation Notes

### Monitoring Script (Pseudocode)

```python
# Run daily at 09:00 UTC
markets = pendle_api.get_markets(chain="ethereum")

for market in markets:
    if market.tvl < 2_000_000: continue
    if market.pt_outstanding < 1_000_000: continue
    if not (1 <= market.days_to_maturity <= 14): continue
    if market.underlying not in APPROVED_UNDERLYINGS: continue

    pt_price = market.pt_price_in_underlying
    discount = 1 - pt_price
    ann_yield = (discount / pt_price) * (365 / market.days_to_maturity)

    if discount >= 0.01 and ann_yield >= 0.20:
        alert(f"ENTRY SIGNAL: {market.name} | Discount: {discount:.2%} | "
              f"Ann. Yield: {ann_yield:.1%} | Days: {market.days_to_maturity}")
```

### Redemption Execution

- At maturity, call `PendleMarket.redeemPyToToken(receiver, netPyIn, output)` directly.
- Do not rely on UI — write a redemption script that monitors maturity block and executes within 10 blocks of maturity to avoid gas competition.
- Test redemption script on a small position ($100 notional) before deploying full capital.

---

## Open Questions for Researcher Review

1. **Has any Pendle V2 pool ever had a redemption failure?** Confirm via subgraph query before go-live.
2. **What is the actual historical distribution of PT discounts in T-14 window?** Hypothesis is that discounts >1% exist; needs empirical confirmation.
3. **Is the YT short executable at meaningful size?** YT liquidity is reportedly <$500k — this may make the secondary trade impractical above $10k notional.
4. **Does the Hyperliquid hedge introduce basis risk?** If underlying is stETH and Hyperliquid only lists ETH perps, the hedge is imperfect. Quantify the basis.
5. **What is the tax treatment of PT redemption at par?** In some jurisdictions, the discount captured may be treated as ordinary income, not capital gains — affects net yield calculation.

---

*This document is a hypothesis specification. No backtest has been run. No live trading has occurred. All yield figures are illustrative. Do not deploy capital until go-live criteria are satisfied.*
