---
title: "Gnosis Conditional Token Merge Arbitrage — Forced NAV Reconstitution"
status: HYPOTHESIS
mechanism: 9
implementation: 2
safety: 7
frequency: 2
composite: 252
categories:
  - defi-protocol
  - options-derivatives
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a Gnosis Protocol conditional token pair (YES + NO) trades at a combined price below the collateral value, a risk-free arbitrage exists: buy both legs, hold to resolution, redeem the winning token at par. The smart contract enforces 1:1 redemption — this is not a probabilistic bet on convergence, it is a contractual guarantee.

**Causal chain:**
1. A prediction market condition is created on Gnosis Protocol with USDC (or DAI) as collateral
2. The collateral is split into YES and NO outcome tokens, each redeemable for $1.00 of collateral if their outcome wins
3. Liquidity fragmentation across AMM pools causes unsophisticated LPs to misprice the two legs independently — they do not monitor the combined sum
4. The sum of YES + NO prices falls below $1.00 (e.g., $0.94), creating a guaranteed spread
5. An arb bot buys both legs at the observed prices, paying gas
6. At resolution, the oracle reports the outcome; the winning token redeems 1:1 for collateral via `CTHelperV2.redeemPositions()`; the losing token is worthless
7. Net P&L = $1.00 − (YES_price + NO_price) − gas_cost_in_USD − slippage

The discount exists because: (a) retail LPs price each token in isolation, (b) gas friction discourages small arb, (c) resolution timing uncertainty creates a time-value drag that unsophisticated participants over-discount.

---

## Structural Mechanism — Why This MUST Happen

This is not a tendency — it is a smart contract invariant.

**The guarantee:** The Gnosis Conditional Token Framework (CTF) enforces that for any binary condition with USDC collateral:

```
redeemPositions(collateralToken, parentCollectionId, conditionId, [1,0]) → 1.00 USDC per winning YES token
redeemPositions(collateralToken, parentCollectionId, conditionId, [0,1]) → 1.00 USDC per winning NO token
```

This is encoded in the deployed, immutable `ConditionalTokens.sol` contract. No counterparty can refuse redemption. The only failure modes are oracle non-resolution (condition never settles) or a contract exploit — neither of which is a market risk.

**Why the discount persists:**
- AMM pools (Balancer, Uniswap v2 forks on Gnosis Chain) price YES and NO in separate pools with no shared liquidity
- LPs set prices based on their probability estimate of the outcome, not on the arbitrage-free constraint that YES + NO must equal $1.00 at expiry
- Gas costs on Ethereum mainnet ($15–80 per transaction) make the arb uneconomical for small positions; on Gnosis Chain (gas ~$0.001) and Polygon (gas ~$0.01), the friction is negligible
- Resolution uncertainty (will the oracle resolve in 1 day or 3 weeks?) creates a time-value cost that rational arbs demand compensation for, but this cost is often over-priced by the market

**The edge is structural, not statistical.** The contract will pay $1.00. The only question is whether the discount exceeds costs.

---

## Entry Rules


### Entry Conditions (ALL must be met simultaneously)

| Parameter | Threshold | Rationale |
|-----------|-----------|-----------|
| Combined price | YES + NO < $0.97 | Minimum 3% gross spread before costs |
| Net spread after gas | > 2.0% of position size | Gas must be pre-calculated at current gwei |
| Slippage per leg | < 0.5% at target fill size | Observed price must be achievable |
| Liquidity depth | Both legs have ≥ $500 depth within 0.5% | Ensures fills are real |
| Time to resolution | < 60 days | Caps capital lockup; longer = higher time-value cost |
| Oracle status | Condition not disputed, oracle is Kleros or UMA with track record | Reduces non-resolution risk |
| Collateral type | USDC or DAI only | Stablecoin collateral; no collateral price risk |

### Execution Sequence
1. Query YES pool price and NO pool price simultaneously (same block or within 2 blocks)
2. Calculate: `gross_spread = 1.00 - (YES_ask + NO_ask)`
3. Calculate: `gas_cost_USD = (gas_units_buy_YES + gas_units_buy_NO + gas_units_redeem) × gwei × ETH_price`
4. Calculate: `net_spread = gross_spread - gas_cost_USD / position_size - slippage_estimate`
5. If `net_spread > 0.02`: execute both buys atomically (same transaction via multicall if possible)
6. Record: condition ID, collateral address, YES token address, NO token address, prices paid, gas paid, expected resolution date

## Exit Rules

### Exit Rules
- **Primary exit:** Hold to on-chain resolution. Monitor `ConditionResolution` event on `ConditionalTokens` contract. Call `redeemPositions()` within 24 hours of resolution event
- **No early exit:** There is no secondary market exit that guarantees profit — the arb only closes at resolution. Do not attempt to sell legs early unless the combined price has risen above entry cost (rare)
- **Timeout kill:** If condition is unresolved after 90 days (30 days past expected resolution), flag for manual review. Do not auto-redeem — check oracle dispute status first

### What NOT to Do
- Do not leg into the trade (buy YES first, then NO later) — this creates directional exposure
- Do not enter if either leg has < $200 of on-chain liquidity — the observed price is not real
- Do not enter on Ethereum mainnet for positions < $5,000 — gas will consume the spread

---

## Position Sizing

**Base rule:** Risk-adjusted flat sizing per opportunity, not Kelly.

- **Per-trade size:** $500–$2,000 per condition pair
- **Rationale:** The arb is binary (win the spread or lose to non-resolution/gas). Sizing is limited by liquidity depth, not by conviction. Larger sizes increase slippage and erode the spread
- **Maximum concurrent positions:** 10 open conditions simultaneously
- **Maximum capital deployed:** $15,000 total across all open conditions (capital is locked until resolution)
- **Scaling rule:** If net spread > 5%, size up to $3,000 per pair. If net spread < 2.5%, skip — margin of safety is too thin
- **Gas budget:** Pre-fund a dedicated wallet with 0.05 ETH (Gnosis Chain) or 5 MATIC (Polygon) for gas. Replenish when below 0.02 ETH / 2 MATIC

**Capital lockup consideration:** Capital is illiquid from entry to resolution. Model this as a zero-coupon bond: annualized return = `(net_spread / days_to_resolution) × 365`. Minimum acceptable annualized return: 15%.

---

## Backtest Methodology

### Data Sources

| Data | Source | Endpoint/URL |
|------|--------|--------------|
| Conditional token prices (historical) | The Graph — Gnosis CTF subgraph | `https://thegraph.com/hosted-service/subgraph/gnosis/conditional-tokens` |
| Polymarket historical prices | Polymarket CLOB API | `https://clob.polymarket.com/markets` + `https://data-api.polymarket.com` |
| Resolution timestamps | CTF contract events | Gnosis Chain RPC: `eth_getLogs` on `ConditionalTokens` address `0xCeAfDD6bc0bEF976fdCd1112955828E00543c0Ce` |
| Gas costs (Gnosis Chain) | Gnosis Chain explorer | `https://gnosisscan.io/gastracker` — historical via Blockscout API |
| Collateral prices | CoinGecko API | `https://api.coingecko.com/api/v3/coins/usd-coin/market_chart` |

### Backtest Period
- **Target:** January 2022 – December 2024 (36 months)
- **Why this period:** Covers bull/bear/sideways regimes; Gnosis Chain was active; Polymarket migrated to Polygon mid-2022

### Backtest Steps

1. **Extract all resolved conditions** from CTF contract logs on Gnosis Chain. Filter: collateral = USDC or DAI, binary (2 outcomes only), resolved within 60 days of creation
2. **Reconstruct price series** for each YES/NO pair using The Graph subgraph. Sample at 1-hour intervals
3. **Identify entry signals:** For each condition, find all 1-hour timestamps where YES_price + NO_price < $0.97
4. **Simulate entry:** At signal timestamp, record YES_ask + NO_ask (use mid + 0.25% as ask estimate). Calculate gas cost using median gas price for that date × 200,000 gas units (estimated for two swaps + one redemption)
5. **Simulate exit:** At resolution timestamp, record $1.00 redemption for winning token, $0.00 for losing token
6. **Calculate P&L per trade:** `1.00 - (YES_entry + NO_entry) - gas_USD / position_size`
7. **Aggregate metrics** (see below)

### Metrics to Report

| Metric | Target | Kill threshold |
|--------|--------|----------------|
| Win rate | > 90% (losses = gas > spread) | < 75% |
| Median net spread | > 2.5% | < 1.5% |
| Annualized return on deployed capital | > 20% | < 10% |
| Max drawdown | < 15% | > 30% |
| Average days capital locked | < 30 days | > 60 days |
| Number of qualifying opportunities | > 50 over 36 months | < 20 (too thin) |
| % of conditions that failed to resolve | < 5% | > 10% |

### Baseline Comparison
- Compare annualized return against: (a) holding USDC in Aave (risk-free rate proxy), (b) a naive strategy of buying only YES tokens on all conditions

### Known Backtest Limitations
- Historical AMM prices from The Graph may not reflect actual executable prices — treat as upper bound on spread
- Gas costs are estimates; actual costs vary with network congestion
- Liquidity depth is not captured in subgraph data — some "opportunities" may not have been fillable
- Polymarket data pre-2023 is sparse; Gnosis Chain native markets are the primary dataset

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show ALL of the following:

1. **≥ 50 qualifying opportunities** identified over the backtest period (proves the opportunity set is real, not a handful of flukes)
2. **Median net spread ≥ 2.5%** after realistic gas costs
3. **≥ 90% of trades profitable** (losses should only occur when gas spikes eat the spread — this is a near-riskless arb)
4. **Annualized return on deployed capital ≥ 20%** (must beat DeFi lending rates by a meaningful margin to justify operational complexity)
5. **Zero instances of oracle non-resolution** causing permanent capital loss (or < 2% of conditions unresolved after 120 days)
6. **Opportunity frequency ≥ 2 per month** on average (below this, the monitoring infrastructure cost is not justified)

If backtest passes, paper trade for 30 days with simulated fills before committing real capital.

---

## Kill Criteria

Abandon the strategy (live or in backtest) if ANY of the following occur:

| Trigger | Action |
|---------|--------|
| Net spread on live opportunities consistently < 1.5% after gas | Suspend entries; re-evaluate gas chain |
| Oracle dispute rate > 10% of open conditions | Halt all new entries; audit oracle selection criteria |
| Any condition results in permanent capital loss (oracle never resolves, contract exploit) | Full stop; post-mortem before resuming |
| Opportunity frequency drops below 1 per month for 3 consecutive months | Archive strategy; market has become efficient |
| Gas costs on target chain increase 5× from baseline | Recalculate minimum position size; suspend if minimum > $5,000 |
| A competing bot is observed front-running entries within 1 block | Evaluate MEV protection (private mempool); if not viable, kill |

---

## Risks

### High Severity

**Oracle non-resolution / dispute:** If the condition's oracle (Kleros, UMA, Chainlink) fails to resolve or enters a dispute, capital is locked indefinitely. Kleros disputes can take 2–8 weeks. UMA disputes require governance votes. This is the primary tail risk. *Mitigation: Only trade conditions with established oracles and clear, objective resolution criteria (e.g., "ETH price > $2000 on Jan 1" not "Did X happen?")*

**Smart contract exploit:** The CTF contract is immutable and audited, but a critical exploit could drain collateral. *Mitigation: Position size limits; do not concentrate > $5,000 in a single condition.*

### Medium Severity

**Liquidity illusion:** The price observed on-chain may not be fillable at size. A pool showing YES at $0.45 may only have $50 of depth at that price. *Mitigation: Enforce the $500 depth requirement; simulate fills using pool reserves, not just spot price.*

**Gas spike at entry:** If gas spikes between signal detection and execution, the trade may be unprofitable by the time it executes. *Mitigation: Set a maximum gas price limit in the execution script; cancel if gwei exceeds threshold.*

**Collateral depegging:** If collateral is USDC and USDC depegs (as in March 2023), the $1.00 redemption value is no longer $1.00. *Mitigation: Monitor collateral peg; halt entries if USDC trades below $0.995.*

**Time-value cost:** Capital locked for 30–60 days earns nothing. If DeFi lending rates rise significantly, the opportunity cost may exceed the spread. *Mitigation: Enforce the 15% annualized return minimum; this implicitly adjusts for opportunity cost.*

### Low Severity

**Competition:** As this strategy becomes known, bots will compress the spread. The opportunity set may shrink over time. *Mitigation: Monitor spread distribution monthly; if median drops below 2%, re-evaluate.*

**Chain migration:** Polymarket and other platforms may migrate to new chains, requiring infrastructure updates. *Mitigation: Build chain-agnostic monitoring; currently prioritize Gnosis Chain and Polygon.*

---

## Data Sources

| Resource | URL / Endpoint | Notes |
|----------|---------------|-------|
| Gnosis CTF contract (Gnosis Chain) | `0xCeAfDD6bc0bEF976fdCd1112955828E00543c0Ce` | Primary contract for event logs |
| Gnosis CTF subgraph | `https://thegraph.com/hosted-service/subgraph/gnosis/conditional-tokens` | Historical price and resolution data |
| Polymarket CLOB API | `https://clob.polymarket.com/markets` | Active market prices on Polygon |
| Polymarket data API | `https://data-api.polymarket.com/prices` | Historical price data |
| Gnosis Chain RPC | `https://rpc.gnosischain.com` | Direct contract queries |
| Blockscout (Gnosis) | `https://gnosis.blockscout.com/api` | Gas history, transaction data |
| Omen prediction market | `https://omen.eth.limo` | UI for Gnosis CTF markets; useful for manual verification |
| The Graph (Omen subgraph) | `https://thegraph.com/hosted-service/subgraph/protofire/omen-xdai` | Omen-specific market data on Gnosis Chain |
| UMA oracle docs | `https://docs.uma.xyz` | Oracle resolution mechanics |
| Kleros oracle docs | `https://kleros.io/developers` | Dispute resolution timelines |

### Recommended First Step
Query the Omen subgraph for all resolved binary markets from 2022–2024. Extract `outcomeTokenMarginalPrices` at hourly intervals. Filter for conditions where sum < 0.97. This gives the raw opportunity set before any execution assumptions.

```graphql
{
  fixedProductMarketMakers(
    where: {
      collateralToken: "0xddafbb505ad214d7b80b1f830fccc89b60fb7a83"  # USDC on Gnosis
      outcomeSlotCount: 2
    }
    first: 1000
  ) {
    id
    condition { id resolutionTimestamp }
    outcomeTokenMarginalPrices
    liquidityMeasure
  }
}
```

---

*This document is a hypothesis specification. No backtest has been run. All claims about spread frequency and magnitude require empirical validation against historical data before capital is committed.*
