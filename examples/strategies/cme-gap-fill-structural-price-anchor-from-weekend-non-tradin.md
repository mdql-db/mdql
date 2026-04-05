---
title: "CME Gap Fill — Weekend Non-Trading Price Anchor"
status: HYPOTHESIS
mechanism: 4
implementation: 7
safety: 6
frequency: 5
composite: 840
categories:
  - calendar-seasonal
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When BTC spot price moves >1.5% during the CME weekend closure (Friday 4:00pm ET to Sunday 5:00pm ET), institutional participants who set delta hedges against CME closing prices are immediately offside at Sunday open. Their rebalancing flow creates mean-reversion pressure toward Friday's CME close price during the first 2–4 hours of the Sunday session. The trade fades the gap direction at CME open and exits before the US equity open on Monday removes the structural pressure.

**Causal chain:**

1. Friday 4:00pm ET — CME BTC front-month futures close. Institutional desks (options market makers, basis traders, delta-hedged funds) lock their books against this reference price.
2. Friday 4:00pm → Sunday 5:00pm ET — BTC spot trades continuously. A macro event, Asia session move, or weekend liquidity event pushes spot materially away from Friday's CME close.
3. Sunday 5:00pm ET — CME reopens. The front-month futures contract opens with a gap reflecting the weekend spot move.
4. Immediately post-open — Participants with Friday-reference delta hedges are now mishedged. A fund that was delta-neutral at Friday close is now long or short delta by the gap amount. They must rebalance. This rebalancing flow is directionally opposite to the gap, creating mean-reversion pressure.
5. Secondary mechanism — Systematic futures strategies (CTAs, vol-targeting funds) that use CME closing prices as NAV reference also generate rebalancing orders at Sunday open.
6. Tertiary mechanism — Basis traders running CME/spot arbitrage desks re-anchor their spread models to Friday close, generating additional convergence flow.

**Why this is NOT pure TA:** The gap fill tendency, if it exists, is caused by a specific population of actors with a specific mechanical obligation — not by chart patterns or retail psychology. The edge degrades or reverses when the weekend move is macro-driven (news event), because in that case hedgers mark their books to the new reality rather than rebalancing to Friday's reference.

---

## Structural Mechanism

**Strength: Partially structural, partially probabilistic. Score 5/10.**

The mechanism is real but not contractually forced:

| Factor | Assessment |
|--------|------------|
| Hedger rebalancing obligation | Real, but hedgers have discretion — they can choose to mark books rather than rebalance if the move is news-driven |
| CME as institutional reference price | Real — CME settlement prices are used in fund NAV calculations, margin calls, and options expiry |
| Rebalancing flow direction | Opposite to gap — this is the structural prediction |
| Timing of rebalancing | Concentrated at Sunday open, dissipates by Monday open when new information flow dominates |
| Forced vs. discretionary | Discretionary — this is the key weakness vs. Zunid's token unlock standard |

**Comparison to Zunid's token unlock standard:** Token unlocks are contractually guaranteed supply events. CME gap fills are mechanically *likely* but not *forced*. A hedger who decides the weekend move is a new regime will trade with the gap, not against it. This is why the score is 5, not 7+.

**The edge is real only if:** The population of discretionary-rebalancing weekends (quiet macro, no major news) is large enough to dominate the dataset. The strategy must filter for this.

---

## Entry/Exit Rules

### Universe
- Instrument: BTC perpetual futures on Hyperliquid (execution), tracked against CME BTC front-month (reference)
- Timeframe: Sunday 5:00pm ET to 8:00pm ET (3-hour window)

### Gap Definition
- **CME Friday close price:** Last traded price of CME BTC front-month futures at 4:00pm ET Friday
- **CME Sunday open price:** First traded price of CME BTC front-month futures at 5:00pm ET Sunday
- **Gap %:** `(Sunday_open - Friday_close) / Friday_close × 100`

### Entry Conditions
| Condition | Requirement |
|-----------|-------------|
| Gap size | `abs(Gap %) > 1.5%` |
| Direction | Short if Gap % > +1.5%; Long if Gap % < -1.5% |
| News filter | No major macro event during weekend (see Risks section for filter definition) |
| VIX filter | VIX at Friday close < 30 (high-VIX regimes = news-driven moves, not quiet rebalancing) |
| Entry timing | Market order at 5:05pm ET Sunday (5 minutes after CME open to avoid the first-minute spike) |

### Exit Conditions (first trigger wins)
| Exit | Rule |
|------|------|
| Target | Gap 50% filled: price returns halfway between Sunday open and Friday CME close |
| Time stop | 8:00pm ET Sunday (3 hours after entry) |
| Hard stop | Gap extends to 2× original gap size from entry price |

### Example
- Friday CME close: $100,000
- Sunday CME open: $103,000 (gap = +3%)
- Entry: Short at $103,000 (or market at 5:05pm)
- Target: $101,500 (50% fill = halfway back to $100,000)
- Hard stop: $106,000 (gap extends to 6% = 2× original)
- Time exit: 8:00pm ET Sunday if neither target nor stop hit

---

## Position Sizing

### Base sizing
- Risk per trade: **0.5% of account** (reduced from standard 1% due to 5/10 score — hypothesis phase)
- Position size calculation: `Position = (Account × 0.005) / (Stop distance in $)`
- Stop distance: Gap size × 1.0 (e.g., 3% gap → stop is 3% from entry)
- Maximum position: 2% of account notional (hard cap regardless of calculation)

### Scaling rules
- **Do not scale in.** Single entry at 5:05pm ET.
- **Do not average down** if trade moves against you before stop.
- Paper trade phase: Use 0.1% risk per trade (sizing is irrelevant for hypothesis validation — focus on win rate and R-multiple distribution).

### Leverage
- Use maximum 3× leverage on Hyperliquid
- At 0.5% risk and 3% stop, this implies ~16% of account in the position — within 3× leverage at normal account utilization

---

## Backtest Methodology

### Data Sources
See Data Sources section below for URLs.

### Dataset
- **Period:** January 2021 – December 2024 (4 years, ~208 weekends)
- **Expected qualifying events:** Estimate 30–60 weekends with gaps >1.5% (hypothesis — needs measurement)
- **Minimum sample for statistical validity:** 30 qualifying events per regime (quiet vs. news weekends)

### Step-by-Step Backtest Construction

**Step 1: Build the gap dataset**
- Pull CME BTC front-month OHLCV (daily + intraday 1-minute)
- For each Friday: record 4:00pm ET close price
- For each Sunday: record 5:00pm ET open price
- Calculate Gap % for all 208 weekends
- Flag weekends where `abs(Gap %) > 1.5%`

**Step 2: Apply news filter**
- For each qualifying gap weekend, check: Was there a major macro event (FOMC, CPI, geopolitical shock) between Friday 4pm and Sunday 5pm?
- Source: FRED economic calendar, major news archives
- Create two subsets: `QUIET` (no major news) and `NEWS` (major news present)
- Hypothesis: Gap fill rate should be higher in `QUIET` subset

**Step 3: Apply VIX filter**
- For each qualifying gap weekend, record VIX at Friday 4:00pm ET close
- Create two subsets: `LOW_VIX` (VIX < 30) and `HIGH_VIX` (VIX ≥ 30)

**Step 4: Simulate trades**
- For each qualifying weekend in the filtered dataset:
  - Entry: Price at 5:05pm ET Sunday (use 1-minute CME data or Coinbase spot as proxy)
  - Track price every minute from 5:05pm to 8:00pm ET
  - Record: Did price hit 50% fill target? Did it hit 2× stop? What was price at 8:00pm?
  - Calculate P&L in R-multiples (risk units)

**Step 5: Segment analysis**
Run the following cuts:
- All gaps vs. gaps >3% vs. gaps >5%
- QUIET vs. NEWS weekends
- LOW_VIX vs. HIGH_VIX
- Bull market regime (2021, 2023-24) vs. bear market regime (2022)
- Month of year (any seasonality?)

### Key Metrics to Calculate

| Metric | Minimum acceptable | Target |
|--------|-------------------|--------|
| Win rate (50% fill before stop) | >55% | >65% |
| Average R-multiple per trade | >0.3R | >0.5R |
| Expectancy (win rate × avg win − loss rate × avg loss) | >0 | >0.2R |
| Max consecutive losses | <6 | <4 |
| Sharpe (annualized on trade series) | >0.5 | >1.0 |
| Gap fill rate in QUIET subset vs. NEWS subset | QUIET > NEWS by >10pp | QUIET > NEWS by >20pp |

### Baseline Comparison
- **Null hypothesis:** Random direction trade at 5:05pm Sunday, same exit rules → should produce ~0 expectancy
- **TA baseline:** Standard gap fill strategy without news/VIX filter → compare to filtered version
- If filtered version does not outperform unfiltered by >10pp win rate, the filter thesis is wrong

---

## Go-Live Criteria

All of the following must be true before moving to paper trading:

1. **Sample size:** ≥30 qualifying trades in the backtest period after all filters applied
2. **Win rate:** ≥58% in the QUIET/LOW_VIX subset (not the full unfiltered dataset)
3. **Positive expectancy:** Expectancy > 0.15R per trade after realistic transaction costs (0.05% per side on Hyperliquid)
4. **News filter validation:** Gap fill rate in QUIET subset must be statistically higher than NEWS subset (p < 0.10 acceptable at hypothesis stage)
5. **No single-year dominance:** Strategy must show positive expectancy in at least 3 of 4 years tested (2021, 2022, 2023, 2024) — if it only works in one regime, it's not structural
6. **Drawdown:** Maximum drawdown on the trade series < 15R (i.e., 15 risk units, equivalent to 7.5% of account at 0.5% risk per trade)

---

## Kill Criteria

### During backtesting
- If unfiltered gap fill rate is <52% for gaps >1.5%, abandon — the base mechanism is not present
- If QUIET and NEWS subsets show identical fill rates, the news filter thesis is wrong — the mechanism is TA, not structural; abandon or reclassify to score 3/10
- If sample size after filtering is <20 trades, insufficient data — park until more history available

### During paper trading (after go-live)
- 10 consecutive paper trades with negative R-multiple → pause and review
- Paper trade expectancy after 20 trades is <0 → kill
- If a structural change occurs (CME changes trading hours, major institutional shift away from CME as reference) → immediate kill and reassess

### Regime kill
- If BTC volatility regime shifts such that >80% of weekends have gaps >1.5% (meaning the threshold is no longer selective), recalibrate threshold or suspend

---

## Risks

### Primary risks

**1. News-driven gaps dominate the sample**
The core risk: if most large weekend gaps are caused by macro news (ETF approval, regulatory action, geopolitical event), hedgers will NOT rebalance to Friday's reference — they will mark their books to the new reality. In this case, the gap fill rate will be low and the strategy loses. Mitigation: news filter is mandatory, not optional.

**2. CME is no longer the primary institutional reference**
As crypto-native institutions grow (Coinbase Prime, Binance institutional, Hyperliquid itself), CME may become less central as a reference price. If the hedger population using CME as their reference shrinks, the rebalancing flow shrinks. This is a secular risk — monitor CME open interest as a % of total BTC futures OI quarterly.

**3. Crowded trade**
If CME gap fill is a well-known pattern, the rebalancing flow may be front-run, causing the gap to fill *before* CME opens (in spot/perp markets) rather than after. Check: does the gap already partially close between Friday 4pm and Sunday 5pm in spot markets? If spot has already mean-reverted 50%+ before CME opens, the trade opportunity is gone.

**4. Liquidity at Sunday 5pm ET**
Sunday evening is low-liquidity on Hyperliquid perps. A 2% position at 3× leverage may move the market on entry. Backtest must use realistic slippage assumptions: assume 0.1% slippage on entry and exit (conservative for a non-HFT firm).

**5. Stop placement**
A 2× gap extension stop may be too wide for small accounts or too tight for volatile regimes. The stop is mechanical (not a trailing stop) — a fast move through the stop level may result in significant slippage beyond the stop price.

### Secondary risks
- **Time zone errors:** CME uses ET. Daylight saving transitions (March and November) shift the CME open by 1 hour relative to UTC. Backtest must handle DST correctly.
- **Contract roll:** CME front-month rolls quarterly. Use continuous contract or handle roll dates explicitly — do not use expiring contract data across roll dates.
- **Funding rate interaction on Hyperliquid:** If the gap trade is short and funding is negative (shorts pay longs), holding through the 3-hour window has a small funding cost. At 3 hours, this is negligible but should be included in cost calculation.

---

## Data Sources

| Data | Source | URL / Endpoint |
|------|--------|----------------|
| CME BTC front-month OHLCV (daily) | Quandl/Nasdaq Data Link | `https://data.nasdaq.com/data/CHRIS/CME_BTC1` |
| CME BTC intraday 1-minute | CME DataMine (paid) | `https://www.cmegroup.com/market-data/datamine-historical-data.html` |
| CME BTC intraday (free proxy) | Barchart.com historical download | `https://www.barchart.com/futures/quotes/BTZ24/historical-download` |
| BTC spot 1-minute (Coinbase) | Coinbase Advanced Trade API | `https://api.exchange.coinbase.com/products/BTC-USD/candles?granularity=60` |
| BTC spot historical (Kaiko, paid) | Kaiko | `https://docs.kaiko.com/` |
| BTC spot historical (free) | Cryptodatadownload | `https://www.cryptodatadownload.com/data/coinbase/` |
| VIX daily close | CBOE via Yahoo Finance | `yfinance` ticker `^VIX` |
| Economic calendar (news filter) | FRED / Investing.com | `https://fred.stlouisfed.org/` |
| Hyperliquid perp data (execution venue) | Hyperliquid API | `https://api.hyperliquid.xyz/info` |

### Data assembly notes
- CME 1-minute data is the preferred source for precise gap measurement. If using Barchart free data, verify the Friday 4:00pm ET close and Sunday 5:00pm ET open timestamps are correct — Barchart sometimes uses exchange local time.
- Coinbase spot can substitute for CME intraday if CME intraday is unavailable — the gap in spot is a reasonable proxy for the CME gap, though not identical.
- For the news filter, build a manual calendar of major crypto/macro events (ETF approvals, FOMC dates, CPI releases, major exchange collapses) and flag weekends where these events occurred. This is manual work — approximately 2–3 hours to build for 2021–2024.
- DST handling: Convert all timestamps to UTC before analysis, then apply ET offset correctly for each date.

---

## Summary Assessment

This strategy has a real structural story but sits at the boundary between structural and pattern-based. The key test is whether the news filter creates a meaningful split in gap fill rates between QUIET and NEWS weekends. If it does, the mechanism is real and the strategy is worth paper trading. If it does not, this is a TA pattern dressed in structural clothing and should be abandoned.

**The backtest is the arbiter. Do not paper trade until the news filter validation is confirmed.**
