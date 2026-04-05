---
title: "DAO Treasury TWAP Sell — Multi-Day Programmatic Pressure Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 3
composite: 540
categories:
  - governance
  - defi-protocol
  - token-supply
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DAO deploys an on-chain TWAP sell order for its native token, it creates a mechanically guaranteed, schedule-fixed supply overhang that is publicly readable before the first swap executes. The governance vote, the TWAP contract address, the total notional, and the execution duration are all observable on-chain. This is not a rumor or inference — it is a deployed smart contract with a deterministic sell schedule. A short position entered after vote passage and before TWAP initiation captures the price depression caused by this known, time-bounded selling program. The edge is information asymmetry between on-chain readers and market participants who rely on price feeds alone.

---

## Structural Mechanism

### Why this edge must exist (not just tends to exist)

1. **The sell is contractually committed.** Once a Milkman order, CoW TWAP, or Uniswap TWAP contract is deployed and funded, the DAO cannot easily reverse execution without a second governance vote (typically 3–7 days of delay). The sell schedule is not discretionary — it is a smart contract loop.

2. **The schedule is front-readable.** Total notional, duration, and interval size are readable from contract storage before execution begins. Daily sell rate = `totalAmount / durationDays` is calculable with no inference required.

3. **Market makers cannot fully absorb without compensation.** A TWAP selling X% of average daily volume (ADV) for 7–14 consecutive days creates persistent one-sided flow. Market makers widen spreads and lean their inventory short, amplifying downward drift. This is a structural response to known flow, not speculation.

4. **The exit is also contractually defined.** TWAP completion is observable on-chain (contract drained, final swap emitted). This gives a clean, non-discretionary exit trigger.

5. **Governance-to-execution lag creates entry window.** Most DAOs require a 2–7 day timelock between vote passage and contract deployment. This window allows entry before selling begins, capturing the anticipation discount as well as the execution discount.

### Flow diagram

```
Governance vote passes
        │
        ▼
Timelock period (2–7 days) ◄── ENTRY WINDOW
        │
        ▼
TWAP contract deployed + funded ◄── CONFIRM ENTRY / ADD
        │
        ▼
Daily TWAP swaps execute (3–14 days)
        │
        ▼
Contract drained (final swap event) ◄── EXIT TRIGGER
```

### Analogous structural edge

This is mechanically similar to Zunid's token unlock shorts. The difference: unlock shorts target *new* supply entering circulation; this strategy targets *existing* treasury supply being converted to stablecoins via a fixed, public schedule. Both have contractually guaranteed supply events. Both have probabilistic price impact. Both have observable on-chain exit triggers.

---

## Universe Definition

### Eligible tokens

| Criterion | Threshold | Rationale |
|---|---|---|
| TWAP notional / 30-day ADV | ≥ 5% per day | Below this, market absorption is trivial |
| Token market cap | ≥ $50M | Smaller = your short is the market |
| Perp open interest available | Required | Need a liquid short instrument |
| Governance vote margin | ≥ 60% approval | Contested votes may be reversed |
| Timelock remaining at entry | ≥ 24 hours | Avoid chasing after execution starts |

### Disqualify if:

- DAO has a concurrent buyback program of comparable size
- Token is already down >30% in the 30 days prior (exhausted sellers)
- TWAP contract uses a price floor that may halt execution
- Token is a stablecoin or LST (no directional short thesis)
- Perp funding rate is already deeply negative (market already short)

---

## Entry Rules

### Signal detection

Monitor the following in real time or daily sweep:

1. **Snapshot / Tally** — governance proposals tagged with "treasury diversification," "runway extension," "sell," "USDC conversion"
2. **Milkman contract factory** — watch for new deployments funded with DAO treasury tokens
3. **CoW Protocol TWAP orders** — monitor `GPv2Settlement` and CoW TWAP factory for large single-token sell orders
4. **Uniswap TWAP** — watch for `LongTermOrder` contract deployments (TWAMM)
5. **Dune dashboards** — DAO treasury outflow trackers (e.g., DeepDAO, Llama, custom queries)

### Entry trigger (two-stage)

**Stage 1 — Anticipation entry (optional, smaller size):**
- Governance vote has passed with ≥60% approval
- Timelock has begun but TWAP not yet deployed
- Enter 25% of target position
- This captures the "vote passage discount" before execution begins

**Stage 2 — Confirmation entry (primary, larger size):**
- TWAP contract is deployed and funded on-chain (verifiable)
- First swap has not yet executed OR fewer than 20% of total notional has been sold
- Enter remaining 75% of target position
- This is the high-conviction entry: the sell is now mechanically active

### Entry instrument

- **Preferred:** Hyperliquid perpetual futures short on the relevant token
- **Fallback:** Spot borrow-and-sell if perp unavailable (requires liquid borrow market)
- **Do not use:** Options (theta decay works against a multi-day hold; also illiquid for most DAO tokens)

---

## Exit Rules

### Primary exit — TWAP completion (mechanical)

- Monitor TWAP contract for drain event: token balance reaches zero or final swap emits
- Close 100% of position within 4 hours of confirmed drain
- This is the non-discretionary exit. Do not hold through recovery.

### Secondary exit — Time stop

- If TWAP is not completed within `expectedDuration × 1.5`, exit 50% of position
- Reason: price floor conditions or governance intervention may be pausing execution
- Reassess remaining 50% based on contract state

### Stop loss — Structural invalidation

Exit immediately (full position) if any of the following occur:

1. DAO passes a second governance vote to cancel or pause the TWAP
2. A buyback program of comparable notional is announced
3. Token price rises >15% from entry (market absorption stronger than modeled; structural thesis weakening)
4. Perp funding rate goes deeply negative (>0.1% per 8h) — crowded short, risk of squeeze

### Profit target — Partial take

- At 50% of TWAP notional executed (observable on-chain), take 40% of position off
- Rationale: the steepest price impact typically occurs in the first half of a TWAP as market makers reprice inventory; the second half is often partially absorbed

---

## Position Sizing

### Base formula

```
Position size = min(
    TWAP_daily_notional × impact_multiplier,
    max_position_cap
)

Where:
  TWAP_daily_notional = total_TWAP_notional / duration_days
  impact_multiplier   = 0.5 (conservative; scale up post-backtest)
  max_position_cap    = 2% of portfolio NAV per trade
```

### Scaling logic

| TWAP daily sell / token ADV | Position size (% of max cap) |
|---|---|
| 5–10% | 25% |
| 10–20% | 50% |
| 20–40% | 75% |
| >40% | 100% (but flag for liquidity risk review) |

### Concentration limits

- No more than 3 simultaneous TWAP short positions
- No single position >2% of portfolio NAV
- Aggregate TWAP short exposure capped at 5% of portfolio NAV

---

## Backtest Methodology

### Data collection (manual phase — this is ugly work)

This strategy requires assembling a dataset that does not exist pre-packaged. That is a moat.

**Step 1 — Build the event database**

Query the following for the period 2021–present:

- Snapshot API: proposals with keywords "treasury diversification," "sell," "USDC," "runway" that passed
- Tally GraphQL API: same keyword filter, on-chain governance
- Milkman GitHub / contract deployments: all funded Milkman orders
- CoW Protocol subgraph: large TWAP orders (>$500K notional)
- Manual review of major DAO forums: Uniswap, Aave, Compound, Lido, ENS, Optimism, Arbitrum, dYdX, Synthetix

Target: 30–80 qualifying events. Fewer than 20 is insufficient for statistical inference.

**Step 2 — Define event windows**

For each event, record:
- `T0`: governance vote passage timestamp
- `T1`: TWAP contract deployment timestamp
- `T2`: first swap execution timestamp
- `T3`: TWAP completion timestamp
- Total notional (USD)
- Duration (days)
- Token ADV (30-day average at T0)
- TWAP daily sell / ADV ratio

**Step 3 — Price series**

Pull hourly OHLCV for each token from T0 − 7 days to T3 + 7 days. Sources: CoinGecko, Kaiko, or on-chain DEX data via Dune.

**Step 4 — Measure**

For each event, calculate:
- Return from T0 to T3 (full window)
- Return from T1 to T3 (confirmation entry to completion)
- Return from T3 to T3 + 7 days (post-TWAP recovery)
- Maximum adverse excursion (MAE) during holding period
- Correlation between (TWAP daily sell / ADV) and price return

**Step 5 — Segment analysis**

Break results by:
- TWAP size relative to ADV (small / medium / large)
- Market regime at T0 (bull / bear / sideways — use BTC 30-day return as proxy)
- Token category (DeFi governance, L1, L2, gaming)
- TWAP duration (short ≤5 days vs. long ≥10 days)

**Step 6 — Simulate execution**

Apply realistic assumptions:
- Entry slippage: 0.3% (perp market order)
- Exit slippage: 0.3%
- Funding cost: assume 0.01% per 8h (annualizes to ~4.5%) for duration of hold
- Borrow cost if using spot short: 5–15% annualized depending on token

**Minimum viability threshold for go-live:**
- Median return (T1 to T3) > 5% net of costs
- Win rate > 55% across all events
- Win rate > 65% when TWAP daily sell / ADV > 15%
- No single event loss > 20% (position sizing discipline check)

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

| Criterion | Requirement |
|---|---|
| Backtest sample size | ≥ 25 qualifying events |
| Median net return | > 5% per trade |
| Win rate (all events) | > 55% |
| Win rate (high ADV impact) | > 65% |
| Max drawdown per trade | < 20% |
| Monitoring pipeline | Automated alert on Milkman/CoW deployments |
| Execution infrastructure | Hyperliquid perp short confirmed available for ≥5 test tokens |
| Paper trade period | ≥ 3 live events paper traded with documented P&L |

---

## Kill Criteria (Post Go-Live)

Suspend the strategy immediately if:

1. **3 consecutive losses** exceeding 10% each — suggests structural change in market absorption
2. **Perp funding persistently negative** across multiple tokens simultaneously — strategy is crowded
3. **DAO governance meta-shift** — if DAOs begin using private OTC deals instead of on-chain TWAP (removes the information edge entirely)
4. **Regulatory action** on DAO governance tokens that freezes treasury activity
5. **Realized Sharpe < 0.5** over trailing 6-month live period

---

## Risks

### Primary risks

| Risk | Severity | Mitigation |
|---|---|---|
| Market absorption stronger than modeled | Medium | Size by ADV ratio; partial exit at 50% TWAP completion |
| Coordinated buy pressure / community defense | Medium | Stop loss at +15% price move from entry |
| TWAP paused by governance | Medium | Time stop at 1.5× expected duration |
| Perp short squeeze (crowded trade) | High | Monitor funding rate; exit if >0.1%/8h negative |
| Token illiquidity — your short moves the market | High | Hard cap: market cap ≥ $50M, perp OI ≥ $5M |
| TWAP uses price floor — execution halts | Low-Medium | Read contract parameters before entry |
| Information edge commoditized | Long-term | Monitor if Dune dashboards go mainstream; reassess moat |

### Second-order risks

- **Reflexivity:** If this strategy becomes known and widely traded, DAOs may switch to private OTC treasury sales, eliminating the on-chain signal entirely. This is a self-defeating edge at scale — keep position sizes small and do not publish the signal.
- **Governance token correlation:** In a broad market selloff, all governance tokens fall together. The TWAP short may be profitable but for the wrong reason — do not over-attribute alpha.
- **Funding rate drag:** For long-duration TWAPs (10–14 days), funding costs can erode 1–3% of notional. Model this explicitly in sizing.

---

## Data Sources

| Source | Use | Access |
|---|---|---|
| Snapshot API | Governance vote detection | Public REST API |
| Tally GraphQL | On-chain governance (Compound, Uniswap, etc.) | Public GraphQL |
| Milkman GitHub + Etherscan | Contract deployment monitoring | Public |
| CoW Protocol subgraph (The Graph) | TWAP order detection | Public GraphQL |
| Uniswap TWAMM contracts | Long-term order detection | Etherscan event logs |
| Dune Analytics | DAO treasury outflow dashboards | Public (custom queries) |
| DeepDAO / Llama | Treasury composition and history | Public API |
| CoinGecko / Kaiko | Historical OHLCV for backtest | CoinGecko free; Kaiko paid |
| Hyperliquid | Perp execution and funding data | Public API |

---

## Open Questions for Researcher Review

1. **What fraction of DAO treasury sales actually use on-chain TWAP vs. OTC?** If OTC is dominant, the observable universe shrinks significantly. Needs manual survey of 2022–2024 major DAO treasury actions.

2. **Is the price impact front-loaded or distributed?** Hypothesis: steepest decline occurs in the 24–48 hours after TWAP deployment (market makers reprice on flow information), not linearly across the TWAP duration. This would favor a shorter hold than full TWAP duration.

3. **Does the governance vote passage itself move price?** If yes, Stage 1 entry captures more alpha but requires faster detection. Measure T0 price reaction across the backtest sample.

4. **Can Hyperliquid perps be reliably shorted for mid-cap DAO tokens?** Many governance tokens are not listed on Hyperliquid. Fallback to spot borrow markets (Aave, Morpho) needs liquidity verification per token.

5. **What is the base rate of TWAP cancellation?** If >20% of TWAPs are cancelled mid-execution, the time stop rule needs tightening.

---

## Summary

This strategy is a structural short on mechanically guaranteed, publicly scheduled sell flow. The edge is not "DAOs tend to sell their tokens" — it is "this specific DAO has deployed a smart contract that will sell X tokens per day for Y days, and I can read the contract." The information is public but requires active on-chain monitoring that most market participants do not perform. The exit is as mechanical as the entry: contract drained = close position.

The primary uncertainty is price impact magnitude, not direction. The sell is guaranteed; how much the market discounts it in advance is the variable. Backtest priority is establishing the relationship between TWAP size / ADV ratio and realized price impact, segmented by market regime.

**Next step:** Build the event database. Start with Uniswap DAO, Aave, Compound, ENS, and Optimism treasury actions from 2022–2024. These are the largest, best-documented, and most likely to have had on-chain TWAP execution.
