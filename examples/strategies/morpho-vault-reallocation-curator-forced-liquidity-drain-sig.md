---
title: "Morpho Vault Reallocation — Curator-Forced Liquidity Drain Signal"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 3
composite: 540
categories:
  - defi-protocol
  - lending
  - liquidation
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a Morpho Blue curator (Gauntlet, B.Protocol, Re7, Steakhouse) executes a large reallocation that drains liquidity from a single lending market, the mechanical consequence is an instantaneous utilization spike in that market. Borrowers in that market now face elevated borrow rates above the kink threshold. A subset of those borrowers — specifically leveraged yield farmers and looping strategies — face negative carry and are economically compelled to unwind within hours. Unwinding requires repaying debt and selling or withdrawing collateral. The collateral asset therefore faces net sell pressure that is predictable, directional, and causally linked to an on-chain event that is publicly visible before the price impact fully materializes.

**The causal chain must hold at every link for the trade to work:**
1. Curator tx drains >$30M from Market A → verifiable in same block
2. Post-reallocation utilization in Market A crosses above kink → verifiable within same block via Morpho subgraph
3. Borrow rate in Market A jumps to the steep post-kink slope → deterministic from hard-coded rate curve
4. Leveraged borrowers face negative carry → probabilistic but strongly incentivized
5. Borrowers unwind within 0–24 hours → probabilistic, speed depends on borrower sophistication
6. Collateral asset faces net selling → probabilistic, magnitude depends on position size and market depth

Links 1–3 are **contractually guaranteed** by Morpho's rate model. Links 4–6 are **strongly incentivized but not guaranteed**. The 6/10 score reflects this split.

---

## Structural Mechanism

### 2.1 Morpho Blue Architecture

Morpho Blue is a permissionless lending primitive. Vaults (MetaMorpho) sit on top and aggregate depositor capital across multiple isolated lending markets. Curators are appointed entities with the authority — and in many cases the contractual obligation under their vault mandates — to reallocate capital between markets in response to risk parameter breaches.

**Key protocol rules that create the edge:**

- **Isolated markets:** Each Morpho market is a separate (collateral asset, loan asset, oracle, LLTV) tuple. Liquidity cannot flow between markets automatically — only curators can move it.
- **Hard-coded interest rate curves:** Each market has a fixed rate model with a kink utilization (typically 80–92%). Below kink, rates are flat and low. Above kink, rates escalate steeply (often 10x slope increase). The exact rate at any utilization is deterministic and publicly readable from the contract.
- **No automatic rebalancing:** Unlike Aave, Morpho has no protocol-level mechanism to auto-rebalance utilization. Only curator action or organic new deposits can reduce utilization. This means a curator drain creates a utilization spike that persists until external capital enters.
- **Curator transaction visibility:** All curator reallocation calls (`reallocate()` on MetaMorpho) are on-chain, emit events, and are indexed by the Morpho subgraph in real time.

### 2.2 Rate Curve Math

For a market with kink at utilization `U_kink`, the borrow rate function is:

```
If U ≤ U_kink:
    rate = base_rate + (U / U_kink) * slope1

If U > U_kink:
    rate = base_rate + slope1 + ((U - U_kink) / (1 - U_kink)) * slope2
```

Where `slope2 >> slope1` (typically 4–10x larger). A reallocation that pushes utilization from 75% to 95% can move the annualized borrow rate from ~4% to ~40%+. At 40% annualized, a leveraged borrower paying 3x leverage faces ~120% annualized cost on their equity — negative carry that demands immediate action.

### 2.3 Borrower Composition

The borrowers most likely to unwind rapidly are:

- **Looping strategies** (e.g., deposit wstETH, borrow USDC, buy more wstETH, repeat): These are rate-sensitive because their yield is the staking rate (~4% APY). Any borrow rate above ~4% destroys the trade.
- **Yield aggregator vaults** (Yearn, Beefy, Contango): These have automated position managers that monitor borrow rates and deleverage when carry turns negative.
- **Retail leveraged longs**: Less sophisticated, slower to react, but still respond within 24 hours when rates spike visibly on dashboards.

The fastest unwinds come from automated vaults — these can respond within minutes. Retail unwinds extend the tail to 24–48 hours.

---

## Signal Definition

### 3.1 Primary Signal Conditions (ALL must be true)

| Condition | Threshold | Source |
|-----------|-----------|--------|
| Curator reallocation tx detected | Any `reallocate()` call from known curator addresses | Morpho subgraph / on-chain event |
| Net liquidity drained from single market | > $30M USD equivalent | Morpho subgraph `marketState` |
| Post-reallocation utilization | > kink threshold for that market | Morpho contract `market()` call |
| Collateral asset has liquid perp on Hyperliquid | OI > $10M, 24h volume > $5M | Hyperliquid API |
| Time since reallocation tx | < 30 minutes | Block timestamp |

### 3.2 Signal Strength Tiers

**Tier 1 (highest conviction):** Drain > $50M AND post-reallocation utilization > 95% AND collateral is a mid-cap asset (not ETH/BTC — see Section 8.1)

**Tier 2 (standard):** Drain $30–50M AND utilization > kink AND collateral has meaningful open interest on Hyperliquid

**Tier 3 (monitor only, no trade):** Drain < $30M OR utilization < kink OR collateral is ETH/BTC

### 3.3 Known Curator Addresses to Monitor

Maintain a live registry. Starting list (verify on-chain before use):
- Gauntlet: published on Morpho governance forum and their website
- B.Protocol: published on their documentation
- Re7 Capital: published on Morpho app vault pages
- Steakhouse Financial: published on Morpho app vault pages
- Block Analitica: published on Morpho governance

**Action:** Pull current curator addresses from `MetaMorpho` contract `curator()` getter for each active vault. Re-verify monthly as new vaults launch.

---

## Entry Rules

### 4.1 Entry Procedure

1. **Detect signal:** Curator `reallocate()` tx confirmed on-chain. Parse event logs to identify source market, amount moved, and destination market.
2. **Verify utilization:** Query `market(marketId)` on Morpho Blue contract to confirm post-reallocation utilization > kink. Do not rely on subgraph alone — subgraph indexing can lag 1–3 minutes.
3. **Identify collateral asset:** Read `marketParams(marketId)` to get collateral token address. Map to Hyperliquid perp ticker.
4. **Check perp liquidity:** Confirm Hyperliquid OI > $10M and 24h volume > $5M for the collateral perp. If not, abort — price impact of our own trade will be too large.
5. **Enter short:** Market order on Hyperliquid perp for the collateral asset. Execute within 30 minutes of reallocation tx confirmation.
6. **Log entry:** Record block number of reallocation tx, entry price, utilization at entry, borrow rate at entry, market ID.

### 4.2 Entry Timing Window

- **Hard deadline:** 30 minutes post-reallocation tx. After 30 minutes, the signal is stale — either borrowers have already unwound (price impact realized) or new supply has entered (utilization normalized).
- **Preferred entry:** Within 10 minutes. The fastest automated vaults respond in minutes; being early captures the most price impact.
- **Do not enter** if the collateral asset has moved > 1% in the direction of the trade since the reallocation tx — the move may already be priced in.

---

## Exit Rules

### 5.1 Primary Exit Triggers (first to fire wins)

| Trigger | Action |
|---------|--------|
| 24-hour timeout from entry | Close entire position at market |
| Market utilization drops below kink | Close entire position — the rate pressure has been relieved |
| Collateral perp moves > 1.5% against position (stop-loss) | Close entire position |
| Collateral perp moves > 3% in favor (take-profit) | Close 75% of position, trail stop on remainder at 1% from peak |

### 5.2 Utilization Monitoring for Exit

Poll `market(marketId)` every 15 minutes post-entry. When utilization drops below kink, the borrow rate incentive for unwinding has been removed — the trade thesis has expired regardless of P&L.

### 5.3 Partial Exit Logic

At +1.5% profit: close 50% of position to lock gains and reduce risk.
At +3% profit: close additional 25%, trail remaining 25% with 1% trailing stop.
This structure captures the fast initial move while allowing the tail of slower retail unwinds to play out.

---

## Position Sizing

### 6.1 Base Sizing Rule

Risk **0.5% of portfolio** per trade (not 1% — this is a hypothesis-stage strategy with unproven price impact).

**Formula:**
```
Position size (USD) = (Portfolio value × 0.005) / Stop distance
Stop distance = 0.015 (1.5% stop)

Example: $100,000 portfolio
Risk per trade = $500
Position size = $500 / 0.015 = $33,333 notional
```

### 6.2 Tier Adjustment

- Tier 1 signal: 1.0× base size
- Tier 2 signal: 0.5× base size
- Never exceed 2% of portfolio notional on a single trade regardless of signal tier

### 6.3 Leverage

Use 3–5× leverage on Hyperliquid perp. Do not exceed 5× — the 24-hour holding period and 1.5% stop are incompatible with high leverage if funding rates spike adversely.

### 6.4 Funding Rate Check

Before entry, check Hyperliquid funding rate for the collateral perp. If funding rate is already negative (shorts are paying longs) at an annualized rate > 50%, reduce position size by 50% — the cost of carry on the short may exceed the expected price move.

---

## Backtest Methodology

### 7.1 Data Collection

**Step 1: Build curator reallocation event database**
- Source: Morpho subgraph (`https://api.thegraph.com/subgraphs/name/morpho-labs/morpho-blue`)
- Query: All `reallocate()` events from MetaMorpho contracts, filtered by known curator addresses
- Fields needed: `blockNumber`, `timestamp`, `marketId`, `supplyAssets` (before/after), `vault`
- Time range: Pull all available history (Morpho Blue launched November 2023)
- Expected event count: Estimate 200–500 qualifying events (>$30M drain) over available history

**Step 2: Reconstruct market utilization at each event**
- For each reallocation event, query `market(marketId)` at the block immediately before and after the tx
- Calculate: `utilization = totalBorrowAssets / totalSupplyAssets`
- Flag events where post-reallocation utilization > kink for that market
- Kink values: Read from each market's `IRM` (Interest Rate Model) contract

**Step 3: Map collateral assets to Hyperliquid perps**
- For each qualifying event, identify collateral token from `marketParams(marketId)`
- Map to Hyperliquid perp ticker (manual mapping table required — maintain as static file)
- Filter out events where no liquid perp exists

**Step 4: Pull perp price data**
- Source: Hyperliquid historical data API or Coingecko/CoinMarketCap for spot proxy
- Pull OHLCV at 5-minute resolution for ±48 hours around each event
- Align timestamps to block timestamps (use Ethereum block time, not wall clock)

### 7.2 Backtest Logic

For each qualifying event:
1. Record entry price = close of 5-minute candle containing the reallocation tx + 1 candle (simulate 5–10 minute execution delay)
2. Simulate short entry at that price
3. Apply exit rules in order: 1.5% stop, 3% take-profit, 24h timeout, utilization-below-kink exit
4. Record: P&L, holding period, exit reason, post-reallocation utilization at entry, drain size

### 7.3 Key Metrics to Compute

| Metric | Target for Go-Live |
|--------|-------------------|
| Win rate | > 55% |
| Average win / average loss | > 1.5 |
| Expected value per trade | > 0.3% of notional |
| Max consecutive losses | < 5 |
| Sharpe (annualized, trade-level) | > 1.0 |
| Sample size | > 30 qualifying events |

### 7.4 Segmentation Analysis

Run backtest separately for:
- Tier 1 vs Tier 2 signals
- ETH/BTC collateral vs altcoin collateral
- Events during high market volatility (BTC 30-day realized vol > 60%) vs low volatility
- Events where drain > 50% of market's total supply vs < 50%
- Weekday vs weekend (curator activity patterns may differ)

### 7.5 Critical Backtest Caveat

**The backtest cannot fully simulate execution timing.** The 30-minute entry window is critical — if the price impact occurs within 5 minutes (automated vault unwinds), a backtest using 5-minute candles will understate slippage and overstate achievable entry prices. Flag any event where the collateral asset moved > 0.5% within the first 5-minute candle post-reallocation — these events may not be tradeable in practice.

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Backtest sample size:** ≥ 30 qualifying events in historical data
2. **Backtest EV:** Expected value per trade > 0.3% of notional after 0.1% round-trip transaction cost assumption
3. **Win rate:** > 55% on backtest
4. **Monitoring infrastructure live:** Automated alert system watching curator addresses fires within 2 minutes of on-chain tx confirmation (test with 10 historical events replayed)
5. **Paper trade validation:** Run 5 live paper trades with full signal logging before first real trade
6. **Collateral-to-perp mapping table:** Complete and verified for all active Morpho markets with >$10M TVL
7. **Funding rate baseline:** Confirm that average funding rate cost over backtest period does not consume > 30% of gross P&L

---

## Kill Criteria

Suspend the strategy immediately if any of the following occur:

| Condition | Action |
|-----------|--------|
| 5 consecutive losing trades | Pause, review signal quality, do not resume without researcher sign-off |
| Realized EV per trade drops below 0% over trailing 20 trades | Kill strategy, return to backtest |
| Monitoring system misses a qualifying event by > 30 minutes | Pause trading until system reliability is confirmed |
| Morpho protocol upgrade changes rate model or curator mechanics | Pause immediately, re-verify mechanism, re-backtest |
| Curator addresses change without detection | Pause, rebuild address registry |
| Any single trade loss > 3% of portfolio (system failure scenario) | Kill strategy, investigate execution |

---

## Risks

### 10.1 Price Impact Too Small on Large Collateral Assets

**Risk:** For ETH or BTC collateral markets, even a $50M liquidity drain is negligible relative to the total market cap and perp OI. The borrow rate spike may not produce measurable price impact.

**Mitigation:** Tier 3 filter excludes ETH/BTC collateral. Focus on mid-cap collateral assets (wstETH, cbBTC, USDC-denominated markets with altcoin collateral) where the borrower base is smaller and more concentrated.

**Backtest check:** Segment results by collateral market cap. If EV is zero for assets with market cap > $5B, add market cap filter to signal definition.

### 10.2 New Supply Enters Before Borrowers Unwind

**Risk:** The utilization spike attracts new depositors (high rates are attractive to lenders) who enter within minutes, normalizing utilization before borrowers unwind. The rate spike is too brief to force unwinds.

**Mitigation:** Monitor utilization in real time post-entry. If utilization drops below kink within 30 minutes of entry (before borrowers have had time to unwind), exit immediately — the thesis has been invalidated by supply-side response.

**Structural note:** This risk is higher in bull markets when yield-seeking capital is abundant. Track whether supply normalization speed has increased over time.

### 10.3 Curator Reallocation Is Anticipated

**Risk:** Sophisticated market participants monitor the same curator addresses and front-run the reallocation itself, pricing in the impact before the tx is confirmed.

**Mitigation:** Check whether collateral asset price moves > 0.3% in the 30 minutes *before* the reallocation tx. If yes, the signal may be front-run and entry should be skipped. Log this as a separate backtest filter.

### 10.4 Reallocation Is Destination-Driven, Not Risk-Driven

**Risk:** Some curator reallocations move capital *toward* a high-yield market (chasing rates) rather than *away* from a risky market. In this case, the drained market may have had low utilization to begin with, and the drain does not push it above kink.

**Mitigation:** The kink-crossing filter (Condition 3 in Section 3.1) eliminates these events. Only act when post-reallocation utilization is confirmed above kink.

### 10.5 Hyperliquid Perp Funding Rate Adversity

**Risk:** If the collateral asset is already heavily shorted on Hyperliquid (negative funding), entering a short means paying funding to longs. Over a 24-hour hold, this cost can be significant.

**Mitigation:** Pre-entry funding rate check (Section 6.4). If annualized funding cost > 50%, reduce size. If > 100%, skip trade entirely.

### 10.6 Morpho Protocol Risk

**Risk:** A Morpho smart contract bug, oracle manipulation, or governance attack could cause abnormal market behavior that invalidates the rate model assumptions.

**Mitigation:** This strategy does not hold positions in Morpho itself. The risk is that a protocol incident causes correlated selling of the collateral asset for reasons unrelated to the signal — this would appear as a win but is not repeatable. Flag any trade that coincides with a Morpho incident in the trade log.

### 10.7 Curator Behavior Changes

**Risk:** Curators change their reallocation policies (e.g., Gauntlet adopts gradual reallocation over 6 hours instead of single large txs) reducing the magnitude of individual utilization spikes.

**Mitigation:** Monitor curator reallocation size distribution monthly. If median qualifying event size drops below $20M, re-evaluate the $30M threshold and re-backtest.

---

## Data Sources

| Data | Source | Access | Cost |
|------|--------|--------|------|
| Curator reallocation events | Morpho subgraph (The Graph) | Free API | $0 |
| Market utilization (real-time) | Morpho Blue contract `market()` getter | Free RPC call | $0 (use public RPC or Alchemy free tier) |
| Market parameters (collateral, kink) | Morpho Blue contract `marketParams()` and IRM contract | Free RPC call | $0 |
| Curator wallet addresses | Morpho app vault pages, Morpho governance forum | Public | $0 |
| Historical perp OHLCV | Hyperliquid historical data API | Free | $0 |
| Real-time perp funding rates | Hyperliquid API | Free | $0 |
| Collateral token metadata | Etherscan token API or CoinGecko | Free tier | $0 |
| Block timestamps | Ethereum RPC or Etherscan API | Free tier | $0 |

**Total data cost: $0.** All required data is publicly available and free.

---

## Implementation Checklist

### Pre-Backtest (Current Stage)
- [ ] Pull all MetaMorpho vault addresses from Morpho factory contract
- [ ] Extract curator address for each vault using `curator()` getter
- [ ] Build static mapping: curator address → vault → markets managed
- [ ] Query Morpho subgraph for all `reallocate()` events since November 2023
- [ ] For each event, reconstruct pre/post utilization from contract state at that block
- [ ] Apply signal filters, count qualifying events
- [ ] If < 30 qualifying events: lower drain threshold to $20M and re-count; document threshold sensitivity

### Backtest Stage
- [ ] Pull Hyperliquid perp OHLCV for all collateral assets at 5-minute resolution
- [ ] Run backtest simulation with full exit rule logic
- [ ] Compute all metrics in Section 7.3
- [ ] Run segmentation analysis in Section 7.4
- [ ] Document results in standard Zunid backtest report format

### Infrastructure Stage (if backtest passes)
- [ ] Build real-time curator tx monitor (webhook or polling every 30 seconds)
- [ ] Build utilization checker (RPC call triggered by curator tx detection)
- [ ] Build Hyperliquid perp liquidity checker
- [ ] Build alert system (Telegram/Slack notification within 2 minutes of signal)
- [ ] Test alert system against 10 historical events replayed in simulation

### Paper Trade Stage
- [ ] Execute 5 paper trades with full signal logging
- [ ] Compare paper trade outcomes to backtest predictions
- [ ] If paper trade EV > 0 and execution timing is achievable: proceed to live

---

## Open Questions for Researcher Review

1. **Borrower response speed:** Is there on-chain data showing how quickly leveraged borrowers in Morpho markets unwind after rate spikes? Query historical borrow repayment events in the 24 hours following past utilization spikes to estimate the response distribution.

2. **Supply response speed:** How quickly do new depositors enter high-rate markets? If supply normalizes in < 30 minutes consistently, the trade window may be too short for non-automated execution.

3. **Collateral concentration:** For each qualifying market, what fraction of total borrows are from automated strategies (identifiable by contract addresses) vs EOAs? Higher automated fraction = faster unwind = tighter execution window required.

4. **Curator motivation:** Are curators reallocating in response to *current* risk (reactive) or *anticipated* risk (proactive)? Proactive reallocations may not produce utilization spikes if the market was already low-utilization. Verify by checking pre-reallocation utilization distribution across historical events.

5. **Cross-market correlation:** When a curator drains Market A to fund Market B, does Market B's collateral asset face buying pressure (leveraged longs being enabled)? This could create a simultaneous long/short pair trade opportunity.

---

*This document is a hypothesis specification. No backtest has been run. No live trading should occur until Section 8 go-live criteria are fully satisfied. All thresholds ($30M drain, 30-minute window, 1.5% stop) are initial estimates requiring validation against historical data.*
