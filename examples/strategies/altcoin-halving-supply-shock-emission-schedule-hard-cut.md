---
title: "Altcoin Halving Miner Sell-Side Vacuum"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 1
composite: 180
categories:
  - token-supply
  - calendar-seasonal
created: "2025-07-11T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a PoW altcoin's block reward halves, the daily token issuance to miners drops 50% overnight. Miners are structurally compelled to sell a portion of rewards to cover fixed operating costs (electricity, hardware depreciation, hosting). This creates a **mechanical sell-side flow** that is proportional to emission rate. Post-halving, that flow is cut in half — permanently, by consensus rule.

The edge is **not** the halving event itself (widely anticipated, typically front-run). The edge is the **post-halving steady-state reduction in miner sell pressure**, specifically in coins where miner daily emissions represent a measurable fraction (>0.5%) of average daily trading volume. In these thin markets, removing that sell-side flow creates a persistent bid-side imbalance that takes weeks to months to fully reprice.

**Causal chain:**

1. Halving block confirmed on-chain → daily new supply drops 50% (guaranteed by consensus code)
2. Miners receive 50% fewer tokens for same electricity spend → forced selling of rewards drops proportionally
3. In coins where miner emissions ÷ avg daily volume > 1% pre-halving, the post-halving drop in sell flow > 0.5% of daily volume — a non-trivial removal of structural sell pressure
4. Market makers and spot buyers face reduced overhead supply → bid-ask dynamics shift gradually
5. Price drifts upward over 30–90 days as the market reprices the new equilibrium supply rate
6. Effect is amplified if marginal miners shut off (hash rate drops) → further reduces future emission velocity until difficulty adjusts

**Why 14-day delay on entry:** Pre-halving hype creates a pump-and-dump cycle. Waiting 14 days post-halving flushes speculative longs, lets funding rates normalise, and enters after the "sell the news" flush has cleared.

---

## Structural Mechanism — Why This MUST Happen

The emission reduction is **encoded in consensus rules** — it is not a corporate decision, not a governance vote, not a probability. The halving block height is calculable from genesis block parameters. Once that block is mined, the reward schedule is immutable without a hard fork.

What is NOT guaranteed: the price impact. That depends on:
- What fraction of miner rewards are actually sold (vs. held)
- Whether speculative volume dwarfs miner flow
- Whether the halving was fully front-run

The structural guarantee is only: **daily new token supply drops 50%**. Everything downstream is probabilistic. This is why the score is 5/10, not 8/10.

The filter (emission ÷ volume > 1% pre-halving) is the mechanism for isolating coins where the guaranteed supply drop is large enough relative to market activity to matter. Without this filter, the strategy degenerates into a generic "buy the halving" trade with no structural edge.

**Miner economics forcing sell pressure (the causal link):**
- Industrial miners operate at 40–70% gross margins at equilibrium
- Electricity bills are paid in fiat, not tokens
- Miners must sell a minimum of (monthly electricity cost ÷ token price) tokens per month regardless of market conditions
- This creates a **price-inelastic sell floor** that is proportional to emission rate
- Post-halving, this floor drops 50% in token terms (assuming price doesn't immediately double)

---

## Entry Rules


### Pre-Entry Screening (run once per identified halving)

**Step 1 — Identify upcoming halvings:**
- Monitor block explorers for coins within 90 days of halving block height
- Target list: KAS, RVN, ERG, ZEC, LTC, ETC, DASH, BCH, FLUX, XNA, and any PoW coin with CMC rank < 200

**Step 2 — Calculate Emission/Volume Ratio (EVR):**
```
Pre-halving daily emission = current block reward × avg blocks per day
EVR = pre-halving daily emission (USD) ÷ 30-day avg daily volume (USD)
```
- **Minimum threshold:** EVR > 1.0% pre-halving (meaning emission drop will be > 0.5% of volume)
- **Preferred threshold:** EVR > 2.0% (stronger signal)
- Discard coin if EVR < 1.0% — miner flow is too small to matter

**Step 3 — Liquidity check:**
- 30-day avg daily volume (CEX + DEX combined) > $500k USD
- If volume < $500k, slippage will consume the edge
- If volume > $50M, EVR is almost certainly below threshold anyway

**Step 4 — Funding rate check (perp only):**
- If using perpetual futures: funding rate at entry must be < 0.05%/8h
- If funding > 0.05%/8h, crowded long — skip or use spot only
- If funding > 0.1%/8h, hard kill — do not enter

### Entry

- **Trigger:** 14 calendar days after halving block is confirmed on-chain
- **Instrument:** Spot preferred (no funding drag). Perp acceptable if funding < 0.05%/8h
- **Entry execution:** TWAP over 4 hours using limit orders to minimise market impact on thin books
- **Entry price:** Record VWAP of entry session as reference price

## Exit Rules

### Exit

**Take profit:** +40% from entry VWAP → close 100% of position  
**Time stop:** 90 calendar days post-halving → close 100% regardless of P&L  
**Trailing variant (optional):** At +20%, move stop to breakeven; at +30%, trail stop at -10% from peak  

### Stop Loss

- **Hard stop:** -20% from entry VWAP → close 100%
- **Funding stop (perp only):** If funding rate exceeds 0.1%/8h for 3 consecutive 8h periods → close perp, optionally roll to spot
- **Hash rate collapse stop:** If network hash rate drops >40% within 30 days post-halving (miner capitulation spiral) → close position. Check via coin-specific explorer APIs daily.

---

## Position Sizing

**Base allocation:** 1–3% of portfolio per trade  
**Scaling by EVR:**
- EVR 1–2%: 1% allocation
- EVR 2–4%: 2% allocation
- EVR > 4%: 3% allocation (rare; only very thin coins)

**Maximum concurrent positions:** 3 (halvings rarely cluster, but can overlap)  
**Maximum total exposure:** 6% of portfolio in this strategy at any time  

**Rationale for small sizing:** These are illiquid, high-volatility small caps. The edge is real but the noise-to-signal ratio is high. Sizing must reflect that a -20% stop loss is a realistic outcome on any individual trade.

**Leverage:** None on spot. Maximum 2x on perp, only if EVR > 3% and funding < 0.02%/8h. Default is 1x.

---

## Backtest Methodology

### Data Sources

| Data Type | Source | Notes |
|-----------|--------|-------|
| Historical halving dates/blocks | Coin-specific block explorers | See Data Sources section |
| OHLCV price data | CoinGecko API (`/coins/{id}/market_chart`) | Free tier: 365 days; paid for full history |
| Historical volume | CoinMarketCap API (`/v1/cryptocurrency/ohlcv/historical`) | Requires API key |
| Block reward history | Block explorer APIs or manually from protocol docs | Verify against actual block data |
| Funding rates (perp) | Coinglass API (`/api/pro/v1/futures/fundingRate/chart`) | For coins with perp markets |
| Hash rate history | CoinWarz, Bitinfocharts, or coin-specific explorers | |

### Historical Halvings to Backtest

| Coin | Halving Date | Notes |
|------|-------------|-------|
| LTC | Aug 2023, Aug 2019, Aug 2015 | 3 events, good data |
| ZEC | Nov 2024, Nov 2020 | 2 events |
| BCH | Apr 2024, Apr 2020 | 2 events |
| ETC | Mar 2024, Mar 2022, Mar 2020 | 3 events |
| DASH | Jan 2024, Feb 2021, Apr 2018 | 3 events |
| RVN | Jan 2022 | 1 event, limited data |
| ERG | ~2024 emission reduction | Check protocol schedule |

**Minimum sample:** 10+ halving events across all coins to draw any conclusions. Current universe gives ~15 events with adequate price history.

### Backtest Steps

**Step 1 — Calculate EVR for each historical halving:**
- Pull 30-day avg daily volume ending on halving date
- Calculate pre-halving daily emission in USD (block reward × blocks/day × price on halving date)
- Compute EVR = emission USD ÷ volume USD
- Tag each event: EVR bucket (< 1%, 1–2%, 2–4%, > 4%)

**Step 2 — Simulate entries:**
- Entry date = halving date + 14 calendar days
- Entry price = VWAP of that day (use (H+L+C)/3 as proxy if tick data unavailable)
- Record funding rate on entry date (for perp-eligible coins)
- Apply kill filter: skip if funding > 0.1%/8h on entry date

**Step 3 — Simulate exits:**
- Track daily close prices for 90 days post-entry
- Exit at first of: +40% from entry, -20% from entry, or day 90
- Record exit price, holding period, P&L

**Step 4 — Segment results:**
- Primary cut: EVR < 1% vs. EVR > 1% vs. EVR > 2%
- Secondary cut: coins with perp available vs. spot only
- Secondary cut: bull market context vs. bear market context (BTC trend as proxy)

### Metrics to Calculate

| Metric | Minimum Acceptable | Target |
|--------|-------------------|--------|
| Win rate | > 50% | > 60% |
| Average win / average loss | > 1.5 | > 2.0 |
| Expectancy per trade | > 5% | > 15% |
| Max drawdown (strategy-level) | < 30% | < 20% |
| Sharpe ratio (annualised) | > 0.8 | > 1.2 |
| EVR > 1% subset outperforms EVR < 1% | Required | Confirms filter works |

### Baseline Comparison

Compare against two baselines:
1. **Naive halving long:** Enter on halving date, same exit rules — tests whether the 14-day delay adds value
2. **Random entry control:** Enter on a random date ±60 days from halving, same exit rules — tests whether halving timing matters at all

If the strategy doesn't beat both baselines on expectancy, the structural mechanism is not generating alpha beyond noise.

---

## Go-Live Criteria

All of the following must be true before moving to paper trading:

1. **Sample size:** ≥ 10 completed backtest trades (not just halving events — actual trades that passed all filters)
2. **Expectancy:** > 10% per trade on EVR-filtered subset
3. **EVR filter validation:** EVR > 1% subset shows materially better expectancy than EVR < 1% subset (confirms the mechanism, not just the event)
4. **Baseline beat:** Strategy expectancy > naive halving long AND > random entry control
5. **No single event dominance:** Remove the single best trade — strategy still shows positive expectancy
6. **Drawdown:** No single trade exceeds -25% (validates stop loss is realistic, not just theoretical)

**Paper trading duration before live:** Minimum 2 halving events observed in real-time (not backtested). Given halving frequency, this may take 6–18 months.

---

## Kill Criteria

**Abandon the strategy entirely if:**

1. Backtest shows EVR filter provides no differentiation — EVR > 1% and EVR < 1% subsets have similar expectancy (means the mechanism isn't the driver)
2. Backtest expectancy < 5% per trade after realistic slippage assumptions (1–3% on entry + exit for thin coins)
3. Win rate < 45% with average win/loss < 1.5 (negative expectancy)
4. All positive returns cluster in 2021 bull market only — strategy is just "buy altcoins in a bull market"
5. Live paper trading: 3 consecutive losses hitting the -20% stop (suggests regime change or strategy decay)
6. A major coin (LTC, ZEC) undergoes a halving with EVR > 1% and the strategy loses — single high-quality data point that contradicts the thesis

---

## Risks

**Risk 1: Pre-halving front-running (HIGH probability)**
The halving is public knowledge months in advance. Sophisticated participants may fully price in the supply reduction before the event. The 14-day delay helps but doesn't eliminate this. Mitigation: the EVR filter targets coins where the structural effect is large enough that even partial front-running leaves residual alpha.

**Risk 2: Speculative volume dwarfs miner flow (HIGH probability for large caps)**
For LTC or ZEC, daily speculative volume is orders of magnitude larger than miner emissions. The EVR filter is designed to exclude these, but even at EVR > 1%, speculative sentiment can overwhelm the structural signal. Mitigation: strict EVR threshold, small position sizing.

**Risk 3: Miner capitulation spiral (MEDIUM probability)**
If price doesn't rise post-halving, marginal miners shut off. Hash rate drops → blocks slow → difficulty adjusts down → remaining miners get same rewards but network is weaker → further price decline. This is the opposite of the thesis. Mitigation: hash rate collapse stop (-40% hash rate triggers exit).

**Risk 4: Liquidity illusion (MEDIUM probability)**
Reported volume on small-cap coins is frequently wash-traded. CMC/CoinGecko volume figures for coins ranked 100–500 may be 50–90% fake. If real volume is 10x lower than reported, EVR is 10x higher — which sounds better but means the coin is actually untradeable at scale. Mitigation: cross-reference volume across multiple sources; use DEX on-chain volume where available as a sanity check; cap position size at 0.5% of 30-day avg reported volume.

**Risk 5: Regulatory or exchange delisting (LOW-MEDIUM probability)**
Small PoW coins face delisting risk (especially privacy coins like ZEC, DASH). A delisting announcement during the holding period would be catastrophic. Mitigation: monitor exchange announcements; avoid coins with active regulatory pressure at entry.

**Risk 6: The effect is real but already arbitraged away**
If this strategy has been run by others, the pre-halving pump may now fully price in the post-halving supply reduction, leaving no residual edge. The backtest will reveal this — if recent halvings (2022–2024) show worse performance than older ones (2018–2020), the edge is decaying.

**Risk 7: Slippage destroys the edge**
On a coin with $1M daily volume, a $30k position is 3% of daily volume. Entry and exit slippage combined could easily be 2–5%. A 10% expected return becomes 5% after slippage. Mitigation: TWAP entry over 4 hours, limit orders only, size cap at 0.5% of daily volume.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|--------|---------------|-------------|
| CoinGecko API | `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=365` | OHLCV, volume history |
| CoinMarketCap API | `https://pro-api.coinmarketcap.com/v1/cryptocurrency/ohlcv/historical` | Volume cross-reference |
| Litecoin explorer | `https://litecoinspace.org/api/blocks/tip/height` | Block height, reward |
| ZEC explorer | `https://zcashblockexplorer.com/api/v1/` | Block height, reward |
| ETC explorer | `https://blockscout.com/etc/mainnet/api` | Block height, reward |
| RVN explorer | `https://ravencoin.network/api/` | Block height, reward |
| Bitinfocharts | `https://bitinfocharts.com/comparison/hashrate-ltc.html` | Hash rate history (manual scrape or use API) |
| CoinWarz | `https://www.coinwarz.com/mining/litecoin/hashrate-chart` | Hash rate history |
| Coinglass | `https://open-api.coinglass.com/public/v2/funding` | Funding rates for perp-eligible coins |
| MiningPoolStats | `https://miningpoolstats.stream/` | Pool hash rate, estimated miner revenue |
| Halving countdown aggregator | `https://www.nicehash.com/blog/post/when-is-the-next-litecoin-halving` | Cross-reference halving dates |
| CryptoCompare | `https://min-api.cryptocompare.com/data/v2/histoday?fsym=LTC&tsym=USD&limit=365` | Alternative OHLCV source |

**Note on data quality:** For coins ranked below CMC #100, treat all volume figures as suspect until cross-referenced with on-chain DEX data (Uniswap subgraph, DexScreener API) where applicable. Block reward data should always be verified against actual block explorer data, not just protocol documentation, as some coins have had mid-schedule adjustments.

---

## Open Questions for Backtest Phase

1. Does the 14-day delay materially improve returns vs. entry on halving date? (Test both)
2. Is EVR > 1% the right threshold, or should it be higher? (Test 0.5%, 1%, 2%, 3% cutoffs)
3. Does the hash rate collapse stop improve or hurt returns? (Test with and without)
4. Is the effect stronger in bear markets (where miner sell pressure is a larger fraction of total selling) or bull markets (where momentum amplifies the signal)?
5. Do coins with concentrated mining (few large pools) show stronger effects than coins with distributed mining? (Pool concentration data from MiningPoolStats)
