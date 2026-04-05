---
title: "Tether Quarterly Attestation Window — USDT Risk Premium Pre-Announcement Compression"
status: HYPOTHESIS
mechanism: 3
implementation: 2
safety: 6
frequency: 2
composite: 72
categories:
  - stablecoin
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## 1. Hypothesis

Tether's quarterly reserve attestations create a predictable uncertainty cycle. In the weeks before each attestation, the market prices a small but measurable risk premium into USDT — visible as a discount to USDC on Curve and OTC markets — because no new reserve data has arrived since the prior attestation. When an attestation is published and confirms stable or improved reserves, this uncertainty premium compresses mechanically toward zero. The trade captures the spread between "uncertainty priced" and "uncertainty resolved."

There are two sub-strategies:

- **Sub-strategy A (Post-release compression):** Enter long USDT/USDC immediately after attestation publication if reserves are confirmed stable or improved. Exit within 48–72 hours as the discount compresses to par.
- **Sub-strategy B (Pre-release discount capture):** Enter long USDT/USDC when the discount exceeds a threshold (5bp) in the 2-week window before expected attestation. Hold through release. Exit on compression or stop-loss if attestation reveals problems.

The edge is **behavioural-structural**: the uncertainty cycle is structural (attestations are periodic, not continuous), but the price response is behavioural (markets price uncertainty, then reprice on resolution). This is why the score is 5/10 rather than 7+. The mechanism is real; the magnitude and consistency are unproven.

---

## 2. Structural Mechanism

### Why the discount exists

USDT is the largest stablecoin by market cap (~$100B+ as of 2025). Unlike USDC, which publishes monthly attestations with Circle's full reserve breakdown, Tether publishes quarterly attestations through BDO Italia. Between attestations, the market has no new verified data on reserve composition. This creates a recurring information vacuum.

During this vacuum:
- Arbitrageurs and large holders face unquantifiable tail risk (what if the next attestation reveals problems?)
- Risk-averse actors prefer USDC, creating mild but measurable selling pressure on USDT
- The Curve 3pool ratio drifts slightly USDT-heavy (more USDT than USDC in the pool = USDT trading at a discount)

### Why the discount compresses after attestation

When Tether publishes an attestation confirming reserves:
- The tail risk that was being priced is resolved for another quarter
- Arbitrageurs who were holding USDC as a hedge rotate back to USDT (higher yield in DeFi, wider liquidity)
- The Curve pool rebalances as USDT demand recovers
- OTC desks tighten their USDT/USDC spreads

This is not a guaranteed mechanical convergence (no smart contract forces it), but it is a **structurally motivated** convergence. The uncertainty that caused the discount is removed by a discrete, public event.

### Why this is not fully priced

1. Attestation release dates are irregular — no precise calendar exists, making systematic positioning difficult for most participants
2. The discount is small (typically 2–10bp), below the threshold of interest for large funds
3. The trade requires active monitoring of Tether's website and Curve pool ratios simultaneously
4. Most quant funds avoid USDT-specific risk entirely, leaving the trade to a narrow set of participants

---

## 3. Market Structure

| Parameter | Detail |
|---|---|
| Primary venue | Curve Finance 3pool (USDT/USDC/DAI) |
| Secondary venue | Curve USDT/USDC stableswap pool, OTC desks |
| Instrument | USDT/USDC spot ratio |
| Typical spread | 2–10bp discount on USDT pre-attestation |
| Target compression | Return to par (0bp) or slight premium |
| Holding period | 48–72 hours (Sub-A), up to 2 weeks (Sub-B) |
| Capital efficiency | High — stablecoin-to-stablecoin, minimal margin required |

---

## 4. Entry Rules

### Sub-strategy A: Post-release compression trade

**Trigger conditions (ALL must be met):**
1. Tether publishes a new quarterly attestation on tether.to
2. Attestation confirms: (a) total reserves ≥ total liabilities, (b) cash + cash equivalents + T-bills ≥ 85% of reserves, (c) no material adverse findings noted by BDO
3. USDT is trading at a discount ≥ 3bp to USDC on Curve 3pool at time of publication
4. Entry within 2 hours of attestation publication (before compression begins in earnest)

**Entry mechanics:**
- Swap USDC → USDT on Curve 3pool
- Size: up to position limit (see Section 6)
- Record entry price (USDT/USDC ratio at execution)

### Sub-strategy B: Pre-release discount capture

**Trigger conditions (ALL must be met):**
1. We are within 14 calendar days of the expected attestation window (based on prior-year cadence — see Section 8 for calendar)
2. USDT discount to USDC on Curve 3pool exceeds 5bp (i.e., 1 USDT buys < 0.9995 USDC)
3. No negative Tether news in prior 7 days (regulatory action, banking partner issues, major redemption spike)
4. Prior attestation was published within the last 120 days (confirms we are in a normal cycle, not a delayed/disrupted cycle)

**Entry mechanics:**
- Swap USDC → USDT on Curve 3pool
- Scale in: 50% at first trigger, 50% if discount widens further to 8bp+
- Record entry price and expected attestation window

---

## 5. Exit Rules

### Sub-strategy A exits

| Condition | Action |
|---|---|
| USDT/USDC spread compresses to ≤ 1bp | Exit full position (target hit) |
| 72 hours elapsed since entry | Exit full position regardless of spread |
| USDT discount widens > 15bp post-attestation | Exit immediately — market is rejecting the attestation |
| Tether issues correction or addendum to attestation | Exit immediately |

### Sub-strategy B exits

| Condition | Action |
|---|---|
| Attestation published, confirms stable reserves | Hold through compression, then follow Sub-A exit rules |
| Attestation published, reveals material problems | Exit immediately at market — this is the tail risk event |
| Attestation delayed > 30 days past expected window | Exit 50% immediately; reassess |
| USDT discount widens > 20bp without attestation | Exit full position — market is pricing something we don't know |
| 21 calendar days elapsed with no attestation | Exit full position |

---

## 6. Position Sizing

### Rationale

This is a low-yield, low-risk trade in normal conditions with a fat left tail (Tether failure). Position sizing must reflect the asymmetry: small gains in base case, potentially large losses in tail case.

### Sizing framework

```
Max position = min(
    [Portfolio NAV × 5%],
    [Liquidity limit: 0.5% of Curve 3pool depth at entry]
)
```

**Example:**
- Portfolio NAV: $1,000,000
- 5% cap = $50,000
- Curve 3pool depth (USDT side): ~$200M → 0.5% = $1,000,000
- Binding constraint: $50,000

**Expected P&L per trade (Sub-A):**
- Entry discount: 5bp
- Exit at par: 5bp gain
- On $50,000: ~$25 gross
- Less gas/slippage (~$5–15 on Curve): ~$10–20 net

**This is a small-dollar trade.** The value is in proving the mechanism exists and building toward larger sizing if the edge is confirmed. Do not over-size chasing yield on a stablecoin trade.

### Tail risk sizing check

Before each Sub-B entry, ask: "If USDT goes to $0.90 (a severe but not unprecedented stablecoin stress event), what is the loss?"
- $50,000 position × 10% loss = $5,000
- This must be acceptable given portfolio context

---

## 7. Backtest Methodology

### Dataset construction

**Step 1: Build the attestation calendar**

Collect all Tether attestation publication dates from 2021–present from:
- tether.to/transparency (archived versions via Wayback Machine)
- BDO Italia press releases
- Crypto media coverage (The Block, CoinDesk) for exact publication timestamps

Expected data points: ~12–16 attestations (quarterly since 2021)

**Step 2: Collect USDT/USDC price data**

For each attestation date, collect:
- Curve 3pool USDT/USDC ratio: 30 days pre-attestation through 7 days post-attestation
- Source: The Graph (Curve subgraph), Dune Analytics, or CoinMetrics
- Granularity: hourly

**Step 3: Collect OTC/CEX spread data**

- Kaiko stablecoin spread data (USDT/USDC on major CEXs)
- CoinMetrics reference rates
- This provides a cross-check on Curve data

**Step 4: Identify control periods**

For each attestation window, identify a matched control period (same calendar period, prior year, no attestation) to distinguish attestation-driven effects from seasonal or market-wide stablecoin dynamics.

### Metrics to compute

For each attestation event:

| Metric | Definition |
|---|---|
| Pre-window discount | Average USDT discount in days -14 to -1 |
| Peak pre-window discount | Maximum USDT discount in days -14 to -1 |
| Post-release T+0 discount | USDT discount at attestation publication hour |
| Post-release T+48h discount | USDT discount 48 hours after publication |
| Compression magnitude | (T+0 discount) - (T+48h discount) |
| Compression speed | Hours to reach ≤ 1bp from publication |
| Sub-A gross return | Compression magnitude × position size |
| Sub-B gross return | (Entry discount - exit discount) × position size |

### Statistical tests

1. **Is the pre-window discount real?** t-test: mean USDT discount in -14 to -1 window vs. matched control periods. H0: no difference.
2. **Does compression occur post-attestation?** Paired t-test: T+0 discount vs. T+48h discount across all attestation events. H0: no compression.
3. **Is the effect consistent?** Count: in how many of N attestation events did compression of ≥ 3bp occur within 72 hours?

### Honest limitations of the backtest

- **N is small:** ~12–16 events since 2021. Statistical power is low. Results will be directional, not definitive.
- **Survivorship bias:** We are backtesting a world where Tether survived. The tail risk (Tether failure) has not occurred in sample.
- **Curve pool depth has changed:** 2021 pool dynamics differ from 2024. Normalize by pool depth.
- **Gas costs matter:** In 2021–2022, Ethereum gas costs could exceed the spread entirely. Account for this in historical P&L.

---

## 8. Attestation Calendar (Historical Reference)

*To be verified against primary sources during backtest construction.*

| Period | Expected Release Window | Notes |
|---|---|---|
| Q4 2020 | ~Q1 2021 | Early BDO attestations, irregular |
| Q1 2021 | ~May 2021 | |
| Q2 2021 | ~August 2021 | |
| Q3 2021 | ~November 2021 | |
| Q4 2021 | ~February 2022 | |
| … | … | Continue through 2025 |

**Key observation:** Release dates have historically varied by 2–6 weeks from the "expected" quarter-end + 6-week lag. This fuzziness is part of why the trade is difficult to systematize and why it scores 5/10.

---

## 9. Go-Live Criteria

Before moving to paper trading, the backtest must show:

| Criterion | Threshold |
|---|---|
| Compression occurs post-attestation | ≥ 70% of events show ≥ 3bp compression within 72h |
| Pre-window discount is real | Statistically significant (p < 0.10) vs. control periods |
| Sub-A positive in majority of events | ≥ 75% of events profitable after gas |
| No single event loss > 50bp | Confirms tail risk is bounded in sample |
| Sharpe (annualised, Sub-A only) | > 1.0 on gross basis |

Before moving to live trading from paper trading:

| Criterion | Threshold |
|---|---|
| Paper trade ≥ 4 attestation events | Minimum sample |
| Execution slippage < 2bp per trade | Confirms Curve liquidity is sufficient |
| Monitoring system operational | Automated alert on tether.to publication |

---

## 10. Kill Criteria

### Immediate kill (exit all positions, halt strategy)

- Tether attestation reveals reserves < liabilities
- BDO withdraws from engagement or issues qualified opinion
- USDT discount exceeds 50bp on Curve (systemic stress event)
- Tether announces banking partner failure or regulatory seizure
- U.S. or EU regulatory action targeting Tether operations

### Soft kill (pause, review, do not enter new positions)

- Three consecutive attestation events show no compression post-release
- Backtest reveals the pre-window discount is not statistically significant
- Curve 3pool depth falls below $50M (liquidity insufficient for meaningful sizing)
- Attestation cadence becomes irregular (> 6 months between releases)

---

## 11. Risks

### Risk 1: Tether reserve failure (CRITICAL — tail risk)
**Description:** Attestation reveals material reserve shortfall or Tether cannot meet redemptions.
**Probability:** Low in base case; non-zero given historical opacity.
**Impact:** USDT could trade to $0.90 or below in acute stress.
**Mitigation:** Hard position cap (5% NAV), immediate exit trigger on any negative attestation finding, Sub-B stop-loss at 20bp widening.

### Risk 2: Attestation delay (MODERATE)
**Description:** Tether delays attestation beyond expected window, extending the uncertainty period.
**Probability:** Has occurred historically.
**Impact:** Sub-B position bleeds carry cost (opportunity cost of holding USDT vs. USDC yield).
**Mitigation:** 21-day hard exit rule for Sub-B regardless of attestation status.

### Risk 3: Compression doesn't occur (MODERATE)
**Description:** Market has already priced in the attestation result, or the discount was driven by unrelated factors.
**Probability:** Unknown — this is what the backtest must determine.
**Impact:** Sub-A trade is flat or slightly negative after gas.
**Mitigation:** 72-hour hard exit prevents prolonged capital lock-up.

### Risk 4: Curve pool mechanics change (LOW-MODERATE)
**Description:** Curve governance changes 3pool parameters, or USDT is removed from the pool.
**Probability:** Low but non-zero.
**Impact:** Primary execution venue disappears.
**Mitigation:** Monitor Curve governance; identify backup venues (Uniswap v3 USDT/USDC, OTC desks).

### Risk 5: Regulatory action on Curve or DeFi (LOW)
**Description:** Regulatory action restricts access to Curve Finance.
**Impact:** Execution venue unavailable.
**Mitigation:** OTC desk relationships as backup.

### Risk 6: The edge is too small to matter (STRUCTURAL)
**Description:** At 5% NAV cap and 5bp compression, gross P&L per trade is ~$25 on a $1M portfolio. This is noise.
**Mitigation:** This strategy is primarily a **mechanism validation exercise**. If the edge is confirmed, it justifies larger sizing (10–20% NAV) or building toward a more automated, higher-frequency version. Do not expect meaningful dollar P&L at current sizing until the mechanism is proven.

---

## 12. Data Sources

| Data | Source | Access |
|---|---|---|
| Tether attestation dates | tether.to/transparency | Public; archive via Wayback Machine |
| BDO attestation documents | bdo.it / tether.to | Public PDFs |
| Curve 3pool USDT/USDC ratio | The Graph (Curve subgraph) | Free API |
| Curve pool depth (historical) | Dune Analytics | Free (rate-limited) |
| USDT/USDC CEX spreads | Kaiko, CoinMetrics | Paid; CoinMetrics has free tier |
| USDT reference rate | CoinMetrics, Messari | Free tier available |
| On-chain Tether redemptions | Tether treasury wallet (Etherscan) | Public |

---

## 13. Researcher Notes

**What would make this a 7/10 strategy:**
If the backtest shows that USDT discount reliably exceeds 5bp in the pre-window AND compresses within 48h post-attestation in ≥ 80% of events, the mechanism is strong enough to justify larger sizing and a more automated implementation. The score would rise because the pattern would have a clear structural cause (information vacuum → resolution) with consistent empirical confirmation.

**What would kill this strategy:**
If the pre-window discount is not statistically distinguishable from normal Curve pool noise, the entire hypothesis collapses. The discount may simply reflect random liquidity imbalances, not attestation-driven uncertainty. This is the most likely failure mode.

**Adjacent opportunity worth investigating:**
Tether publishes daily reserve breakdowns (less detailed than attestations) at tether.to/transparency. If these daily updates show material changes in reserve composition (e.g., T-bill % drops sharply), does the Curve pool react? This could be a higher-frequency version of the same mechanism with more data points.

**Honest assessment:**
This strategy is worth building the dataset for. The mechanism is logically coherent. The backtest will either confirm a real edge or cleanly falsify the hypothesis. Either outcome is valuable. Do not allocate real capital until the backtest is complete and the go-live criteria are met.

---

*Next step: Assign to data team to build attestation calendar and pull Curve 3pool historical ratios. Target: backtest results within 3 weeks.*
