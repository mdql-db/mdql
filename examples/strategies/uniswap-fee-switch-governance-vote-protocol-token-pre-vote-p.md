---
title: "UNI Governance Catalyst — Pre-Vote Long / Failed-Vote Short"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 1
composite: 180
categories:
  - governance
  - defi-protocol
created: "2025-07-11T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a binding Uniswap governance vote is scheduled on Tally (on-chain, not just Snapshot temperature check), UNI is mispriced in the 48 hours before the vote closes because:

1. **Pre-vote long:** Market participants who model UNI as a zero-cash-flow governance token have not yet updated their valuation framework. A vote *passing* forces an immediate model revision — UNI becomes a cash-flow-bearing asset. This is a category change, not a marginal price update. The repricing is not gradual; it is step-function because the asset's fundamental classification changes at a discrete moment.

2. **Failed-vote short:** A scheduled vote that fails removes a priced-in optionality premium. Traders who bought the rumour must exit. There is no remaining catalyst to hold the position. The failed catalyst dump is more mechanically reliable than the pass rally because it does not require a new valuation model — it simply removes an existing one.

**Causal chain (pass scenario):**
Binding vote scheduled → market prices in probability-weighted optionality → vote passes → probability collapses to 1.0 → quant funds/DCF models must reprice UNI with a cash-flow yield → institutional buyers who were waiting for confirmation enter → price steps up.

**Causal chain (fail scenario):**
Binding vote scheduled → optionality premium accumulates in price → vote fails → optionality premium = 0 → holders who bought the catalyst have no remaining thesis → forced exits → price steps down.

---

## Structural Mechanism — WHY This Must Happen

This is **not** a pure pattern trade. The mechanism is game-theoretic and model-driven:

**The category-change forcing function:** Financial models for governance tokens and dividend-bearing tokens use different discount rate frameworks. A zero-cash-flow governance token is valued on narrative and optionality. A cash-flow token is valued on yield, comparable to a DeFi bond. No single market participant can bridge this gap continuously — the repricing requires a discrete trigger (the vote passing) that forces model updates simultaneously across participants.

**Analogy precision:** This mirrors a company initiating its first-ever dividend. Academic literature (Asquith & Mullins, 1983; subsequent replications) documents that first-ever dividend initiations produce abnormal returns of 3–5% in the 2-day window around announcement. The mechanism is identical: asset reclassification forces simultaneous model updates. The crypto version is noisier but the forcing function is the same.

**Why the failed-vote short is stronger:** The failed catalyst short does not require a new model — it requires only the *removal* of an existing premium. This is mechanically simpler and historically more reliable in event-driven equity literature. Failed M&A deals, failed FDA approvals, and failed governance votes all share the same mechanic: removal of priced-in optionality.

**Limitation — why this scores 5, not 7+:** The vote outcome is not contractually guaranteed to pass. The edge is in the *asymmetry of repricing speed*, not in a guaranteed outcome. The fee switch has also been discussed publicly for 3+ years, meaning some probability is already priced into UNI's baseline. The sample size of binding on-chain votes (as opposed to Snapshot temperature checks) is small — approximately 3–5 relevant votes in UNI's history.

---

## Entry Rules


### Trigger Conditions (both legs require ALL conditions met)

| Condition | Requirement |
|-----------|-------------|
| Vote type | Must be on-chain binding vote on Tally (not Snapshot) |
| Vote subject | Must directly concern fee switch activation or UNI staking yield |
| Quorum status | Must be on track to meet quorum (check 72h before close) |
| Market regime | BTC 7-day trend must not be >15% down (avoid entering in broad crash) |

### Leg A — Pre-Vote Long

- **Entry:** Open long UNI/USDC perp on Hyperliquid at T-48h before vote close
- **Size:** See position sizing section
- **Exit on pass:** Close 50% of position at T+4h post-result; close remaining 50% at T+72h
- **Exit on fail:** Immediate market close within 15 minutes of result confirmation on Tally
- **Exit on no-result/delay:** Close at vote's originally scheduled close time regardless of outcome if vote is extended or withdrawn

### Leg B — Failed-Vote Short (independent trade, not a hedge)

- **Entry:** Open short UNI/USDC perp on Hyperliquid at T-48h before vote close (same entry as Leg A, but this is a separate decision)
- **Execution:** This leg is only activated if Leg A is closed on failure. Do not hold both simultaneously.
- **Alternatively:** Enter short *after* confirmed failure result, accepting worse fill in exchange for certainty. This is the conservative version.
- **Exit:** Cover short at T+24h post-failure or when price stabilises (defined as <1% move in 4h candle), whichever comes first

### Practical note on simultaneity

Do not run Leg A and Leg B as a straddle. The pre-vote long and the failed-vote short are sequential, not simultaneous. If vote passes, Leg A captures the upside. If vote fails, close Leg A immediately and optionally open Leg B.

---

## Exit Rules

Defined within Entry Rules section.
## Position Sizing

- **Base allocation:** 2% of portfolio per trade (both legs independently)
- **Leverage:** Maximum 3x on Hyperliquid perp. Do not use higher leverage — vote timing and outcome are uncertain, and liquidation risk on a delayed/modified vote is real
- **Stop-loss:** Hard stop at -8% from entry on Leg A (pre-vote long). Rationale: if UNI drops 8% before the vote closes, either the market has information we don't, or broader market conditions have changed materially
- **No stop on Leg B (failed-vote short):** Exit is time-based (T+24h), not stop-based, because the short thesis is immediate and time-bounded
- **Kelly sizing note:** With a sample size of <10 events, do not attempt Kelly sizing. Use fixed fractional (2%) until sample size justifies parameter estimation

---

## Backtest Methodology

### Data Sources

| Data | Source | URL/Endpoint |
|------|--------|--------------|
| UNI price history (hourly OHLCV) | Binance public API | `https://api.binance.com/api/v3/klines?symbol=UNIUSDT&interval=1h` |
| Governance vote history | Tally API | `https://api.tally.xyz/query` (GraphQL, free tier) |
| Snapshot vote history | Snapshot API | `https://hub.snapshot.org/graphql` |
| Vote timestamps | Tally on-chain data | Cross-reference with Etherscan for block timestamps |
| BTC price (regime filter) | Binance public API | Same endpoint, BTCUSDT |

### Event Universe Construction

1. Pull all Uniswap governance proposals from Tally from 2020-present
2. Filter to: binding on-chain votes only (exclude Snapshot temperature checks)
3. Further filter to: proposals directly related to fee switch, protocol fee, or UNI staking yield
4. Record for each event: vote open timestamp, vote close timestamp, outcome (pass/fail), quorum met (Y/N)
5. Expected universe size: 5–15 events. This is a small sample — acknowledge this explicitly in results.

### Price Window Construction

For each event:
- T-168h to T-48h: pre-entry baseline (measure drift before entry)
- T-48h: entry point
- T+0h: vote close (result)
- T+72h: exit point for pass scenario
- T+24h: exit point for fail scenario

Calculate for each event:
- Return from T-48h to T+0h (pre-vote drift)
- Return from T+0h to T+72h (post-pass drift) or T+0h to T+24h (post-fail drift)
- BTC return over same windows (benchmark)
- UNI excess return vs BTC

### Metrics to Report

| Metric | Target | Minimum acceptable |
|--------|--------|--------------------|
| Win rate (pass scenario) | >55% | >45% |
| Win rate (fail scenario) | >65% | >55% |
| Average return per trade (pass) | >3% | >1.5% |
| Average return per trade (fail) | >4% | >2% |
| Max drawdown per trade | <10% | <15% |
| Sharpe (annualised, if sample allows) | >1.0 | >0.5 |
| UNI excess return vs BTC | Positive | Positive |

### Baseline Comparison

Compare all returns against:
1. Random 48h UNI long (same duration, random entry dates) — tests whether the vote window adds alpha vs. noise
2. BTC return over same windows — tests whether this is just beta
3. UNI buy-and-hold return over the full period — tests opportunity cost

### Confounds to Control For

- **Bull/bear market regime:** Separate results by BTC trend (up/flat/down)
- **Vote size/quorum:** Did votes with higher participation produce larger moves?
- **Pre-announcement drift:** Was the move already captured before T-48h? If so, move entry earlier
- **Time-of-day effects:** Votes closing during US hours vs. off-hours

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **Minimum 5 events** in the event universe (if fewer, flag as insufficient sample and do not proceed)
2. **Failed-vote short win rate ≥ 60%** with average return ≥ 2% per trade
3. **Pre-vote long excess return vs. random entry is positive** (even if small)
4. **No single event produces a loss > 15%** (tail risk check)
5. **BTC regime filter demonstrably improves results** (if it doesn't, remove it — don't overfit)
6. **Results are not entirely explained by 1–2 outlier events** — remove the best and worst event and check if the strategy still has positive expectancy

If the sample is 5–8 events, proceed to paper trading with **half position size** and explicit acknowledgement that results are statistically fragile.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

| Trigger | Action |
|---------|--------|
| Backtest shows <5 qualifying events | Archive — insufficient sample, revisit if more votes occur |
| Failed-vote short win rate <50% in backtest | Kill — the core mechanical thesis is not supported |
| Pre-vote long underperforms random entry by >2% | Downgrade to failed-vote short only; remove Leg A |
| Paper trading: 3 consecutive losses | Pause and re-examine; do not continue without review |
| Paper trading: single loss >12% | Kill immediately; position sizing or stop logic is wrong |
| Fee switch is permanently shelved by governance | Archive — the catalyst no longer exists |
| Fee switch is already activated | Archive — the category-change event has already occurred; no future edge |

---

## Risks

### High-severity risks

**1. The fee switch is already priced in.**
UNI has been trading with fee switch optionality for 3+ years. If the market has already assigned a 30–40% probability to eventual activation, a passing vote may produce only a small incremental move. This is the single biggest risk to Leg A. *Mitigation: backtest will reveal whether post-pass returns are statistically distinguishable from noise.*

**2. Small sample size makes all statistics unreliable.**
With 5–10 events, a 60% win rate could easily be 40% in reality. Every metric from the backtest should be treated as a directional signal, not a precise estimate. *Mitigation: use conservative position sizing (2%, 3x max) until live sample grows.*

**3. Vote timing is unpredictable.**
Votes can be extended, cancelled, or modified mid-vote. A position entered at T-48h may be stranded if the vote is withdrawn. *Mitigation: hard exit rule — close at originally scheduled close time regardless of vote status.*

**4. Governance meta-game.**
Large UNI holders (a16z, Hayden Adams, etc.) may vote at the last minute, creating a false read on outcome probability during the T-48h window. *Mitigation: do not attempt to predict outcome; the trade is on the repricing event, not on predicting the vote.*

### Medium-severity risks

**5. Liquidity on Hyperliquid UNI perp.**
UNI perp on Hyperliquid may have thin order books. Check open interest and 24h volume before entry. If bid-ask spread >0.3% at entry size, skip the trade. *Threshold: minimum $500k 24h volume on UNI perp before entry.*

**6. Funding rate drag.**
If UNI perp is in heavy contango pre-vote (market already long), funding costs may erode returns on Leg A. *Mitigation: check funding rate at entry. If annualised funding >50%, reduce size by 50%.*

**7. Correlation with broader DeFi sentiment.**
UNI moves with DeFi sector. A macro DeFi event during the vote window could swamp the governance signal entirely. *Mitigation: BTC regime filter partially addresses this; no full mitigation available.*

### Low-severity risks

**8. Regulatory risk on fee switch itself.**
If the fee switch is interpreted as creating a security, UNI could face regulatory pressure post-activation. This is a tail risk that would invert the pass scenario. *Mitigation: monitor regulatory environment; this risk is currently low but non-zero.*

---

## Data Sources

| Resource | URL | Notes |
|----------|-----|-------|
| Tally governance (Uniswap) | `https://www.tally.xyz/gov/uniswap` | Browse all proposals; API at `https://api.tally.xyz/query` |
| Tally API docs | `https://docs.tally.xyz/` | GraphQL schema for proposal queries |
| Snapshot (Uniswap space) | `https://snapshot.org/#/uniswap` | Temperature checks only — do not use as entry trigger |
| Snapshot API | `https://hub.snapshot.org/graphql` | Historical vote data |
| UNI price history | `https://api.binance.com/api/v3/klines?symbol=UNIUSDT&interval=1h&limit=1000` | Free, no auth required |
| Hyperliquid UNI perp | `https://app.hyperliquid.xyz/trade/UNI` | Check OI and funding before entry |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | Historical funding rates, OI |
| Etherscan (vote block timestamps) | `https://etherscan.io/` | Cross-reference Tally timestamps with on-chain data |
| Uniswap governance forum | `https://gov.uniswap.org/` | Early warning on upcoming votes; monitor for fee switch proposals |

---

## Implementation Notes

**Monitoring setup:** Set a Tally webhook or daily cron job to check for new Uniswap governance proposals. Alert when a proposal matching keywords ["fee switch", "protocol fee", "fee tier", "UNI staking"] moves to on-chain vote status.

**Manual override required:** This strategy requires human review before each trade. Do not automate entry without a human confirming: (a) vote is binding on-chain, (b) subject matter is directly fee-switch related, (c) quorum is on track, (d) no extraordinary market conditions exist. The sample size is too small to trust automated execution.

**Next step:** Build the event universe from Tally historical data. If fewer than 5 qualifying events exist, this strategy cannot be backtested meaningfully and should be placed in a monitoring queue pending future votes.
