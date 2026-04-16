---
title: "Governance Timelock Sell-the-News"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - governance
  - calendar-seasonal
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a governance proposal passes a quorum vote, the market prices in the expected upgrade benefit immediately. The subsequent 48–72h timelock period is a mandatory waiting window during which no new information arrives but the "buy the rumor, buy the vote" crowd exits. This creates a structural supply overhang: late retail buyers who bought on vote passage have no catalyst to hold through the timelock and sell into any remaining strength. Short the governance token 2–6h after vote confirmation, cover at or just before execution block.

---

## Structural Mechanism

The edge is **semi-structural, not fully contractual**. Here is the causal chain:

1. **Timelock is a hard protocol constraint.** Compound Governor Bravo, Aave's governance module, and MakerDAO's GSM all enforce a mandatory delay (typically 48–72h) between `ProposalQueued` and `execute()` being callable. No actor can accelerate this — it is enforced at the smart contract level. This part is guaranteed.

2. **Information arrival stops at queue time.** Once queued, the upgrade is known and priced. No new fundamental information arrives during the timelock. The timelock is informationally dead air.

3. **Forced seller cohort exists.** Event-driven traders who bought on proposal passage have a defined exit window. They cannot benefit from holding through the timelock (no additional upside catalyst) and face execution risk if they hold to the block (smart contract bugs, last-minute cancellations via guardian multisig). Rational actors sell during the timelock.

4. **Guardian/veto risk creates asymmetric downside.** Most protocols retain a guardian or security council that can cancel a queued proposal. This optionality is a mild put against longs — it does not exist for shorts. This slightly favors the short side during the timelock window.

5. **Why this is NOT fully structural (score cap at 6):** The price pump on vote passage is probabilistic, not guaranteed. Low-salience proposals (parameter tweaks, treasury transfers) may not pump at all, eliminating the fade opportunity. Market-wide conditions can overwhelm the local sell pressure. The mechanism is real but the magnitude is variable.

---

## Universe

| Token | Protocol | Governance Contract | Timelock Duration |
|-------|----------|--------------------|--------------------|
| COMP | Compound | Governor Bravo | 48h |
| UNI | Uniswap | Governor Bravo fork | 48h |
| AAVE | Aave | AaveGovernanceV2 | 24–72h (proposal-type dependent) |
| MKR | MakerDAO | DSChief + GSM | 48h (GSM delay) |
| CRV | Curve | Aragon + custom | 24–72h |

**Exclusion filter:** Only trade proposals that cause a measurable price move of ≥3% in the 0–2h window after `ProposalQueued` event on-chain. Proposals with <3% move are low-salience and should be skipped — the fade has no room to run.

---

## Entry Rules

1. **Trigger event:** `ProposalQueued` event emitted on-chain (not Snapshot — must be on-chain mainnet governance contract).
2. **Salience filter:** Token price must be up ≥3% vs. 24h-prior close at time of queue event. If not, skip this instance.
3. **Entry window:** Open short position 2–6h after `ProposalQueued` timestamp. The 2h delay avoids the initial momentum spike; the 6h cap avoids entering too late into a fade already underway.
4. **Entry price:** Use TWAP of the 30 minutes centered on entry time (15 min before, 15 min after) to avoid slippage on a single print. Execute on Hyperliquid perp for COMP, AAVE, UNI, MKR, CRV where available; use spot short via margin on Binance/Bybit as fallback.
5. **Confirmation check:** Verify proposal has not been cancelled between vote passage and entry. Check `state()` on the Governor contract — must return `Queued (4)` not `Cancelled (2)`.

---

## Exit Rules

1. **Primary exit:** Cover 1 block before the earliest callable `execute()` timestamp. Calculate this as: `ProposalQueued block timestamp + timelock_duration - 15 minutes`. Set a limit order or automated cover at this time.
2. **Stop loss:** Cover immediately if price rises ≥8% above entry price at any point during the hold. This caps loss at approximately 8% gross (before funding).
3. **Accelerated exit:** If price drops ≥6% below entry (i.e., trade is working well), take 50% profit and trail the stop on the remainder to entry price (breakeven stop).
4. **Cancellation windfall:** If the proposal is cancelled during the timelock (guardian veto), cover immediately at market — this is an unexpected gift (price likely drops further) but do not chase; take the gain and close.
5. **Do not hold through execution block.** Post-execution, the upgrade is live and a new "buy the news" move is possible. The structural thesis only applies during the timelock window.

---

## Position Sizing

- **Base size:** 0.5% of portfolio NAV per trade.
- **Maximum concurrent exposure:** 2 positions simultaneously (1.0% NAV total), since governance events across protocols can cluster.
- **Rationale for small size:** This is a low-frequency, hypothesis-stage strategy. Sizing is deliberately conservative until backtest confirms positive expectancy. Governance tokens are illiquid relative to BTC/ETH — 0.5% NAV keeps market impact negligible.
- **Leverage:** 2–3x maximum on Hyperliquid perp. Higher leverage is not warranted given the probabilistic (not guaranteed) nature of the fade.
- **Funding cost budget:** At 3x leverage over a 72h hold, funding costs at typical rates (~0.01%/8h) total ~0.09% — acceptable drag, monitor if rates spike.

---

## Backtest Methodology

### Data Collection

1. **Governance event log:** Pull all historical `ProposalQueued` events from each Governor contract using Etherscan API or The Graph subgraph for each protocol. Record: proposal ID, queue timestamp, queue block, timelock duration, execute timestamp.
   - Compound subgraph: `https://thegraph.com/explorer/subgraphs/ABAqVHHrRhxrSjYEZmFLuu8UKmxMwDmdTsKMuSJu7ZeG`
   - Uniswap governance subgraph: `https://thegraph.com/explorer/subgraphs/FQ6JYszEKApsBpAmiHesRsd9Ygc6mzmpNRANeVQFYoVX`
   - Aave governance: query `AaveGovernanceV2` contract `0xEC568eFE...` via Etherscan events
   - MakerDAO: `https://vote.makerdao.com/api/executive` for historical spells + GSM timestamps
   - Curve: Aragon DAO events via Etherscan

2. **Price data:** Pull 1-minute OHLCV for COMP, UNI, AAVE, MKR, CRV from Binance spot (primary) or CoinGecko historical API (fallback) for the full history of each token. Source: `https://api.binance.com/api/v3/klines`

3. **Match events to prices:** For each `ProposalQueued` event, extract:
   - Price at queue time (T=0)
   - Price at T+2h, T+4h, T+6h (entry candidates)
   - Price at T+timelock_duration−15min (exit)
   - Max price during hold (for stop-loss simulation)
   - Min price during hold (for profit-take simulation)

### Backtest Logic

```
For each ProposalQueued event:
  1. Apply salience filter: skip if price_at_T0 / price_24h_prior < 1.03
  2. Entry price = TWAP(T+2h to T+2h30m)  [test T+2h, T+4h, T+6h separately]
  3. Simulate short at entry price
  4. Apply stop: if max_price_during_hold > entry * 1.08, exit at entry * 1.08
  5. Apply profit-take: if min_price_during_hold < entry * 0.94, 
     take 50% at entry * 0.94, trail remainder
  6. Primary exit: price at T+timelock_duration−15min
  7. Record: gross PnL, net PnL (after 0.05% taker fee each way + funding)
```

### Metrics to Report

- Win rate (% of trades with positive net PnL)
- Average net PnL per trade (%)
- Sharpe ratio (annualised, using trade-level returns)
- Maximum drawdown across all trades
- Breakdown by token (COMP vs UNI vs AAVE vs MKR vs CRV)
- Breakdown by proposal salience (3–5% pump vs >5% pump at queue)
- Breakdown by timelock duration (24h vs 48h vs 72h)
- Number of qualifying events per year (expected frequency)

### Expected Sample Size

Rough estimate: ~5–15 qualifying proposals per protocol per year across 5 tokens = **25–75 events total** over a 4-year backtest window (2020–2024). This is a small sample — treat backtest results as directional, not statistically conclusive. Require p < 0.10 (not 0.05) given sample size constraints.

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Positive expectancy:** Backtest shows average net PnL per trade > +1.5% after fees and funding across all qualifying events.
2. **Win rate ≥ 55%:** Below this, the strategy is too coin-flip to justify operational overhead.
3. **No single token dominates:** Remove any single token from the backtest — strategy must remain profitable on the remaining 4. This tests robustness.
4. **Salience filter validated:** Trades filtered out by the <3% rule must show worse average PnL than trades that pass. If the filter adds no value, the entry logic is broken.
5. **Paper trade confirmation:** Run 3 live paper trades (real events, simulated execution) with documented entry/exit timestamps and prices. At least 2 of 3 must be profitable.
6. **Monitoring infrastructure live:** Automated on-chain event listener for `ProposalQueued` events must be operational before going live. Manual monitoring is not acceptable for production.

---

## Kill Criteria

Abandon the strategy (stop trading, archive) if any of the following occur:

1. **5 consecutive losing trades** in live trading (not paper).
2. **Cumulative live loss exceeds 3% of portfolio NAV** allocated to this strategy.
3. **Governance architecture changes:** If major protocols migrate to optimistic governance (no timelock) or dramatically shorten timelocks to <12h, the structural mechanism is broken. Monitor governance upgrade proposals themselves.
4. **Crowding detected:** If average entry-to-exit move compresses to <1% net across 10 consecutive qualifying events, the edge has been arbitraged away.
5. **Regulatory event:** If a major protocol's governance token is classified as a security in a key jurisdiction, liquidity and tradability may be impaired — exit all positions and pause.

---

## Risks

### Primary Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Proposal cancelled by guardian | Low severity (helps short) | None needed — asymmetric benefit |
| Market-wide rally overwhelms local sell pressure | High severity | Stop loss at 8%; avoid entry during macro risk-on events (e.g., BTC up >5% same day) |
| Low-salience proposal — no pump to fade | Medium severity | Salience filter (≥3% pump required) |
| Timelock duration varies by proposal type | Medium severity | Read `delay()` from contract at entry, not from documentation |
| Thin liquidity on Hyperliquid for smaller tokens | Medium severity | Size cap at 0.5% NAV; check open interest before entry |
| Funding rate spikes against short | Low severity | 72h max hold; funding budget modeled at 3x leverage |

### Tail Risks

- **Smart contract exploit during timelock:** If the protocol is exploited during the timelock window (unrelated to the queued proposal), the governance token may crash violently. This helps the short but creates basis risk if the perp delists or circuit-breaks.
- **Fork or airdrop announcement during hold:** An unexpected positive announcement (e.g., UNI fee switch activation) during the timelock window could cause a violent squeeze. The 8% stop loss is the primary protection.
- **Governance capture / contentious vote:** If a proposal passes with a narrow majority and a counter-campaign emerges during the timelock, price behavior becomes unpredictable. Check vote margin — skip proposals that passed with <60% of quorum.

---

## Data Sources

| Data Type | Source | URL |
|-----------|--------|-----|
| Compound governance events | The Graph | `https://thegraph.com/explorer/subgraphs/ABAqVHHrRhxrSjYEZmFLuu8UKmxMwDmdTsKMuSJu7ZeG` |
| Uniswap governance events | The Graph | `https://thegraph.com/explorer/subgraphs/FQ6JYszEKApsBpAmiHesRsd9Ygc6mzmpNRANeVQFYoVX` |
| Aave governance | Etherscan Events | `https://etherscan.io/address/0xEC568eFE...#events` |
| MakerDAO executive votes | MakerDAO API | `https://vote.makerdao.com/api/executive` |
| Curve governance | Etherscan / Aragon | `https://etherscan.io/address/0x...` (Aragon DAO) |
| Live governance monitoring | Tally | `https://www.tally.xyz` |
| Snapshot (off-chain votes — exclude from entry trigger) | Snapshot | `https://snapshot.org` |
| Token OHLCV (1m candles) | Binance API | `https://api.binance.com/api/v3/klines` |
| Token OHLCV (fallback) | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| Hyperliquid perp execution | Hyperliquid | `https://app.hyperliquid.xyz` |
| On-chain event listener | Alchemy / Infura | `https://www.alchemy.com` |

---

## Open Questions for Backtest Phase

1. Does the 3% salience filter correctly separate signal from noise, or should the threshold be higher (5%)?
2. Is T+2h or T+4h the better entry point — does the initial momentum persist longer than expected?
3. Do 48h timelocks produce larger fades than 24h timelocks (more time for sellers to exit)?
4. Is MKR structurally different due to its dual-token system (MKR burn mechanics may create persistent buy pressure that offsets the fade)?
5. Does the strategy perform better in bear markets (more motivated sellers) vs. bull markets (FOMO overrides rational exit)?

---

*Next step: Assign to backtest engineer. Deliver results within 2 sprint cycles. Flag if qualifying event count falls below 20 total — sample size will be insufficient for any statistical inference.*
