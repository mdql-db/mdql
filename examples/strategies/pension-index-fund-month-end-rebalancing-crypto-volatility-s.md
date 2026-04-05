---
title: "Crypto Index Fund Rebalancing Flow"
status: HYPOTHESIS
mechanism: 6
implementation: 7
safety: 6
frequency: 4
composite: 1008
categories:
  - index-rebalance
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Public crypto index products (Bitwise 10, CoinDesk 20, and equivalents) rebalance on published schedules using published methodologies. When constituents drift from target weights due to price divergence, the fund **must** transact to restore targets. This is not discretionary — it is contractually mandated by the fund's index methodology. The transactions are predictable in direction and estimable in size days before execution. The market does not fully price this flow because crypto participants do not systematically track index fund AUM or rebalancing calendars.

**The core claim:** For small/mid-cap constituents where estimated rebalancing flow exceeds ~2% of ADV, the mechanical selling or buying creates a measurable price distortion in the T-2 to T+1 window that can be traded against.

**Null hypothesis to disprove:** Rebalancing flows from current crypto index products are too small relative to market depth to create any measurable price impact, and any observed effect is noise.

---

## Structural Mechanism

### Why the flow MUST happen

1. **Index methodology is a legal document.** BITW's methodology specifies rebalancing dates, weight calculation rules, and constituent eligibility. The fund's NAV tracking obligation means deviations from target weights create tracking error — a compliance and marketing liability. Rebalancing is not optional.

2. **Weight drift is deterministic given prices.** If BTC is 30% target weight and BTC has outperformed the basket by 20% over the month, BTC's actual weight is now ~36%. The fund must sell approximately 6% of its BTC position to restore target. This calculation is exact, not estimated, given public price data and the published methodology.

3. **Execution is concentrated.** Unlike a discretionary manager who might spread execution over weeks, index funds typically execute rebalancing within a 1-3 day window around the published rebalancing date to minimise tracking error. This concentrates the flow.

4. **The information is public but ignored.** BITW publishes daily holdings. CoinDesk 20 publishes its methodology and constituent weights. The rebalancing schedule is in the prospectus. No API aggregates this into a tradeable signal — it requires manual assembly, which is why it is uncrowded.

### Flow estimation formula

```
Estimated_Rebalance_Flow_USD(asset_i) =
    (Current_Weight_i - Target_Weight_i) × Total_AUM

Current_Weight_i = (Holdings_i × Price_i) / Σ(Holdings_j × Price_j)
Target_Weight_i  = Published in methodology (often market-cap weighted within rules)

Flow_as_pct_ADV  = |Estimated_Rebalance_Flow_USD| / ADV_30day_USD
```

A positive flow (current > target) means the fund must SELL → short signal.
A negative flow (current < target) means the fund must BUY → long signal.

### Why the market doesn't fully arbitrage this

- **AUM is small in absolute terms** (~$500M BITW), making the signal unattractive to large desks
- **Assembly cost is high** — requires tracking multiple fund methodologies, daily holdings scrapes, and calendar management
- **Crypto-native traders don't read fund prospectuses**
- **The effect is most pronounced in small/mid-cap constituents** where the same dollar flow has larger price impact — these are less monitored by sophisticated players

---

## Universe

### Primary targets (confirmed public methodology + published rebalancing schedule)

| Product | AUM (approx) | Rebalancing Schedule | Methodology Source |
|---|---|---|---|
| Bitwise 10 Crypto Index Fund (BITW) | ~$500M | Monthly, last business day | bitwise.com/funds/bitwise-10 |
| CoinDesk 20 Index (CF Benchmarks) | Index only (no fund AUM yet) | Monthly | coindesk.com/indices |
| Bitwise DeFi Crypto Index Fund | ~$30M | Monthly | bitwise.com |
| Grayscale Diversified Large Cap (GDLC) | ~$500M | Quarterly | grayscale.com |

### Secondary targets (monitor for AUM growth)

- Any new spot Bitcoin ETF products that hold multi-asset baskets
- Crypto 401k products (ForUsAll, etc.) as they scale
- On-chain index products (Index Coop: DPI, MVI) — rebalancing is **on-chain and fully transparent**

### Index Coop special case (score: 7/10 for this sub-strategy)

Index Coop's DeFi Pulse Index (DPI) and Metaverse Index (MVI) rebalance on-chain. Every rebalancing transaction is visible in the mempool before execution. The methodology is a published smart contract. This is a higher-confidence variant — the flow is not just estimable, it is **observable in real time on-chain**. Treat this as a separate sub-strategy with higher priority for backtesting.

---

## Entry Rules


### Signal generation (run daily, T-5 through T-1 before rebalancing date)

```
FOR each constituent asset_i in each tracked index:

    1. Pull current holdings from fund's daily disclosure
    2. Calculate current_weight_i using live prices
    3. Calculate target_weight_i from published methodology
    4. Calculate estimated_flow_USD = (current_weight_i - target_weight_i) × AUM
    5. Calculate flow_pct_ADV = |estimated_flow_USD| / ADV_30day_USD

    IF flow_pct_ADV >= 0.02 (2% of ADV):          # minimum impact threshold
        IF estimated_flow_USD > 0:                  # fund must SELL
            signal = SHORT
        IF estimated_flow_USD < 0:                  # fund must BUY
            signal = LONG
    ELSE:
        signal = SKIP (flow too small to matter)
```

### Entry

- **Entry time:** T-2 (two trading days before published rebalancing date), market open (00:00 UTC for crypto)
- **Entry instrument:** Hyperliquid perpetual futures for the relevant asset
- **Entry type:** Limit order within 0.1% of mid-price; if not filled within 2 hours, use market order
- **Rationale for T-2:** Early enough to capture pre-rebalancing drift as other informed participants front-run; late enough that the weight drift calculation is near-final

## Exit Rules

### Exit

- **Primary exit:** T+1 close (one day after rebalancing date) — rebalancing flow is complete, mean-reversion begins
- **Stop loss:** 3% adverse move from entry (hard stop, no exceptions)
- **Partial exit option:** Exit 50% of position at T+0 close if position is profitable (lock in gains before rebalancing execution risk)
- **Do not hold past T+2:** Any alpha from the flow has decayed; holding longer converts this into a directional bet

### Trade management

```
Entry:      T-2 open
Checkpoint: T-1 close — recalculate flow estimate with updated prices
            If flow estimate has reversed sign (asset moved against drift):
            → Close position immediately (flow may have already been executed early)
Exit:       T+1 close (or stop loss, whichever comes first)
```

---

## Position Sizing

### Base sizing formula

```
Position_Size_USD = min(
    Kelly_fraction × Estimated_Edge_USD,
    Max_Position_Cap_USD
)

Where:
    Kelly_fraction         = 0.25 (quarter-Kelly for hypothesis-stage strategy)
    Estimated_Edge_USD     = flow_pct_ADV × ADV × empirical_price_impact_coefficient
    Max_Position_Cap_USD   = $50,000 per trade (pre-live cap)
    Max_total_exposure     = $200,000 across all open rebalancing trades simultaneously
```

### Scaling by flow significance

| Flow as % of ADV | Position Scale | Rationale |
|---|---|---|
| < 2% | 0 (skip) | Below noise floor |
| 2–5% | 25% of max | Marginal impact |
| 5–15% | 50% of max | Meaningful flow |
| 15–30% | 75% of max | Significant flow |
| > 30% | 100% of max | High-conviction; fund is a major market participant this day |

### Leverage

- Maximum 3x leverage
- Prefer 1–2x for hypothesis stage
- No leverage on assets with < $5M ADV (liquidity risk too high)

---

## Backtest Methodology

### Data requirements

```
Required datasets:
1. BITW daily holdings history (available from Bitwise website, 2020–present)
2. BITW rebalancing dates (from prospectus + press releases)
3. GDLC quarterly rebalancing dates and holdings
4. Daily OHLCV for all BITW/GDLC constituents (CoinGecko, Kaiko, or Tardis)
5. 30-day rolling ADV for each constituent (derived from #4)
6. Index Coop DPI/MVI on-chain rebalancing transactions (Dune Analytics)
```

### Backtest procedure

```
Step 1: Reconstruct historical weight drift
    For each rebalancing event (BITW: ~monthly, 2020–present):
        - Calculate constituent weights at T-5, T-2, T-1 using historical prices
        - Calculate estimated rebalancing flow per constituent
        - Apply flow_pct_ADV filter (>= 2%)
        - Record direction (long/short) and estimated size

Step 2: Simulate trades
    For each qualifying signal:
        - Entry: T-2 open price
        - Exit: T+1 close price (or stop loss at -3%)
        - Apply 0.1% slippage assumption (conservative for mid-caps)
        - Apply 0.05% funding rate per day (Hyperliquid perp cost)

Step 3: Calculate metrics
    - Win rate per direction (long vs. short)
    - Average return per trade
    - Sharpe ratio of trade returns
    - Maximum drawdown
    - Performance stratified by flow_pct_ADV bucket
    - Performance stratified by asset market cap tier

Step 4: Sensitivity analysis
    - Vary entry timing: T-3, T-2, T-1
    - Vary exit timing: T+0, T+1, T+2
    - Vary stop loss: 2%, 3%, 5%
    - Test with and without the 2% ADV filter
```

### Minimum sample size requirement

- BITW has been live since 2020 — approximately 48–60 monthly rebalancing events
- After applying the ADV filter, expect 3–8 qualifying trades per rebalancing event
- Target: minimum 100 qualifying trades before drawing conclusions
- If sample is insufficient from BITW alone, include Index Coop on-chain rebalancing events (Dune Analytics provides full history)

### Expected backtest output format

```
Strategy: Crypto Index Rebalancing Flow
Period: 2020-01-01 to 2025-12-31
Total rebalancing events: N
Qualifying trades (flow > 2% ADV): N
Win rate: X%
Avg return per trade: X% (gross) / X% (net of costs)
Sharpe (trade-level): X
Max drawdown: X%
Best performing bucket: [flow_pct_ADV tier]
Worst performing bucket: [flow_pct_ADV tier]
```

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Backtest win rate ≥ 55%** on net-of-costs basis across ≥ 100 qualifying trades
2. **Backtest Sharpe ≥ 0.8** on trade-level returns
3. **Effect is monotonic with flow size** — larger flow_pct_ADV buckets must show larger average returns (confirms the mechanism, not noise)
4. **Paper trade for 3 full rebalancing cycles** with live signal generation and simulated execution — paper trade P&L must be positive
5. **No single trade exceeds 15% of total strategy drawdown** in paper trading (concentration risk check)
6. **Index Coop on-chain sub-strategy backtests independently** with similar results (cross-validation of mechanism)

---

## Kill Criteria

Abandon or pause the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| 5 consecutive losing trades | Pause, investigate whether AUM has declined or methodology changed |
| Backtest shows no monotonic relationship between flow size and returns | Reject hypothesis — effect is noise |
| BITW AUM drops below $100M | Suspend — flows too small for any impact |
| Index methodology changes to reduce predictability (e.g., randomised execution window) | Immediate suspension |
| Flow_pct_ADV filter never triggers (market depth has grown faster than AUM) | Archive strategy, revisit if AUM grows |
| Paper trade Sharpe < 0.3 over 3 cycles | Do not go live |
| Crypto market structure changes (e.g., major new index ETF launches with much larger AUM) | Re-evaluate — could be opportunity upgrade OR crowding risk |

---

## Risks

### Primary risks

**1. AUM is too small (highest probability failure mode)**
BITW's ~$500M AUM means maximum rebalancing flow is ~$30–50M per event, spread across 10 constituents. For BTC and ETH, this is noise. The strategy only works for smaller constituents where this flow represents meaningful ADV. If the ADV filter eliminates all trades, the strategy is dead on arrival.
*Mitigation:* The ADV filter is the primary defence. Do not trade assets where flow < 2% ADV.

**2. Front-running by other informed participants**
If other desks are running this signal, the alpha is captured before T-2 entry. The price impact occurs at T-5 or T-3 rather than T-2 to T+1.
*Mitigation:* Test multiple entry timings in backtest. If T-3 entry dominates, adjust. If no timing works, the strategy is crowded.

**3. Execution timing uncertainty**
Index funds may execute rebalancing over multiple days or use TWAP algorithms. If execution is spread over T-3 to T+3, the concentrated flow assumption fails.
*Mitigation:* Monitor actual BITW holdings changes day-by-day around rebalancing dates to empirically map execution patterns. This is observable from daily holdings disclosures.

**4. Methodology changes**
Index providers can change rebalancing rules, dates, or execution windows. BITW changed its methodology in 2021.
*Mitigation:* Subscribe to methodology update notifications. Treat any methodology change as a kill trigger pending re-evaluation.

**5. Funding rate drag**
Holding perp positions for 3–5 days incurs funding costs. In high-funding environments, this can erase small edges.
*Mitigation:* Include funding costs explicitly in all P&L calculations. If expected funding cost exceeds 0.5% for the holding period, skip the trade.

**6. Correlation with broader market moves**
If BTC rallies 20% in a month, BITW must sell BTC — but BTC may continue rallying through the rebalancing window, making the short painful. The rebalancing flow is real but may be overwhelmed by directional momentum.
*Mitigation:* The stop loss at -3% is the primary defence. Consider adding a momentum filter: if the asset's 5-day return is strongly positive (> +10%), reduce short position size by 50%.

### Secondary risks

- **Liquidity risk** for small-cap constituents: wide spreads can eliminate edge
- **Delisting risk**: constituent removed from index before rebalancing completes
- **Regulatory risk**: crypto index products face ongoing SEC scrutiny

---

## Data Sources

| Data | Source | Cost | Notes |
|---|---|---|---|
| BITW daily holdings | bitwise.com/funds/bitwise-10/holdings | Free | Scrape daily, store in database |
| BITW rebalancing calendar | BITW prospectus + Bitwise press releases | Free | Manual calendar maintenance |
| GDLC holdings | grayscale.com/products/grayscale-digital-large-cap-fund | Free | Quarterly rebalancing |
| CoinDesk 20 methodology | coindesk.com/indices | Free | Index only, no fund AUM yet |
| Index Coop DPI/MVI rebalancing | Dune Analytics (query: Index Coop rebalances) | Free | On-chain, fully transparent |
| OHLCV historical prices | CoinGecko API (free tier) or Kaiko (paid) | Free/~$500/mo | Need tick data for small caps |
| ADV calculations | Derived from OHLCV | — | 30-day rolling average |
| Hyperliquid perp funding rates | Hyperliquid API | Free | For cost calculations |

### Data pipeline (minimum viable)

```
Daily cron job (runs at 00:30 UTC):
1. Scrape BITW holdings page → store to database
2. Pull CoinGecko prices for all constituents → calculate current weights
3. Compare to target weights from methodology → calculate flow estimates
4. Calculate flow_pct_ADV for each constituent
5. If within T-5 to T-1 of rebalancing date AND flow_pct_ADV >= 2%:
   → Generate signal alert (Telegram/email)
6. Log all calculations for backtest validation
```

---

## Variants and Extensions

### Variant A: Index Coop on-chain rebalancing (score: 7/10)
DPI and MVI rebalancing is executed on-chain via smart contract. The rebalancing transaction can be observed in the mempool. This is a higher-confidence variant because the flow is not estimated — it is observable. The limitation is small AUM (~$20–50M for DPI). **Prioritise this for initial backtesting.**

### Variant B: Multi-index aggregation
If multiple index products rebalance in the same direction on the same asset in the same window, aggregate the flows. Coincident rebalancing from BITW + GDLC + any other product creates a larger combined flow. Track all products simultaneously and sum estimated flows before applying the ADV filter.

### Variant C: AUM growth monitoring
The strategy's value scales with AUM. Monitor for new crypto index ETF approvals (SEC pipeline). A $5B crypto index ETF with monthly rebalancing would make this a 7–8/10 strategy. Set up alerts for new index product filings.

### Variant D: Reverse signal — trade the post-rebalancing mean reversion
After rebalancing completes (T+1), the mechanical selling/buying pressure is gone. Assets that were sold may bounce, assets that were bought may retrace. This is a secondary signal with weaker structural backing but worth testing in the same backtest framework.

---

## Open Questions for Backtest

1. Is there a measurable price impact in the T-2 to T+1 window for constituents where flow_pct_ADV > 5%? (Core hypothesis test)
2. Does the effect scale monotonically with flow_pct_ADV? (Mechanism validation)
3. What is the empirical execution pattern for BITW? (Do holdings changes cluster on the published rebalancing date or spread across multiple days?)
4. Does the Index Coop on-chain variant show a cleaner signal than the BITW off-chain variant? (If yes, prioritise on-chain products)
5. Is the effect stronger for short signals (sell pressure) or long signals (buy pressure)? (Asymmetry check — crypto markets may respond differently to forced selling vs. forced buying)

---

*This document represents a hypothesis. No backtest has been run. Do not allocate capital until go-live criteria are satisfied.*
