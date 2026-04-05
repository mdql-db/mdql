---
title: "LUSD Redemption Threshold Front-Run — Riskiest Trove Closure Pressure"
status: HYPOTHESIS
mechanism: 9
implementation: 2
safety: 7
frequency: 3
composite: 378
categories:
  - defi-protocol
  - stablecoin
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When LUSD trades below peg on Curve, Liquity's smart contract redemption mechanism creates two predictable, mechanical flows:

1. **Arb flow:** Buyers of discounted LUSD can redeem at face value ($1.00 of ETH per LUSD), guaranteeing a spread capture minus the redemption fee. This is a smart-contract-enforced convergence trade.

2. **Trove closure pressure:** Redemptions forcibly close the lowest-collateral-ratio troves first — a deterministic ordering enforced by the `SortedTroves` contract. Trove owners facing imminent forced closure have a dominant game-theoretic incentive to self-close before being redeemed (avoiding the redemption fee being charged to them). Both forced redemptions and voluntary pre-emptive closures produce the same mechanical flow: ETH collateral is returned to the market and LUSD is burned. This is a calculable, on-chain-observable pressure event.

The edge is not "ETH tends to fall when LUSD depegs." The edge is: **a specific, enumerable set of ETH positions will be liquidated or self-liquidated within a bounded time window, and the size of that flow is readable from the chain before it happens.**

---

## Structural Mechanism

### Liquity Redemption Mechanics (the "dam")

Liquity Protocol enforces a hard floor on LUSD via its redemption module:

- Any LUSD holder may call `redeemCollateral()` at any time
- The protocol redeems LUSD at exactly $1.00 face value in ETH (using Chainlink oracle price)
- Redemptions are processed starting from the **lowest ICR (Individual Collateral Ratio) trove**, ascending
- The redemption fee is `max(0.5%, baseRate)` where `baseRate` decays over time but spikes after each redemption event
- Trove owners whose troves are redeemed receive their ETH collateral back **minus the redemption fee** — they have no veto

This creates a **contractually guaranteed** price floor mechanism. When LUSD < $1.00 - redemption_fee, the arb is risk-free in the absence of execution failure. The floor is not statistical — it is enforced by Solidity.

### The Sorted Troves Queue (the "hit list")

The `SortedTroves` contract maintains a doubly-linked list of all troves ordered by ICR in real time. The bottom of this list is public, deterministic, and updated on every block. There is no ambiguity about which troves will be redeemed first.

### Game-Theoretic Forced Action

Trove owners at the bottom of the sorted list face a binary choice when LUSD depegs:

| Action | Outcome |
|---|---|
| Do nothing | Trove is redeemed by arb bot; owner receives ETH minus redemption fee (0.5–2%+) |
| Self-close trove | Owner repays LUSD debt, receives full ETH collateral, pays only 0.5% one-time fee |
| Top up collateral | Moves up the sorted list, escaping the redemption queue |

Self-closing requires buying LUSD on the open market (or using held LUSD) to repay debt → **LUSD buy pressure**. Returning ETH collateral to the owner who then sells → **ETH sell pressure**. Both flows are directionally predictable and size-calculable from on-chain data.

### Why This Is Not Priced In

- The trove list is public but requires active monitoring of a niche protocol contract
- Most trove owners are retail users who opened positions and forgot them
- The time window between LUSD depeg onset and redemption execution is typically 30 minutes to several hours — not milliseconds
- The ETH flow from returned collateral is diffuse (many small troves) and not tracked by standard market data feeds

---

## Sub-Strategy A: LUSD Depeg Arb (Direct Redemption)

### Entry Rules

1. Monitor LUSD/3CRV Curve pool price continuously (on-chain or via The Graph)
2. **Trigger:** LUSD spot price ≤ $0.9950 (i.e., discount ≥ 0.50%) for ≥ 2 consecutive 5-minute candles
3. Fetch current `baseRate` from `TroveManager` contract to calculate exact redemption fee
4. **Execute only if:** `(1.00 - LUSD_spot_price) > redemption_fee + gas_cost_in_pct`
5. Buy LUSD on Curve, call `redeemCollateral()`, receive ETH

### Exit Rules

- Position closes atomically on redemption — ETH received is the exit
- Immediately sell ETH on spot or perp if net long ETH exposure is undesired
- No time-based exit needed; redemption is synchronous

### Position Sizing

- Size = `min(available_LUSD_liquidity_at_price, trove_collateral_at_bottom_of_queue)`
- Do not attempt to redeem more LUSD than exists in the bottom troves at target ICR — partial redemptions leave residual troves and increase baseRate for subsequent redemptions
- Maximum single redemption: 50% of Curve pool depth at trigger price to avoid self-slippage
- Capital at risk: LUSD purchase price (stablecoin risk only during transaction window, typically <60 seconds)

### Expected P&L

| Scenario | Gross Spread | Redemption Fee | Gas (~$15) | Net |
|---|---|---|---|---|
| LUSD = $0.995, baseRate = 0% | 0.50% | 0.50% | ~0.05% on $30k | ~0% |
| LUSD = $0.990, baseRate = 0% | 1.00% | 0.50% | ~0.03% on $50k | ~0.47% |
| LUSD = $0.985, baseRate = 1% | 1.50% | 1.00% | ~0.02% on $75k | ~0.48% |

**Honest assessment:** Sub-Strategy A is competitive. Other arb bots monitor the same signal. The edge here is being fast enough on-chain (not HFT-fast — normal transaction submission is sufficient) and having capital pre-positioned. This is a **known arb** with thin but real margins. Score: **6/10** standalone.

---

## Sub-Strategy B: Trove Closure Pressure Trade (Perp Short)

This is the higher-alpha, less-competed leg.

### Entry Rules

1. **Condition 1:** LUSD/USD ≤ $0.998 on Curve for ≥ 30 consecutive minutes
2. **Condition 2:** Fetch bottom 20 troves from `SortedTroves.getFirst()` / `getNext()` — calculate total ETH collateral at risk (`Σ ETH_collateral_i` for troves with ICR < 115%)
3. **Condition 3:** Total at-risk ETH collateral ≥ 50 ETH (minimum flow threshold to justify trade)
4. **Condition 4:** Current ETH perp funding rate is not strongly positive (>0.05%/8h) — avoid paying excessive carry against the short
5. **Execute:** Short ETH perp on Hyperliquid, sized to 30–60% of at-risk ETH collateral (see sizing below)

### Exit Rules

| Trigger | Action |
|---|---|
| LUSD reprices above $0.999 for >15 min | Close short — redemption pressure abating |
| 80% of at-risk troves confirmed closed on-chain | Close short — flow has occurred |
| 48 hours elapsed since entry | Close short — if flow hasn't happened, thesis is stale |
| ETH perp moves +5% against position | Stop loss — close 50% of position |
| ETH perp moves +8% against position | Stop loss — close remaining position |

### Position Sizing

```
at_risk_eth = Σ(ETH_collateral for troves with ICR < 115%)
base_size_eth = at_risk_eth × 0.40
max_size_usd = min(base_size_eth × ETH_price, $200,000)
leverage = 2x–3x (low leverage; this is a flow trade, not a momentum trade)
```

**Rationale for 40% scalar:** Not all at-risk trove owners will sell returned ETH immediately. Some will re-open troves. Some redemptions will be partial. 40% is a conservative estimate of net ETH market sell flow. This scalar should be calibrated against historical redemption events.

### Expected P&L Profile

- **Win case:** 2–5% ETH move down over 6–24 hours as trove closures cascade. At 3x leverage on 40% of collateral, this is a 2.4–4% return on capital deployed.
- **Loss case:** ETH rallies (increasing ICR of at-risk troves, reducing redemption pressure). Stop at 5–8% adverse move.
- **Expected Sharpe (hypothesis):** Unknown — requires backtest. Directional ETH trades have high variance; the structural edge narrows but does not eliminate this variance.

---

## Combined Strategy Execution

```
MONITOR LOOP (every 5 minutes):
  lusd_price = get_curve_pool_price(LUSD_3CRV)
  base_rate = TroveManager.baseRate()
  at_risk_troves = get_bottom_troves(SortedTroves, n=20)
  at_risk_eth = sum(trove.collateral for trove in at_risk_troves if trove.ICR < 1.15)

  IF lusd_price < 0.998 AND duration > 30min:
    IF (1.00 - lusd_price) > base_rate + 0.005 + gas_pct:
      EXECUTE Sub-Strategy A (direct redemption arb)
    IF at_risk_eth > 50:
      EXECUTE Sub-Strategy B (ETH perp short)

  IF lusd_price > 0.999:
    CLOSE Sub-Strategy B positions
    RESET duration counter
```

---

## Backtest Methodology

### Target Events

Identify all historical LUSD depeg events (price < $0.998 for >30 min) from Curve pool data. Known candidates:

- **June 2022:** LUSD dropped to ~$0.97 during Terra/LUNA contagion
- **November 2022:** FTX collapse, LUSD ~$0.975
- **March 2023:** USDC depeg contagion, LUSD ~$0.985
- **Multiple smaller events:** 2022–2023 bear market

### Data Required

| Dataset | Source | Availability |
|---|---|---|
| LUSD/3CRV price history | Curve subgraph / The Graph | ✅ Public |
| Trove state history (ICR, collateral) | Liquity subgraph / Ethereum archive node | ✅ Public |
| Redemption events (tx history) | Etherscan / Dune Analytics | ✅ Public |
| ETH spot/perp price history | Hyperliquid, Binance | ✅ Public |
| baseRate history | TroveManager contract events | ✅ Public |

### Backtest Steps

1. **Reconstruct trove queue state** at each 5-minute interval during depeg events using Liquity subgraph
2. **Simulate Sub-Strategy A:** For each depeg event, calculate theoretical arb P&L net of fees and gas
3. **Simulate Sub-Strategy B:** At each entry trigger, record at-risk ETH collateral, then measure ETH price change over next 6/12/24/48 hours
4. **Measure actual redemption flow:** Cross-reference with on-chain redemption transactions to validate that at-risk troves were actually closed and in what timeframe
5. **Calibrate the 40% scalar:** Compare actual ETH sold post-redemption vs. at-risk collateral
6. **Calculate Sharpe, max drawdown, win rate** for Sub-Strategy B across all events

### Key Backtest Questions

- What fraction of at-risk troves self-close vs. get force-redeemed?
- What is the typical lag between LUSD depeg onset and first redemption transaction?
- Does ETH price move directionally during redemption windows, or is it dominated by macro?
- Is the arb (Sub-Strategy A) competitive — i.e., are redemptions executed within minutes by bots, leaving no manual window?

---

## Go-Live Criteria

| Criterion | Threshold |
|---|---|
| Minimum historical events backtested | ≥ 5 depeg events |
| Sub-Strategy A net positive P&L | >0 after fees across all events |
| Sub-Strategy B win rate | ≥ 55% |
| Sub-Strategy B Sharpe (annualised) | ≥ 0.8 |
| Trove closure lag validated | Median lag ≥ 20 minutes (confirms non-HFT window) |
| Paper trade period | 4 weeks live monitoring, ≥ 1 live event observed |

---

## Kill Criteria

| Trigger | Action |
|---|---|
| Liquity V2 / protocol upgrade changes redemption ordering | Suspend immediately; re-analyse |
| LUSD supply drops below $50M (reduced trove count) | Reduce position sizes proportionally; review viability |
| Sub-Strategy A: arb bots consistently front-run within 1 block | Abandon Sub-Strategy A; retain Sub-Strategy B |
| Sub-Strategy B: 3 consecutive stop-loss hits | Suspend; review ETH macro regime |
| Hyperliquid ETH perp liquidity < $5M depth at 2% | Reduce max position size |
| LUSD depegs permanently (protocol failure) | Emergency close all positions |

---

## Risks

### Protocol Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Liquity smart contract exploit | HIGH | Monitor protocol TVL; set hard capital limit ($50k max exposure) |
| Oracle manipulation (Chainlink ETH price) | MEDIUM | Cross-check with secondary oracle; don't trade during oracle anomalies |
| baseRate spikes unexpectedly | LOW | Always fetch baseRate on-chain before executing Sub-Strategy A |
| Protocol governance changes redemption rules | MEDIUM | Monitor Liquity governance forums; kill switch |

### Market Risks

| Risk | Severity | Mitigation |
|---|---|---|
| ETH rallies sharply during depeg (macro override) | HIGH | Hard stop-loss at 8%; low leverage (2–3x) |
| LUSD depeg caused by ETH crash (correlated) | MEDIUM | In ETH crash, trove ICRs fall → more troves at risk → thesis strengthens, but ETH short may already be profitable |
| Redemption flow smaller than modelled | MEDIUM | 40% scalar is conservative; calibrate in backtest |
| Curve pool illiquidity during stress | LOW | Check pool depth before entry; size accordingly |

### Execution Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Gas spike during Ethereum congestion | LOW | Pre-set gas limits; use EIP-1559 priority fee |
| Hyperliquid perp slippage on entry | LOW | Use limit orders; size within 1% of order book depth |
| LUSD arb front-run by MEV bots | MEDIUM | Accept this risk for Sub-Strategy A; Sub-Strategy B is not MEV-sensitive |

### Structural Risks

- **Liquity V2** has been deployed with modified mechanics. Confirm which version holds the majority of LUSD supply before trading. As of 2024, Liquity V1 remains the primary LUSD issuance mechanism but this must be verified at go-live.
- **LUSD supply has declined** from peak (~$1.5B) to lower levels. Fewer troves means smaller redemption events. Minimum viable event size must be confirmed.

---

## Data Sources

| Source | Use | Access |
|---|---|---|
| Liquity Subgraph (The Graph) | Trove state, ICR history, redemption events | `https://thegraph.com/hosted-service/subgraph/liquity/liquity` |
| Curve Finance Subgraph | LUSD/3CRV price history | The Graph / Curve API |
| Dune Analytics | Pre-built Liquity dashboards, redemption history | `dune.com` (free tier) |
| Etherscan | Raw redemption transaction history | Public |
| Chainlink ETH/USD feed | Oracle price validation | On-chain |
| Hyperliquid API | ETH perp price, funding rate, order book | `api.hyperliquid.xyz` |
| Ethereum archive node | Historical trove state reconstruction | Alchemy / Infura (paid) |

---

## Open Questions for Researcher Review

1. **Is Sub-Strategy A still viable?** Given MEV bot competition, the direct redemption arb may require sub-second execution. Needs empirical check: what is the median time between LUSD hitting $0.995 and the first redemption transaction on-chain?

2. **Liquity V1 vs V2 split:** What percentage of LUSD is now issued via V1 (original redemption mechanics) vs. V2 (different stability mechanisms)? This affects the addressable trove pool.

3. **Trove owner sophistication:** Are bottom-of-queue trove owners monitored by their own bots? If so, the self-closure lag shrinks. Historical data on self-closure vs. forced redemption ratio is needed.

4. **ETH correlation during depeg events:** In the 2022–2023 events, was ETH already falling when LUSD depegged (making the short redundant) or was the depeg an independent signal?

5. **Minimum event frequency:** How many LUSD depeg events (>0.5% discount, >30 min duration) occurred per year historically? If fewer than 6/year, the strategy has insufficient trading frequency to justify infrastructure costs.

---

*Next step: Assign to data pipeline. Pull Liquity subgraph data for all LUSD depeg events 2022–2024. Build trove queue reconstruction script. Measure redemption lag distribution. Report back before proceeding to step 3 (backtest execution).*

## Entry Rules

TBD

## Exit Rules

TBD

## Position Sizing

TBD
