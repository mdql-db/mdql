---
title: "Polymarket Settlement Convergence — Protocol Token Lag"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 3
composite: 540
categories:
  - defi-protocol
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a Polymarket prediction market resolves on a binary protocol event (mainnet launch, governance vote outcome, partnership confirmation), the resolution timestamp constitutes a verifiable, public information event. The corresponding protocol token on CEX/DEX perpetual markets has not yet priced this resolution because the majority of token market participants monitor CEX price feeds and social media — not Polymarket's resolution feed. A measurable lag exists between the moment Polymarket's smart contract settles and the moment that information propagates into token order books.

**Causal chain:**

1. Polymarket market resolves YES or NO via UMA oracle or admin key — this is on-chain and queryable via public API
2. Resolution implies a discrete probability jump: a market trading at 60% YES that resolves YES represents a 40-percentage-point information update
3. Token holders who were uncertain about the event outcome now have certainty — but most learn this through social channels (Twitter, Discord, news aggregators), not Polymarket
4. Social propagation takes minutes to hours; Polymarket resolution is instantaneous
5. The gap between resolution timestamp and social propagation creates a window where the token is mispriced relative to the now-known outcome
6. Entering long (YES resolution) or short (NO resolution) during this window captures the price adjustment as information diffuses

**The edge is NOT that Polymarket predicts the future better. The edge is that Polymarket's settlement is a faster, machine-readable signal than the social channels most token traders use.**

---

## Structural Mechanism

**Why this CAN happen (not MUST — honest assessment):**

Polymarket resolution is a discrete, timestamped, on-chain event. It is not a soft signal — it is a binary settlement. The information content is unambiguous. The mechanism that creates the lag:

- **Audience fragmentation:** Polymarket's active user base (~50k monthly) is a small subset of the token trading population. A protocol token may have 500k holders; fewer than 1% are watching Polymarket markets on that token.
- **No automated arbitrage bridge:** There is no standard infrastructure that pipes Polymarket resolution events into CEX market-making algorithms. A CEX market maker for a mid-cap token is not running a Polymarket listener.
- **Oracle delay adds legitimacy:** Polymarket uses UMA's optimistic oracle with a 2-hour dispute window for some markets, or admin resolution for others. The resolution event is final and verifiable — not a rumor.
- **Information asymmetry is cross-system, not cross-speed:** This is not latency arbitrage. It is a case where information exists in system A (Polymarket blockchain state) and has not yet been processed by participants in system B (token spot/perp markets). The gap is measured in minutes to hours, not microseconds.

**Why this is NOT guaranteed (honest score of 6/10):**

- The event may already be priced if the Polymarket market was trading at >90% before resolution (market consensus = near-certainty = token already moved)
- Sophisticated bots may already be monitoring Polymarket resolutions and front-running the window
- The token may be illiquid enough that entering a position itself moves the price before the signal propagates
- Some resolutions confirm events that were announced days earlier (e.g., a mainnet that launched last week, market just resolving now) — no new information

---

## Entry/Exit Rules

### Universe Selection

Only trade markets meeting ALL of the following criteria:

1. **Token must be listed on Hyperliquid perp OR have >$500k daily spot volume on a DEX** (ensures executable position)
2. **Polymarket market must have resolved with >$10,000 total volume** (ensures the market was meaningful, not a ghost market)
3. **Pre-resolution probability must be between 20% and 80%** (measured at T-24h before resolution). Markets outside this range are near-foregone conclusions — the token has likely already moved.
4. **Token price must NOT have moved >3% in the 30 minutes prior to resolution** (screens out cases where the resolution was leaked or anticipated via other channels)
5. **Event type must be protocol-specific and token-relevant:** mainnet launches, governance votes with treasury/tokenomics implications, major partnership confirmations, regulatory decisions affecting the specific protocol. Exclude: macro events (BTC ETF approval), events affecting the whole sector.

### Entry

- **Trigger:** Polymarket market status changes to `resolved` via API (see Data Sources)
- **Direction:** Long if resolved YES on positive event; Short if resolved NO on positive event (or YES on negative event)
- **Entry window:** Enter within **T+0 to T+10 minutes** of resolution timestamp
- **Entry price:** Market order on Hyperliquid perp or limit order at mid ± 0.2% (use limit to avoid slippage on thin books)
- **Do not enter if:** Token has already moved >3% from its price at T-30min before resolution (signal already propagated)

### Exit

- **Primary exit:** T+4 hours after entry — hard time stop regardless of P&L
- **Profit target:** +5% from entry (close 100% of position)
- **Stop loss:** -2% from entry (close 100% of position)
- **Secondary exit trigger:** If token volume spikes >3x its 24h average within the first 30 minutes post-entry, close 50% immediately (signal propagating faster than expected — take partial profit)

### Position Direction Logic

| Event Type | Resolves YES | Resolves NO |
|---|---|---|
| "Will X launch mainnet by date?" | LONG | SHORT |
| "Will X governance vote pass?" | Depends on vote content | Depends on vote content |
| "Will X be listed on Binance?" | LONG | SHORT |
| "Will X be hacked/exploited?" | SHORT | LONG |

For governance votes: manually classify the vote's token impact before the market resolves. If classification is ambiguous, skip the trade.

---

## Position Sizing

- **Base position:** 1% of total portfolio per trade
- **Maximum concurrent positions:** 3 (these events are rare; don't force trades)
- **Leverage:** 2x maximum on Hyperliquid perp. This is an information-lag trade, not a high-conviction directional bet. Leverage amplifies slippage risk on thin books.
- **Scale-in rule:** None. Enter full position at once within the entry window. Scaling in wastes the time-sensitive window.
- **Adjust down to 0.5% if:** Token's 24h spot volume is <$2M (liquidity risk increases)

---

## Backtest Methodology

### Data Required

**Polymarket resolution data:**
- Source: Polymarket API (see Data Sources)
- Fields needed: `market_id`, `question`, `resolution_time`, `outcome`, `volume`, `last_trade_price` (at T-24h, T-1h, T-30min before resolution)
- Historical range: January 2023 to present (Polymarket volume grew substantially in 2023; earlier data is sparse)
- Filter to protocol/token-specific markets only — manually tag each market with its corresponding token ticker

**Token OHLCV data:**
- Source: Hyperliquid historical data API or Binance/Bybit REST API for minute-level OHLCV
- Fields needed: Open, High, Low, Close, Volume at 1-minute resolution
- Window: T-60min to T+480min around each resolution event
- Minimum: 50 qualifying events to have statistical validity

### Backtest Steps

1. **Pull all Polymarket resolutions** from Jan 2023 onward. Filter to markets with >$10k volume.
2. **Tag each market** with a token ticker (manual step — no automated mapping exists). Expect ~200-400 qualifying markets total; maybe 50-100 with liquid tokens.
3. **Apply universe filters** (pre-resolution probability 20-80%, token volume >$500k, no pre-move >3%).
4. **For each qualifying event:** extract token price at T-30min, T-0 (resolution), T+10min, T+30min, T+1h, T+2h, T+4h.
5. **Simulate entry** at T+5min (conservative — assumes 5-minute reaction time) at the T+5min close price.
6. **Simulate exit** at T+4h close price, or earlier if stop/target hit (use minute OHLCV to check intrabar stops).
7. **Account for costs:** 0.05% taker fee each way on Hyperliquid, 0.1% slippage assumption on entry (conservative for mid-cap tokens).

### Key Metrics

| Metric | Target | Minimum Acceptable |
|---|---|---|
| Win rate | >55% | >50% |
| Average win / Average loss | >2.0 | >1.5 |
| Expected value per trade | >0.8% net of fees | >0.3% |
| Max drawdown (on strategy allocation) | <15% | <25% |
| Number of qualifying events | >50 | >30 |
| Sharpe ratio (annualised) | >1.5 | >1.0 |

### Baseline Comparison

Compare returns against:
1. **Random entry baseline:** Enter long/short on the same tokens at random times (same holding period), to confirm the resolution signal adds value over noise
2. **Delayed entry baseline:** Enter at T+60min instead of T+5min — quantifies how fast the window closes
3. **Pre-resolution probability baseline:** Does trading in the direction of the pre-resolution probability (i.e., betting on the favourite before resolution) outperform? If yes, the edge may be in the pre-resolution period, not post.

### Segmentation Analysis

Break results down by:
- Pre-resolution probability bucket: [20-40%], [40-60%], [60-80%]
- Token market cap: <$100M, $100M-$1B, >$1B
- Time to entry: T+0-5min vs T+5-10min (does speed matter?)
- Event type: mainnet launch vs governance vs listing vs other

---

## Go-Live Criteria

All of the following must be true before moving to paper trading:

1. **≥40 qualifying historical events** identified and backtested (below this, results are noise)
2. **Expected value per trade ≥ +0.5% net of fees** across the full sample
3. **Win rate ≥ 52%** with average win/loss ratio ≥ 1.8
4. **The T+5min entry outperforms the T+60min entry** — if they're equivalent, the window is already closed and the edge is gone
5. **Segmentation shows at least one sub-universe** (e.g., tokens <$500M market cap, pre-resolution probability 40-60%) with materially better metrics — this is where to focus live trading
6. **No single event accounts for >20% of total strategy P&L** (concentration risk check)

---

## Kill Criteria

Abandon the strategy (do not proceed to live trading) if ANY of the following:

1. **Expected value per trade is <0% net of fees** in backtest
2. **The T+5min and T+60min entries show identical performance** — means the window is already closed; bots have eliminated the lag
3. **<20 qualifying events found** in the historical data — the opportunity set is too small to be a systematic strategy; would need to be manual/discretionary
4. **Win rate <48%** — below this, even a good win/loss ratio doesn't save the strategy given transaction costs
5. **During paper trading:** 10 consecutive losses, or drawdown >10% of paper allocation — pause and re-evaluate universe filters

---

## Risks

### Primary Risks

**1. Window already closed (highest risk)**
Polymarket has grown significantly in 2024. It is plausible that automated bots already monitor resolution events and have compressed the lag to <1 minute — faster than any manual or semi-automated system can react. The backtest will reveal whether the T+5min entry still has edge. If not, this strategy is dead.

**2. Pre-resolution price discovery**
For high-profile events, the token price often moves in the days/hours before resolution as the Polymarket probability shifts. By the time of resolution, the token has already priced the outcome. The 20-80% probability filter and the 3% pre-move filter are designed to screen this out, but they won't catch all cases.

**3. Thin liquidity on relevant tokens**
The tokens most likely to have Polymarket markets with genuine uncertainty (mid-cap, newer protocols) are also the tokens with the thinnest order books. A 1% portfolio position may be $5,000-$50,000 — manageable — but slippage on entry and exit can easily consume the edge. Model slippage conservatively in backtest.

**4. Oracle/resolution disputes**
Polymarket uses UMA's optimistic oracle for some markets. A resolution can be disputed and reversed within a 2-hour window. If you enter on a resolution that gets disputed and reversed, you're holding a position based on a false signal. **Mitigation:** For markets using UMA oracle, wait for the dispute window to close before entering. This sacrifices speed but eliminates dispute risk. For admin-resolved markets, dispute risk is lower but not zero.

**5. Event classification error**
Manually classifying whether a YES resolution is bullish or bearish for the token requires judgment. A governance vote that passes might be bearish (e.g., large treasury spend, token dilution). Misclassification directly inverts the trade direction. **Mitigation:** Pre-classify all open markets weekly; don't classify under time pressure at resolution.

**6. Correlation with broader market**
If a major macro event (BTC crash, regulatory news) coincides with a protocol event resolution, the token move will be dominated by macro, not the Polymarket signal. The 4-hour holding period is long enough to be exposed to macro noise. **Mitigation:** Check BTC/ETH move in the same window; if >3% move in either, exclude the event from the sample in backtest.

### Secondary Risks

- **Hyperliquid listing lag:** Not all tokens with Polymarket markets are listed on Hyperliquid perp. Some trades will require DEX spot execution (higher slippage, no shorting without borrowing).
- **Regulatory risk on Polymarket itself:** Polymarket's US regulatory status is uncertain. Platform disruption would eliminate the data source.
- **Small sample size:** Even with 2+ years of data, qualifying events may number only 30-60. Statistical significance is limited; treat backtest results as directional, not definitive.

---

## Data Sources

### Polymarket

- **REST API (markets + resolutions):** `https://gamma-api.polymarket.com/markets`
  - Filter by `closed=true` and `resolved=true`
  - Fields: `question`, `endDate`, `resolutionTime`, `outcomePrices`, `volume`, `outcomes`
- **CLOB API (order book + trade history):** `https://clob.polymarket.com/`
  - Use for pre-resolution probability time series (price at T-24h, T-1h, T-30min)
- **Polymarket subgraph (on-chain resolution events):** Available via The Graph — query `ConditionResolution` events for exact on-chain timestamps
- **Docs:** `https://docs.polymarket.com/`

### Token Price Data

- **Hyperliquid historical candles:** `https://api.hyperliquid.xyz/info` — POST with `{"type": "candleSnapshot", "req": {"coin": "TOKEN", "interval": "1m", "startTime": ..., "endTime": ...}}`
- **Binance 1-minute OHLCV:** `https://api.binance.com/api/v3/klines?symbol=TOKENUSDT&interval=1m&startTime=...&endTime=...`
- **Bybit 1-minute OHLCV:** `https://api.bybit.com/v5/market/kline?category=linear&symbol=TOKENUSDT&interval=1&start=...&end=...`
- **CoinGecko historical (fallback for smaller tokens):** `https://api.coingecko.com/api/v3/coins/{id}/ohlc?vs_currency=usd&days=1` (note: CoinGecko free tier is 5-minute resolution minimum — use only as fallback)

### Reference

- **UMA oracle dispute tracking:** `https://oracle.uma.xyz/` — verify resolution finality status
- **Token unlock schedules (cross-reference):** `https://token.unlocks.app/` — check if a token unlock coincides with the resolution event (confounding factor)

---

## Implementation Notes

**Monitoring setup required:**
A lightweight script polling `https://gamma-api.polymarket.com/markets?closed=true&resolved=true` every 60 seconds, filtering for newly resolved markets in the universe, and alerting via Telegram/Discord with: market question, resolution outcome, pre-resolution probability, corresponding token ticker, and current token price vs T-30min price. This is the minimum viable monitoring system. No execution automation required for initial paper trading — manual execution within the 10-minute window is feasible.

**Manual pre-work required weekly:**
Review all open Polymarket markets with >$5k volume, tag token tickers, pre-classify directional impact of YES/NO resolution. This prevents classification errors under time pressure. Estimated time: 30 minutes/week.
