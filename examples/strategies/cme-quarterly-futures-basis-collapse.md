---
title: "CME Quarterly Futures Basis Collapse"
status: HYPOTHESIS
mechanism: 9
implementation: 4
safety: 8
frequency: 2
composite: 576
categories:
  - basis-trade
  - calendar-seasonal
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

CME Bitcoin quarterly futures are contractually required to settle at the CME CF Bitcoin Reference Rate (BRR) on expiry day. This means the basis (CME futures price minus BRR-equivalent spot price) **must** converge to exactly zero at settlement — not probabilistically, but by contract. When the annualised basis exceeds 0.5% with 7 days remaining, a short CME future / long spot (or perp) position captures the guaranteed convergence as risk-adjusted carry. The edge is not predictive — it is mechanical. The only question is whether execution costs and capital requirements leave a net positive return.

---

## Structural Mechanism

**Why this MUST happen (not just tends to happen):**

1. CME BTC quarterly futures settle in **cash** to the BRR, published by CF Benchmarks at 4:00 PM London time on the last Friday of March, June, September, and December.
2. The BRR is a volume-weighted average of BTC/USD trades across constituent exchanges (currently Bitstamp, Coinbase, Kraken, itBit, Gemini, LMAX Digital) during the 3:00–4:00 PM London window on settlement day.
3. Any futures price above BRR on settlement day represents a direct cash loss to the long side and gain to the short side — the exchange enforces this via daily mark-to-market and final cash settlement. There is **no delivery optionality** that could prevent convergence.
4. The basis therefore has a hard expiry: it cannot survive past 4:00 PM London on settlement Friday regardless of market conditions, sentiment, or liquidity.
5. The long leg (spot or perp) hedges directional BTC exposure, leaving only the basis as the P&L driver.

**Why the basis exists at all:**
- CME futures carry a convenience yield / funding premium because they are regulated, USD-settled, and accessible to institutions that cannot hold spot BTC.
- Contango is the normal state; backwardation is rare and typically signals extreme spot demand.
- The basis is widest 30–90 days before expiry and compresses as settlement approaches, but it does not always compress smoothly — the final 7 days tend to show accelerated convergence as arbitrageurs close positions and new longs avoid holding into settlement.

---

## Entry Rules

| Parameter | Value |
|---|---|
| Instrument (short leg) | CME BTC quarterly future (front quarterly contract) |
| Instrument (long leg) | BTC spot (Coinbase Pro) OR Hyperliquid BTC-PERP |
| Entry trigger | Annualised basis > 0.5% AND days-to-expiry (DTE) = 7 calendar days |
| Basis calculation | `(CME_futures_price / spot_price - 1) × (365 / DTE) × 100` |
| Entry timing | CME open (9:30 AM Chicago / 3:30 PM London) on the Monday 7 days before expiry Friday |
| Minimum raw basis | > 0.10% absolute (not annualised) to cover fees regardless of DTE |

**Entry checklist (manual gate before live execution):**
- [ ] Confirm settlement date via CME Group calendar (do not rely on calculated date)
- [ ] Confirm BRR constituent exchanges are operational (check CF Benchmarks status page)
- [ ] Confirm CME contract has > 500 open interest contracts (liquidity check)
- [ ] Confirm no scheduled hard fork or major protocol event within the settlement window

---

## Exit Rules

| Scenario | Action |
|---|---|
| **Primary exit** | Let CME future expire and cash-settle to BRR on settlement Friday; simultaneously close spot/perp long at 4:00 PM London (during BRR calculation window to minimise tracking error) |
| **Early exit — basis collapses** | If raw basis drops below 0.05% before expiry, close both legs immediately (most of the edge has been captured; remaining carry does not justify open position risk) |
| **Early exit — basis widens** | If raw basis widens beyond 0.40% absolute (i.e., position moves against us by 0.30%), close both legs and investigate cause before re-entering |
| **Stop-loss** | Hard stop at 0.50% absolute basis widening from entry (position has moved structurally wrong; something is broken) |

**Settlement mechanics note:** When letting the CME leg expire, the final settlement P&L is credited/debited the next business day. The spot/perp leg must be closed independently. Close the spot/perp leg during the 3:00–4:00 PM London window on settlement Friday to track the BRR as closely as possible — this is the single most important execution timing decision in the strategy.

---

## Position Sizing

**Constraint 1 — CME contract size:**
One CME BTC futures contract = 5 BTC. Minimum position is therefore 5 BTC notional on each leg. This is a large minimum; size accordingly.

**Constraint 2 — Capital allocation:**
- CME initial margin: approximately $50,000–$80,000 per contract (check CME SPAN margin, varies with volatility). Source: [CME Group margin calculator](https://www.cmegroup.com/tools-information/quikstrike/margin-estimator.html)
- Spot/perp long leg: full notional required if spot; ~10–20% if using perp with 5–10x leverage
- Total capital per contract: approximately $130,000–$200,000 at current BTC prices (~$85,000)

**Constraint 3 — Basis dollar value:**
At 0.5% annualised basis with 7 DTE, the raw basis ≈ `0.5% × 7/365 ≈ 0.0096%` absolute, or roughly **$8 per BTC** at $85,000 BTC. Per contract (5 BTC): **~$40 gross P&L**. This is extremely thin. The strategy only becomes meaningful at higher absolute basis levels or when scaling to multiple contracts.

**Practical sizing rule:**
- Only enter if expected gross P&L per contract > $150 (i.e., raw basis > ~0.035% absolute at current prices)
- Maximum allocation: 20% of total portfolio notional per expiry cycle
- Scale linearly with basis magnitude: larger basis → larger position, capped at 20%

**Fee budget:**
| Cost item | Estimate |
|---|---|
| CME futures commission | $5–$10 per contract per side |
| Spot/perp taker fee | 0.05–0.10% of notional |
| Perp funding (if using perp long) | Variable; check 7-day average before entry |
| Total round-trip cost estimate | ~$100–$200 per contract |

**Minimum viable basis:** Raw basis must exceed total fees by at least 2x. At current prices, this means raw basis > ~0.05% absolute before entering.

---

## Backtest Methodology

**Data sources:**
- CME BTC quarterly futures OHLCV: [Quandl/Nasdaq Data Link — CME BTC futures](https://data.nasdaq.com/data/CHRIS/CME_BTC1) or [Barchart historical data](https://www.barchart.com/futures/quotes/BTZ25/historical-download)
- BRR historical values: [CF Benchmarks BRR data](https://www.cfbenchmarks.com/data/BRR) — free historical download available
- Spot BTC (Coinbase): [Coinbase Pro API historical trades](https://api.exchange.coinbase.com/products/BTC-USD/candles) or [Kaiko](https://www.kaiko.com)
- Hyperliquid perp funding history: [Hyperliquid public API](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api)

**Backtest period:** January 2018 – present (24+ quarterly expiries)

**Backtest steps:**

1. **Build basis time series:** For each quarterly expiry, calculate daily basis = `(CME_settle_price / BRR_equivalent_spot - 1)` for the 30 days preceding expiry.
2. **Identify entry signals:** Flag all instances where basis at T-7 exceeds 0.10% raw (absolute).
3. **Simulate entry:** Record entry basis at CME open on T-7 Monday. Apply realistic slippage: 0.02% on CME leg (bid-ask spread), 0.05% on spot leg.
4. **Simulate exit:** Record exit basis at settlement (should be zero by definition). For early exits, use actual price data.
5. **Calculate net P&L per contract:** `(Entry basis × 5 BTC × BTC_price) - total fees`.
6. **Calculate annualised Sharpe:** Use risk-free rate = 0% (short duration, cash-like).
7. **Stress test:** Identify the 3 worst-performing expiries and investigate cause (was basis negative at settlement? Was there a BRR anomaly?).
8. **Perp funding drag analysis:** For the perp-long variant, subtract actual 7-day cumulative funding from gross P&L for each expiry.

**Key metric targets for hypothesis validation:**
- Win rate > 85% of expiries where entry signal triggered
- Average net P&L per contract > $200
- Maximum drawdown per expiry < $500 per contract
- No expiry where BRR settlement caused a loss > $1,000 per contract

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

- [ ] Backtest covers ≥ 20 quarterly expiries with positive expectancy
- [ ] Net P&L after fees is positive in ≥ 80% of triggered expiries
- [ ] No single expiry loss exceeds 3× average win
- [ ] Perp funding drag is quantified and incorporated into entry threshold
- [ ] CME margin requirements confirmed with broker (Interactive Brokers or direct CME member)
- [ ] Execution SOP written for the T-7 entry and settlement-day exit timing
- [ ] BRR constituent exchange monitoring process established

**Paper trading period:** 2 consecutive quarterly expiries (6 months minimum) before live capital deployment.

---

## Kill Criteria

Abandon or pause the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| Two consecutive expiries with net losses | Pause; full review before next entry |
| CME changes settlement methodology (BRR replacement) | Immediate halt; re-evaluate structural mechanism |
| BRR constituent exchange count drops below 4 | Halt; BRR becomes manipulable |
| Basis at settlement deviates from zero by > 0.05% in any expiry | Investigate immediately; may indicate data or execution error |
| Perp funding rate averages > 0.03%/8hr over 7-day window | Skip that expiry (funding cost exceeds basis) |
| Regulatory change restricting CME/spot arb | Immediate halt |

---

## Risks

### Risk 1 — Basis widening before convergence (PRIMARY RISK)
**Mechanism:** Large institutional buyers can push CME futures premium higher in the 7-day window before expiry, causing mark-to-market losses on the short CME leg before final convergence.
**Mitigation:** Hard stop at 0.50% absolute basis widening. Accept that the structural guarantee only applies at expiry, not intraday.
**Severity:** Medium. The position will always converge at expiry unless CME fails, but interim margin calls are real.

### Risk 2 — BRR manipulation / anomaly
**Mechanism:** The BRR is calculated over a 1-hour window on 6 exchanges. A flash crash or exchange outage during 3:00–4:00 PM London on settlement Friday could cause BRR to deviate significantly from "fair" spot price.
**Historical precedent:** March 2020 COVID crash occurred near a settlement date; basis behaved unusually.
**Mitigation:** Monitor constituent exchange status on settlement day. If > 1 exchange is down during the BRR window, close both legs before 3:00 PM London rather than letting CME settle.
**Severity:** Low probability, high impact.

### Risk 3 — Perp funding drag (if using perp long)
**Mechanism:** If BTC perp funding is strongly positive (longs pay shorts), the 7-day funding cost on the long perp leg can exceed the basis being captured.
**Mitigation:** Calculate expected 7-day funding cost before entry using trailing 7-day average. Only enter if `raw_basis > expected_funding_cost × 1.5`.
**Severity:** Medium. This is the most likely reason a trade is unprofitable.

### Risk 4 — CME margin call / capital inefficiency
**Mechanism:** If BTC price rises sharply, the short CME leg generates unrealised losses requiring additional margin. The spot/perp long generates offsetting gains but these may not be in the same account.
**Mitigation:** Maintain 2× initial margin as buffer in CME account. Pre-fund with excess capital before entry.
**Severity:** Operational risk, not structural. Manageable with proper capital allocation.

### Risk 5 — Execution timing mismatch on settlement day
**Mechanism:** If the spot/perp long is closed at a different time than the BRR calculation window, the hedge is imperfect and residual directional exposure creates P&L noise.
**Mitigation:** Use a TWAP order on the spot/perp leg during the 3:00–4:00 PM London window. This is the single most operationally critical step.
**Severity:** Low-medium. Introduces noise but not structural loss.

### Risk 6 — Regulatory / access risk
**Mechanism:** CME requires a futures account with a registered broker. Some jurisdictions restrict access. Interactive Brokers and tastytrade both offer CME BTC futures access.
**Mitigation:** Confirm broker access before strategy development investment.
**Severity:** Operational. Not a market risk.

---

## Data Sources

| Data | Source | URL | Cost |
|---|---|---|---|
| CME BTC futures prices (historical) | Barchart | https://www.barchart.com/futures/quotes/BTZ25/historical-download | Free (limited) / $99/mo |
| CME BTC futures prices (historical) | Nasdaq Data Link | https://data.nasdaq.com/data/CHRIS/CME_BTC1 | Free tier available |
| BRR historical settlement values | CF Benchmarks | https://www.cfbenchmarks.com/data/BRR | Free download |
| BTC spot (Coinbase) | Coinbase Pro API | https://api.exchange.coinbase.com/products/BTC-USD/candles | Free |
| BTC spot (aggregated) | Kaiko | https://www.kaiko.com | Paid; ~$500/mo |
| Hyperliquid perp funding history | Hyperliquid API | https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api | Free |
| CME contract specs & calendar | CME Group | https://www.cmegroup.com/markets/cryptocurrencies/bitcoin/bitcoin.contractSpecs.html | Free |
| CME margin requirements | CME SPAN calculator | https://www.cmegroup.com/tools-information/quikstrike/margin-estimator.html | Free |
| BRR methodology | CF Benchmarks | https://www.cfbenchmarks.com/indices/BRR | Free |

---

## Open Questions for Backtest Phase

1. **What is the empirical distribution of basis at T-7?** How often does it exceed the 0.10% minimum threshold? If it rarely exceeds the threshold, the strategy has low frequency and high capital cost.
2. **What is the average basis at T-7 vs. T-1?** Is most convergence happening in the last 24 hours, or is it spread across the 7-day window?
3. **Perp vs. spot for the long leg:** Does perp funding drag historically exceed the basis in any expiry cycle? Quantify this.
4. **Is there a better entry DTE?** T-7 is assumed; T-3 or T-1 might offer better risk/reward if basis converges non-linearly.
5. **What happened in March 2020, May 2021, and November 2022 (FTX collapse)?** These are the three most likely stress events for this strategy. Manually inspect each.

---

*Next step: Assign to quant researcher for data pull and backtest implementation. Estimated time: 2–3 days for initial results. Use Python with `pandas` and `ccxt` for data pipeline. Prioritise answering Open Question #1 first — if entry signal triggers < 10 times in 6 years, strategy is not worth pursuing.*
