---
title: "New Listing Funding Carry — Structural Market Maker Hedging Bias on Hyperliquid Perpetuals"
status: HYPOTHESIS
mechanism: 5
implementation: 7
safety: 6
frequency: 3
composite: 630
categories:
  - funding-rates
  - exchange-structure
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a new perpetual futures contract lists on Hyperliquid, market makers who provide spot liquidity systematically short the perp to hedge their delta exposure. This creates a structural short-heavy open interest imbalance in the first 1–14 days post-listing, which mechanically forces funding rates negative (shorts pay longs). Going long the perp during this window captures the funding carry. The edge decays as retail long demand builds or as market makers unwind spot inventory and close hedges.

**Specific causal chain:**

1. Hyperliquid lists a new perp. Spot liquidity is thin at launch.
2. Market makers (MMs) quote spot markets and accumulate net long spot inventory as they fill retail buy orders.
3. MMs hedge delta by shorting the perp. They are indifferent to directional exposure; they want the spread, not the beta.
4. MM short perp positions dominate early OI because organic long perp demand from retail speculators has not yet built.
5. Funding rate formula: when shorts > longs in OI, funding is negative → shorts pay longs.
6. This imbalance is temporary. It resolves when: (a) retail long demand grows and absorbs MM shorts, (b) MMs unwind spot inventory and close perp hedges, or (c) the token's spot liquidity matures enough that MMs no longer need large hedges.
7. The carry window is estimated at 7–14 days based on the mechanism's decay logic, but this is the primary empirical question the backtest must answer.

**Null hypothesis to disprove:** Funding rates on new Hyperliquid listings in the first 14 days are not systematically different from funding rates on the same assets after day 30.

---

## Structural Mechanism — WHY This Happens

This is a **probabilistic structural edge**, not a contractually guaranteed one (hence 6/10, not 8+).

The mechanism is grounded in market microstructure:

- **Delta-neutral MM behavior is not optional.** A market maker who quotes spot without hedging accumulates directional risk that exceeds their risk limits. The perp is the cheapest, most liquid hedge available. This is not a choice — it is a mechanical consequence of being a liquidity provider.
- **Perp OI composition is observable.** Hyperliquid's API exposes funding rates in real time. If the mechanism is real, it will show up as statistically elevated negative funding in the first N days post-listing versus the baseline period.
- **The imbalance is self-limiting.** As the token matures, spot liquidity deepens, MM inventory turns over faster, and retail long perp demand grows. The structural cause of the imbalance disappears, so the funding normalizes. This gives the trade a natural exit signal.
- **Confounding factor:** Some new listings arrive with pre-existing retail hype (e.g., airdrop tokens, high-profile launches). In these cases, retail long demand may immediately overwhelm MM hedging, producing positive funding from day 1. The entry filter (3 consecutive negative funding periods) is designed to screen these out.

**What this is NOT:** This is not a claim that "new listings tend to have negative funding historically." That would be a pattern. The claim is that a specific mechanical process (MM delta hedging) creates the imbalance, and that process is observable and predictable from first principles.

---

## Entry Rules


### Entry Conditions (all must be true)

1. **New listing trigger:** A new perpetual contract has been listed on Hyperliquid within the last 72 hours. Source: Hyperliquid API `/info` endpoint polling for new `coin` entries, cross-referenced with announcement timestamp.
2. **Funding confirmation:** The 8-hour funding rate has been negative (< −0.005% per 8h, i.e., −0.015% per day) for **3 consecutive funding periods** (= 24 hours of data). This filters out tokens where retail demand immediately dominates.
3. **Minimum OI filter:** Open interest > $500,000 USD at time of entry. Below this threshold, a single large trade can distort funding and the signal is noise.
4. **Volatility filter (optional, test both):** 24h price change of the underlying < ±20%. Extreme moves at listing suggest the directional risk will dwarf the carry.

### Entry Execution

- Enter long perp at market on the open of the 4th funding period (i.e., immediately after the 3rd consecutive negative period confirms).
- Record entry funding rate, entry price, and entry timestamp.

## Exit Rules

### Exit Conditions (first trigger wins)

1. **Funding normalization:** Funding rate turns positive (> +0.005% per 8h) for **2 consecutive periods** (16 hours). This signals the structural imbalance has resolved.
2. **Time stop:** T+14 calendar days from entry, regardless of funding state.
3. **Directional stop-loss:** Position mark-to-market loss exceeds **5% of position notional** (not 5% of NAV — see sizing). This caps the directional bleed from unhedged exposure.
4. **Funding reversal spike:** If funding drops below −0.1% per 8h (extreme negative), exit immediately — this signals a potential squeeze or manipulation event, not the structural carry.

### Optional Delta Hedge

If spot is available on a CEX (Binance, OKX) for the same token:
- Short equivalent notional in spot to neutralize directional exposure.
- Hedge ratio: 1:1 notional. Rebalance if spot/perp price diverges > 2%.
- If no spot available, accept unhedged directional risk and enforce the 5% stop-loss strictly.
- Proxy hedge: if token is BTC-correlated (beta > 0.7 over prior 30 days), short BTC perp for 50% of notional as partial hedge. Test whether this improves Sharpe in backtest.

---

## Position Sizing

- **Base size:** 0.5% of total NAV per trade, unhedged.
- **Hedged size:** 1.0% of total NAV per trade if delta-hedged with spot short.
- **Maximum concurrent positions:** 3 (total exposure: 1.5% NAV unhedged or 3.0% NAV hedged). New listings cluster; cap prevents overconcentration in a single market regime.
- **Rationale:** The directional risk on new listings is high. Funding carry at −0.02% per 8h = −0.06% per day = ~2.2% per month. A 10% adverse price move wipes out 4+ months of carry. Small size is mandatory unless fully hedged.
- **Do not scale up mid-trade** based on funding rate deepening — deeper negative funding may signal a squeeze, not a better carry opportunity.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Endpoint/URL |
|---|---|---|
| Hyperliquid funding rate history | Hyperliquid API | `https://api.hyperliquid.xyz/info` → `fundingHistory` method, params: `{coin, startTime, endTime}` |
| Hyperliquid listing dates | Hyperliquid API + Discord archive | Poll `metaAndAssetCtxs` for first appearance of each coin; cross-check with #announcements Discord channel |
| Hyperliquid OI history | Hyperliquid API | `openInterest` field in `metaAndAssetCtxs` response |
| Perp price (mark price) | Hyperliquid API | `markPx` field, 1h candles via `candleSnapshot` |
| Spot price (for hedge simulation) | Binance API | `https://api.binance.com/api/v3/klines` |

### Backtest Universe

- All perps listed on Hyperliquid from **January 2023 to present** (Hyperliquid's full history).
- Exclude: BTC, ETH (these had pre-existing deep liquidity at listing; MM hedging dynamic may not apply). Include all altcoin listings.
- Expected universe size: ~50–150 listing events depending on Hyperliquid's listing history. This is a small sample — statistical power will be limited. Flag this explicitly in results.

### Metrics to Compute

**Primary:**
- **Funding carry captured per trade** (sum of 8h funding payments received during holding period)
- **Net P&L per trade** = funding carry + mark-to-market price change (unhedged) or funding carry only (if perfectly hedged)
- **Win rate** (% of trades where net P&L > 0)
- **Average holding period** (days until exit trigger fires)

**Secondary:**
- **Funding carry vs. directional P&L decomposition** — what % of total P&L came from funding vs. price movement? If directional dominates, the strategy is not a carry trade, it's a directional bet with a funding bonus.
- **Funding rate on day 1–3 vs. day 7–14** — does the carry decay as hypothesized?
- **Sharpe ratio** (annualized, using daily P&L)
- **Max drawdown per trade** and **portfolio-level max drawdown**

**Baseline comparison:**
- Compare funding carry in days 1–14 post-listing vs. days 30–44 post-listing (same assets, same duration). If the mechanism is real, early-period funding should be statistically more negative. Run a paired t-test or Wilcoxon signed-rank test on the difference.
- Compare against a naive strategy of going long any perp with negative funding (not just new listings) to isolate the "new listing" alpha from the general "negative funding" carry.

### Backtest Execution Logic

```
for each listing_event in hyperliquid_listings:
    t0 = listing_timestamp
    funding_series = get_funding(coin, t0, t0 + 14_days)
    
    # Find entry point
    entry_period = find_3_consecutive_negative(funding_series, threshold=-0.005%)
    if entry_period is None:
        continue  # No entry signal; skip
    
    entry_time = entry_period.end
    entry_price = mark_price(coin, entry_time)
    
    # Simulate holding
    for each 8h period after entry:
        collect funding payment
        check exit conditions (funding normalization, time stop, price stop)
        if exit triggered:
            record exit_price, exit_time, total_funding_collected
            break
    
    compute trade_pnl = funding_collected + (exit_price - entry_price) / entry_price
```

### Known Backtest Limitations

- **Survivorship bias:** Hyperliquid has delisted some tokens. Include delisted tokens if data is available; if not, flag the bias.
- **Slippage:** New listing perps have wide spreads. Model 0.1% round-trip slippage on entry and exit (conservative for thin markets).
- **Funding rate manipulation:** Some new listings show extreme funding spikes that are not representative of the structural mechanism. Flag outliers (> 3 standard deviations from mean new-listing funding) and run results with and without them.
- **Small sample:** ~50–150 events is not large. Do not over-fit entry/exit parameters. Test the primary hypothesis (is early funding more negative?) before optimizing thresholds.

---

## Go-Live Criteria

The following must all be satisfied before promoting to paper trading:

1. **Statistical significance:** Funding rate in days 1–14 post-listing is statistically more negative than days 30–44 (p < 0.05, Wilcoxon signed-rank test). If this fails, the structural mechanism is not confirmed and the strategy is abandoned.
2. **Positive net P&L:** Median net P&L per trade > 0 after 0.1% slippage, across the full backtest universe.
3. **Carry dominates:** Funding carry component accounts for > 50% of winning trades' P&L. If directional P&L dominates, this is not a carry strategy and sizing/hedging rules must be revised before paper trading.
4. **Drawdown acceptable:** No single trade loses more than 8% of position notional (after stop-loss). Portfolio-level max drawdown < 3% NAV.
5. **Minimum sample:** At least 20 qualifying trades (entry signal triggered) in the backtest period. If fewer, the strategy is too rare to evaluate and is parked, not abandoned.
6. **Decay confirmed:** Average funding rate in days 1–7 is more negative than days 8–14, confirming the mechanism decays as hypothesized. If funding is equally negative throughout, the exit logic needs revision.

---

## Kill Criteria

Abandon the strategy (do not proceed to live trading) if any of the following occur:

1. **Mechanism not confirmed:** Backtest shows no statistically significant difference between early and late post-listing funding rates. The causal story is wrong.
2. **Directional risk dominates:** In the backtest, > 60% of total P&L variance is explained by price movement, not funding. The carry is too small relative to the noise.
3. **Hyperliquid changes listing mechanics:** If Hyperliquid introduces a pre-listing liquidity bootstrapping mechanism (e.g., pre-market trading, initial margin subsidies) that changes the MM hedging dynamic, the structural basis is invalidated.
4. **Paper trading failure:** After 10 paper trades, net P&L is negative or Sharpe < 0.5 annualized. Stop and re-evaluate.
5. **Opportunity set collapses:** Hyperliquid listing pace drops below 2 new perps per month, making the strategy too infrequent to be worth the operational overhead.
6. **Funding rate API reliability:** If Hyperliquid's historical funding data is found to have gaps or inconsistencies that prevent reliable backtesting, park the strategy until data quality improves.

---

## Risks

### Primary Risk: Directional Exposure on New Listings

New listing tokens are among the most volatile assets in crypto. A token can drop 30–50% in the first week post-listing. At −0.02% per 8h funding, you collect ~2% per month in carry. A single 10% adverse move wipes out 5 months of carry. **This is the dominant risk.** Mitigation: strict 5% stop-loss on position notional, small sizing, and delta hedge where possible.

### Secondary Risk: Funding Rate Manipulation

Large players can temporarily push funding negative to attract long counterparties, then unwind. The 3-period confirmation filter reduces but does not eliminate this risk. The exit rule (exit if funding drops below −0.1% per 8h) is designed to catch manipulation events.

### Liquidity Risk

New listing perps on Hyperliquid may have insufficient liquidity to enter/exit at reasonable prices. The $500k OI filter partially addresses this. Model 0.1% slippage in backtest; in live trading, use limit orders and accept partial fills.

### Counterparty / Protocol Risk

Hyperliquid's HLP vault is often the counterparty on new listing perps. If HLP is net long (taking the other side of MM shorts), it may have incentives to manage funding in ways that disadvantage retail longs. This is a structural conflict of interest that is difficult to model. Monitor HLP position disclosures if available.

### Mechanism Decay Risk

As Hyperliquid matures and more sophisticated participants recognize this pattern, the early-listing funding anomaly may compress or disappear. The strategy has a finite shelf life. Re-run the backtest annually on the most recent 12 months of data to check for decay.

### Correlation Risk (Proxy Hedge)

If using BTC perp as a proxy hedge for altcoin exposure, correlation can break down precisely during high-volatility new listing events (idiosyncratic moves). The proxy hedge may provide false comfort. Test hedge effectiveness in the backtest by measuring residual P&L variance after hedging.

---

## Data Sources

| Source | URL / Endpoint | Notes |
|---|---|---|
| Hyperliquid REST API (funding history) | `https://api.hyperliquid.xyz/info` POST `{"type": "fundingHistory", "coin": "TOKEN", "startTime": UNIX_MS}` | Returns 8h funding rate history. Free, no auth required. |
| Hyperliquid REST API (meta + asset contexts) | `https://api.hyperliquid.xyz/info` POST `{"type": "metaAndAssetCtxs"}` | Returns all listed coins with OI, mark price. Poll daily to detect new listings. |
| Hyperliquid REST API (candles) | `https://api.hyperliquid.xyz/info` POST `{"type": "candleSnapshot", "req": {"coin": "TOKEN", "interval": "1h", "startTime": UNIX_MS, "endTime": UNIX_MS}}` | 1h OHLCV for mark price. |
| Hyperliquid Discord announcements | `https://discord.gg/hyperliquid` → #announcements | Manual archive needed for listing timestamps pre-API coverage. |
| Binance spot API (hedge prices) | `https://api.binance.com/api/v3/klines?symbol=TOKENUSDT&interval=1h` | Free, no auth for public endpoints. |
| Hyperliquid GitHub / community data | `https://github.com/hyperliquid-dex` | Check for community-maintained historical datasets that may predate API availability. |

**Data collection script priority:** Build a polling script that hits `metaAndAssetCtxs` every 4 hours and logs the first appearance of each new coin with a timestamp. This becomes the ground truth listing database for the backtest. Start collecting now — historical reconstruction from the API is possible but verify completeness against Discord announcements.
