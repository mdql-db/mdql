---
title: "Protocol Migration Selling Pressure — Token Swap Deadline Short"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - token-supply
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a protocol runs a time-limited token swap (V1→V2, merger, rebranding), the conversion deadline forces a cohort of previously inactive holders — grant recipients, farming-origin wallets, early airdrop recipients — to convert old tokens to new ones or lose everything. These holders had effectively written off the position; the new tokens they receive are treated as found money. The result is a concentrated, deadline-driven sell wave in the new token that temporarily depresses its price below fair value relative to the old chain.

**Specific causal chain:**

1. Protocol announces migration with fixed swap ratio and hard deadline
2. Active traders convert early; passive/forgotten wallets convert in the final 3–5 days as deadline pressure mounts
3. Last-minute converters receive new tokens with zero cost basis (old tokens were written off) → immediate sell pressure
4. New token price dips below implied fair value (old token price × swap ratio)
5. Post-deadline, forced selling exhausts; price mean-reverts upward
6. Old token simultaneously trades at a discount to swap ratio NAV in the final days as risk-averse holders sell rather than navigate the conversion process

The edge is **not** that price always drops — it is that a known, contractually bounded cohort of sellers is forced to act within a specific window, creating a predictable supply shock with a known end date.

---

## Structural Mechanism — WHY This Must Happen

The mechanism is contractual and game-theoretic, not statistical:

**Contractual guarantee:** The swap contract has a hard deadline encoded on-chain. After block N, `convertOldToNew()` reverts. Old tokens become permanently non-redeemable. This is not a soft deadline — it is enforced by immutable code.

**Forced action on passive holders:** Unlike a voluntary sale, conversion is use-it-or-lose-it. Holders who would otherwise never touch their position are compelled to act. This surfaces supply that is normally locked away in dormant wallets.

**Zero cost basis psychology:** Grant/farming/airdrop recipients received old tokens at zero cash cost. After a migration announcement, many mentally write off the position if the old token is illiquid or the protocol is obscure. When the deadline forces action, the new tokens received feel like a windfall — there is no anchoring to a purchase price, so the sell threshold is effectively $0. This is structurally different from a normal token unlock where holders have a cost basis they may defend.

**Concentration in final window:** Blockchain data consistently shows conversion volume spikes in the last 48–72 hours before any deadline (tax deadlines, governance votes, claim windows all exhibit this). This is a well-documented human behavioral pattern (procrastination) that is predictable and exploitable.

**Pair trade arithmetic:** If old token trades at $0.90 and swap ratio is 1:1 with new token at $1.00, there is a 10% discount. This discount exists because: (a) conversion friction/gas costs, (b) counterparty risk on the swap contract, (c) holders selling old rather than converting. The discount must converge to zero by deadline — either old rises, new falls, or both. This convergence is mechanically guaranteed if the swap contract is solvent.

---

## Entry / Exit Rules

### Leg 1 — Directional Short (New Token)

| Parameter | Rule |
|-----------|------|
| **Entry trigger** | T-5 calendar days before swap deadline, at market open (00:00 UTC) |
| **Instrument** | New token perpetual future on Hyperliquid, or spot short via borrow if perp unavailable |
| **Entry price** | VWAP of first 1-hour candle after T-5 open |
| **Exit — base case** | T+2 after deadline, market close (23:59 UTC) |
| **Exit — early** | If new token price drops >15% from entry before deadline, take 50% profit, trail stop on remainder |
| **Stop loss** | Hard stop at +10% above entry price (i.e., if new token rises 10%, exit) |
| **Minimum liquidity** | Only enter if new token has >$500k 24h volume on target exchange |

### Leg 2 — Pair Trade (Old Token Long + New Token Short)

Only execute if: old token discount to swap ratio NAV > 5% at T-5.

| Parameter | Rule |
|-----------|------|
| **Long leg** | Buy old token on spot (no leverage); size = short leg notional × swap ratio |
| **Short leg** | Short new token perp; size = 1x notional |
| **Entry trigger** | Same as Leg 1 (T-5) |
| **Exit — base case** | Convert old tokens via swap contract at T-1 (one day before deadline to avoid gas wars); close short at same time |
| **Exit — if conversion fails** | Close both legs at market if swap contract is congested or gas cost exceeds 2% of position |
| **Stop loss** | If spread widens beyond 20% (old token discount grows to >20%), exit both legs — something is wrong with the swap contract |
| **Minimum old token liquidity** | >$100k 24h volume; if lower, skip Leg 2 entirely |

### Execution Notes

- Check swap contract on-chain before entry: verify `deadline` variable, `conversionRatio`, and contract balance (ensure new tokens are available for redemption)
- Confirm swap contract is not paused or upgradeable by a multisig that could change terms
- Gas cost for conversion must be <1% of position size to make pair trade viable

---

## Position Sizing

**Base position size:** 1% of portfolio per event (directional short only)

**Pair trade:** 0.75% per leg (1.5% total gross exposure) — lower because old token liquidity is often thin

**Scaling rules:**
- If old token discount > 10%: scale pair trade to 1.5% per leg
- If new token has >$5M 24h volume: scale directional short to 1.5%
- Never exceed 3% gross exposure on a single migration event
- If running multiple events simultaneously (rare): cap total migration exposure at 5% of portfolio

**Leverage:** Maximum 2x on short leg. Old token long is always unlevered spot (liquidity too thin for margin).

**Kelly note:** With a hypothesized win rate of ~55% and average win/loss ratio of ~1.5x (to be validated in backtest), full Kelly is ~8% — use 1/8 Kelly = ~1% as above until edge is confirmed.

---

## Backtest Methodology

### Universe Construction

Identify all protocol token migrations from 2019–present with the following criteria:
- Hard on-chain deadline (not soft/extendable)
- Old token had >$1M market cap at announcement
- New token was tradeable on a CEX or DEX with price history
- Swap ratio was fixed (not variable/auction-based)

**Target sample size:** Minimum 15 events. Expect to find 20–40 qualifying events.

**Known candidates to seed the universe:**
- Synthetix SNX (various reward migrations)
- Uniswap UNI claim deadline (September 2021)
- Compound COMP → various
- Sushiswap migrations
- Bancor BNT v3 migration
- Olympus OHM → gOHM conversion
- Tribe/Fei merger (TRIBE → FEI redemption)
- Tornado Cash TORN (post-sanction claim windows)
- Any Curve gauge migration with token conversion

### Data Sources

| Data type | Source | URL/Endpoint |
|-----------|--------|--------------|
| Migration announcements | Protocol governance forums | Snapshot.org, Commonwealth.im, individual Discourse forums |
| On-chain conversion events | Etherscan, Dune Analytics | `https://dune.com` — query `Transfer` events to swap contract address |
| Old/new token price history | CoinGecko historical API | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| DEX price history | The Graph | Uniswap v2/v3 subgraph, `https://thegraph.com/hosted-service/subgraph/uniswap/uniswap-v3` |
| Perp funding/price | Hyperliquid API | `https://api.hyperliquid.xyz/info` — `candleSnapshot` endpoint |
| Wallet conversion timing | Dune Analytics | Custom query: `SELECT block_time, amount FROM erc20_transfers WHERE to = '<swap_contract>'` |

### Dune Query Template

```sql
-- Conversion volume by day relative to deadline
SELECT
  DATE_TRUNC('day', block_time) AS day,
  SUM(value / 1e18) AS old_tokens_converted,
  COUNT(DISTINCT "from") AS unique_converters
FROM erc20_ethereum.evt_Transfer
WHERE contract_address = '{{old_token_address}}'
  AND to = '{{swap_contract_address}}'
  AND block_time BETWEEN '{{deadline}}' - INTERVAL '14 days' AND '{{deadline}}'
GROUP BY 1
ORDER BY 1
```

### Metrics to Measure

For each event, record:

| Metric | Definition |
|--------|------------|
| `new_token_return_T-5_to_T+2` | % price change of new token from entry to exit |
| `new_token_return_vs_BTC` | Same, beta-adjusted against BTC return in same window |
| `old_token_discount_at_T-5` | (swap_ratio × new_token_price - old_token_price) / (swap_ratio × new_token_price) |
| `conversion_volume_spike` | Ratio of T-3 to T-7 daily conversion volume |
| `pair_trade_pnl` | Combined PnL of long old + short new, normalized to 1% position |
| `max_adverse_excursion` | Worst drawdown during holding period |
| `funding_rate_cost` | Cumulative funding paid on short leg during holding period |

### Baseline

Compare new token return in the T-5 to T+2 window against:
1. BTC return in same window (market beta control)
2. New token's own return in the T-30 to T-10 window (pre-event baseline)
3. A matched control group: same token, same calendar window, one year prior (if token existed)

**Minimum viable sample:** 15 events with complete price data. If fewer than 15, report results as "indicative only."

---

## Go-Live Criteria

All three conditions must be met before paper trading:

1. **Win rate ≥ 55%** on directional short leg (new token underperforms BTC in T-5 to T+2 window), across ≥ 15 events
2. **Average beta-adjusted return ≥ +3%** per trade (after estimated 0.5% trading costs and 0.1%/day funding)
3. **Pair trade spread convergence rate ≥ 80%** — in at least 80% of events where old token discount > 5%, the discount narrowed by deadline

If pair trade passes but directional short fails: run pair trade only, no directional short.

If directional short passes but pair trade data is insufficient (<8 events with qualifying discount): run directional short only, flag pair trade for future monitoring.

---

## Kill Criteria

Abandon the strategy (both paper and live) if any of the following occur:

| Trigger | Threshold | Action |
|---------|-----------|--------|
| Backtest win rate | < 50% on beta-adjusted basis | Do not proceed to paper trade |
| Paper trade drawdown | 3 consecutive losses OR -5% cumulative on migration allocation | Stop, re-examine hypothesis |
| Structural change | Protocols begin offering unlimited/rolling swap windows (no hard deadline) | Edge disappears; kill immediately |
| Liquidity deterioration | New token perp unavailable on Hyperliquid and no borrow market exists | Skip event; if this becomes systemic, kill strategy |
| Funding rate cost | Average funding > 0.15%/day on short leg (annualizes to >54%) | Edge is eaten by carry; kill directional short leg |

---

## Risks — Honest Assessment

**High severity:**

- **Protocol team price support:** Teams often buy back new tokens around migration events to signal confidence. This is the primary risk to the directional short. Mitigation: check treasury wallet activity on-chain before entry; if team wallet is accumulating new tokens, skip the trade.

- **Market has already priced it:** If the migration was announced 3 months ago and is widely covered, sophisticated traders may have front-run the selling wave. The discount may already be baked in. Mitigation: measure new token performance from announcement to T-5; if already down >20%, the trade may be exhausted.

- **Old token liquidity:** Many old tokens trade on obscure DEXes with $10k–$50k daily volume. Slippage on the long leg can exceed the theoretical discount. Mitigation: enforce the $100k minimum volume filter strictly; size down to 0.25% if volume is $100k–$200k.

**Medium severity:**

- **Swap contract risk:** If the swap contract has a bug, is paused, or runs out of new tokens, the pair trade collapses. Mitigation: verify contract balance > 110% of remaining old token supply before entry; read the contract for pause/upgrade mechanisms.

- **Gas cost during deadline rush:** On Ethereum mainnet, gas can spike 10–50x in the final hours before a deadline. Conversion cost can exceed the discount. Mitigation: convert at T-1, not T-0; use L2 if swap is deployed there.

- **Low event frequency:** Qualifying events may occur only 3–6 times per year. This is not a systematic strategy — it is an opportunistic overlay. Do not force trades on marginal events to hit a quota.

**Low severity:**

- **Funding rate on short:** If the market is already short the new token, funding will be negative (shorts pay longs). This adds carry cost. Mitigation: check funding rate at entry; if >0.05%/8h, reduce position size by 50%.

- **Correlation to broader market:** A strong bull run during the T-5 to T+2 window can overwhelm the selling pressure. The beta-adjusted return metric in the backtest will reveal how often this occurs.

---

## Data Sources Summary

| Source | Purpose | Access |
|--------|---------|--------|
| Dune Analytics | On-chain conversion volume, wallet timing | Free tier sufficient; `dune.com` |
| CoinGecko API | Historical OHLCV for old/new tokens | Free, no key required for basic; `api.coingecko.com` |
| The Graph | DEX price history for tokens not on CoinGecko | Free; `thegraph.com` |
| Etherscan | Contract verification, swap ratio, deadline variable | Free; `api.etherscan.io` |
| Hyperliquid API | Perp price, funding rate history | Free; `api.hyperliquid.xyz/info` |
| Snapshot / Commonwealth | Migration announcement dates and terms | Manual scrape; no API |
| DefiLlama | Protocol TVL context, token metadata | Free; `api.llama.fi` |

**First backtest task:** Build the event universe. Spend 4–6 hours manually cataloguing migrations from 2019–2024 using CoinGecko's "migrated" token tag (`https://www.coingecko.com/en/categories/migrated-tokens`) and cross-referencing with governance forum announcements to extract exact deadline dates and swap ratios. This is the hardest part — the data does not exist in a clean feed.
