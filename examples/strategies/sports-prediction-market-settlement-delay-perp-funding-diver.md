---
title: "Fan Token Sentiment Overshoot — Prediction Market Settlement Gap"
status: HYPOTHESIS
mechanism: 3
implementation: 2
safety: 3
frequency: 3
composite: 54
categories:
  - calendar-seasonal
  - exchange-structure
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a major sports team wins or loses a high-profile match, retail traders immediately buy or sell that team's Chiliz fan token on pure sentiment. This move happens within 0–15 minutes of the final whistle. Simultaneously, prediction markets for the same event (Polymarket, SportX) have confirmed the outcome in the real world but have **not yet settled** — oracle aggregation, dispute windows, and UMA/Chainlink resolution processes take 15–120 minutes to push the contract to 1.0 or 0.0.

The fan token has no NAV anchor, no mechanical settlement, and no redemption mechanism. It is pure sentiment. The prediction market *does* have a hard mechanical anchor: it will settle to exactly 1.0 or 0.0. The gap between these two systems creates a window where:

1. The fan token has overshot (or undershot) based on retail emotion
2. The prediction market price has not yet moved to terminal value, confirming the outcome is not yet "officially" priced in by the slower, more deliberate capital
3. The fan token move has no fundamental driver to sustain it — no earnings, no dividend, no protocol revenue change

**Causal chain:**
- Final whistle → retail buys/sells fan token aggressively (0–15 min)
- Fan token spikes/dumps >8% on pure sentiment flow
- Prediction market oracle begins aggregation; contract still at pre-settlement price (15–120 min delay)
- Retail sentiment exhausts; no new buyers/sellers with structural reason to hold the new price
- Fan token mean-reverts partially toward pre-match level as sentiment fades
- Prediction market settles; this event acts as a "clock" — if it settles before reversion, the window has closed

**The edge is not that fan tokens always revert. The edge is that the prediction market settlement timestamp gives us a bounded, observable window during which the fan token is most likely to be at peak sentiment distortion.**

---

## Structural Mechanism — WHY This Happens

This is **not** a guaranteed mechanical convergence. Score 5/10 reflects this honestly. The mechanism is structural but probabilistic:

**Why the overshoot occurs (structural):**
- Chiliz fan tokens have no fundamental valuation model. There is no cash flow, no protocol fee, no buyback mechanism tied to match outcomes. A team winning a match does not change the token's intrinsic value in any calculable way.
- Fan token holders are predominantly retail sports fans, not quant traders. They react emotionally and immediately to match outcomes.
- Chiliz exchange and connected DEXs have thin order books. A small burst of market orders moves price disproportionately.

**Why the prediction market settlement acts as a structural clock:**
- UMA's Optimistic Oracle has a defined dispute window (typically 2 hours for sports markets on Polymarket). The settlement timestamp is deterministic once the event ends.
- Polymarket's resolution process is publicly observable on-chain. You can watch the oracle state transition in real time.
- The settlement timestamp is known in advance (dispute window length is fixed per market). This makes the exit window calculable, not guessed.

**Why the reversion is likely (but not guaranteed):**
- No new structural buyers exist after the initial sentiment burst. Fan token utility (voting rights, exclusive content access) does not change based on match outcome.
- Market makers on thin books will widen spreads after a volatile move, reducing further momentum.
- The same retail traders who bought on victory will take profits or lose interest within 30–90 minutes.

**What this is NOT:**
- This is not a guaranteed arbitrage. The fan token does not converge to any fixed value.
- This is not a cross-market arb between the fan token and the prediction market — they are not the same asset.
- The prediction market settlement is used as a **timing signal and confirmation tool**, not as a convergence anchor.

---

## Entry Rules


### Entry Conditions (ALL must be true simultaneously)

1. **Event confirmation:** Final whistle confirmed via sports data API (not TV, not Twitter — API timestamp required). Acceptable sources: API-Football, SportRadar, or ESPN API. Latency target: confirmation within 60 seconds of actual final whistle.

2. **Fan token move threshold:** Fan token has moved ≥8% from its pre-match baseline (defined as the 4-hour VWAP ending 30 minutes before kickoff) in the direction consistent with the outcome (up on win, down on loss).

3. **Prediction market not yet settled:** Polymarket or SportX contract for the same match is still in "pending resolution" state — price has not reached 0.95+ (win) or 0.05- (loss). Verify via Polymarket API or on-chain oracle state.

4. **Volume confirmation:** Fan token volume in the 15 minutes post-whistle is ≥3x its average 15-minute volume from the prior 48 hours. This confirms the move is driven by a real sentiment burst, not a thin-book artifact.

5. **Time gate:** Entry must occur within 20 minutes of final whistle. If conditions are not met within 20 minutes, no trade.

**Direction:**
- Team wins → SHORT the fan token (fading the victory spike)
- Team loses → LONG the fan token (fading the defeat dump)

**Execution:**
- Primary venue: Hyperliquid perp if listed (check current listings: https://app.hyperliquid.xyz/trade)
- Secondary venue: Chiliz DEX spot (https://exchange.chiliz.net) or any CEX listing (Binance, KuCoin)
- Use limit orders within 0.3% of mid to avoid paying wide spreads on thin books. If not filled within 2 minutes, cancel and skip the trade.

## Exit Rules

### Exit Rules

**Exit trigger 1 (primary):** Prediction market settles (contract reaches 0.97+ or 0.03-). Exit immediately at market. Rationale: the structural clock has expired; the "confirmation gap" window is closed.

**Exit trigger 2 (time stop):** 90 minutes post-whistle, exit at market regardless of P&L. Rationale: sentiment half-life is empirically short; holding beyond 90 minutes is no longer trading the structural gap.

**Exit trigger 3 (profit target):** Fan token retraces 50% of the post-whistle move from entry price. Take partial exit (50% of position) and trail stop on remainder.

**Exit trigger 4 (hard stop):** 4% adverse move from entry price. Exit full position at market. No exceptions.

**Priority order:** Hard stop > Prediction market settlement > Profit target > Time stop.

---

## Position Sizing

**Base position size:** 1% of trading capital per trade.

**Rationale for small size:**
- Liquidity risk is real. Fan token order books are thin. A 1% capital allocation at typical account sizes ($50k–$500k) may already move the market on entry/exit.
- Strategy is unproven. Pre-backtest sizing should be minimal.

**Scaling rule:** Do not increase position size until backtest shows Sharpe >1.5 and paper trading shows ≥20 trades with positive expectancy.

**Maximum concurrent positions:** 2 (different tokens/events only — no doubling up on the same team).

**Liquidity check before entry:** Verify that the 2% market depth on the order book is ≥5x your intended position size. If not, reduce position size or skip.

---

## Backtest Methodology

### Data Required

| Dataset | Source | URL/API |
|---|---|---|
| Fan token OHLCV (1-min) | CoinGecko API | `https://api.coingecko.com/api/v3/coins/{id}/ohlc` (free tier: 1-min not available; use CoinGecko Pro or CryptoCompare) |
| Fan token OHLCV (1-min, historical) | CryptoCompare | `https://min-api.cryptocompare.com/data/v2/histominute` |
| Fan token OHLCV (tick-level if possible) | Kaiko | `https://docs.kaiko.com/` (paid) |
| Match results + timestamps | API-Football | `https://www.api-football.com/` (free tier available) |
| Polymarket resolution timestamps | Polymarket API | `https://gamma-api.polymarket.com/markets` + on-chain UMA oracle events |
| Polymarket historical market data | Polymarket CLOB API | `https://clob.polymarket.com/` |
| SportX resolution data | The Graph (SportX subgraph) | `https://thegraph.com/hosted-service/subgraph/sportx-bet/sportx` |

### Token Universe

Focus on the 6 most liquid fan tokens by historical volume:
- $PSG (Paris Saint-Germain)
- $BAR (FC Barcelona)
- $JUV (Juventus)
- $ACM (AC Milan)
- $CITY (Manchester City)
- $ATM (Atletico Madrid)

Exclude tokens with <$500k average daily volume in the backtest period.

### Event Universe

- UEFA Champions League knockout rounds (higher stakes = stronger sentiment reaction)
- Major domestic league title-deciding matches
- World Cup / Euro matches involving the relevant clubs' national teams (fan token reaction is weaker here — test separately)

Exclude: group stage matches with low stakes, pre-season friendlies.

### Backtest Period

- Target: January 2021 – December 2024 (covers multiple Champions League seasons)
- Minimum: 2022–2024 (fan token market was more mature post-2021)

### Methodology Steps

1. **Build event database:** Pull all qualifying matches from API-Football. Record: kickoff time, final whistle time (use match duration + added time), result, home/away.

2. **Build prediction market database:** Pull all Polymarket sports markets. Match to events by team name + date. Record: market creation time, resolution timestamp, final settlement price.

3. **Simulate entry:** For each event, check fan token price at T+0 (final whistle), T+5, T+10, T+15, T+20 minutes. Apply entry conditions. Record entry price and timestamp.

4. **Simulate exit:** Apply exit rules in priority order. Record exit price, P&L, exit reason.

5. **Apply realistic transaction costs:**
   - Spread cost: assume 0.5% round-trip on Chiliz DEX (conservative)
   - Slippage: assume 0.3% additional on entry and exit (thin books)
   - Funding rate (if perp): pull actual historical funding from Hyperliquid or Binance

6. **Calculate metrics:**
   - Win rate
   - Average win / average loss
   - Expectancy per trade
   - Sharpe ratio (annualised, using trade-level returns)
   - Maximum drawdown
   - Profit factor
   - Breakdown by: token, event type, home/away, margin of victory

### Baseline Comparison

Compare against two null hypotheses:
1. **Random entry:** Enter short/long randomly after each match (same sizing, same exits) — tests whether timing matters
2. **Immediate entry (no threshold):** Enter at T+0 without the 8% threshold — tests whether the threshold adds value

### Key Metrics to Examine

- Does the 8% threshold filter out noise or does it mean you're entering after the move is already exhausted?
- What is the distribution of prediction market settlement times? Is 90 minutes the right time stop?
- Does the edge differ by token liquidity tier?
- Is the edge stronger in high-stakes matches vs. routine matches?

---

## Go-Live Criteria

The following must ALL be satisfied before moving to paper trading:

1. **Positive expectancy:** Expected value per trade > 0.5% after costs across ≥40 qualifying events
2. **Win rate:** ≥55% (given asymmetric stop/target, lower win rate may be acceptable if expectancy is met)
3. **Sharpe ratio:** ≥1.2 annualised on trade-level returns
4. **Maximum drawdown:** ≤15% of allocated capital
5. **No single token dependency:** Strategy must be profitable on ≥3 of the 6 tokens independently
6. **Baseline outperformance:** Must beat both null hypotheses (random entry and immediate entry) by ≥2% expectancy per trade
7. **Sufficient sample size:** ≥40 qualifying trades in backtest (not events — trades that met all entry conditions)

---

## Kill Criteria

Abandon the strategy (backtest or live) if any of the following occur:

**At backtest stage:**
- Fewer than 25 qualifying trades found in 3 years of data (insufficient frequency to be a systematic strategy)
- Expectancy is negative after realistic transaction costs
- The edge disappears entirely when slippage assumption is increased by 50% (fragile to execution quality)
- The 8% threshold means >60% of events are missed entirely (strategy is too selective to be useful)

**At paper trading stage:**
- 10 consecutive losing trades
- Drawdown exceeds 20% of paper capital allocated
- Average fill quality is >1% worse than backtest assumptions (execution is not replicable)
- Prediction market settlement times have changed (e.g., Polymarket shortens dispute window — the structural clock changes)

**Structural kill:**
- Chiliz fan tokens are delisted from major venues or liquidity drops below $100k daily volume
- Polymarket exits sports markets or changes resolution mechanism
- Fan token utility expands to include match-outcome-linked rewards (this would create a real fundamental anchor, changing the mechanism entirely)

---

## Risks

### Execution Risks (HIGH)
- **Liquidity:** Fan token order books are genuinely thin. $PSG and $BAR are the most liquid but still pale compared to major crypto assets. A 1% capital position at $100k account = $1,000 notional — this is manageable. At $1M account, this strategy may not scale.
- **Spread costs:** 0.5% round-trip is an optimistic assumption for Chiliz DEX. Real spreads post-volatility event may be 1–2%. This alone can eliminate the edge.
- **Entry timing:** The 8% move may be fully priced within 5 minutes of the whistle. If you need 20 minutes to confirm conditions, you may be entering at the peak of the overshoot, not the beginning of the reversion.

### Structural Risks (MEDIUM)
- **No guaranteed reversion:** Unlike LST depegs or token unlock shorts, there is no mechanism that FORCES the fan token back to any specific price. Reversion is probabilistic.
- **Sentiment can sustain:** A team winning a major trophy (not just a regular match) may sustain fan token buying for days, not minutes. The strategy needs to distinguish between routine wins and landmark events.
- **Prediction market timing is not always predictable:** UMA dispute windows can be extended if a dispute is filed. The "clock" can lengthen unpredictably.

### Data Risks (MEDIUM)
- **1-minute OHLCV is insufficient:** To properly backtest entry timing, tick-level or at minimum 1-minute data is needed. CoinGecko free tier does not provide this historically. Kaiko or CryptoCompare Pro required.
- **Match timestamp accuracy:** API-Football provides final whistle times but these can be 1–3 minutes off actual broadcast time. This matters when you're trading a 20-minute window.

### Regulatory Risks (LOW-MEDIUM)
- **Sports betting adjacency:** Some jurisdictions may classify this as sports betting. Zunid should obtain legal opinion before live trading if operating in regulated jurisdictions.
- **Chiliz regulatory status:** CHZ and fan tokens have faced regulatory scrutiny in some EU jurisdictions. Monitor.

### Model Risks (MEDIUM)
- **Overfitting:** With only ~40–80 qualifying events over 3 years, the backtest sample is small. Any parameter tuning (8% threshold, 90-minute stop) risks overfitting to a small sample.
- **Regime change:** Fan token markets in 2021 were different from 2024. Liquidity, retail participation, and market structure have all changed. Backtest results from 2021 may not be representative of current conditions.

---

## Data Sources

| Source | Purpose | Access | Cost |
|---|---|---|---|
| API-Football (`https://www.api-football.com/`) | Match results, timestamps, lineups | REST API | Free tier: 100 req/day; Pro: ~$10/mo |
| CryptoCompare (`https://min-api.cryptocompare.com/`) | Fan token 1-min OHLCV | REST API | Free tier available; historical depth limited |
| Kaiko (`https://www.kaiko.com/`) | Tick-level fan token data | REST API | Paid; contact for pricing |
| Polymarket CLOB API (`https://clob.polymarket.com/`) | Market resolution timestamps, prices | REST API | Free |
| Polymarket Gamma API (`https://gamma-api.polymarket.com/markets`) | Market metadata, resolution status | REST API | Free |
| UMA on-chain oracle (Polygon) | Dispute window state, settlement confirmation | Etherscan/Polygonscan event logs | Free |
| Chiliz Chain Explorer (`https://explorer.chiliz.com/`) | On-chain fan token transfer volume | Block explorer | Free |
| Hyperliquid (`https://app.hyperliquid.xyz/`) | Perp listings, funding rates | REST + WebSocket API | Free |
| Binance API (`https://api.binance.com/`) | Fan token spot prices if listed | REST API | Free |

### Recommended Data Pipeline

```
API-Football (match end timestamp)
    → trigger fan token OHLCV pull (CryptoCompare/Kaiko)
    → trigger Polymarket API poll (resolution status)
    → log: entry conditions met Y/N, entry price, exit price, exit reason
```

Build this pipeline first as a monitoring tool before attempting any live execution. Run it in observation mode for 2–3 months to validate that the data feeds are reliable and that the entry conditions fire at the expected frequency.

---

## Summary Assessment

This strategy has a plausible structural story but sits at the boundary between "structural edge" and "sentiment pattern." The prediction market settlement clock is a genuine structural element — it gives a bounded, observable window. But the fan token reversion is not mechanically forced; it is probabilistically likely given the absence of fundamental anchors.

**The strategy is worth backtesting specifically to answer:** Does the prediction market settlement window add timing value, or is the fan token reversion (if it exists) independent of that clock? If the backtest shows the edge exists but is independent of the prediction market timing, the strategy should be redesigned as a pure sentiment-reversion play with different entry/exit logic.

**Minimum viable outcome from backtest:** Confirm that qualifying events occur with sufficient frequency (≥15/year) and that transaction costs do not eliminate the edge. If either condition fails, kill the strategy before paper trading.
