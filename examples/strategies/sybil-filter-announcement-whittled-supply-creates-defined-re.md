---
title: "Sybil Appeal Re-Inclusion Short"
status: HYPOTHESIS
mechanism: 4
implementation: 5
safety: 5
frequency: 1
composite: 100
categories:
  - token-supply
  - airdrop
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a protocol announces resolution of Sybil appeal batches — granting tokens to wallets previously disqualified — the newly included recipients constitute a structurally distinct seller cohort with near-100% sell probability. These wallets have zero cost basis, zero protocol loyalty (they were initially rejected), and received an unexpected windfall. The announcement is a datestamped, verifiable forced supply event. Price should decline within 24–72 hours as this cohort claims and dumps. Shorting the token within 1 hour of the announcement captures the front-end of this selling wave before it fully clears.

**Causal chain:**

1. Protocol runs Sybil filter post-snapshot → disqualifies wallets
2. Disqualified wallets appeal → protocol reviews in batches
3. Protocol announces re-inclusion batch (governance post, blog, or on-chain merkle root update)
4. Re-included wallets claim tokens → immediate sell pressure (zero cost basis, surprise windfall)
5. Price declines in compressed 24–72 hour window
6. Selling exhausts → price stabilises or recovers → short closed

---

## Structural Mechanism (WHY This MUST Happen)

This is not a "tends to happen" pattern. The mechanism is grounded in three structural facts:

**1. Zero cost basis = no hold incentive.**
Standard airdrop recipients may hold because they have community affiliation or believe in the project. Re-included Sybil appellants were *told they were excluded* — they had already mentally written off the tokens. When re-included, the tokens are pure found money. Behavioural finance and rational actor models both predict near-100% sell rate for zero-cost windfalls with no sentimental attachment.

**2. Compressed claim window.**
Protocols typically set claim deadlines (30–180 days), but the bulk of claiming happens in the first 48–72 hours post-announcement (observable in standard airdrop on-chain data — claim curves are front-loaded). Re-inclusion batches are smaller and less publicised, so the claiming cohort is more concentrated and acts faster.

**3. Supply event is contractually scheduled.**
The merkle root update or contract parameter change is an on-chain, immutable event. Once the transaction is confirmed, the supply is unlockable. This is not a rumour — it is a verifiable state change. The sell pressure is not probabilistic in direction, only in magnitude.

**What is NOT guaranteed:** The magnitude of price impact. If the re-inclusion batch is <0.1% of circulating supply, the effect may be noise. This is why batch size screening is a required pre-trade filter (see Entry Rules).

---

## Entry / Exit Rules

### Pre-Trade Filters (must ALL pass before entry)

| Filter | Threshold | Rationale |
|---|---|---|
| Batch size | ≥ 0.5% of circulating supply | Below this, price impact likely noise |
| Token liquidity | ≥ $500k average daily volume (7-day) | Minimum for a short position to be executable and closeable |
| Perp availability | Token must have a perpetual on Hyperliquid or Binance | Required for shorting |
| Funding rate | Not already deeply negative (< −0.10% per 8h) | Avoid paying excessive funding into a crowded short |
| Time since snapshot | Announcement must be ≥ 14 days post-snapshot | Earlier = tokens not yet claimable, timing uncertain |

### Entry

- **Trigger A (preferred):** On-chain detection of merkle root update to the airdrop distributor contract adding new leaf hashes. Monitor via contract event logs (`MerkleRootUpdated` or equivalent).
- **Trigger B (fallback):** Official governance forum post or blog post announcing Sybil appeal resolution with explicit wallet count and token amount.
- **Entry timing:** Market short within 60 minutes of trigger confirmation.
- **Entry instrument:** Perpetual futures short on Hyperliquid (preferred) or Binance. If perp unavailable, skip — do not use spot borrow (too slow, too expensive).
- **Entry price:** Market order. Do not use limit orders — the edge is time-sensitive and slippage is acceptable given the expected move.

### Exit

- **Primary exit:** Close short 48–72 hours after entry (time-based, not price-based). The selling wave is front-loaded; holding longer introduces unrelated market risk.
- **Profit target (optional overlay):** If position is +15% or more in profit before 48h, take 50% off and trail the remainder with a 5% stop from high-water mark.
- **Stop loss:** Close entire position if price moves +8% adverse from entry price (not from any subsequent high-water mark).
- **Funding stop:** If cumulative funding paid exceeds 0.5% of position notional, close regardless of time or P&L.

### Position Sizing

- **Base size:** 1% of total portfolio per trade.
- **Scale up to 2%** if batch size ≥ 1.5% of circulating supply AND funding rate is neutral (−0.01% to +0.01% per 8h).
- **Never exceed 2%** — event frequency is low and each event is idiosyncratic. Concentration risk is not justified.
- **Leverage:** 3–5x maximum. This is not a high-conviction size-up trade; it is a moderate-conviction, low-frequency event trade.
- **Portfolio-level cap:** Maximum 2 simultaneous positions from this strategy (unlikely given event frequency, but stated for completeness).

---

## Backtest Methodology

### Data Required

| Data type | Source | Notes |
|---|---|---|
| Sybil re-inclusion announcements | Manual: governance forums (Snapshot.org, Commonwealth, protocol Discord announcements) | Must be manually catalogued — no automated feed exists |
| On-chain claim events | Etherscan / Arbiscan / chain-specific explorer APIs; The Graph subgraphs for airdrop contracts | Pull `Claimed` events from distributor contracts |
| Token price (tick/OHLCV) | CoinGecko API (free), Kaiko (paid), or exchange APIs (Binance, Hyperliquid) | Need 1-minute OHLCV for entry/exit simulation |
| Circulating supply at event date | CoinGecko `/coins/{id}/history` endpoint | For batch-size-as-%-of-supply filter |
| Funding rates | Hyperliquid API, Coinglass API | For funding stop filter |

### Event Universe Construction

1. Search governance forums for keywords: "Sybil appeal", "re-inclusion", "appeal results", "disqualified wallets", "second round eligibility" — from 2021 to present.
2. Cross-reference with on-chain distributor contract updates (merkle root changes post-initial-deployment).
3. Target protocols: Arbitrum (ARB), Optimism (OP), Blur, dYdX, Uniswap, Starknet (STRK), ZKsync (ZK), LayerZero (ZRO), Wormhole (W), Eigenlayer (EIGEN). These are the largest airdrops with documented Sybil filtering and appeal processes.
4. Record for each event: announcement date/time, batch token count, circulating supply at time, token price at announcement, token price at T+24h, T+48h, T+72h.

### Metrics to Compute

- **Primary:** Median and mean return of short position at T+48h (entry = announcement price, exit = T+48h close price)
- **Win rate:** % of events where price declined from entry to T+48h exit
- **Max adverse excursion (MAE):** Worst intraday move against the short before T+48h
- **Batch size correlation:** Pearson correlation between batch-size-as-%-of-supply and T+48h return
- **Baseline comparison:** Compare against a randomly sampled 48h short on the same token (same day of week, same market regime) to isolate the event effect from general market drift

### Minimum Sample Size

- Target: ≥ 15 qualifying events (passing all pre-trade filters)
- If fewer than 15 events found, the strategy cannot be validated statistically — flag as "insufficient history" and monitor prospectively

### Backtest Assumptions

- Entry at the 1-hour-post-announcement open price (conservative — assumes some delay)
- Exit at the T+48h close price
- Slippage: 0.15% per side (conservative for mid-cap tokens)
- Funding cost: use actual historical funding rates from Coinglass for each event
- No look-ahead bias: filters (circulating supply, volume) must use data available at announcement time, not post-event data

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Sample size:** ≥ 15 qualifying events in backtest universe
2. **Win rate:** ≥ 60% of events show negative price return at T+48h
3. **Median return:** Median short P&L ≥ +3% at T+48h (net of 0.15% slippage each side and estimated funding)
4. **Batch size filter validation:** Events with batch size ≥ 0.5% of supply must outperform events below this threshold — confirms the filter is doing real work
5. **No single event dominates:** No single event should account for >40% of total backtest P&L (concentration check)
6. **MAE check:** Median MAE must be < 6% (confirms the 8% stop is not being triggered on most trades)

---

## Kill Criteria

Abandon the strategy (stop paper trading or live trading) if any of the following occur:

| Criterion | Threshold |
|---|---|
| Live paper trade win rate | < 50% after 10 events |
| Consecutive losses | 4 or more in a row |
| Average P&L per trade | Negative after 10 events (net of costs) |
| Event frequency drops | Fewer than 3 qualifying events in a 12-month period (strategy becomes too infrequent to maintain operational readiness) |
| Structural change | Protocols adopt instant on-chain Sybil resolution with no announcement lag — removes the entry window |
| Market structure change | Perp markets for small/mid-cap tokens become unavailable on Hyperliquid/Binance, making shorting impractical |

---

## Risks

### Honest Assessment

**1. Batch size is often small.**
Most Sybil re-inclusion batches are 0.1–0.5% of circulating supply. At these sizes, the price impact may be entirely absorbed by normal market noise. The 0.5% filter is designed to screen these out, but it will also dramatically reduce event frequency — possibly to fewer than 5 qualifying events per year across all protocols.

**2. Governance forum leakage.**
Re-inclusion announcements are often discussed in public forums for days before the official announcement. By the time the merkle root is updated on-chain, sophisticated participants may have already shorted. The on-chain trigger (Trigger A) may arrive after the price has already moved. Mitigation: monitor governance forums continuously and use Trigger B (forum post) as the entry signal, not the on-chain confirmation.

**3. Positive sentiment offset.**
Some market participants interpret Sybil re-inclusion as a positive signal ("the protocol is being fair and inclusive"). This can create a short-term price pump immediately post-announcement before the sell pressure hits. This is an adverse entry scenario. Mitigation: wait 30–60 minutes post-announcement before entering, allowing the initial sentiment pop to fade.

**4. Low event frequency.**
5–10 qualifying events per year is not enough to run this as a standalone strategy. It must be part of a broader airdrop event suite (alongside initial airdrop shorts, unlock shorts, etc.). Standalone Sharpe ratio will be poor due to long idle periods.

**5. Perp availability.**
Many tokens that run Sybil re-inclusion processes are newer, smaller-cap tokens. Hyperliquid and Binance may not list perps for them. If no perp is available, the trade cannot be executed. Spot shorting via borrow is too slow and expensive for a 48–72 hour window.

**6. Regulatory / protocol risk.**
If a protocol pauses its distributor contract or delays claim windows after the announcement (e.g., due to a bug or governance vote), the sell pressure is deferred and the short may be stopped out before the event materialises.

**7. Backtest data quality.**
The event universe must be manually constructed from governance forums. There is no clean database of Sybil re-inclusion events. This introduces selection bias risk — if only the "obvious" events are catalogued, the backtest will overstate performance.

---

## Data Sources

| Source | URL / Endpoint | Use |
|---|---|---|
| Snapshot governance | https://snapshot.org/#/ | Search for Sybil appeal proposals |
| Commonwealth | https://commonwealth.im/ | Protocol governance discussions |
| Etherscan API | https://api.etherscan.io/api?module=logs&action=getLogs | Pull contract event logs for merkle root updates |
| The Graph | https://thegraph.com/explorer | Subgraphs for airdrop distributor contracts |
| CoinGecko API | https://api.coingecko.com/api/v3/coins/{id}/market_chart | OHLCV price history |
| CoinGecko history | https://api.coingecko.com/api/v3/coins/{id}/history?date={dd-mm-yyyy} | Circulating supply at event date |
| Coinglass | https://www.coinglass.com/pro/futures/FundingRate | Historical funding rates |
| Hyperliquid API | https://api.hyperliquid.xyz/info | Live perp data, funding rates |
| Binance API | https://api.binance.com/api/v3/klines | OHLCV for perp entry/exit simulation |
| Dune Analytics | https://dune.com/queries | Custom SQL queries on airdrop claim events (community dashboards exist for ARB, OP, ZRO, etc.) |

**Recommended starting point for event universe:** Search Dune Analytics for existing dashboards on "airdrop claims" for ARB, OP, ZRO, ZK, STRK, W, EIGEN. These often include timestamps of claim spikes that can be reverse-engineered to identify re-inclusion batches.

---

*This specification is sufficient to begin manual event cataloguing and backtest construction. The critical path item is building the event universe — without ≥15 qualifying events, no statistical conclusion is possible and the strategy should not proceed to paper trading.*
