---
title: "Pump.fun Bonding Curve Graduation — DEX Listing Overshoot Short"
status: HYPOTHESIS
mechanism: 5
implementation: 2
safety: 2
frequency: 9
composite: 180
categories:
  - defi-protocol
  - liquidation
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a Pump.fun token graduates to Raydium, the mechanical transition from bonding curve to open AMM pool triggers a predictable retail FOMO spike into thin liquidity. This spike is structurally unsustainable: the graduation event adds no new value, the initial liquidity pool is shallow (~$12K), and early bonding curve participants have been waiting for exactly this moment to exit. The token should mean-revert sharply within 15–90 minutes of the post-graduation peak.

**The core claim:** The graduation spike is a liquidity mirage. Retail buyers interpret DEX listing as legitimacy signal and buy aggressively. Early holders interpret the same event as their exit window and sell into that buying. The asymmetry between informed sellers (bonding curve participants with cost basis near zero) and uninformed buyers (FOMO retail) creates a predictable, short-lived overshoot.

**What this strategy is NOT claiming:** That all graduating tokens dump. The claim is specifically that tokens which spike >3x within 10 minutes of Raydium listing are exhibiting the FOMO overshoot pattern and are likely to retrace materially from that spike peak — not from the graduation price.

---

## Structural Mechanism

### 2a. The Hard-Coded Graduation Trigger

Pump.fun's bonding curve contract graduates a token to Raydium when the token's market cap reaches exactly **$69,000 USD** (denominated in SOL at time of graduation). This threshold is immutable in the smart contract. At graduation:

- The bonding curve contract is permanently locked — no further buys/sells on Pump.fun
- Approximately **$12,000 of liquidity** (in SOL + token) is automatically seeded into a new Raydium AMM pool
- The LP tokens from this seed are **burned** — the liquidity is permanently locked at launch
- The token becomes freely tradeable on Raydium and visible on Dexscreener/Birdeye

This is a **smart contract invariant**. Every graduating token undergoes identical mechanics. The $12K seed liquidity figure is fixed by the contract parameters at the time of writing; verify against current contract state before backtesting.

### 2b. Why the Spike Happens (Causal Story)

1. **Legitimacy signal misread:** Retail participants treat Raydium listing as a quality filter. It is not — graduation is purely mechanical and requires only $69K in bonding curve buys.
2. **Aggregator visibility:** Graduation triggers appearance on Dexscreener, Birdeye, and Jupiter. This creates a new audience of buyers who never saw the bonding curve phase.
3. **Thin initial pool:** $12K of liquidity means even modest buy pressure ($5–20K) moves price dramatically. A 3x spike on $12K liquidity requires only ~$24K of net buying — achievable by a handful of wallets.
4. **Reflexive momentum:** Price moving up on aggregators attracts more buyers, compounding the spike briefly.

### 2c. Why the Reversion Happens (Causal Story)

1. **Bonding curve holders have near-zero cost basis:** Participants who bought early on the bonding curve paid fractions of a cent. At 3x post-graduation spike, they are sitting on 10–100x returns. Graduation is their first liquid exit.
2. **No new fundamental value:** The token has no utility, no team commitment post-graduation (in most cases), no locked team allocation. The graduation event is purely mechanical.
3. **Liquidity is thin in both directions:** The same thin pool that allowed the spike up will allow rapid price collapse on sell pressure. There is no institutional bid to absorb selling.
4. **Time decay of attention:** Dexscreener shows hundreds of new tokens daily. Retail attention moves on within minutes to the next graduation event.

### 2d. Why This Is Structural, Not Just Historical

The mechanism is **game-theoretic and forced**: bonding curve holders MUST wait for graduation to exit (the bonding curve locks them in until $69K). Graduation is therefore a **contractually scheduled exit event** for early holders, analogous to a token unlock. The FOMO spike is the exit liquidity they are waiting for. This is not a tendency — it is the rational dominant strategy for early holders.

---

## Entry Rules

### Pre-conditions (all must be true)
- [ ] Token has graduated to Raydium within the last 15 minutes (confirmed via on-chain graduation event or Pump.fun API)
- [ ] Token price has spiked **≥3x** from the graduation price within 10 minutes of Raydium listing
- [ ] Current Raydium pool liquidity is **≥$30,000** (below this, slippage makes shorting uneconomical)
- [ ] A clear **reversal candle** has printed on the 1-minute chart (lower high + increased sell volume) — this is the entry trigger, not the spike itself
- [ ] No single wallet holds >20% of supply (check on-chain; extreme concentration = manipulation risk, skip)

### Entry Execution
- **Instrument:** Spot short via Jupiter aggregator lending (if available for the specific token) OR Drift Protocol isolated margin (if perp exists — rare for new tokens)
- **Realistic assessment:** For most graduating tokens, **direct short selling is not feasible**. The primary executable expression is: **do not hold long positions entered during the bonding curve phase past the graduation spike peak**. The short is a secondary, opportunistic expression.
- **Entry price:** Market order at reversal candle close, accepting up to 3% slippage
- **Entry timing:** 10–20 minutes post-graduation (not at graduation itself — wait for spike and reversal confirmation)

---

## Exit Rules

### Take Profit
- **Primary target:** 40% decline from entry price
- **Secondary target:** Return to graduation price ($69K market cap equivalent) — this represents full mean reversion

### Stop Loss
- **Hard stop:** 25% adverse move from entry (token continues spiking after entry)
- **Rationale:** If the token is still moving up 20+ minutes post-graduation, the FOMO cycle has more legs than expected; the thesis is wrong for this instance

### Time Stop
- **Maximum hold:** 90 minutes from entry
- **Rationale:** If reversion hasn't occurred within 90 minutes, either a new narrative has attached to the token or a whale is actively supporting price. Exit regardless of P&L.

### Forced Exit Conditions
- Pool liquidity drops below $15,000 (exit becomes impossible without catastrophic slippage)
- Token is delisted from Raydium or pool is migrated

---

## Position Sizing

### Hard Limits
- **Maximum position size:** $2,000 per trade (liquidity constraint — larger sizes create self-defeating slippage)
- **Maximum concurrent positions:** 2 (these tokens are highly correlated in risk-off scenarios)
- **Maximum daily exposure:** $4,000 across all graduation shorts

### Sizing Logic
- Size to **1% of pool liquidity** at entry, not to a fixed dollar amount
- If pool liquidity = $50K, max position = $500
- If pool liquidity = $200K, max position = $2,000 (hard cap)
- This is a **volume-constrained** strategy, not a capital-constrained one

### Expected Return Profile (Hypothesis — Not Backtested)
- Win rate hypothesis: 55–65% of qualifying setups (tokens that spike >3x and show reversal candle)
- Average win: 35–45% gain on position
- Average loss: 20–25% loss on position
- Expected value per trade: Positive if win rate >45% given the asymmetric payoff — **needs backtest to confirm**

---

## Backtest Methodology

### Data Requirements
- **Pump.fun graduation events:** Available via Pump.fun API (`/coins` endpoint filtered by `raydium_pool` field becoming non-null) or on-chain via Solana RPC (program ID: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`)
- **Raydium price data:** On-chain via Raydium AMM program logs or Birdeye historical OHLCV API (free tier available)
- **Pool liquidity data:** Raydium pool state accounts, queryable via Solana RPC
- **Wallet concentration:** On-chain token account data

### Backtest Period
- **Minimum:** 90 days of graduation events
- **Recommended:** 6 months (captures varying market regimes — bull/bear/sideways)
- **Sample size target:** ≥200 qualifying graduation events (tokens that spike >3x)

### Backtest Steps

**Step 1 — Event identification**
Extract all graduation timestamps from on-chain data. Record graduation price (SOL price × token supply at graduation).

**Step 2 — Spike filter**
For each graduation, compute max price within 10 minutes of first Raydium trade. Flag events where max price ≥ 3× graduation price.

**Step 3 — Reversal candle identification**
On 1-minute OHLCV, identify the first candle after the spike peak where close < open AND volume > prior candle volume. Record this as simulated entry price.

**Step 4 — Simulate exits**
Apply take profit (−40%), stop loss (+25%), and time stop (90 min) rules. Record outcome for each trade.

**Step 5 — Liquidity filter**
Exclude any trade where pool liquidity at entry < $30K. Record how many qualifying events are excluded by this filter.

**Step 6 — Slippage adjustment**
Apply 3% slippage penalty to all entries and exits (conservative estimate for thin pools). Recompute P&L.

**Step 7 — Regime analysis**
Segment results by: (a) SOL price trend at time of graduation, (b) number of graduations that day (proxy for market heat), (c) time of day (UTC).

### Key Metrics to Report
- Win rate (raw and slippage-adjusted)
- Average win / average loss ratio
- Maximum drawdown (sequence of losses)
- Sharpe ratio (if sample size permits)
- % of graduation events that pass the >3x spike filter
- % of qualifying events excluded by liquidity filter

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

- [ ] Backtest shows **positive expected value after 3% slippage** across ≥200 qualifying events
- [ ] Win rate ≥ 50% in backtest
- [ ] Strategy is profitable in **at least 3 of 4 quarterly sub-periods** in backtest (no single-period fluke)
- [ ] A reliable, automated **graduation event detector** is built and tested (manual monitoring is not scalable)
- [ ] At least one **short execution pathway** is confirmed live (Jupiter lending or Drift) — if no short pathway exists, strategy is reclassified as "long exit signal only"
- [ ] Paper trading for **30 days** with ≥20 qualifying events shows results consistent with backtest

---

## Kill Criteria

Abandon or pause the strategy if any of the following occur:

### During Backtesting
- Backtest expected value is negative after slippage on full sample
- Win rate < 45% on full sample
- Strategy is profitable only in bull market sub-periods (regime-dependent, not structural)
- Fewer than 5 qualifying events per week on average (insufficient frequency to justify infrastructure cost)

### During Paper/Live Trading
- 5 consecutive losing trades (pause, review, do not increase size)
- Live results deviate from backtest win rate by >15 percentage points over 30+ trades (execution or data issue)
- Pump.fun changes graduation threshold or liquidity seeding mechanics (structural change invalidates thesis)
- Raydium pool structure changes (e.g., concentrated liquidity migration changes slippage dynamics)
- Regulatory action targeting Pump.fun or Solana meme coin infrastructure

### Permanent Kill
- Pump.fun graduation mechanic is materially altered by protocol upgrade
- Short selling infrastructure for new Solana tokens becomes unavailable

---

## Risks

### Execution Risks (HIGH)
- **Slippage:** $12–50K pools mean even $1K orders move price 2–5%. Modeled at 3% but could be worse.
- **Short availability:** Most new Raydium tokens cannot be shorted. Jupiter lending requires existing lenders; Drift perps rarely list tokens this new. **This is the primary execution blocker.**
- **Front-running:** On-chain graduation events are public. Other bots may be executing the same trade faster. We are not an HFT firm — we accept we will not get the best entry.

### Market Risks (HIGH)
- **Whale continuation:** A single whale can keep buying post-spike, triggering the stop loss. Thin liquidity cuts both ways.
- **Narrative attachment:** Occasionally a graduating token gets picked up by a KOL or Crypto Twitter narrative mid-spike. The thesis breaks entirely in these cases. No reliable way to filter in advance.
- **Rug pull mechanics:** Some tokens are designed to dump immediately at graduation (team holds large supply). This is actually directionally correct for the short but may happen too fast to enter.

### Structural Risks (MEDIUM)
- **Pump.fun protocol changes:** The $69K graduation threshold and $12K liquidity seed are current parameters. A protocol upgrade could change these, altering the mechanics entirely. Monitor Pump.fun GitHub and governance.
- **Market saturation:** If this pattern becomes widely known and traded, the spike may not materialize (no FOMO buyers) or may be front-run so aggressively the entry window disappears.

### Operational Risks (MEDIUM)
- **Data pipeline failure:** Missing a graduation event or getting stale price data leads to missed entries or wrong entries. Requires robust real-time data infrastructure.
- **Token scam/honeypot:** Some tokens have transfer restrictions that prevent selling. Always verify token contract is not a honeypot before entering any position (use RugCheck.xyz or equivalent).

### Honest Assessment of Score
The **5/10 score reflects genuine uncertainty**. The graduation mechanic is contractually fixed (structural), but the overshoot pattern is probabilistic (not guaranteed). The bigger problem is execution: the most natural expression of this edge (short selling) is operationally very difficult for tokens this new and illiquid. The strategy is more defensible as a **"do not hold long past graduation spike"** rule for bonding curve participants than as a standalone short strategy. Reclassification to a long-exit signal (score: 6/10 for that specific expression) may be warranted after backtesting.

---

## Data Sources

| Data Type | Source | Cost | Notes |
|---|---|---|---|
| Graduation events | Pump.fun API (`/coins`) | Free | Poll for `raydium_pool` field becoming non-null |
| Graduation events (on-chain) | Solana RPC / Helius RPC | Free tier available | More reliable than API; parse program logs |
| Token OHLCV (historical) | Birdeye API | Free tier | 1-min candles available for most tokens |
| Token OHLCV (real-time) | Dexscreener WebSocket | Free | Good for live monitoring |
| Pool liquidity | Raydium pool state (Solana RPC) | Free | Parse AMM account data |
| Wallet concentration | Solana RPC token accounts | Free | Compute top-10 holder % |
| Honeypot check | RugCheck.xyz API | Free | Pre-trade safety check |
| SOL price | CoinGecko API | Free | For USD normalization |
| Short availability | Drift Protocol API | Free | Check if perp market exists for token |

---

## Related Strategies & Variants

- **Variant A (Preferred):** Long exit signal for bonding curve participants — exit long positions at graduation spike peak rather than holding through reversion. Simpler execution, no short selling required.
- **Variant B:** Long the graduation itself (buy at $69K graduation price, sell into the spike) — this is the other side of the same trade, with better execution feasibility but requires being in the bonding curve phase.
- **Related:** Token unlock shorts (Zunid's existing strategy) — same structural logic (contractually scheduled supply event creates predictable selling pressure), different protocol layer.

---

*Next step: Build graduation event data pipeline and run Step 1–3 of backtest methodology on 90 days of Pump.fun data. Prioritize confirming (a) frequency of >3x spike events and (b) whether short execution is feasible before investing further in full backtest.*
