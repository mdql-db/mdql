---
title: "Nexus Mutual Claim Vote Deadline — Cover Asset Short"
status: HYPOTHESIS
mechanism: 4
implementation: 5
safety: 5
frequency: 1
composite: 100
categories:
  - defi-protocol
  - governance
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large Nexus Mutual claim (>$500K) is submitted, the market systematically underprices two distinct forced-selling pressures that activate upon claim approval:

1. **NXM dilution pressure:** Approved claims are paid from the Nexus Mutual capital pool. If the pool's ETH/DAI reserves are insufficient, the protocol mints new NXM to cover the shortfall, directly diluting existing holders. Even when reserves are sufficient, capital pool NAV per NXM decreases mechanically.

2. **Covered protocol signal pressure:** A submitted claim on Protocol X is public evidence that an exploit or loss event has occurred. Claim approval is a second-order confirmation signal — it means independent assessors (NXM stakers with skin in the game) have verified the loss is real and covered. This is structurally different from rumour; it is a staked-capital vote.

**Causal chain:**
```
Claim submitted (public, on-chain)
        ↓
Market partially prices exploit risk, but underweights:
  (a) Probability-weighted capital pool liquidation
  (b) Assessor confirmation signal value
        ↓
Voting window closes → claim approved
        ↓
Protocol MUST execute payout within defined timelock
        ↓
Capital pool assets liquidated / NXM minted → forced sell pressure
Covered protocol token: assessor confirmation triggers second wave of exits
        ↓
Price impact materialises over 24–72h post-verdict
```

**Why the market underprices this:** Nexus Mutual claims are niche, low-frequency events. Most market participants do not monitor `app.nexusmutual.io` claim feeds. The voting window (7 days) creates a slow-moving information release that fast money ignores and slow money hasn't processed yet. The structural edge is in the *gap between on-chain information availability and market pricing*.

---

## Structural Mechanism — WHY This MUST Happen

### Mechanism A: Capital Pool Payout (Contractually Forced)

Nexus Mutual's smart contracts enforce payout execution upon claim approval. Specifically:

- Claims are assessed via the `ClaimsAssessment` contract. Once quorum is reached and the voting window closes with a YES majority, the `Claims` contract triggers a payout call that cannot be vetoed by any party (including the Nexus Mutual Foundation).
- Payout is denominated in ETH or DAI depending on the cover type. The capital pool holds a mix of ETH, DAI, and staked assets. If ETH must be sourced, the pool either uses reserves directly (reducing NAV/NXM) or liquidates other positions.
- This is **not discretionary**. The smart contract executes. There is no governance override post-approval.

**Quantification:** A $1M claim against a pool with ~$300M TVL reduces pool NAV by ~0.33%. NXM price tracks pool NAV (the bonding curve formula: `A + (mcap / C)` where A and C are constants). Therefore NXM price MUST decline by approximately the same proportion, all else equal. For a $5M claim, that's ~1.65% forced NXM price reduction from NAV mechanics alone, before any panic selling.

### Mechanism B: Assessor Confirmation as Information Event

NXM stakers who vote on claims have their stake locked during the voting period and face slashing if they vote against the eventual consensus. This creates a costly signal: a YES vote is a staker putting capital at risk to confirm the loss is real. Claim approval therefore carries more information content than an anonymous on-chain report of an exploit.

This is structurally similar to a credit rating downgrade — not new information per se, but a credentialed confirmation that triggers rule-based selling by participants who were waiting for verification.

### Mechanism C: Cover Expiry Cliff (Secondary)

If a claim is denied, cover holders may attempt to exit their positions in the covered protocol anyway (loss of insurance = loss of risk mitigation = rational rebalancing). This creates a *weaker* secondary signal even on denial — but this is probabilistic, not structural, and is NOT the primary trade.

---

## Entry/Exit Rules

### Instrument Selection

**Leg 1 — NXM short:**
- Use wNXM (wrapped NXM) on Uniswap/Sushiswap if no perp available
- Check Hyperliquid and Binance for NXM/wNXM perp availability at time of trade; if unavailable, skip Leg 1 or use spot short via borrow
- wNXM trades at a discount/premium to NXM NAV; track the spread before entry

**Leg 2 — Covered protocol token short:**
- Only trade if the covered protocol token has a liquid perp on Hyperliquid or Binance (>$1M daily volume minimum)
- If no liquid perp exists, skip Leg 2 entirely — do not force illiquid trades

### Entry Conditions (ALL must be true)

| Condition | Threshold | Data Source |
|-----------|-----------|-------------|
| Claim submitted | Confirmed on-chain | Nexus Mutual claim tracker |
| Claim size | >$500K USD equivalent | Claim details page |
| Time to vote close | >48h remaining | Voting deadline on claim page |
| Covered protocol token | Liquid perp exists (>$1M/day volume) | Hyperliquid, Binance |
| Claim type | Smart contract exploit or oracle manipulation (NOT custody/CeFi) | Claim category field |
| Prior claims on same protocol | No open claim already priced in | Manual check |

**Entry timing:** Enter within 4 hours of claim submission being confirmed on-chain. Do not chase if >24h has already elapsed since submission.

**Entry split:**
- 60% of position in covered protocol token short (higher conviction, more liquid)
- 40% of position in NXM/wNXM short (structural NAV mechanism, less liquid)

### Exit Rules

| Scenario | Action | Timing |
|----------|--------|--------|
| Claim APPROVED | Hold full position | Through verdict |
| Post-approval | Scale out 50% | Within 6h of approval announcement |
| Post-approval | Scale out remaining 50% | 48h after payout execution confirmed on-chain |
| Claim DENIED | Exit 100% immediately | Within 1h of denial announcement |
| Claim still pending at T-6h to deadline | Reduce position by 50% | Uncertainty hedge |
| NXM spot moves >15% against position | Stop loss, exit 100% | Immediately |
| Covered token moves >20% against position | Stop loss, exit 100% | Immediately |

**Do not hold through a second voting round** if the claim is sent back for re-assessment — treat as denial and exit.

---

## Position Sizing

### Base Sizing

- Maximum position per trade: **1% of total portfolio** (this is a low-frequency, binary-outcome trade)
- Split: 0.6% in covered token short, 0.4% in NXM short
- Never size up based on "high confidence" — the binary outcome means Kelly sizing is punishing on losses

### Kelly Approximation (for reference, not hard rule)

```
f* = (p × b - q) / b

Where:
  p = estimated claim approval probability
  q = 1 - p
  b = expected return if approved (estimated price move)

Example: p=0.35, b=0.15 (15% price move on approval), q=0.65
f* = (0.35 × 0.15 - 0.65) / 0.15 = (0.0525 - 0.65) / 0.15 = -3.98

Negative Kelly → this is a negative EV trade at these parameters UNLESS
the price move on approval is large enough. Requires b > q/p = 1.86 (186% move)
to be positive Kelly at 35% approval rate.
```

**Implication:** The trade only has positive EV if:
- Approval probability is higher than base rate (you have an informational edge on this specific claim), OR
- The expected price move on approval is very large (>50% for covered token), OR
- The cost of carry (funding, borrow) is low enough that the option-like payoff justifies the position

**Practical sizing rule:** Use 0.5–1% of portfolio maximum. Treat this as a binary option, not a directional trade. Do not average in.

### Liquidity Constraint

- NXM/wNXM: Do not exceed $50K notional in wNXM — liquidity is thin and slippage will destroy edge
- Covered token: Size to max 0.5% of 24h volume to avoid moving the market

---

## Backtest Methodology

### Data Collection

**Step 1: Build the claim database**

Source: Nexus Mutual subgraph on The Graph
- Endpoint: `https://api.thegraph.com/subgraphs/name/nexusmutual/nexus-mutual`
- Query: All claims from protocol launch (May 2019) to present
- Fields needed: `claimId`, `coverId`, `submissionDate`, `votingDeadline`, `outcome`, `payoutAmount`, `coveredProtocol`, `coverAsset`

Cross-reference with: `https://app.nexusmutual.io/claims` (manual verification for large claims)

Expected dataset size: ~150–200 total claims; filter to >$500K → likely 20–40 events

**Step 2: Price data**

- NXM/wNXM: CoinGecko API (`https://api.coingecko.com/api/v3/coins/wrapped-nxm/market_chart`) — daily OHLCV from 2020
- Covered protocol tokens: CoinGecko or Binance API for each token, pulling OHLCV for the window: T-7 days to T+7 days around each claim submission date
- ETH price (for NAV calculation): Same sources

**Step 3: Define measurement windows**

For each qualifying claim event:
- `T0` = claim submission timestamp
- `T1` = voting deadline
- `T2` = payout execution (if approved)
- Measure price change: `T0` to `T1`, `T0` to `T2`, `T1` to `T2`

### Metrics to Calculate

| Metric | Definition |
|--------|------------|
| Hit rate | % of approved claims where covered token fell >5% from T0 to T2 |
| Average return (approved) | Mean price change T0→T2 for covered token, approved claims only |
| Average return (denied) | Mean price change T0→T1 for covered token, denied claims only |
| NXM NAV impact | Actual NXM price change vs. theoretical NAV change from payout |
| Max adverse excursion | Worst intra-trade drawdown before exit |
| Slippage estimate | wNXM bid-ask spread at time of each event (if data available) |

### Baseline Comparison

Compare covered token returns during claim windows against:
1. ETH return over same window (market beta control)
2. Random 7-day windows for same tokens (no claim) — 100 random samples per token
3. Sector index (DeFi tokens) return over same window

**Null hypothesis:** Covered token returns during claim windows are not statistically different from random windows. Reject at p<0.05.

### What to Look For

- Approved claims: Is there a consistent negative return in covered token from T0→T2 that exceeds market beta?
- Is the return concentrated in T1→T2 (post-verdict) or T0→T1 (during voting)? This determines optimal entry timing.
- NXM: Does wNXM price decline track theoretical NAV reduction from payout? If not, why not (wNXM discount dynamics)?
- Are there specific claim types (smart contract exploit vs. custody) with different return profiles?

### Sample Size Warning

**This is the critical limitation.** With ~20–40 qualifying events over 5 years, statistical significance is nearly impossible to achieve. The backtest is primarily useful for:
1. Confirming the mechanism is real (not zero effect)
2. Estimating magnitude of moves for position sizing
3. Identifying which claim types drive the effect
4. Finding the optimal entry/exit timing within the window

Do not over-fit to 20 events. The backtest is hypothesis validation, not optimisation.

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show ALL of the following:

| Criterion | Threshold |
|-----------|-----------|
| Qualifying events identified | ≥15 events with complete data |
| Approved claim covered token return | Median return < -5% from T0 to T2 |
| Denied claim covered token return | Median return within ±3% (confirms denial kills the trade) |
| NXM NAV tracking | wNXM price decline within 2x of theoretical NAV reduction on approval |
| Positive EV estimate | Expected value positive after estimated slippage and funding costs |
| No single event dominates | No single event accounts for >40% of total P&L |

If any criterion fails, the strategy does not advance to paper trading. Return to hypothesis revision.

---

## Kill Criteria

### Kill during backtest

- Fewer than 10 qualifying events found in historical data → insufficient sample, strategy is untestable
- Approved claims show no consistent directional move in covered token (median return > -2%)
- wNXM liquidity is insufficient to execute even $25K notional without >3% slippage

### Kill during paper trading

- First 5 paper trades show 0 profitable outcomes (even accounting for binary nature, 0/5 is informative)
- A structural change to Nexus Mutual's claims process (e.g., migration to v2 with different mechanics) invalidates the mechanism
- Nexus Mutual TVL drops below $50M (claim events become too small to generate meaningful price impact)
- Claim frequency drops below 2 qualifying events per year (strategy becomes operationally irrelevant)

### Kill in live trading

- 10 consecutive losing trades
- Sharpe ratio below 0.5 after 20 live trades
- NXM/wNXM liquidity deteriorates to the point where position sizing must drop below $10K notional (not worth operational overhead)

---

## Risks

### Risk 1: Claim denial (primary risk, ~60–70% of events)
The majority of claims are denied. A denial means the short was wrong and must be exited immediately. The stop-loss on denial is the most important rule in this strategy. Slippage on exit after a denial (when the covered token may spike) can be severe.

**Mitigation:** Size small. Treat as binary option. Never add to a losing position during the voting window.

### Risk 2: NXM/wNXM illiquidity
wNXM on Uniswap has thin liquidity. A $100K short could move the market 5–10%. The wNXM/NXM arbitrage mechanism is also imperfect — wNXM can trade at a persistent discount to NAV that doesn't close on the expected timeline.

**Mitigation:** Cap NXM leg at $50K notional. Accept that NXM leg may be unexecutable for some events.

### Risk 3: Pre-pricing
For high-profile exploits (e.g., a major protocol hack), the covered token may already be down 50–80% before the claim is even submitted. The claim submission is lagging information, not leading.

**Mitigation:** Check covered token price action in the 48h before claim submission. If the token is already down >30% from its pre-exploit price, skip the trade — the move has happened.

### Risk 4: Claim size vs. pool size
A $500K claim against a $300M pool has negligible NAV impact (~0.17%). The NXM leg only makes sense for claims that are large relative to pool TVL (>0.5% of pool).

**Mitigation:** Add a filter: claim size must be >0.3% of current Nexus Mutual capital pool TVL. Check pool TVL at `https://app.nexusmutual.io/` at time of claim submission.

### Risk 5: Protocol migration / mechanism change
Nexus Mutual has undergone significant upgrades (v1 → v2). The claims mechanism may change again, invalidating the structural logic.

**Mitigation:** Re-read the claims contract code (`https://github.com/NexusMutual/smart-contracts`) before each trade to confirm the payout mechanism is unchanged.

### Risk 6: Regulatory / counterparty risk on wNXM
wNXM is a wrapped token with its own smart contract risk. If the wNXM wrapper is exploited, the short position becomes worthless.

**Mitigation:** Use only established wNXM contracts. Do not hold wNXM short positions for longer than necessary.

### Risk 7: Low frequency makes this operationally expensive
At 2–5 qualifying events per year, the operational overhead (monitoring, execution, review) may not justify the expected P&L at 1% position sizing.

**Mitigation:** Automate the claim monitoring step. Set up a webhook or cron job to query the Nexus Mutual subgraph daily and alert on new large claims.

---

## Data Sources

| Data | Source | URL / Endpoint |
|------|--------|----------------|
| Nexus Mutual claims (live) | Nexus Mutual app | `https://app.nexusmutual.io/claims` |
| Nexus Mutual claims (historical, programmatic) | The Graph subgraph | `https://api.thegraph.com/subgraphs/name/nexusmutual/nexus-mutual` |
| Nexus Mutual smart contracts | GitHub | `https://github.com/NexusMutual/smart-contracts` |
| Capital pool TVL | Nexus Mutual app / DefiLlama | `https://defillama.com/protocol/nexus-mutual` |
| wNXM price history | CoinGecko | `https://api.coingecko.com/api/v3/coins/wrapped-nxm/market_chart?vs_currency=usd&days=max` |
| NXM price history | CoinGecko | `https://api.coingecko.com/api/v3/coins/nxm/market_chart?vs_currency=usd&days=max` |
| Covered protocol token prices | CoinGecko / Binance API | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| Perp availability / volume | Hyperliquid | `https://api.hyperliquid.xyz/info` |
| Historical exploit database (cross-reference) | Rekt News | `https://rekt.news` |
| Historical exploit database (programmatic) | DeFiHackLabs | `https://github.com/SunWeb3Sec/DeFiHackLabs` |
| wNXM Uniswap pool liquidity | Uniswap v3 subgraph | `https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3` |

### Monitoring Setup (Operational)

To catch claims within the 4-hour entry window, set up:

```python
# Pseudocode for claim monitor
# Query every 30 minutes

query = """
{
  claims(
    orderBy: submittedAt
    orderDirection: desc
    first: 10
    where: { submittedAt_gt: $last_check_timestamp }
  ) {
    id
    coverId
    submittedAt
    votingDeadline
    amount
    coverAsset
    status
  }
}
"""
# Alert if: amount > 500000 AND status == "PENDING" AND votingDeadline - now > 48h
```

Alert channel: Telegram bot or Discord webhook. Do not rely on manual monitoring — the 4-hour entry window is too tight.

---

## Open Questions for Backtest Phase

1. Does the covered token move more during T0→T1 (voting window) or T1→T2 (post-verdict)? This determines whether to enter before or after the verdict.
2. Is there a claim approval probability signal in the early vote tally (if vote counts are public before deadline)?
3. Do NXM stakers who vote YES on a claim also sell NXM before the verdict (insider-ish behaviour)? Check NXM wallet flows around large claims.
4. What is the wNXM/NXM spread behaviour during claim events? Does the discount widen (fear) or narrow (arb)?
5. Are there any claims where the covered token had no liquid perp but the strategy would have been highly profitable — i.e., are we systematically missing the best events due to liquidity filters?
