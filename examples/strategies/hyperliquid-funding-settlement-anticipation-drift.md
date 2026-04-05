---
title: "Hyperliquid Funding Settlement Anticipation Drift"
status: HYPOTHESIS
mechanism: 5
implementation: 8
safety: 6
frequency: 10
composite: 2400
categories:
  - funding-rates
  - exchange-structure
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Traders paying funding on Hyperliquid perpetuals have a rational, time-bounded incentive to close positions before each 8-hour settlement mark (00:00, 08:00, 16:00 UTC). When funding rates are sufficiently elevated, this creates a measurable, directional price distortion in the 15–30 minutes preceding settlement, followed by partial mean-reversion after the settlement timestamp passes. The distortion is not random — it is mechanically anchored to a fixed, public schedule and scales with the cost of holding through settlement.

**Null hypothesis to disprove:** Price action in the T-30min to T+30min window around HL settlement marks is indistinguishable from random walk when conditioned on elevated funding rates.

---

## Structural Mechanism

### Why this must happen (the causal chain)

1. **Fixed settlement schedule:** Hyperliquid settles funding at exactly 00:00, 08:00, and 16:00 UTC. These times are public, immutable, and known to all participants. There is no ambiguity about when the next funding payment occurs.

2. **Funding is a cash flow, not a price signal:** A trader long BTC-PERP paying 0.10%/8hr on a $100,000 position pays $100 every 8 hours. At T-30min, they face a binary decision: hold and pay $100 in 30 minutes, or close now and avoid that payment. This is not a sentiment decision — it is arithmetic.

3. **Aggregated rational exits create directional flow:** When hundreds of leveraged longs face the same arithmetic simultaneously, their exits aggregate into net selling pressure. The pressure is front-loaded toward the settlement mark because the incentive to exit increases as settlement approaches (time value of the avoided payment rises).

4. **Post-settlement pressure release:** Once the settlement timestamp passes, the funding clock resets to zero. The urgency to exit evaporates. Any traders who closed pre-settlement and wish to re-enter now face no immediate funding cost, creating mild re-entry buying pressure.

5. **Threshold effect:** Below a funding rate threshold (~0.03%/8hr), the dollar cost of holding through settlement is small relative to bid-ask spread and slippage costs of exiting and re-entering. The incentive to exit only becomes dominant when funding is materially elevated. This threshold creates a conditional signal — the effect should only appear when funding exceeds a meaningful level.

### Why this is not fully arbitraged away

- The trade requires precise timing (minute-level execution), which deters manual traders.
- The profit per trade is small in absolute terms, deterring large institutional capital.
- The effect is conditional on funding rate level, making it invisible to researchers who don't filter by funding regime.
- Counterparties who receive funding (the other side) have the opposite incentive — they want to hold through settlement — which partially offsets but does not eliminate the closing pressure from payers.

### Known weaknesses in the mechanism

- Funding receivers (e.g., basis traders short perp, long spot) have an equal and opposite incentive to hold, which dampens the effect.
- If the market knows longs will exit pre-settlement, sophisticated participants may front-run the front-runners, compressing or inverting the signal.
- High-funding environments often coincide with strong directional momentum, which may overwhelm the micro-timing effect.

---

## Market Universe

**Primary markets:** BTC-PERP, ETH-PERP (highest liquidity, tightest spreads, most reliable data).

**Secondary markets (phase 2 only):** SOL-PERP, ARB-PERP, any HL market with >$5M average daily volume.

**Exclusion criteria:**
- Markets with average daily volume <$5M (slippage will consume edge).
- Markets in active liquidation cascade (funding signal is noise during cascades).
- Any market where the funding rate has flipped sign in the prior 8-hour window (unstable regime).

---

## Signal Definition

### Funding rate trigger

Check funding rate at **T-45 minutes** before each settlement mark.

| Condition | Signal |
|---|---|
| Funding rate > +0.07%/8hr | STRONG POSITIVE — expect pre-settlement selling pressure |
| Funding rate +0.04% to +0.07%/8hr | WEAK POSITIVE — reduced size, track only |
| Funding rate -0.04% to +0.04%/8hr | NO TRADE — insufficient incentive |
| Funding rate -0.04% to -0.07%/8hr | WEAK NEGATIVE — reduced size, track only |
| Funding rate < -0.07%/8hr | STRONG NEGATIVE — expect pre-settlement buying pressure |

**Threshold rationale:** 0.07%/8hr annualizes to ~76% APR. At this level, a $50,000 position pays $35 per settlement period. Round-trip transaction costs on HL for a $50,000 trade are approximately $5–10 (0.01–0.02% taker fee each way). The arithmetic incentive to exit is approximately 3–7x the transaction cost, making exit rational for any trader not paying attention to micro-timing.

### Regime filter (mandatory)

Do not trade if any of the following are true at T-45min:
- BTC 1-hour realized volatility > 2% (high vol overwhelms micro-timing signal).
- A major economic data release (CPI, FOMC, NFP) is scheduled within 2 hours of settlement.
- The market has moved >1.5% in the prior 30 minutes in either direction.

---

## Entry and Exit Rules

### Trade A: Pre-Settlement Fade (primary trade)

**Condition:** Strong positive funding (>0.07%/8hr).

| Parameter | Value |
|---|---|
| Entry | Market order at T-30 minutes before settlement |
| Direction | SHORT |
| Exit | Market order at T-5 minutes before settlement |
| Hold time | ~25 minutes |
| Target | 0.15–0.25% price decline from entry |
| Hard stop | 0.30% adverse move from entry price |

**Condition:** Strong negative funding (<-0.07%/8hr).

| Parameter | Value |
|---|---|
| Entry | Market order at T-30 minutes before settlement |
| Direction | LONG |
| Exit | Market order at T-5 minutes before settlement |
| Hold time | ~25 minutes |
| Target | 0.15–0.25% price increase from entry |
| Hard stop | 0.30% adverse move from entry price |

### Trade B: Post-Settlement Recovery (secondary trade)

**Condition:** Strong positive funding (>0.07%/8hr) — enter after settlement passes.

| Parameter | Value |
|---|---|
| Entry | Market order at T+2 minutes after settlement |
| Direction | LONG |
| Exit | Market order at T+30 minutes after settlement |
| Hold time | ~28 minutes |
| Target | 0.10–0.20% price recovery |
| Hard stop | 0.25% adverse move from entry price |

**Note:** Trade B is lower conviction than Trade A. Run Trade B only after Trade A has demonstrated positive expectancy in backtest. Do not run Trade B in isolation.

### Execution notes

- Use **market orders** for entry and exit — this is a timing trade and limit order fill uncertainty destroys the edge.
- Do not enter if the bid-ask spread at entry time exceeds 0.05% (spread is consuming the expected edge).
- Cancel and skip any trade if the regime filter triggers between T-45min check and T-30min entry.

---

## Position Sizing

**Base position size:** 0.5% of total trading capital per trade.

**Rationale:** Expected move is 0.15–0.25%. At 0.5% capital allocation with 1x leverage, a 0.20% move generates 0.10% portfolio return per trade. At 3 settlements/day × 2 markets × ~40% signal frequency, this produces approximately 0.24% daily gross return in favorable conditions — meaningful but not so large that adverse runs cause significant drawdown.

**Leverage:** Maximum 3x. Higher leverage is not warranted given the small expected move and the 0.30% stop.

**Scaling rules:**
- Reduce to 0.25% capital if the strategy has produced 3 consecutive losing trades.
- Increase to 0.75% capital only after 30 live trades with positive expectancy confirmed.
- Never exceed 1% capital on a single trade regardless of signal strength.

**Maximum simultaneous exposure:** 2 trades open at once (e.g., BTC and ETH both triggering at the same settlement mark). Do not open a third position even if a third market triggers.

---

## Backtest Methodology

### Data requirements

| Data type | Source | Granularity |
|---|---|---|
| OHLCV price data | Hyperliquid public API (`/info` endpoint, candles) | 1-minute bars |
| Funding rate history | Hyperliquid public API (`fundingHistory`) | Per settlement period |
| Trade timestamps | Derive from settlement schedule (00:00/08:00/16:00 UTC) | Exact UTC timestamps |

**Minimum backtest period:** 12 months of HL data (HL launched May 2023; use all available data).

**Markets to backtest:** BTC-PERP and ETH-PERP first. Add SOL-PERP in phase 2.

### Backtest procedure

**Step 1 — Build the event table.**
For every settlement mark in the backtest period, record: settlement timestamp, funding rate at T-45min, 1-minute OHLCV for T-60min to T+60min window, BTC 1-hour realized vol at T-45min.

**Step 2 — Apply filters.**
Flag each event as TRADE / NO-TRADE based on funding threshold and regime filters. Record the reason for each NO-TRADE.

**Step 3 — Simulate Trade A.**
For each TRADE event: record entry price at T-30min open, exit price at T-5min open, stop hit (yes/no), gross PnL in percent. Apply 0.02% round-trip transaction cost (taker fee both sides).

**Step 4 — Simulate Trade B.**
For each TRADE event where Trade A was profitable: record entry at T+2min, exit at T+30min, stop hit, gross PnL.

**Step 5 — Stratify results.**
Break results down by: funding rate bucket (0.07–0.10%, 0.10–0.15%, >0.15%), time of day (00:00 vs 08:00 vs 16:00 UTC), market (BTC vs ETH), and market regime (trending vs ranging, defined by 4-hour ATR percentile).

**Step 6 — Statistical validation.**
Compute: win rate, average win/loss ratio, Sharpe ratio, maximum drawdown, profit factor. Run a permutation test — shuffle the settlement timestamps randomly and re-run the backtest 1,000 times to confirm the observed edge exceeds random chance at p < 0.05.

### Backtest red flags (abort if any are true)

- Win rate below 45% on Trade A after transaction costs.
- Average loss > 1.5x average win (negative expectancy even with decent win rate).
- Edge disappears when funding threshold is raised to 0.10% (suggests threshold-sensitivity, not a real effect).
- Permutation test shows p > 0.10 (result is not statistically distinguishable from noise).
- Edge is entirely concentrated in one 3-month period (regime-specific, not structural).

---

## Go-Live Criteria

All of the following must be true before live deployment:

1. **Backtest expectancy:** Net profit factor > 1.3 after transaction costs across full backtest period.
2. **Statistical significance:** Permutation test p < 0.05.
3. **Consistency:** Positive expectancy in at least 3 of 4 calendar quarters in the backtest period.
4. **Paper trade confirmation:** 30 paper trades completed with results within 30% of backtest expectancy (win rate and average PnL both within range).
5. **Spread check:** Median bid-ask spread at entry time < 0.04% for BTC-PERP and ETH-PERP (confirms execution is feasible at expected cost).
6. **Regime robustness:** Edge present in both high-vol and low-vol regimes (defined by monthly realized vol above/below median).

---

## Kill Criteria

**Immediate suspension** (same day):
- 5 consecutive losing trades in live trading.
- Single trade loss exceeds 0.5% of portfolio (indicates position sizing error or extreme slippage).
- Hyperliquid changes its funding settlement schedule or mechanism.

**Review and likely suspension** (within 1 week):
- Rolling 20-trade win rate drops below 40%.
- Rolling 20-trade profit factor drops below 0.9.
- Average slippage on market orders exceeds 0.05% (execution environment has changed).

**Permanent retirement:**
- 60-trade rolling window shows no statistical edge (p > 0.15 on permutation test of live results).
- A credible research paper or public strategy post documents this exact mechanism, suggesting it will be arbitraged away within 1–3 months.

---

## Risks

### Risk 1: The effect is already arbitraged
**Probability:** Medium. This mechanism is discussed in perp trading communities. If enough capital is already positioned to exploit it, the signal is compressed or inverted.
**Mitigation:** The backtest will reveal this — if the effect is gone in recent data (last 3 months) but present in older data, the strategy is dead.

### Risk 2: Funding receivers offset the signal
**Probability:** High. Basis traders (short perp, long spot) receive funding and have equal incentive to hold through settlement. Their inaction partially cancels the closing pressure from payers.
**Mitigation:** This is a magnitude question, not a binary. The backtest measures the net effect. If payers outnumber receivers in dollar terms (common in bull markets), the signal survives.

### Risk 3: Momentum overwhelms micro-timing
**Probability:** Medium-high. High funding environments often coincide with strong directional trends. A 0.30% stop is easily hit in a trending market.
**Mitigation:** The 1.5% prior-30-min move filter and the 2% realized vol filter are designed to exclude trending conditions. Verify in backtest that these filters materially improve Sharpe.

### Risk 4: Execution slippage destroys edge
**Probability:** Medium for smaller HL markets, Low for BTC/ETH.
**Mitigation:** Restrict to BTC-PERP and ETH-PERP until live slippage data confirms smaller markets are viable. Monitor average fill vs. mid-price on every trade.

### Risk 5: Settlement time changes
**Probability:** Low. Hyperliquid's settlement schedule is a core protocol parameter.
**Mitigation:** Monitor HL protocol announcements. Any change to settlement timing or mechanism triggers immediate strategy suspension.

### Risk 6: Crowding at the exact entry time
**Probability:** Medium. If many traders enter at exactly T-30min, the signal is front-run to T-35min or T-40min.
**Mitigation:** Test entry at T-35min and T-40min in backtest as alternative entry points. If the optimal entry has shifted earlier over time, this is evidence of crowding and the strategy should be retired.

---

## Data Sources

| Source | URL / Endpoint | Data type |
|---|---|---|
| Hyperliquid REST API | `https://api.hyperliquid.xyz/info` | Candles, funding history, mark price |
| Hyperliquid funding history | POST `/info` with `{"type": "fundingHistory", "coin": "BTC"}` | Per-period funding rates |
| Hyperliquid candles | POST `/info` with `{"type": "candleSnapshot", ...}` | 1-minute OHLCV |
| Coinglass (secondary) | `coinglass.com` | Cross-exchange funding rate comparison |
| TradingView (manual check) | `tradingview.com` | Visual confirmation of pre-settlement patterns |

**Data collection script:** Build a Python script that pulls all HL funding history and 1-minute candles for BTC-PERP and ETH-PERP from HL launch date to present. Store locally in Parquet format. Re-pull weekly to extend the dataset. Total data volume is small (<500MB for full history at 1-minute resolution).

---

## Open Questions for Backtest Phase

1. Does the effect scale linearly with funding rate magnitude, or is there a sharp threshold above which it becomes reliable?
2. Is the 00:00 UTC settlement different from 08:00 and 16:00 UTC (lower liquidity at midnight may amplify or dampen the effect)?
3. Does the effect appear in the 5-minute candle immediately before settlement, or is it spread across the full 30-minute window?
4. Is Trade B (post-settlement recovery) independently viable, or does it only work as a complement to Trade A?
5. Has the magnitude of the effect changed over time as HL has grown (more sophisticated participants may have compressed it)?

---

## Next Steps

| Step | Owner | Deadline |
|---|---|---|
| Pull full BTC-PERP and ETH-PERP funding + candle history from HL API | Researcher | Week 1 |
| Build event table (all settlement marks + funding rates + OHLCV windows) | Researcher | Week 1 |
| Run Trade A backtest with full filter set | Researcher | Week 2 |
| Run permutation test for statistical significance | Researcher | Week 2 |
| Stratify results by funding bucket, time of day, market regime | Researcher | Week 3 |
| Decision gate: proceed to paper trade or retire | Zunid | End of Week 3 |
