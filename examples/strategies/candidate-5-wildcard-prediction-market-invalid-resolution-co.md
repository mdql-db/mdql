---
title: "UMA Dispute INVALID Front-Run — Prediction Market Collateral Arb"
status: HYPOTHESIS
mechanism: 7
implementation: 3
safety: 6
frequency: 1
composite: 126
categories:
  - defi-protocol
  - governance
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a UMA DVM dispute is filed on a Polymarket or Gnosis prediction market, and early DVM votes lean toward INVALID resolution (>60% of committed votes), the YES and NO tokens frequently trade below their guaranteed redemption floor of $0.50 each ($1.00 combined). Buying both legs before INVALID is confirmed locks in a risk-adjusted profit equal to ($1.00 − sum_of_entry_prices − gas_costs), with the $0.50-per-token redemption enforced by smart contract.

**Causal chain:**

1. A dispute is filed on a Polymarket market via UMA's Optimistic Oracle (OO).
2. The dispute enters UMA's DVM voting round (48-hour commit, 48-hour reveal cycle).
3. Early committed votes are visible on the UMA voter dashboard before the reveal phase closes.
4. Market participants — mostly retail — do not monitor UMA DVM vote tallies. They see a chaotic market and either panic-sell or stop trading entirely.
5. YES and NO prices decouple from their $0.50 INVALID floor because the information (early vote tally) exists in one system (UMA voter UI) but is not priced into another (Polymarket order book).
6. This is a cross-system information asymmetry: the structural outcome is partially visible before it is priced.
7. Once INVALID is confirmed on-chain, the Polymarket/Gnosis contract allows both YES and NO holders to redeem at exactly $0.50 per token against USDC collateral. This is not a market transaction — it is a contract call.

---

## Structural Mechanism (WHY This MUST Happen)

The $0.50 redemption is not a market expectation — it is a smart contract invariant.

**Polymarket INVALID redemption (Conditional Token Framework / CTF):**
- Polymarket uses Gnosis's Conditional Token Framework. On INVALID resolution, the condition is resolved with a payout vector of [0.5, 0.5] for [NO, YES].
- Contract: `ConditionalTokens.sol` — `reportPayouts()` sets the payout numerators. INVALID sets both to 0.5 × denominator.
- Any holder of YES or NO tokens calls `redeemPositions()` and receives exactly $0.50 USDC per token. No counterparty needed. No market price involved.

**UMA DVM resolution path:**
- When a Polymarket assertion is disputed, it escalates to UMA's Data Verification Mechanism (DVM).
- UMA tokenholders vote. If >50% vote INVALID (or the result is ambiguous), the DVM returns `INVALID` to the Polymarket oracle adapter.
- The Polymarket oracle adapter then calls `reportPayouts([0.5, 0.5])` on the CTF contract.
- This is deterministic: DVM output → oracle adapter → CTF payout. No human discretion after the vote.

**The information asymmetry window:**
- UMA commit-reveal voting: commit phase is 48 hours, reveal phase is 48 hours.
- After the commit phase closes, committed (but not yet revealed) votes are visible as encrypted hashes — not useful.
- However, UMA's voter interface at `vote.umaproject.org` shows revealed votes in real time during the reveal phase.
- The reveal phase is the entry window: INVALID votes are visible and tallying live, but Polymarket prices have not yet adjusted because most participants don't watch UMA governance.

---

## Entry/Exit Rules

### Monitoring (Pre-Entry)

1. Poll UMA's subgraph every 15 minutes for new `PriceRequestAdded` events on the `VotingV2` contract (Polygon/Ethereum).
2. Filter for requests where `identifier` matches Polymarket's oracle adapter address.
3. When a new dispute is detected, log the associated Polymarket market ID and begin tracking.
4. During the **reveal phase** of the DVM vote, poll `vote.umaproject.org` API (or directly query `VotingV2.getVoteCount()`) every 5 minutes for INVALID vote share.

### Entry Trigger (ALL conditions must be met)

| Condition | Threshold |
|-----------|-----------|
| INVALID vote share (revealed votes) | ≥ 60% |
| Minimum revealed votes cast | ≥ 100 UMA tokens (to avoid noise from tiny early reveals) |
| YES price + NO price (mid) | ≤ $0.96 |
| Liquidity: YES side depth to fill position | ≥ $500 within 2% of mid |
| Liquidity: NO side depth to fill position | ≥ $500 within 2% of mid |
| Time remaining in reveal phase | ≥ 2 hours (avoid last-minute vote flips with no time to exit) |

**Entry execution:**
- Buy YES and NO in equal USDC notional amounts simultaneously (or within 60 seconds).
- Use limit orders at mid + 0.5% to avoid moving a thin book.
- If one leg fills and the other does not within 60 seconds, cancel the unfilled leg and exit the filled leg at market. Do not hold a one-sided position.

### Exit

**Primary exit (expected):** After INVALID is confirmed on-chain, call `redeemPositions()` on the CTF contract for both YES and NO tokens. Receive $0.50 USDC per token. No market sale needed.

**Secondary exit (INVALID not declared):** If the DVM vote resolves YES or NO (not INVALID):
- The losing token goes to $0.00.
- The winning token goes to $1.00.
- Net position: $0.50 per token pair (same as INVALID redemption) — **only if you hold equal notional of both legs.**
- This is the natural hedge: equal-notional long YES + long NO always redeems to ~$1.00 total at binary resolution, or exactly $1.00 at INVALID. The only loss scenario is if you paid more than $1.00 total, which the entry rule prevents.

> **Critical note:** Equal USDC notional ≠ equal token count if prices differ. At YES=$0.20, NO=$0.30, buying $500 of each gives 2,500 YES tokens and 1,667 NO tokens. At binary resolution (e.g., YES wins at $1.00, NO at $0.00): receive $2,500 from YES, $0 from NO = $2,500 total on $1,000 invested = 2.5x. At INVALID: receive $1,250 + $833 = $2,083 on $1,000 = 2.08x. **Equal token count** is the correct hedge for INVALID arb. Recalculate: buy equal NUMBER of tokens, not equal dollar value.

**Revised entry:** Determine position size in tokens. Buy N YES tokens and N NO tokens. Total cost = N × (YES_price + NO_price). Guaranteed redemption = N × $1.00. Profit = N × (1.00 − YES_price − NO_price − gas_per_token).

**Kill-switch during hold:** If INVALID vote share drops below 40% at any point during the reveal phase, exit both legs at market immediately. Accept the loss. Do not wait for final resolution.

---

## Position Sizing

- **Maximum position per market:** $2,000 USDC notional (= N tokens × entry sum price).
- **Rationale:** Markets in dispute have thin liquidity. $2,000 is large enough to cover gas and generate meaningful profit, small enough to not move the book.
- **Minimum position:** Only enter if expected profit (N × (1.00 − sum_price) − estimated_gas) > $75 USDC.
- **Gas budget:** Estimate gas for two buy transactions + one `redeemPositions()` call. On Polygon (Polymarket's chain): typically $1–5 total. On Ethereum (some Gnosis markets): $20–80. Factor this into minimum profit threshold.
- **Portfolio-level cap:** No more than 3 concurrent INVALID arb positions. Total exposure cap: $6,000 USDC.
- **No leverage.** This is a cash-and-carry arb. Leverage adds liquidation risk to a position that has no market risk if sized correctly.

---

## Backtest Methodology

### Data Sources

| Data | Source | Notes |
|------|--------|-------|
| UMA DVM vote history | UMA subgraph on The Graph: `https://thegraph.com/explorer/subgraphs/...` (search "UMA Voting") | Query `PriceResolved` events with `price == INVALID` |
| Polymarket historical prices | Polymarket CLOB API: `https://clob.polymarket.com/` — historical trade data per market | Also: Dune Analytics dashboard #2575 (Polymarket trades) |
| Polymarket market metadata | `https://gamma-api.polymarket.com/markets` — includes condition ID, resolution status |
| UMA dispute-to-market mapping | UMA `OptimisticOracleV2` events: `RequestPrice`, `DisputePrice`, `Settle` — filter by Polymarket requester address |
| On-chain redemption confirmation | Polygon PoS: `ConditionalTokens` contract `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` — `ConditionResolution` events |

### Backtest Steps

1. **Pull all UMA DVM resolutions** from 2021–present where outcome = INVALID and the requester is Polymarket's oracle adapter. Expected count: 20–80 events based on ~2–5% INVALID rate across thousands of markets.

2. **For each INVALID event**, retrieve the associated Polymarket market's YES/NO price history from Polymarket CLOB API. Specifically:
   - Price at dispute filing timestamp (T=0)
   - Price at start of reveal phase (T=48h)
   - Price at end of reveal phase / INVALID confirmation (T=96h)
   - Minimum sum price during the reveal phase window

3. **Simulate entry** at the minimum sum price observed during the reveal phase, assuming you could have bought at that price (conservative: use VWAP of trades during reveal phase instead of minimum).

4. **Calculate gross profit** = 1.00 − entry_sum_price per token pair.

5. **Subtract gas costs** (use actual Polygon gas prices from the same timestamps via Polygonscan API).

6. **Calculate net profit per trade** and aggregate: hit rate, average return per trade, total return, max drawdown (from cases where INVALID was NOT declared after early vote showed >60%).

7. **Measure vote-flip rate:** Of all cases where reveal-phase INVALID vote share exceeded 60%, what % ultimately resolved as INVALID? This is the key risk parameter.

### Metrics to Report

- Number of qualifying events (INVALID vote >60% during reveal phase)
- Hit rate (% that resolved INVALID)
- Average gross profit per token pair
- Average net profit after gas
- Average hold time (dispute filing to redemption)
- Worst-case loss (vote flipped, one leg went to $0)
- Sharpe ratio (annualized, using hold period as time unit)
- Liquidity failure rate (% of events where one leg had <$500 depth)

### Baseline Comparison

Compare returns against simply holding USDC (risk-free rate) for the same hold periods. The arb should generate >15% annualized return on deployed capital to justify operational overhead.

---

## Go-Live Criteria (Paper Trading Threshold)

All of the following must be satisfied before paper trading:

1. **≥ 15 qualifying historical events** found (INVALID vote >60% during reveal phase). If fewer than 15 exist in history, the strategy cannot be validated — park it.
2. **Vote-flip rate ≤ 20%** (i.e., ≥80% of cases where early vote showed >60% INVALID ultimately resolved INVALID).
3. **Average net profit per trade ≥ $80** on a $2,000 position (4% net).
4. **Liquidity failure rate ≤ 30%** (i.e., in ≥70% of cases, both legs had sufficient depth to fill $1,000 each).
5. **No single trade loss exceeds $400** in backtest (20% of max position — this would occur if vote flipped and one leg went to $0 while the other went to $1.00, netting $1,000 on $2,000 invested, a $1,000 loss — which means the $0.96 entry cap must be enforced strictly to limit this).

> **Note on loss scenario math:** If you buy N YES at $0.30 and N NO at $0.60 (sum = $0.90, valid entry), and INVALID is NOT declared, and YES wins: receive $1.00 per YES token, $0.00 per NO token = $1.00 total vs $0.90 cost = still profitable. The only loss scenario is if sum > $1.00 at entry, which the entry rule prevents. **Re-evaluate: this strategy may actually have no loss scenario if entry sum < $1.00 and you hold equal token counts.** Backtest must verify this claim explicitly.

---

## Kill Criteria (Abandon Strategy)

- Backtest finds fewer than 10 qualifying events in 3+ years of history → insufficient frequency, not worth building infrastructure.
- Vote-flip rate > 35% in backtest → early vote signal is unreliable.
- Polymarket changes oracle to non-UMA system (e.g., migrates to Chainlink or in-house oracle) → structural mechanism no longer applies. Monitor Polymarket blog and contract upgrades.
- UMA changes DVM to private/encrypted voting throughout (no early reveal visibility) → information asymmetry window closes.
- Gas costs on Polygon spike to >$50 per round trip (e.g., during network congestion) → minimum profit threshold makes most trades unviable.
- After 6 months of paper trading: fewer than 3 live qualifying events observed → frequency too low to justify ongoing monitoring infrastructure.

---

## Risks (Honest Assessment)

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Vote flips: early INVALID lead reverses | High | Medium (~20–30% estimated) | 60% threshold + kill-switch if drops below 40% |
| One-leg liquidity gone (market at $0.01/$0.99) | Medium | High (common in disputed markets) | Liquidity check before entry; skip if either leg <$500 depth |
| INVALID not declared; equal-token position still profitable | Low | High | Entry sum <$1.00 guarantees profit at binary resolution too — verify in backtest |
| Resolution delay (weeks if appealed to UMA governance) | Low | Low (~5% of disputes) | Capital is locked but not at risk; size accordingly |
| Smart contract bug in CTF redemption | Catastrophic | Very Low | Polymarket CTF contract is battle-tested since 2020; audit history available |
| Polymarket API rate limits / data gaps | Operational | Medium | Cache all data locally; use on-chain data as backup |
| UMA oracle migration away from DVM | Strategy-ending | Low (12-month horizon) | Monitor Polymarket governance forum and contract upgrade events |
| Regulatory action freezing Polymarket | Strategy-ending | Low | Diversify to Gnosis markets on Ethereum as backup venue |
| Front-running by other arb bots | Medium | Low (niche enough) | This is not a well-known arb; UMA dispute monitoring is not commoditised |

**Honest overall assessment:** The redemption guarantee is real and contractually enforced. The main uncertainty is whether the early vote signal is reliable enough (vote-flip rate) and whether liquidity exists to enter both legs. The backtest will answer both questions definitively. If vote-flip rate is low and liquidity is adequate in historical data, this is a genuine structural arb with no loss scenario at binary resolution (only at INVALID resolution if entry sum > $1.00, which the entry rule prevents). The frequency constraint (~20–80 qualifying events per year across all Polymarket markets) means this is a supplementary strategy, not a primary one.

---

## Data Sources (Consolidated)

| Resource | URL / Endpoint |
|----------|---------------|
| UMA Voting subgraph | `https://thegraph.com/explorer/` → search "UMA Voting Ethereum" |
| UMA voter dashboard | `https://vote.umaproject.org` |
| UMA OptimisticOracleV2 (Polygon) | `0xee3afe347d5c74317041e2618c49534daf887c24` |
| Polymarket CLOB historical trades | `https://clob.polymarket.com/trades?market={condition_id}` |
| Polymarket market metadata | `https://gamma-api.polymarket.com/markets?closed=true&limit=500` |
| Polymarket CTF contract (Polygon) | `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045` |
| Dune: Polymarket trade history | `https://dune.com/queries/2575` (Polymarket analytics) |
| Polygonscan API (gas history) | `https://api.polygonscan.com/api?module=gastracker` |
| UMA DVM documentation | `https://docs.umaproject.org/protocol-overview/how-does-umas-oracle-work` |
