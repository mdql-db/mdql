---
title: "FOMC Funding Rate Spike and Decay"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 7
frequency: 1
composite: 168
categories:
  - funding-rates
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

On FOMC announcement days (8 per year, 2:00pm ET), retail traders pile into BTC perpetual long positions in the 30–60 minutes preceding the announcement, anticipating a dovish surprise. This mechanical buying pressure causes the perp funding rate to spike above its equilibrium level. When the announcement is neutral or hawkish — or when the initial reaction fades — these leveraged retail longs unwind over the subsequent 2–4 hours, causing funding to decay back toward equilibrium (and potentially overshoot negative).

**Causal chain:**

1. FOMC date is known weeks in advance → retail attention concentrates on BTC perps as a macro proxy
2. Retail longs enter perp market 30–60 min before 2pm ET → open interest skews long → funding rate spikes positive (>0.05%/8h)
3. Announcement arrives → if not dovish, longs exit or get liquidated → funding rate decays toward 0 or goes negative
4. Hyperliquid settles funding every 1 hour → the 2pm and 3pm ET settlement periods capture the spike; the 4pm–5pm periods capture the decay
5. A delta-neutral position (long spot BTC + short BTC perp) entered at 2:15pm ET collects the elevated funding rate during the decay window without directional BTC exposure

The edge is **not** predicting the FOMC outcome. The edge is that the funding rate at 2:15pm ET is temporarily elevated above equilibrium due to a predictable, time-stamped retail behavior pattern, and mean-reversion of that rate is mechanical once the catalyst has passed.

---

## Structural Mechanism

**Why this might happen (the plumbing):**

Hyperliquid perpetual funding is calculated continuously and settled every 1 hour. The funding rate is derived from the premium of the perp price over the spot index price. When retail longs pile in, perp price trades above spot → premium rises → funding rate rises. This is a direct mechanical relationship: funding = f(perp premium), and perp premium = f(net long/short imbalance).

**Why this is NOT fully structural (honest assessment):**

Unlike a token unlock (contractually guaranteed supply), the retail pile-in before FOMC is a behavioral tendency, not a guaranteed mechanical event. The funding spike:
- Does NOT occur if the meeting is fully priced in and vol is suppressed
- Does NOT occur if retail attention is elsewhere (competing narratives)
- May be smaller post-2023 as crypto-macro correlation has weakened

The more structural sub-element: **if** a funding spike above 0.05%/8h is observed at 2:15pm ET, the decay back toward equilibrium is more mechanical — elevated funding creates arbitrage pressure from basis traders who will short perp / long spot to collect it, compressing the premium. This compression is the tradeable event.

**Score rationale:** 5/10 because the trigger (retail pile-in) is probabilistic, but the conditional trade (funding decay after observed spike) has a real mechanical basis. The strategy only activates when the spike is confirmed, which filters out non-events.

---

## Entry Rules


### Pre-conditions (all must be true to enter)
1. Date is an FOMC announcement day (Fed calendar)
2. At 2:15pm ET, BTC-USDC perp funding rate on Hyperliquid is **≥ 0.05% per 8 hours** (annualized: ≥ 21.9%)
3. BTC perp open interest has increased by **≥ 5%** in the 60 minutes prior to 2pm ET (confirms retail pile-in, not just vol spike)
4. No active BTC position already held from a prior strategy (avoid interference)

### Entry
- **Time:** 2:15pm ET on qualifying FOMC days (15 minutes after announcement to let initial chaos settle)
- **Position:** Long spot BTC + Short BTC-USDC perp on Hyperliquid in equal notional size
- **Notional size:** Per position sizing rules below
- **Record:** Funding rate at entry, OI at entry, BTC spot price at entry

## Exit Rules

### Exit — first condition met triggers exit
1. **Time exit:** 4:30pm ET (2h15m window — captures 2 full hourly funding settlements)
2. **Funding decay exit:** Funding rate drops below **0.01%/8h** (equilibrium restored, edge gone)
3. **Directional stop:** If delta-neutral position loses >1.5% on net (accounting for spot/perp divergence from liquidation cascades or extreme moves), exit both legs
4. **Funding overshoot exit:** If funding goes negative (< -0.02%/8h), close perp leg only, hold spot or exit both

### Funding collection mechanics
- On Hyperliquid, funding is paid/received every hour at the top of the hour
- Entering at 2:15pm ET means first settlement collected at 3:00pm ET, second at 4:00pm ET
- Maximum 2 settlements collected in the standard window; exit at 4:30pm captures both

---

## Position Sizing

- **Base position:** 2% of total portfolio notional per trade
- **Maximum:** 4% if funding rate at entry is ≥ 0.10%/8h (double the threshold — stronger signal)
- **Leverage:** Spot leg is unleveraged (or 1x). Perp leg: 1x–2x maximum. Keep delta as close to zero as possible.
- **Delta management:** If BTC moves >3% between entry and exit, rebalance spot/perp legs to restore delta neutrality (or exit)
- **Rationale:** This is a low-frequency (8x/year max), conditional trade. Small size is appropriate given the probabilistic trigger. The funding collected at 0.05%/8h on a 2% position over 2 settlements = ~0.025% gross per trade — this is a **proof-of-concept** trade, not a return driver. Sizing up requires confirmed edge from backtest.

---

## Backtest Methodology

### Data required

| Dataset | Source | URL/Endpoint |
|---|---|---|
| Hyperliquid BTC funding rate history (hourly) | Hyperliquid API | `https://api.hyperliquid.xyz/info` → `fundingHistory` endpoint |
| Hyperliquid BTC perp open interest (hourly) | Hyperliquid API | `https://api.hyperliquid.xyz/info` → `openInterest` in meta endpoint |
| FOMC meeting dates and times | Federal Reserve | `https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm` |
| BTC spot price (hourly OHLCV) | Binance or Coinbase API | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1h` |

**Note:** Hyperliquid launched in late 2023. Full funding history may only cover ~16 FOMC meetings (Nov 2023 – present as of Jan 2025). This is a **small sample** — treat backtest results as directional, not statistically conclusive.

For pre-Hyperliquid data (2022–2023), use Binance BTCUSDT perp funding rate as a proxy:
- Binance funding endpoint: `https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT`
- Binance settles every 8 hours (not 1 hour like Hyperliquid) — adjust methodology accordingly

### Backtest procedure

**Step 1: Build FOMC event table**
- List all FOMC announcement dates 2022–2025 with exact announcement times (usually 2:00pm ET, occasionally 2:30pm)
- Mark as "scheduled" vs "emergency" (exclude emergency meetings — different dynamics)

**Step 2: Extract funding rate snapshots**
For each FOMC date, extract:
- Funding rate at 1:00pm ET (pre-event baseline)
- Funding rate at 2:15pm ET (entry signal check)
- Funding rate at 3:00pm ET (first settlement)
- Funding rate at 4:00pm ET (second settlement)
- Funding rate at 5:00pm ET (post-decay check)

**Step 3: Filter to qualifying trades**
- Apply entry filter: funding at 2:15pm ET ≥ 0.05%/8h
- Count: how many of the ~24 FOMC meetings in 2022–2024 qualify?
- If fewer than 8 qualify, the strategy is too infrequent to be meaningful

**Step 4: Simulate P&L**
For each qualifying trade:
- Gross funding collected = sum of hourly funding rates from 3pm to exit time
- Directional P&L = (perp exit price - perp entry price) × -1 + (spot exit price - spot entry price) × +1 (should be near zero if delta-neutral)
- Transaction costs = 2 × taker fee (entry) + 2 × taker fee (exit) = ~0.10% round trip on Hyperliquid (use maker orders where possible to reduce to ~0.04%)
- Net P&L per trade = gross funding - transaction costs ± directional drift

**Step 5: Measure**

| Metric | Target | Kill threshold |
|---|---|---|
| Win rate (net positive trades) | ≥ 60% | < 45% |
| Average net P&L per trade | ≥ 0.05% of notional | < 0% |
| Funding collected vs. transaction cost ratio | ≥ 2:1 | < 1:1 |
| Max single-trade directional loss | < 1.5% | > 3% |
| Sharpe (annualized, 8 trades/year) | N/A — too few trades | — |

**Step 6: Conditional analysis**
- Split results by FOMC outcome: dovish / neutral / hawkish
- Split by whether OI increased ≥5% pre-announcement (OI filter effectiveness)
- Check: does the 0.05% funding threshold actually predict decay, or does funding stay elevated?

### Baseline comparison
- Compare net P&L to simply holding the delta-neutral position on **non-FOMC days** with the same funding threshold (0.05%/8h) — this tests whether FOMC timing adds anything beyond "just trade when funding is high"
- If non-FOMC high-funding days perform equally well, the FOMC-specific timing is irrelevant and the strategy collapses into a generic "short elevated funding" strategy (which is a different, potentially valid strategy but not this one)

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Sample size:** ≥ 8 qualifying FOMC trades in backtest (if fewer qualify, extend to Binance proxy data and adjust for 8h settlement)
2. **Win rate:** ≥ 60% of qualifying trades net positive after transaction costs
3. **Funding > cost:** Average gross funding collected ≥ 2× round-trip transaction cost
4. **OI filter validation:** Trades with OI increase ≥5% pre-FOMC outperform trades without OI confirmation (confirms the filter adds value)
5. **Baseline test:** FOMC-day trades outperform non-FOMC high-funding trades by a statistically meaningful margin (even if not formally significant given small N)
6. **Directional risk check:** No single backtest trade lost more than 2% net (confirms delta-neutral implementation is viable)

**Paper trading period:** 3 FOMC meetings (approximately 4–5 months) before live capital deployment.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Backtest failure:** Fewer than 6 of 8 go-live criteria are met
2. **Low qualification rate:** Fewer than 30% of FOMC meetings trigger the 0.05% funding threshold (strategy too infrequent to be worth the operational overhead)
3. **Baseline equivalence:** Non-FOMC high-funding days produce equal or better results → FOMC timing adds no edge → strategy is just "trade high funding," which is a different (and crowded) strategy
4. **Paper trade failure:** 2 consecutive paper trades result in net losses after funding collection
5. **Market structure change:** Hyperliquid changes funding settlement frequency or mechanism (invalidates the hourly settlement timing logic)
6. **Correlation breakdown:** Post-2024, BTC-macro correlation drops to near zero → retail no longer uses BTC perps as FOMC proxy → trigger mechanism disappears

---

## Risks

### Primary risks (honest assessment)

**1. Trigger doesn't materialize (highest probability risk)**
The entire strategy depends on retail piling into BTC perps before FOMC. If the meeting is fully priced in, or if crypto has decoupled from macro, the funding spike never occurs and no trade is entered. This is not a loss — it's a non-event. But it means the strategy may go months without a trade.

**2. Directional loss swamps funding gain**
If FOMC is unexpectedly dovish and BTC rips 5% in 30 minutes, the delta-neutral position (long spot + short perp) should be approximately flat — but perp basis can temporarily blow out during violent moves, creating a short-term loss on the perp leg before convergence. In extreme cases, the perp leg may be liquidated before the spot leg can be sold. **Mitigation:** Use low leverage (1x–2x max on perp), set hard stop at 1.5% net loss.

**3. Funding collected is too small to cover costs**
At 0.05%/8h, collecting 2 hourly settlements = ~0.0125% gross funding. Round-trip taker fees on Hyperliquid = ~0.10%. The math only works with maker orders (fees ~0.02% each leg = ~0.04% round trip) or if funding spikes higher (0.10%+/8h). **Mitigation:** Use limit orders for entry/exit; only trade when funding ≥ 0.08%/8h in live trading (raise threshold from backtest).

**4. Small sample size**
8 FOMC meetings per year × 2 years of Hyperliquid data = ~16 events maximum, of which maybe 6–10 qualify. This is not enough for statistical significance. Backtest results will have wide confidence intervals. **Mitigation:** Treat backtest as directional signal only; use paper trading to build live sample.

**5. Crowding**
If this pattern is known, basis traders already arb it, compressing the funding spike before 2:15pm ET entry. Check: has the average funding spike on FOMC days decreased over 2023–2024 vs. 2022? If yes, the edge is being arbed away.

**6. Operational complexity**
Requires monitoring funding rates in real-time on FOMC days, executing two simultaneous legs (spot + perp), and managing delta during volatile 2pm–4pm windows. Manual execution risk is non-trivial. **Mitigation:** Build a simple alert system for funding threshold breach; pre-stage orders.

---

## Data Sources

| Resource | URL | Notes |
|---|---|---|
| Hyperliquid funding rate API | `https://api.hyperliquid.xyz/info` | POST with `{"type": "fundingHistory", "coin": "BTC", "startTime": <unix_ms>}` |
| Hyperliquid open interest | `https://api.hyperliquid.xyz/info` | POST with `{"type": "metaAndAssetCtxs"}` — includes OI per asset |
| Binance BTC perp funding (proxy) | `https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=1000` | 8h settlement; use for 2022–2023 pre-Hyperliquid data |
| Binance BTC spot OHLCV | `https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1h` | For directional P&L calculation |
| FOMC calendar (historical) | `https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm` | Includes exact announcement times |
| FOMC calendar (machine-readable) | `https://www.federalreserve.gov/feeds/press_all.xml` | RSS feed; filter for "FOMC" press releases |
| Coinglass funding rate history | `https://www.coinglass.com/FundingRate` | Cross-exchange funding comparison; useful for sanity-checking Hyperliquid data |

---

## Implementation Notes

**Minimum viable backtest code structure:**
```
1. Load FOMC dates → convert to ET timestamps
2. Load Hyperliquid hourly funding rate history for BTC
3. Load Hyperliquid hourly OI history for BTC
4. For each FOMC date:
   a. Check funding at T+15min (2:15pm ET)
   b. Check OI change from T-60min to T (1pm–2pm ET)
   c. If both thresholds met: simulate entry
   d. Collect funding at T+45min, T+105min settlements
   e. Apply exit rules (time, funding decay, stop)
   f. Record gross funding, directional P&L, net P&L
5. Aggregate results; run baseline comparison on non-FOMC high-funding days
6. Output: trade log, win rate, avg P&L, funding/cost ratio
```

**Key implementation decision:** The strategy's viability hinges almost entirely on whether the funding threshold (0.05%/8h) is met frequently enough AND whether the funding collected exceeds transaction costs. Run the cost sensitivity analysis first — if the math doesn't work at 0.05%/8h with maker fees, raise the threshold to 0.10%/8h and recheck qualification rate. If qualification rate drops below 20% of FOMC meetings, the strategy is not viable regardless of win rate.
