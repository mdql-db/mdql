---
title: "Snapshot Governance Vote Conclusion → Token Price Reaction"
status: HYPOTHESIS
mechanism: 5
implementation: 7
safety: 6
frequency: 4
composite: 840
categories:
  - governance
  - defi-protocol
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a significant governance vote closes on Snapshot or Tally, the result becomes mathematically final and publicly computable at the exact close timestamp — but the majority of market participants learn about the outcome via secondary distribution channels (Twitter/X, Discord, Telegram, newsletters) with a lag of minutes to hours. This creates a structural information asymmetry window: the result exists in one system (Snapshot API) and has not yet been priced into the token's perpetual futures market. The edge is not predictive (we are not forecasting the vote outcome) — it is reactive (we are acting on a confirmed outcome faster than the median market participant). The trade is a speed-of-attention arbitrage, not a speed-of-execution arbitrage, and therefore does not require HFT infrastructure.

The strongest sub-hypothesis is that **fee-switch approvals and buyback approvals** produce the largest and most consistent post-close price moves because (a) they have direct, quantifiable cash-flow implications for token holders, (b) they are frequently debated for weeks before the vote, creating a "sell the rumor, buy the news" or "buy the confirmation" dynamic, and (c) the market systematically underweights the probability of passage during the voting period due to governance participation uncertainty.

---

## Structural Mechanism

### Why the gap exists (and must exist)

1. **Snapshot is a pull system, not a push system.** No exchange, no price feed, and no aggregator automatically subscribes to Snapshot vote close events. Market participants must actively poll or monitor Snapshot to know when a vote closes. The API endpoint `https://hub.snapshot.org/graphql` returns vote results in real time, but almost no retail trader and few institutional desks have automated monitoring of this endpoint.

2. **Vote close timestamps are fixed and known in advance.** Every Snapshot proposal has a hard-coded `end` timestamp set at proposal creation. This means the exact moment the result becomes final is knowable days in advance — we can pre-schedule our monitoring to the second.

3. **Secondary distribution has structural latency.** The fastest human-driven distribution channels (crypto Twitter, Discord bots) require a human or semi-automated process to (a) notice the vote closed, (b) read the result, (c) compose a post, (d) publish it. Even well-run protocol Discord servers typically lag 15–60 minutes. Automated Twitter bots that track Snapshot exist but are rare and followed by a small subset of traders.

4. **The outcome is binary and pre-classified.** Unlike earnings reports (which require interpretation), governance vote outcomes are binary (passed/failed) and the proposal text is available days in advance for pre-classification. We classify the vote type before it closes, so at close timestamp we already know whether a "pass" is bullish or bearish — no interpretation delay.

5. **Governance outcomes have direct on-chain consequences.** A passed fee-switch vote does not require trust — it triggers a smart contract execution (or a multisig action with a known timelock). The cash-flow change is contractually scheduled. This gives the price move a fundamental anchor, not just a sentiment anchor.

### Why the gap closes (and therefore why the trade works)

Within 1–6 hours of vote close, the result propagates through secondary channels and the market re-prices. The trade captures the spread between "result is final in the API" and "result is priced into the perp." The gap closes because information eventually propagates — the structural edge is in the propagation lag, not in any persistent information advantage.

### Vote type classification framework

| Vote Type | Direction | Rationale |
|---|---|---|
| Fee switch ON (protocol begins accruing fees to token holders) | LONG | Direct cash-flow creation for holders |
| Buyback program approved | LONG | Mechanical buy pressure, supply reduction |
| Treasury diversification (sell native token for stables) | SHORT | Announced sell pressure from treasury |
| Emissions increase / inflation schedule acceleration | SHORT | Supply dilution, sell pressure from recipients |
| Large grant to external team (paid in native token) | SHORT | Recipient likely to sell; supply increase |
| Protocol migration that burns/retires current token | SHORT | Existential dilution |
| Fee switch OFF (protocol removes fee accrual) | SHORT | Cash-flow destruction |
| Governance parameter change (quorum, voting power) | NEUTRAL | Skip — no direct price implication |
| Partnership / integration approval | NEUTRAL | Skip — too vague, pre-priced |

**Rule:** Only trade LONG and SHORT classified votes. Skip NEUTRAL. Pre-classify all votes in the monitoring queue before their close timestamp.

---

## Entry Rules

### Monitoring setup

- Poll `https://hub.snapshot.org/graphql` every 60 seconds for proposals with `end` timestamp within the next 24 hours across a pre-defined universe of protocols (see Data Sources).
- For each upcoming vote: retrieve proposal text, classify vote type using the framework above, record the `end` timestamp.
- At `end` timestamp + 30 seconds: query the API for final vote tally. Confirm the vote has closed (state = `closed`). Record winning choice and vote share.

### Entry filters (ALL must pass)

1. **Majority threshold:** Winning side has ≥ 66% of voting power. Votes closer than 66/34 are excluded — close votes indicate genuine community disagreement and are more likely to be re-voted or ignored, reducing price impact.
2. **Participation threshold:** ≥ 50 unique voters AND ≥ $500K in voting power (VP) participated. Filters out low-legitimacy votes that the market will discount.
3. **Protocol size filter:** Protocol TVL ≥ $50M OR token market cap ≥ $100M at time of vote. Ensures the token has sufficient liquidity on Hyperliquid to trade without excessive slippage.
4. **Perp availability:** Token must have an active perpetual market on Hyperliquid with ≥ $500K 24h volume at time of entry.
5. **Pre-pricing filter:** Token must NOT have moved more than 5% in the 4 hours immediately preceding vote close. A large pre-close move suggests the market already priced the outcome — the edge is gone.
6. **Vote type classified:** Vote must be pre-classified as LONG or NEUTRAL/SHORT before close. No post-hoc classification.

### Entry execution

- **Entry window:** 0–5 minutes after vote close timestamp confirmation from API.
- **Entry price:** Market order on Hyperliquid perp. Accept up to 0.3% slippage from mid at time of order.
- **Direction:** Per vote type classification (LONG or SHORT).
- **If entry window is missed** (e.g., API polling delay > 5 minutes): skip the trade. The edge degrades rapidly after 5 minutes as secondary distribution begins.

---

## Exit Rules

### Take profit

- **Primary TP:** +4% from entry price (close full position).
- **Partial TP option (for backtesting comparison):** Close 50% at +2%, trail remainder with 1.5% trailing stop from high.

### Stop loss

- **Hard stop:** -2% from entry price (close full position immediately).
- **Rationale for asymmetric TP/SL:** The structural mechanism produces a directional move when it works; when it fails, the market is indifferent to the vote and the position should be cut quickly.

### Time stop

- **Maximum hold:** 24 hours from entry, regardless of P&L. After 24 hours, the information has fully propagated and any remaining position is no longer structural — it is a directional bet.
- **Close at market** at the 24-hour mark if neither TP nor SL has been hit.

### Re-entry rule

- No re-entry on the same vote after exit. One trade per vote event.

---

## Position Sizing

### Base sizing

- **Risk per trade:** 0.5% of total portfolio NAV.
- **Leverage:** Size the position so that the hard stop (-2% from entry) equals the risk per trade.
  - Formula: `Position size in USD = (Portfolio NAV × 0.005) / 0.02`
  - Example: $100,000 portfolio → position size = $25,000 notional → at 1x leverage, this is $25,000 of token exposure.
- **Maximum leverage:** 3x. If the required notional exceeds 3x available margin, reduce position size to fit within 3x.

### Concentration limits

- **Maximum single position:** 2% of portfolio NAV notional.
- **Maximum concurrent positions:** 3 open trades simultaneously (governance votes can cluster; do not over-concentrate).
- **Correlated positions:** If two votes close within 24 hours on tokens with >0.7 rolling 30-day correlation, treat them as one position for sizing purposes.

### Scaling rule (post-backtest)

- If backtest Sharpe > 1.5 and win rate > 55%, increase risk per trade to 1% of NAV.
- Do not scale before backtest validation.

---

## Backtest Methodology

### Data assembly

**Step 1 — Snapshot historical data**
- Query Snapshot GraphQL API for all closed proposals from January 2021 to present across the target protocol universe.
- Fields required: `id`, `title`, `body`, `start`, `end`, `state`, `scores`, `scores_total`, `votes`, `space.id`.
- Store in a local database. Estimated volume: 50,000–200,000 proposals across all spaces; filter to ~500–2,000 high-quality proposals after applying filters.

**Step 2 — Manual vote classification**
- For each proposal passing the participation and majority filters, manually classify vote type using the framework above.
- Assign one of: LONG, SHORT, NEUTRAL/SKIP.
- Record classification rationale in a notes field.
- Target: classify all proposals for the top 50 protocols by TVL (DeFiLlama ranking). Estimated labor: 20–40 hours.
- Use two independent classifiers for the first 100 proposals to measure inter-rater agreement. Require ≥ 85% agreement before proceeding.

**Step 3 — Price data**
- Pull 1-minute OHLCV data for each token from Binance spot (primary), CoinGecko (secondary for tokens not on Binance), or Hyperliquid historical data where available.
- Align price data to vote close timestamps using UTC.
- Calculate returns at: +5min, +15min, +30min, +1h, +2h, +4h, +8h, +24h from vote close.

**Step 4 — Pre-pricing filter application**
- For each trade, calculate the 4-hour return ending at vote close. Exclude trades where |4h return| > 5%.

**Step 5 — Simulation**
- Apply entry rules: direction per classification, entry price = close price at vote close timestamp + 1 minute (to simulate realistic execution).
- Apply exit rules: TP at +4%, SL at -2%, time stop at 24h.
- Record: entry price, exit price, exit reason (TP/SL/time), hold duration, P&L.

### Metrics to compute

| Metric | Minimum threshold for go-live |
|---|---|
| Total trades | ≥ 50 (for statistical significance) |
| Win rate | ≥ 52% |
| Average win / average loss ratio | ≥ 1.8 |
| Sharpe ratio (annualized) | ≥ 1.0 |
| Maximum drawdown | ≤ 20% of starting capital |
| Profit factor | ≥ 1.3 |
| Median time to TP or SL | < 8 hours (confirms the mechanism is fast) |

### Sub-hypothesis tests

Run the backtest separately for each vote type category (fee switch, buyback, treasury sale, emissions) and report metrics per category. Hypothesis: fee-switch and buyback categories will show the strongest edge. If a category shows negative expectancy, exclude it from live trading.

### Overfitting controls

- Do NOT optimize TP/SL levels on the backtest data. Use the levels specified above (4% TP, 2% SL) as fixed parameters.
- Split data: use 2021–2023 as in-sample, 2024–present as out-of-sample. Report metrics separately.
- Do not add filters post-hoc to improve backtest results. Any filter added after seeing results must be justified by mechanism, not by P&L improvement.

---

## Paper Trading Protocol

### Duration

- Paper trade for 60 days or 20 trades, whichever comes first, before committing live capital.

### Execution simulation

- Record the exact Hyperliquid order book state at the moment of intended entry (screenshot or API snapshot).
- Record the price at which the trade would have been filled (use best ask for longs, best bid for shorts).
- Compare paper trade results to backtest results. If paper trade win rate is more than 10 percentage points below backtest win rate, halt and investigate before going live.

### Monitoring during paper trade

- Track: time from vote close to entry execution (target: < 5 minutes), slippage vs. mid, exit prices vs. targets.
- Log every trade with: vote ID, vote type, entry time, entry price, exit time, exit price, exit reason, P&L.

---

## Go-Live Criteria

All of the following must be satisfied:

1. Backtest shows Sharpe ≥ 1.0 and profit factor ≥ 1.3 on out-of-sample data (2024–present).
2. Paper trade win rate ≥ 50% over ≥ 20 trades.
3. Monitoring infrastructure is automated: API polling runs without manual intervention, alerts fire within 60 seconds of vote close.
4. Vote classification pipeline is documented and reproducible (another team member can classify a new vote in < 5 minutes using the framework).
5. Hyperliquid API integration is tested: orders can be placed and cancelled programmatically with confirmed latency < 2 seconds.

---

## Kill Criteria

Halt live trading immediately if any of the following occur:

1. **Drawdown:** Live trading drawdown exceeds 10% of allocated capital.
2. **Win rate collapse:** Win rate falls below 40% over any rolling 15-trade window.
3. **Edge decay signal:** Median time from vote close to 2% price move exceeds 2 hours (suggests the market is pricing votes faster, compressing the window).
4. **Structural change:** Snapshot introduces push notifications or a major aggregator (e.g., Messari, Token Terminal) begins publishing real-time vote close alerts — the information asymmetry is eliminated.
5. **Liquidity deterioration:** Average slippage on entry exceeds 0.5% of mid price over any 10-trade window.

---

## Risks

### Risk 1: Pre-pricing (primary risk)
**Description:** The market anticipates the vote outcome during the voting period and fully prices it before close. A 95/5 landslide vote that has been trending that way for 5 days is not a surprise at close.
**Mitigation:** The 5% pre-close price filter partially addresses this. Additionally, the backtest should measure whether landslide votes (>90% majority) produce smaller post-close moves than contested-but-clear votes (66–80% majority). If so, add a maximum majority filter (e.g., exclude votes with >90% majority).

### Risk 2: Vote implementation uncertainty
**Description:** A passed Snapshot vote is not always implemented. Snapshot is off-chain; implementation requires multisig execution or on-chain governance follow-through. If the market knows implementation is uncertain, the price move may be muted.
**Mitigation:** Track implementation rates by protocol. Prefer protocols with a documented history of implementing Snapshot votes within 30 days. Deprioritize protocols where Snapshot is purely advisory.

### Risk 3: Thin liquidity on Hyperliquid
**Description:** Many governance-active tokens have thin perp markets. Large position sizes will move the market against us.
**Mitigation:** The $500K 24h volume filter on Hyperliquid perps is a hard gate. Position sizing caps at $25K notional for a $100K portfolio, which is 5% of daily volume — acceptable for most liquid perps.

### Risk 4: Classification error
**Description:** Misclassifying a bearish vote as bullish (or vice versa) produces a loss in the direction of the structural move, not against it — the worst possible outcome.
**Mitigation:** Double-check classification for every trade before entry. Build a classification checklist. For ambiguous votes, default to NEUTRAL/SKIP.

### Risk 5: API reliability
**Description:** Snapshot API has experienced downtime and rate limiting. A polling failure at the critical moment means a missed trade or, worse, a delayed entry outside the 5-minute window.
**Mitigation:** Run two independent polling processes (primary and backup). Use Snapshot's IPFS-pinned data as a secondary source. Log all API failures. If the entry window is missed due to API failure, skip the trade — never chase.

### Risk 6: Regulatory / governance attack surface
**Description:** Governance votes can be manipulated (flash loan voting, whale coordination). A manipulated vote that passes may be reversed or ignored, producing a price reversal after initial move.
**Mitigation:** The 50-voter minimum and $500K VP threshold reduce (but do not eliminate) manipulation risk. Monitor for anomalous voting patterns (single wallet with >50% of VP) and skip those votes.

### Risk 7: Edge crowding
**Description:** If this strategy becomes known and other participants begin monitoring Snapshot API, the window compresses to seconds and the edge disappears.
**Mitigation:** Monitor the median time-to-price-move metric continuously. If it compresses below 2 minutes consistently, the edge is gone. Kill criteria #3 captures this.

---

## Data Sources

| Source | Data | Access | Cost |
|---|---|---|---|
| Snapshot GraphQL API (`hub.snapshot.org/graphql`) | All historical proposals, vote tallies, timestamps, voter counts | Public API, no auth required | Free |
| Tally.xyz API | On-chain governance (Compound, Uniswap, Aave) — supplement for protocols not on Snapshot | Public API, free tier | Free |
| DeFiLlama API | Protocol TVL for size filter | Public REST API | Free |
| CoinGecko API | Historical token prices (1-minute resolution via paid tier) | Paid tier required for 1-min data | ~$129/month |
| Binance historical data | 1-minute OHLCV for tokens listed on Binance | Public REST API | Free |
| Hyperliquid API | Perp market data, order placement, historical funding | Public REST + WebSocket | Free |
| CoinMarketCap | Market cap data for size filter | Free tier (daily data) | Free |

**Total data cost for backtest:** ~$130/month for CoinGecko paid tier during backtest period. Cancel after backtest completes.

---

## Open Questions for Backtest Phase

1. Does the edge concentrate in the first 5 minutes, or does it persist for 1–2 hours? (Determines whether the 5-minute entry window is too tight or too loose.)
2. Do fee-switch votes outperform buyback votes? (Determines category weighting in live trading.)
3. Is there a day-of-week or time-of-day effect? (Many votes are set to close at round UTC hours — does market attention vary by time?)
4. Does the pre-pricing filter (5% pre-close move exclusion) improve or hurt overall expectancy? (Test both with and without the filter.)
5. What is the base rate of vote implementation within 30 days? (Needed to assess Risk 2 severity by protocol.)

---

## Next Steps

1. **Week 1:** Build Snapshot API polling script; pull all historical proposals for top 50 protocols by TVL; store in local database.
2. **Week 2:** Manual classification of all proposals passing participation and majority filters; inter-rater agreement check.
3. **Week 3:** Pull price data; run backtest simulation; compute all metrics.
4. **Week 4:** Review results by category; decide which vote types to include in live trading; document go/no-go decision.
5. **Week 5–8:** Paper trading with automated monitoring; log all trades.
6. **Week 9:** Go-live decision based on paper trade results and go-live criteria above.
