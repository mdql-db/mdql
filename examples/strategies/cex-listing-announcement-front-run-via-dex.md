---
title: "CEX Listing Announcement Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 3
composite: 450
categories:
  - exchange-structure
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a major exchange (Binance, Coinbase, OKX) announces a new spot listing, a predictable 12–72 hour window opens between the announcement and the listing going live. During this window, price appreciation continues on DEXes and smaller CEXes as the narrative spreads to progressively wider audiences. Entering a spot long position within 15 minutes of announcement confirmation and exiting 48 hours after the CEX listing activates captures this structural demand influx.

---

## Why It's an Edge

**Structural, not sentiment:**
The Coinbase/Binance listing effect is one of the most replicated findings in crypto market microstructure research. It is not sentiment-driven noise — it reflects genuinely new incremental demand. A major exchange listing makes a token accessible to its entire user base for the first time. That is a discrete, verifiable supply/demand shock.

**The interstitial window is the specific edge:**
Most participants focus on the listing itself. The pre-listing window is harder to systematise because it requires active announcement monitoring. Zunid's autonomous monitoring capability converts this friction into an advantage — it can watch announcement feeds 24/7 without missing events.

**Detection latency is manageable:**
Unlike order book microstructure strategies, the edge here does not require millisecond execution. The announcement-to-price discovery window is 15 minutes to several hours for mid-cap tokens, because:
- Not all market participants monitor exchange blogs actively
- News aggregators introduce their own delay
- Retail participants learn about listings via Twitter, Reddit, and Telegram — typically 30 minutes to several hours after the official post

This is long enough for Zunid to enter at a non-exhausted price without co-location or HFT infrastructure.

**Differentiation from Catalogue Entry 3.2 (Exchange Listing Momentum):**
Strategy 3.2 in the archetype catalogue is generic. This strategy specifies the interstitial window specifically, uses DEX/smaller-CEX entry rather than the announcing exchange, and operationalises programmatic announcement detection rather than manual observation. The mechanism is the same family but the implementation is distinct and more precise.

**Why the market underprices the window:**
- Retail discovers the listing hours to days after announcement
- Institutional-grade announcement monitoring is not universally deployed
- Market makers on the announcing exchange are not yet active (no market to make)
- DEX liquidity is thin, so price adjusts slowly to the new fundamental reality

---

## Backtest Methodology

### Scope

- **Universe:** All Binance spot listings and all Coinbase spot listings from January 2022 through December 2024
- **Filter:** Token must have had a pre-existing DEX market with ≥$250K 24h volume at announcement time
- **Filter:** Exclude stablecoins, wrapped assets, liquid staking tokens, and any token that was already listed on another major CEX (Bybit, OKX, Kraken) at the time — these have reduced listing premium
- **Estimated sample size:** 60–150 qualifying events across both exchanges

### Data Collection

1. **Announcement timestamps:**
   - Binance: blog.binance.com listing announcement posts — scrape title, URL, and publish timestamp
   - Coinbase: blog.coinbase.com and coinbase.com/en/blog/asset pages — same approach
   - Cross-reference against Twitter/X post timestamps from official accounts for precision
   - Ground truth: the earliest of (blog post timestamp, official Twitter post timestamp)

2. **Entry price:**
   - Simulate entry 15 minutes after announcement timestamp
   - Use DEX price from DexScreener or CoinGecko historical data at that timestamp
   - If 15-minute price is unavailable, use the 1-hour price (conservative)

3. **Listing-live timestamp:**
   - The time the token first appeared tradeable on the announcing exchange
   - Source: exchange trade history APIs (first trade timestamp)

4. **Exit prices:**
   - T+48h after listing-live: primary exit
   - T+0 at listing-live: capture the announcement-to-listing return in isolation
   - T+24h after listing-live: intermediate check

5. **Pre-announcement move:**
   - Token price 7 days before announcement vs. price at announcement — used to assess insider front-running and apply the "already priced in" filter

### Metrics to Compute

| Metric | Definition |
|--------|-----------|
| Announcement-to-listing return | Price at listing-live ÷ price 15m after announcement − 1 |
| Listing-to-exit return | Price at T+48h post-listing ÷ price at listing-live − 1 |
| Full trade return | Price at T+48h post-listing ÷ price at 15m post-announcement − 1 |
| Win rate | % of trades with positive full trade return |
| Median return | Median full trade return across all events |
| Max drawdown within hold period | Worst intraday drawdown from entry to exit |
| Pre-announcement drift | Price change from T−7d to announcement timestamp |
| DEX slippage estimate | Based on pool liquidity at entry; model 0.5% for pools >$1M TVL, 2% for <$500K |

### Baseline Comparison

- **Baseline A:** Buy the same token on announcement day, hold for 10 days — removes the listing-window specificity, tests whether the timing matters
- **Baseline B:** Random 3-day spot holds across the same tokens during the same calendar period — controls for bull/bear market regime
- **Baseline C:** Buy the token 7 days *before* announcement (if pre-announcement drift is large, this implies the edge has already been extracted by insiders)

### Key Question the Backtest Must Answer

1. Is the 15-minute-post-announcement to T+48h-post-listing return significantly positive on average?
2. Is that return concentrated in the announcement-to-listing window or the post-listing window?
3. Does the edge survive after a 2% simulated slippage on entry?
4. Does the "already up >30% before detection" filter meaningfully improve results?
5. Is there a Binance vs. Coinbase difference in edge magnitude?
6. Does edge decay visibly from 2022 to 2024 (crowding over time)?

---

## Entry Rules

**Trigger:**
Binance, Coinbase, or OKX publishes a confirmed spot listing announcement via official blog or official Twitter/X account.

**Confirmation checks (all must pass before entering):**

1. The announcement is for a *spot* listing, not futures-only
2. The announcement is a confirmed listing, not a "monitoring", "voting", or "consideration" stage
3. The token is NOT yet tradeable on the announcing exchange (verify via exchange API)
4. A liquid DEX market exists with ≥$500K 24h volume at the time of detection
5. Token price has NOT already moved >30% since announcement timestamp (staleness filter — implies Zunid was too slow)
6. Token is not a stablecoin, wrapped asset, or LST

**Entry execution:**
- Enter a spot long position via the deepest available DEX pool (highest liquidity, lowest price impact) or the smallest-spread CEX that already lists the token
- Use a market order — do not use limit orders (risk of missing the window entirely)
- Enter within 15 minutes of confirmed announcement detection
- Accept up to 2% slippage on entry for tokens with <$1M pool liquidity; reject the trade if estimated slippage exceeds 3%

**Position size:**
- Paper trading phase: $500 notional per trade, no leverage
- Real capital phase (post-validation): see go-live criteria

---

## Exit Rules

**Primary exit:**
Close the full position 48 hours after the CEX listing goes live (not 48 hours after announcement — after listing activation).

**Rationale:** The "new buyer" flow from the listing exchange's user base peaks and normalises within 24–48 hours post-listing. Holding longer introduces unrelated market risk.

**Alternative exits (whichever triggers first):**

| Trigger | Action |
|---------|--------|
| +40% gain at any point before primary exit | Close full position — take the obvious win |
| −15% from entry price at any point | Close full position — stop-loss |
| Listing delayed >7 days post-announcement | Close full position — the narrative is stale |
| Listing cancelled | Close full position immediately |

**Do not adjust position mid-trade.** No partial exits, no averaging down. Fixed entry, fixed exit — keeps the backtest clean and the system simple.

---

## Position Sizing

### Paper trading phase
- $500 notional per trade, no leverage
- No more than 2 simultaneous open positions (announcement events sometimes cluster)
- If 2 positions are already open when a new announcement fires, skip the new event

### Rationale for no leverage
The expected move is large (20–70% historical range) and the asset is illiquid. Leverage introduces liquidation risk on a volatile, thinly traded position without meaningfully improving expected return given the magnitude of the underlying move. The edge here is position-level return, not leverage-enhanced return.

### Real capital phase (post-validation)
- Size per trade: 2% of total trading capital, maximum $2,000 notional
- Scaling rule: if median return after 10 paper trades exceeds 15% after simulated costs, consider increasing to 3% per trade
- Hard cap: no single trade exceeds $5,000 notional until strategy has 25+ live trades

---

## Go-Live Criteria

Deploy real capital when ALL of the following are met:

1. At least 8 paper trades have closed (listing announcements are infrequent; 8 gives a minimum sample)
2. Median return across closed trades is ≥10% after estimated 2% slippage and exit fees
3. Win rate is ≥55% (not just average driven by outliers)
4. No single paper trade lost more than 15% of notional (confirms stop-loss is functioning)
5. Backtest is completed on historical data with a positive median return after costs
6. Founder approves DEX execution infrastructure (wallet funded, on-chain execution tested)

---

## Kill Criteria

Kill the strategy (stop paper trading, do not deploy capital) if:

- After 5 closed paper trades: median return is negative after 2% slippage → kill or redesign
- After 10 closed paper trades: median return < 5% after all costs → edge too small to justify operational overhead
- After 15 closed paper trades: win rate < 45% → strategy is not reliably directional
- At any point: two consecutive trades trigger the −15% stop-loss → regime has changed; something is broken
- At any point: backtest reveals the entire historical edge was concentrated in 2021 and has since decayed to zero → hypothesis invalidated

---

## Risks

### 1. Detection latency — primary risk
If competitors detect the announcement faster, Zunid enters into a price already elevated by front-runners. The staleness filter (skip if >30% move already occurred) provides partial protection, but there is a continuous tradeoff between speed and position quality. 

*Mitigation:* Optimise announcement scraping to <60 second latency from blog post publish time. Monitor official Twitter/X accounts directly as they often tweet simultaneously or before the blog post is indexed.

### 2. DEX slippage — execution risk
A $500 buy in a pool with $300K TVL can move the price 1–3%. This is a direct tax on every entry and must be modelled in the backtest, not assumed away.

*Mitigation:* Hard filter — do not enter if estimated slippage exceeds 3%. Accept that some valid events will be skipped because liquidity is insufficient.

### 3. Insider front-running — signal contamination
Exchanges have employees and counterparties who know about listings in advance. Price sometimes moves 20–50% in the week before announcement (visible as DEX volume spike). If Zunid is detecting the announcement after insiders have already extracted the move, there is little edge remaining.

*Mitigation:* Track pre-announcement drift (T−7d to T−0). In the backtest, segment events by pre-announcement drift quartile and confirm the edge survives even when pre-announcement drift is high. Consider adding a filter: skip events where pre-announcement drift exceeds 25%.

### 4. Listing delay or cancellation — position stranding
Exchanges occasionally delay listings after announcement (regulatory issues, technical problems) or cancel them. The position becomes a directional spot bet with no structural catalyst.

*Mitigation:* Explicit exit rule: if listing has not gone live within 7 days of announcement, close the position. Stop-loss provides additional downside protection.

### 5. Edge decay over time — strategy lifetime
This is a known, discussed edge in the crypto trading community. As more participants systematise announcement monitoring, the interstitial window compresses. The 2022–2024 backtest period should be analysed in yearly slices to detect decay.

*Mitigation:* Monitor median return on a rolling 10-trade basis during paper trading and live trading. If rolling median drops below 5%, trigger a reassessment.

### 6. Correlated event risk
Multiple listing announcements sometimes occur in the same week during bull markets. If all positions are entered simultaneously and the market sells off broadly, losses are correlated.

*Mitigation:* Cap at 2 simultaneous open positions. Skip additional events while 2 are live.

### 7. Regulatory or exchange policy risk
A regulatory action forcing an exchange to cancel multiple listings simultaneously (e.g., SEC action against specific token categories) would cause correlated losses across open positions.

*Mitigation:* Small position sizes, stop-losses, and the 2-position cap limit maximum concurrent exposure.

---

## Data Sources

| Data | Source | Access |
|------|--------|--------|
| Binance listing announcements | blog.binance.com — scrape title, publish timestamp, token ticker | Free, public |
| Coinbase listing announcements | blog.coinbase.com and coinbase.com/en/blog — scrape same fields | Free, public |
| OKX listing announcements | okx.com/support/hc (announcement section) | Free, public |
| Official exchange Twitter/X posts | Twitter/X API or Nitter mirrors — monitor official accounts | Free tier available |
| DEX token prices (historical) | DexScreener API (`api.dexscreener.com/latest/dex/tokens/{address}`) | Free |
| Token prices at arbitrary timestamps | CoinGecko historical API (`/coins/{id}/history?date=DD-MM-YYYY`) | Free, rate-limited |
| DEX pool liquidity (historical) | DeFiLlama API (`defillama.com/docs`) | Free |
| CEX listing-live timestamp | Exchange public trade history API (first trade timestamp for the new pair) | Free |
| Post-listing price at T+24h, T+48h | Binance/Coinbase OHLCV API | Free |
| Token contract addresses | CoinGecko `coins/list` endpoint | Free |

---

## Implementation Notes

### Announcement detection
Build a lightweight scraper that polls the following on a 30-second interval:

- `https://blog.binance.com/en/category/listing/` — RSS or HTML scrape
- `https://www.coinbase.com/en/blog/tag/listed` — HTML scrape
- `https://twitter.com/binance` and `https://twitter.com/coinbase` — via Twitter API or RSS bridge

Parse for keywords: "will list", "listing", "now available", "spot trading". Filter out futures-only and voting posts. Emit an alert with token name, ticker, announcement timestamp, and source URL.

### DEX execution
For paper trading, simulate fills using DexScreener price at the 15-minute mark post-announcement with a slippage penalty applied. For live execution, integrate with a DEX aggregator (1inch, Paraswap) to get best available price across pools, or execute directly on the deepest single pool for simplicity.

### State management
Track per-trade: entry price, entry timestamp, announcement source, DEX pool used, pool liquidity at entry, estimated slippage, listing-live timestamp, exit price, exit reason, gross return, net return after fees.

Store in a JSON state file analogous to `experiments/paper_state.json`.
