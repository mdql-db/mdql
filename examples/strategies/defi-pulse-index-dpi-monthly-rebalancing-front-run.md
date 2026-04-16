---
title: "DeFi Pulse Index (DPI) Monthly Rebalancing Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 2
composite: 300
categories:
  - index-rebalance
  - defi-protocol
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Index Coop's DPI publishes rebalance targets 48–72 hours before on-chain execution. The rebalance is mechanically executed via Set Protocol's streaming module against Uniswap V3 pools. Tokens receiving weight increases will face predictable buy-side pressure; tokens being trimmed will face sell-side pressure. Because the execution venue is a DEX with finite liquidity, the price impact is absorb-able and front-runnable without speed advantages — the window is measured in hours, not milliseconds.

**The causal chain:**
1. Index Coop governance publishes new target weights (public, on-chain)
2. Set Protocol module executes trades to reach those weights (deterministic, calculable)
3. DEX pools absorb the flow (price impact proportional to AUM / pool depth)
4. Price reverts partially post-execution as arbitrageurs restore pool balance

**The edge is structural but probabilistic.** The rebalance MUST happen (contractual obligation to index holders), but the price impact depends on AUM-to-pool-depth ratio, which varies. This is why the score is 6, not 8+.

---

## Structural Mechanism

### Why the rebalance MUST happen

DPI is a Set Protocol TokenSet. The index methodology obligates Index Coop to rebalance constituents to target weights on a published schedule. Failure to rebalance would break the product's stated methodology, creating legal and reputational liability. This is not discretionary — it is a contractual product obligation.

### Why the price impact is predictable

The rebalance trade size is calculable:

```
Trade size (token X) = DPI AUM × (target_weight_X - current_weight_X)
```

All inputs are public:
- DPI AUM: on-chain (Set Protocol contract)
- Current weights: on-chain (constituent token balances in the SetToken)
- Target weights: published in governance forum 48–72h before execution

The execution route is Uniswap V3 (primary) with potential fallback to other DEX aggregators. Pool depths are queryable in real time.

### Why the window is not HFT

The announcement-to-execution gap is 48–72 hours. This is a governance process, not a mempool race. Entry can be taken at leisure after the governance post is confirmed. No speed advantage required.

### Why the edge degrades post-execution

Post-execution, arbitrageurs restore Uniswap pool prices toward fair value. The front-runner's exit window is the 2–4 hours immediately post-execution, before full reversion. Holding longer risks giving back gains to mean reversion.

---

## Current AUM Context (Critical Risk Factor)

DPI AUM has declined significantly from its 2021 peak (~$600M) to the ~$20–40M range as of 2025. This is the primary threat to strategy viability.

| AUM Level | Estimated Price Impact | Strategy Viability |
|-----------|----------------------|-------------------|
| $500M+ | Meaningful (1–5% on small caps) | Strong |
| $100–500M | Moderate (0.5–2%) | Viable |
| $20–50M | Small (0.1–0.5%) | Marginal — fee drag risk |
| <$20M | Negligible | Dead |

**Pre-backtest gate:** Before any historical analysis, calculate the AUM/pool-depth ratio for each historical rebalance event. If median impact is below 0.3%, the strategy is likely not viable net of fees and slippage, and should be archived.

---

## Entry Rules


### Pre-trade checklist (T-48h to T-24h)

- [ ] Confirm governance post is live on Index Coop forum with new target weights
- [ ] Confirm on-chain proposal matches forum post (cross-reference Set Protocol contract)
- [ ] Calculate weight deltas for all constituents
- [ ] Filter: only trade constituents with |weight delta| > 2%
- [ ] Query Uniswap V3 pool depth for each qualifying constituent
- [ ] Calculate estimated price impact: `Trade size / Pool TVL × impact_coefficient`
- [ ] Confirm estimated impact > 0.5% (minimum threshold for viability after fees)
- [ ] Check constituent has liquid perp market on Hyperliquid or similar (for short leg)

### Entry (T-24h, after checklist passes)

**Long leg (weight increasing constituents):**
- Buy spot on constituent tokens where `target_weight > current_weight + 2%`
- Entry: market order or limit within 0.2% of mid
- Venue: spot DEX or CEX depending on liquidity

**Short leg (weight decreasing constituents):**
- Short perp on constituent tokens where `current_weight > target_weight + 2%`
- Entry: Hyperliquid perp or equivalent
- Venue: perp DEX with sufficient OI

## Exit Rules

### Exit

**Primary exit trigger:** On-chain confirmation that Set Protocol rebalance transaction has executed (monitor SetToken contract for `ComponentExchanged` or equivalent event)

**Exit window:** T+0h to T+4h post-execution
- Exit long positions: sell into the rebalance-driven price elevation
- Exit short positions: cover into the rebalance-driven price depression
- Use limit orders within 0.3% of mid to avoid adding to post-execution reversion

**Hard stop exits:**
- If rebalance is delayed beyond T+96h from announcement: exit all positions at market
- If constituent token suffers exploit/hack during hold period: exit immediately at market
- If price moves adversely by >3% before rebalance executes: exit and reassess

---

## Position Sizing

### Per-trade sizing

```
Position size = min(
    Account_risk_budget × 0.15,          # max 15% of risk budget per event
    Estimated_impact_size × 0.25,        # size to capture 25% of estimated flow
    Pool_depth × 0.05                    # max 5% of pool depth (avoid self-impact)
)
```

### Rationale

- 15% risk budget cap: single monthly event should not dominate portfolio
- 25% of estimated flow: conservative — we are not the only front-runner
- 5% pool depth cap: avoid becoming the price impact we are trying to trade

### Leverage

- Long leg (spot): 1x only — no leverage on spot positions
- Short leg (perp): max 2x — low leverage given multi-day hold period and funding cost drag

### Portfolio-level constraint

- Maximum concurrent exposure across all legs: 30% of total capital
- If multiple constituents qualify, allocate proportionally to estimated impact size

---

## Backtest Methodology

### Data collection

1. **Rebalance history:** Scrape all Index Coop governance forum posts tagged "rebalance" from inception (2020) to present. Extract announcement date, target weights, execution date.
2. **Constituent prices:** Pull OHLCV for each constituent from CoinGecko or Messari API at 1h resolution.
3. **DPI AUM history:** Query Set Protocol contract on Ethereum mainnet via Etherscan or The Graph for historical AUM at each rebalance date.
4. **Pool depth history:** Uniswap V3 subgraph for historical TVL of relevant pools at rebalance dates.
5. **Execution transactions:** Identify SetToken rebalance txs on-chain; extract exact execution timestamp.

### Metrics to calculate per event

```
weight_delta[token] = target_weight - current_weight
estimated_trade_usd[token] = DPI_AUM × weight_delta[token]
estimated_impact[token] = f(estimated_trade_usd, pool_depth)  # use Uniswap V3 impact formula
actual_price_change[token] = price_at_T+2h / price_at_T-48h - 1
alpha[token] = actual_price_change - BTC_return_same_period  # beta-adjusted
round_trip_pnl = alpha_long_legs - alpha_short_legs - fees - funding
```

### Segmentation

Segment results by:
- AUM bucket (>$100M, $50–100M, $20–50M, <$20M)
- Constituent market cap (large cap vs. small cap constituents)
- Weight delta magnitude (2–5%, 5–10%, >10%)
- Market regime (bull/bear/sideways — use BTC 30-day return as proxy)

### Minimum viable backtest

- Minimum 12 rebalance events to draw any conclusions
- Minimum 3 events per AUM bucket for segment-level conclusions
- Report Sharpe, win rate, average alpha per event, max drawdown per event

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

| Criterion | Threshold |
|-----------|-----------|
| Backtest win rate (alpha > 0) | ≥ 60% of events |
| Median alpha per event (beta-adjusted) | ≥ 0.5% |
| Backtest Sharpe (annualised across events) | ≥ 1.0 |
| AUM at go-live | ≥ $25M |
| Estimated impact at go-live | ≥ 0.4% per constituent |
| At least one liquid perp market exists for short leg | Required |

Paper trade for minimum 3 live events before committing real capital.

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

| Trigger | Action |
|---------|--------|
| DPI AUM falls below $15M | Archive — impact too small |
| 3 consecutive events with negative alpha | Pause, investigate structural change |
| Index Coop changes rebalance methodology to remove advance notice | Archive — edge destroyed |
| Index Coop migrates off Set Protocol to a new execution module | Re-evaluate from scratch |
| Median alpha across trailing 6 events falls below 0.2% | Archive — fee drag dominates |
| DPI product is deprecated by Index Coop | Archive |

---

## Risks

### Primary risks

**1. AUM atrophy (HIGH probability, HIGH impact)**
DPI AUM has been declining for years. If AUM falls below the threshold where rebalance trades move markets, the entire edge disappears. This is the single biggest risk and must be monitored monthly.

**2. Crowded front-running (MEDIUM probability, MEDIUM impact)**
If other participants identify the same window, they will front-run the front-runner. The 48–72h window is public. Monitor on-chain for unusual buying in constituents immediately after governance posts. If pre-announcement price moves already capture >50% of expected impact, the edge is being competed away.

**3. Execution route change (LOW probability, HIGH impact)**
If Index Coop switches from Uniswap V3 to a different execution mechanism (e.g., RFQ, OTC, or a new DEX), the price impact dynamics change entirely. Monitor governance for execution methodology changes.

**4. Constituent exploit during hold period (LOW probability, HIGH impact)**
Holding DeFi tokens for 24–72h exposes the long leg to smart contract risk in the constituent protocols. Mitigate by sizing conservatively and monitoring for exploit alerts (Chainalysis, Forta, Rekt.news).

**5. Funding rate drag on short leg (MEDIUM probability, LOW-MEDIUM impact)**
Perp shorts held for 24–72h accumulate funding costs. In bull markets, funding can be significantly positive (shorts pay longs), eroding the short-leg alpha. Calculate expected funding cost before entering short leg; if funding > 0.3% per day, short leg may not be viable.

**6. Governance delay or cancellation (LOW probability, MEDIUM impact)**
Rebalances can be delayed by governance disputes or technical issues. Hard stop at T+96h mitigates this.

### Secondary risks

- Thin perp markets for small-cap constituents may make short leg unexecutable
- Tax treatment of frequent spot trades (jurisdiction-dependent)
- Index Coop may reduce advance notice period in future methodology updates

---

## Data Sources

| Data | Source | Cost | Notes |
|------|---------|------|-------|
| Rebalance announcements | Index Coop governance forum (gov.indexcoop.com) | Free | Manual scrape or RSS |
| Target weights | Index Coop forum + on-chain Set Protocol contract | Free | Cross-reference both |
| DPI AUM (current) | indexcoop.com/dpi or Set Protocol contract | Free | |
| DPI AUM (historical) | The Graph — Set Protocol subgraph | Free | Query historical states |
| Constituent prices (1h OHLCV) | CoinGecko API or Messari API | Free tier available | |
| Uniswap V3 pool depth (current) | Uniswap V3 subgraph or info.uniswap.org | Free | |
| Uniswap V3 pool depth (historical) | Uniswap V3 subgraph (historical queries) | Free | Rate limits apply |
| On-chain execution timestamps | Etherscan API — SetToken contract events | Free tier | Filter `ComponentExchanged` |
| Perp funding rates | Hyperliquid API | Free | |
| BTC price (beta adjustment) | CoinGecko or Binance API | Free | |

### Key contract addresses (Ethereum mainnet)

- DPI SetToken: `0x1494CA1F11D487c2bBe4543E90080AeBa4BA3C2b`
- Index Coop governance: Snapshot space `index-coop.eth`

---

## Open Questions for Backtest Phase

1. What is the historical distribution of AUM/pool-depth ratios at rebalance time? Is there a clear threshold below which alpha disappears?
2. How quickly does price revert post-execution? Is T+2h the right exit, or is T+4h or T+6h better?
3. Are there systematic differences between large-cap constituents (UNI, AAVE, MKR) and small-cap constituents in terms of impact magnitude?
4. Has the advance notice window always been 48–72h, or has it varied? Any events with shorter notice?
5. Is there evidence of other front-runners already active in this window (pre-announcement price drift)?
6. What is the realistic all-in cost (DEX fees + perp fees + funding + slippage) per round trip?

---

## Next Steps

- [ ] **Step 3:** Scrape all historical rebalance events from Index Coop governance forum (2020–present)
- [ ] **Step 4:** Pull on-chain AUM and pool depth data for each event via The Graph
- [ ] **Step 5:** Calculate estimated vs. actual price impact per event, per constituent
- [ ] **Step 6:** Run backtest segmented by AUM bucket and weight delta magnitude
- [ ] **Step 7:** If backtest clears go-live criteria, paper trade next 3 live events
- [ ] **Step 8:** Review paper trade results; if criteria met, allocate real capital
- [ ] **Step 9:** Monthly monitoring of AUM and kill criteria

---

*Strategy authored by Zunid research agent. All claims are hypotheses pending backtest validation. No live capital should be deployed until go-live criteria are satisfied.*
