---
title: "Liquidity Bootstrapping Pool (LBP) Weight Curve Exhaustion Short"
status: HYPOTHESIS
mechanism: 7
implementation: 5
safety: 4
frequency: 2
composite: 280
categories:
  - defi-protocol
  - token-supply
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Liquidity Bootstrapping Pools encode a mathematically fixed weight decay schedule into their smart contracts. This creates a deterministic, protocol-enforced sell pressure on the token throughout the LBP window. Buyers who enter early pay a price inflated by the starting weight imbalance (e.g., 90% token / 10% USDC). As weights mechanically shift toward equilibrium (typically 50/50), the AMM formula forces the implied token price downward regardless of external demand. The strategy is to position short (or equivalently, buy the collateral asset late in the window) to capture the convergence from peak-weight distortion to end-weight equilibrium. The edge is not "tokens tend to fall during LBPs" — it is that the AMM pricing formula *mathematically requires* the price to decay toward the end-weight implied price, absent overwhelming external buy pressure.

---

## Structural Mechanism

### The AMM Pricing Formula

Balancer weighted pools price assets using the invariant:

```
Price(token) = (Balance_collateral / Weight_collateral) / (Balance_token / Weight_token)
```

At LBP launch with weights 90% token / 10% USDC and a pool seeded with 1,000,000 tokens and 100,000 USDC:

```
Implied price = (100,000 / 0.10) / (1,000,000 / 0.90) = 1,000,000 / 1,111,111 ≈ $0.90
```

At end-state with weights 50/50 (assuming no trades, purely mechanical):

```
Implied price = (100,000 / 0.50) / (1,000,000 / 0.50) = 200,000 / 2,000,000 = $0.10
```

The weight shift alone — with zero trades — drops the implied price by ~89%. This is not a prediction. It is arithmetic.

### Why the Decay Is Guaranteed

1. **Hard-coded in the smart contract.** The weight schedule is set at pool deployment. It executes block-by-block on a linear or custom curve. The issuer cannot modify the schedule mid-run without pulling liquidity entirely (which is itself a detectable, tradeable event).

2. **The issuer's incentive aligns with the mechanism.** LBPs are designed to distribute tokens at declining prices to discourage front-running and encourage broad participation. The issuer *wants* the decay to happen.

3. **No external oracle dependency.** The price is purely a function of pool balances and weights. There is no external price feed that can override the formula.

### The Tradeable Distortion

The gap between the launch price and the end-weight implied price is the "distortion window." This window is largest in the first 2–4 hours of a 48–72 hour LBP. The strategy captures the compression of this gap.

**Key insight:** Even with significant buy pressure during the LBP, the weight decay creates a persistent headwind. Buyers must continuously absorb the mechanical sell pressure just to hold price flat. In practice, most LBPs see net price decline over the window because retail FOMO is front-loaded and exhausts itself before the weight decay completes.

---

## Execution Modes

Because most LBP tokens lack perp markets pre-TGE, there are three execution modes ranked by cleanliness:

### Mode A — Direct LBP Participation (Primary)
Buy the collateral asset (USDC/ETH) into the LBP late in the window when the token weight has decayed significantly. You are effectively buying tokens at near-equilibrium price. This is not a short — it is buying the *right side* of the curve. Profit comes from receiving tokens at a price below where early buyers paid, then selling post-TGE listing.

**This is the cleanest execution path but requires post-TGE liquidity to exit.**

### Mode B — Perp Short Post-Listing (Secondary)
If the token lists on a perp exchange (Hyperliquid, dYdX, Binance) within hours of LBP close, short the perp at listing. The thesis: LBP price discovery is artificially elevated by early-window weight distortion. The listing price often anchors to LBP price, not end-weight fair value. Mean reversion to end-weight implied price is the target.

**This is the most Hyperliquid-native execution path.**

### Mode C — OTC / Pre-market Short (Rare)
Some tokens have pre-market OTC markets or prediction markets (Polymarket, Whales Market) where short exposure can be established before TGE. Niche and illiquid but occasionally available for high-profile launches.

---

## Entry Rules

### Pre-LBP Screening (T-24h to T-0)

1. **Source:** Monitor Fjord Foundry public calendar and Copper Launch announcements. Cross-reference with Balancer subgraph for pool deployment transactions.

2. **Calculate distortion ratio:**
   ```
   Distortion Ratio = Launch_implied_price / End_weight_implied_price
   ```
   Only proceed if Distortion Ratio > 3.0 (i.e., end-weight price is less than 33% of launch price, purely from weight mechanics).

3. **Check token availability:** Does the token have an existing perp market? If yes, flag for Mode B. If no, assess Mode A viability (is there a credible post-TGE listing venue?).

4. **Assess pool size:** Minimum $500K USDC collateral seeded. Below this, the pool is too thin and subject to manipulation.

5. **Issuer credibility check:** Is the issuer a known team with a public track record? Anonymous teams with no prior history increase the risk of pool drain (see Risks).

### Entry Trigger — Mode A (Direct LBP)

- **Time:** Enter between T+60% and T+80% of the LBP window duration (e.g., for a 48h LBP, enter between hour 29 and hour 38)
- **Price condition:** Current LBP price must still be > 1.5x the end-weight implied price (confirming distortion has not fully collapsed)
- **Size:** Allocate no more than 2% of portfolio per LBP event
- **Execution:** Buy collateral asset into the LBP via Fjord/Copper UI or direct contract interaction

### Entry Trigger — Mode B (Perp Short)

- **Time:** Within first 2 hours of perp listing on Hyperliquid
- **Price condition:** Listing price > 1.5x end-weight implied price from the completed LBP
- **Funding check:** If funding rate is already deeply negative (shorts paying longs > 0.1% per 8h), reduce size by 50% — crowded short
- **Size:** 1–3% of portfolio, max 5x leverage

---

## Exit Rules

### Mode A Exit
- **Primary:** Sell tokens on first available CEX/DEX listing, targeting the first 30 minutes of listing liquidity
- **Stop:** If post-TGE price is below your Mode A entry cost (i.e., LBP late-entry price), exit immediately — the pool was drained or manipulated
- **Time stop:** If no listing within 14 days of LBP close, reassess; consider OTC exit

### Mode B Exit
- **Target:** End-weight implied price (calculated pre-entry)
- **Stop-loss:** 30% above entry price (hard stop, no exceptions)
- **Time stop:** Close position at 72 hours post-entry regardless of P&L — the mean reversion window is short
- **Partial exit:** Take 50% off at 1.5x implied price convergence, let remainder run to full convergence

---

## Position Sizing

| Parameter | Value |
|-----------|-------|
| Max allocation per event | 2% of portfolio |
| Max concurrent LBP positions | 3 |
| Max total LBP exposure | 6% of portfolio |
| Mode B max leverage | 5x |
| Mode A leverage | None (spot only) |
| Kelly fraction | 0.25 (quarter-Kelly until edge is validated) |

**Rationale for small sizing:** This is a pre-backtest hypothesis. The structural mechanism is sound but execution risk (pool drain, no listing, crowded short) is high enough to warrant conservative sizing until 50+ events are logged.

---

## Backtest Methodology

### Data Requirements

| Data Source | What to Pull | Where |
|-------------|--------------|-------|
| Fjord Foundry | Historical LBP list, start/end times, weight parameters, collateral amounts | Fjord subgraph (The Graph) |
| Balancer subgraph | Pool swap history, weight snapshots, price at each block | Balancer V2 subgraph |
| CoinGecko / CMC | Post-TGE listing price, 7-day price history | API |
| Hyperliquid | Perp listing date, first-hour OHLCV, funding rates | Hyperliquid API |

### Backtest Steps

1. **Collect all LBPs** from Fjord Foundry from inception (2021) to present. Target: 200+ events minimum.

2. **For each LBP:**
   - Record: start weight, end weight, collateral seeded, duration, launch implied price, end-weight implied price, distortion ratio
   - Record: actual price at T+0, T+25%, T+50%, T+75%, T+100% of window
   - Record: post-TGE listing price (if applicable), 24h post-listing price, 7d post-listing price

3. **Apply entry filter:** Distortion ratio > 3.0, pool size > $500K. Count how many events pass.

4. **Simulate Mode A:** Enter at T+70% of window. Exit at listing price (first available). Calculate P&L per event.

5. **Simulate Mode B:** Enter at listing price if > 1.5x end-weight implied. Exit at end-weight implied price or stop (30% above entry) or 72h time stop. Calculate P&L per event.

6. **Aggregate metrics:**
   - Win rate
   - Average P&L per event
   - Sharpe ratio (annualised)
   - Max drawdown
   - P&L distribution (are wins fat-tailed or thin-tailed?)

7. **Segment analysis:**
   - Does distortion ratio > 5x outperform > 3x?
   - Does pool size > $2M outperform $500K–$2M?
   - Does bear market vs. bull market regime affect win rate?

### Known Backtest Limitations

- **Survivorship bias:** LBPs that were drained or cancelled may be underrepresented in subgraph data. Must manually check for pool drain events.
- **Slippage not modelled:** Late-window LBP entries move the price. Must model price impact for realistic position sizes.
- **Post-TGE liquidity assumption:** Mode A assumes a listing occurs. Some LBP tokens never list on a liquid venue. Must track this rate.
- **Perp availability:** Mode B is only applicable to a subset of tokens. Must track what % of LBP tokens list on Hyperliquid within 48h.

---

## Go-Live Criteria

All of the following must be satisfied before live capital deployment:

| Criterion | Threshold |
|-----------|-----------|
| Backtest events | ≥ 50 qualifying LBPs |
| Backtest win rate | ≥ 55% |
| Backtest Sharpe | ≥ 1.0 (annualised) |
| Max backtest drawdown | ≤ 25% |
| Paper trade events | ≥ 10 live LBPs tracked in real time |
| Paper trade win rate | ≥ 50% |
| Pool drain rate | ≤ 15% of qualifying events |
| Perp availability rate | ≥ 20% of qualifying events (for Mode B viability) |

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

| Trigger | Action |
|---------|--------|
| 5 consecutive losses | Pause, review, do not resume without researcher sign-off |
| Live win rate drops below 40% over 20+ events | Kill strategy, return to backtest |
| Pool drain rate in live trading exceeds 25% | Kill Mode A, review Mode B independently |
| Regulatory action against LBP mechanism | Full kill, legal review |
| Fjord Foundry / Copper shut down | Reassess data sourcing; strategy may be dead |
| Single event loss > 5% of portfolio | Immediate position review; likely position sizing error |

---

## Risks

### Risk 1: Pool Drain (HIGH — Mode A killer)
The issuer can withdraw liquidity at any time, typically by calling `exitPool`. This instantly removes all collateral and tokens. Mode A participants are left holding tokens with no exit. **Mitigation:** Issuer credibility screening pre-entry; never enter Mode A with anonymous teams; monitor pool balance in real time.

### Risk 2: No Post-TGE Listing (MEDIUM — Mode A killer)
Some LBP tokens never achieve a liquid secondary market. Mode A profits are unrealisable. **Mitigation:** Only enter Mode A if the token has a confirmed CEX listing or a liquid DEX pool is guaranteed by the issuer.

### Risk 3: Overwhelming Buy Pressure (MEDIUM — affects both modes)
If genuine demand is strong enough, buyers can absorb the weight decay and hold price flat or push it higher. The weight decay creates a headwind, not a guarantee of price decline. **Mitigation:** Distortion ratio filter (> 3x) ensures the mechanical decay is large enough to matter even with moderate buy pressure. Stop-loss on Mode B.

### Risk 4: Crowded Short (MEDIUM — Mode B)
High-profile LBPs attract attention. If everyone is shorting the perp at listing, funding rates go deeply negative (shorts pay longs). The carry cost erodes the trade. **Mitigation:** Funding rate check at entry; reduce size if funding > 0.1%/8h negative.

### Risk 5: Smart Contract Risk (LOW-MEDIUM)
Balancer V2 contracts are audited and battle-tested. Fjord Foundry adds a wrapper layer. Risk is low but non-zero. **Mitigation:** Never allocate more than 2% per event; diversify across multiple LBPs.

### Risk 6: Regulatory Risk (LOW — but watch)
LBPs have been scrutinised as potential unregistered securities offerings in some jurisdictions. If regulators act against the LBP mechanism broadly, the strategy's opportunity set disappears. **Mitigation:** Monitor regulatory developments; this is a tail risk, not an immediate concern.

### Risk 7: Data Latency (LOW — operational)
Subgraph indexing can lag by minutes. For Mode B (perp short at listing), stale data could cause entry at wrong price. **Mitigation:** Use direct RPC calls to read pool state, not subgraph, for live trading.

---

## Data Sources

| Source | URL / Access | Cost | Use |
|--------|-------------|------|-----|
| Fjord Foundry | fjordfoundry.com + subgraph | Free | LBP calendar, pool parameters |
| Copper Launch | copperlaunch.com | Free | Alternative LBP platform |
| Balancer V2 Subgraph | thegraph.com/hosted-service/subgraph/balancer-labs/balancer-v2 | Free | Historical swap data, weight snapshots |
| Alchemy / Infura RPC | alchemy.com | Free tier sufficient | Direct contract reads for live trading |
| Hyperliquid API | hyperliquid.xyz/info | Free | Perp listing data, funding rates, OHLCV |
| CoinGecko API | coingecko.com/api | Free tier | Post-TGE price history |
| Etherscan | etherscan.io | Free | Pool drain event detection, contract verification |

---

## Open Questions for Researcher Review

1. **What is the historical pool drain rate on Fjord Foundry?** This is the single biggest Mode A risk. If it's > 20%, Mode A may not be viable.

2. **What % of Fjord LBP tokens list on Hyperliquid within 48h?** This determines Mode B opportunity frequency. If < 10%, Mode B is too rare to build a strategy around.

3. **Is there a reliable way to detect pool drain in real time?** An on-chain alert (e.g., via Tenderly or a custom event listener) watching for `PoolBalanceChanged` events with large collateral outflows would be the operational requirement for Mode A.

4. **Does the distortion ratio correlate with post-LBP price performance?** Hypothesis: higher distortion ratio → stronger mean reversion. Needs backtest validation.

5. **Are there LBPs on non-Balancer infrastructure** (e.g., custom implementations on Solana, Sui) that follow the same mechanic? If so, the opportunity set may be larger than Fjord/Copper alone.

---

## Summary

The LBP weight curve exhaustion short is a structurally grounded strategy with a clear causal mechanism: the AMM pricing formula mathematically requires price to decay as weights shift, absent overwhelming buy pressure. The edge is real. The execution constraints are also real — perp availability is limited pre-TGE, and pool drain risk is non-trivial. The strategy is best approached as a two-mode system: Mode A (late LBP entry) for tokens with confirmed post-TGE listings, and Mode B (perp short at listing) for the subset that achieve liquid derivatives markets quickly. Both modes require disciplined screening and small position sizes until the backtest validates the edge across 50+ events.

**Next step:** Pull Fjord Foundry subgraph data for all historical LBPs. Calculate distortion ratios and post-LBP price outcomes. Determine pool drain rate and post-TGE listing rate. Report back before proceeding to paper trade.
