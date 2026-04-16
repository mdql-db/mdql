---
title: "Hyperliquid Spot–Perp Auto-Deleveraging (ADL) Queue Mechanics"
status: HYPOTHESIS
mechanism: 4
implementation: 5
safety: 3
frequency: 1
composite: 60
categories:
  - defi-protocol
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Hyperliquid's insurance fund balance drops sharply and a large leveraged position is deeply underwater, Auto-Deleveraging (ADL) becomes structurally inevitable. ADL force-closes profitable counterparties at mark price, creating a mechanical price dislocation between mark price and fair value. By monitoring insurance fund drawdown and open interest concentration in real time, we can position *before* ADL fires and capture the reversion after forced closes complete. The edge is not in predicting *whether* ADL fires — it is in recognising that the system's own rules make it unavoidable once specific, observable thresholds are crossed.

---

## Structural Mechanism

### How ADL Works on Hyperliquid

1. Every perpetual market has an insurance fund denominated in USDC that absorbs losses when a liquidated position cannot be closed at a price that covers the deficit.
2. When a liquidation generates a loss larger than the insurance fund balance, the system cannot socialise the loss through the fund alone.
3. ADL is triggered: the engine selects profitable counterparties ranked by (profit percentage × leverage used) — the ADL queue — and force-closes them at the current mark price, regardless of their limit orders or intentions.
4. The force-close at mark price is a **smart-contract-enforced event**, not a discretionary one. No human can prevent it once the threshold is crossed.
5. After ADL fires, the forced closure of profitable longs (in a short-squeeze scenario) or profitable shorts (in a crash scenario) removes directional pressure, creating a mechanical reversion opportunity.

### Why This Is Structural, Not Pattern-Based

- The insurance fund balance is a **hard number** visible on-chain at all times.
- The ADL queue ranking formula is **published in Hyperliquid's documentation** — it is deterministic.
- The trigger condition (fund balance < liquidation deficit) is a **binary threshold**, not a probabilistic tendency.
- The reversion after ADL is structural: forced closes remove the most profitable directional positions, temporarily exhausting one side of the market.

### The Tradeable Distortion

ADL creates two distinct windows:

| Window | Timing | Trade Direction |
|--------|--------|-----------------|
| **Pre-ADL** | Insurance fund falling, large underwater position visible | Fade the direction of the at-risk position (join the profitable side that will be ADL'd) |
| **Post-ADL** | Immediately after ADL fires, profitable positions force-closed | Fade the reversion — the forced close creates a temporary vacuum on the profitable side |

The post-ADL window is cleaner because the trigger is observable in the transaction log. The pre-ADL window requires probabilistic judgment about whether the fund will be depleted.

---

## Entry Rules

### Signal Construction

**Step 1 — Insurance Fund Monitor**
- Pull insurance fund balance for each perpetual market every 60 seconds via Hyperliquid public API endpoint: `GET /info` with `type: "meta"` and cross-reference with `type: "clearinghouseState"`.
- Calculate 1-hour rolling drawdown of insurance fund: `IF (fund_balance_now / fund_balance_60min_ago - 1) < -15%` → flag as **ADL Watch**.
- Calculate absolute deficit risk: `IF fund_balance < $500,000` → escalate to **ADL Alert** regardless of drawdown rate.

**Step 2 — Underwater Position Scanner**
- Pull all open positions via `GET /info` with `type: "openOrders"` and `type: "allMids"`.
- Identify positions where unrealised PnL < -80% of initial margin AND position notional > $2M.
- Cross-reference with funding rate: if funding rate is extreme (>0.1% per 8h) in the direction that benefits the underwater position, the position is likely being held open deliberately — higher ADL risk.

**Step 3 — ADL Queue Estimation**
- Identify the top 10 profitable counterparties on the opposite side using leaderboard data and open interest distribution (available via `GET /info` with `type: "openInterest"`).
- Estimate which accounts are highest in the ADL queue using the formula: `ADL rank = profit_percentage × leverage`. Accounts with >5x leverage and >50% unrealised profit are highest priority targets.

**Step 4 — Entry Trigger**

*Pre-ADL Entry (probabilistic, lower conviction):*
- All three conditions met: ADL Watch active + underwater position identified + funding rate extreme.
- Enter a position **opposite** to the underwater position (i.e., if a large short is underwater, go long).
- This is the lower-conviction leg — size accordingly (see Position Sizing).

*Post-ADL Entry (structural, higher conviction):*
- ADL event confirmed: observable as a sudden mark price jump with no corresponding order book activity, or via Hyperliquid's ADL notification in the API response.
- Immediately after confirmation, enter **opposite** to the direction of the ADL force-close.
- Rationale: ADL removes the most profitable directional positions; the market temporarily loses its strongest directional participants, creating a vacuum that reverts.

---

## Exit Rules

### Pre-ADL Position

- **Stop loss:** 3% adverse move from entry, hard stop, no exceptions. Pre-ADL is probabilistic; the underwater position may receive fresh capital or the fund may be replenished.
- **Take profit:** Exit 50% of position when ADL fires (confirmed). Exit remaining 50% at 5% profit from entry or 24 hours elapsed, whichever comes first.
- **Time stop:** If ADL does not fire within 4 hours of entry, exit at market. The structural condition has not resolved; holding longer introduces unrelated market risk.

### Post-ADL Position

- **Entry window:** Enter within 60 seconds of confirmed ADL event. After 60 seconds, the market has partially repriced and the edge degrades.
- **Stop loss:** 2% adverse move from entry. Post-ADL reversion is fast or it does not happen.
- **Take profit:** Target 3–5% reversion from the ADL-distorted mark price. Exit in full; do not scale.
- **Time stop:** Exit within 2 hours regardless of PnL. ADL effects are short-duration.

---

## Position Sizing

### Pre-ADL Leg

- Maximum 0.5% of total portfolio per trade.
- Rationale: This is a probabilistic bet on ADL firing. Even with all signals active, ADL may not fire (fund replenishment, position closure by the holder). Treat as a lottery ticket with defined loss.
- No leverage beyond 2x. The pre-ADL window can last hours; leverage amplifies time decay of funding costs.

### Post-ADL Leg

- Maximum 2% of total portfolio per trade.
- Rationale: The trigger is confirmed and structural. The reversion window is short, so higher sizing is justified relative to pre-ADL.
- Maximum 3x leverage. Higher leverage is not warranted because post-ADL mark prices can be volatile and slippage is elevated.

### Portfolio-Level Constraints

- Maximum 2 simultaneous ADL Watch positions across different markets.
- Do not run ADL strategy during periods of broad market stress (BTC 1h volatility > 5%) — insurance funds deplete faster and ADL can cascade across markets, making reversion unreliable.

---

## Backtest Methodology

### Data Collection

1. **Historical insurance fund balances:** Request from Hyperliquid team or reconstruct from on-chain settlement logs. Hyperliquid is an L1; all state transitions are publicly verifiable. Target: full history since mainnet launch (November 2023).
2. **Historical ADL events:** Identify from transaction logs where mark price moved discontinuously without corresponding order book depth. Cross-reference with Hyperliquid's published ADL notifications.
3. **Mark price history:** Available via `GET /info` with `type: "candleSnapshot"` at 1-minute resolution.
4. **Open interest and position concentration:** Available via API; reconstruct historical snapshots from block data.

### Backtest Steps

**Step 1 — Event Identification**
- Scan full history for confirmed ADL events. Expect fewer than 50 events total given Hyperliquid's history. This is a small-sample problem — acknowledge it explicitly.
- For each event, record: asset, direction of ADL (which side was force-closed), mark price at ADL, mark price 5/15/30/60/120 minutes post-ADL.

**Step 2 — Pre-ADL Signal Backtest**
- For each ADL event, walk back 4 hours and check whether the three-condition signal (fund drawdown + underwater position + extreme funding) was active.
- Calculate: signal hit rate (how often all three conditions were active before ADL), false positive rate (how often all three conditions were active but ADL did not fire within 4 hours).

**Step 3 — Post-ADL Reversion Backtest**
- For each confirmed ADL event, simulate entry at mark price + 60 seconds (to account for execution delay).
- Measure reversion at 5/15/30/60/120 minutes.
- Calculate: win rate, average return, Sharpe ratio, maximum adverse excursion.

**Step 4 — Slippage Adjustment**
- Post-ADL markets are illiquid. Apply 0.5% slippage penalty to all simulated entries and exits. If strategy is not profitable after slippage, it does not proceed.

### Expected Sample Size Problem

ADL events are rare. Hyperliquid has been live since late 2023; expect 20–60 historical ADL events across all markets. This is insufficient for statistical significance. Mitigation:

- Treat backtest as **hypothesis validation**, not proof.
- Require that post-ADL reversion is positive in >70% of events AND average return exceeds 2% (after slippage) before proceeding to paper trading.
- Supplement with cross-exchange ADL data (BitMEX, Bybit have longer histories with similar mechanics) to increase sample size, noting that Hyperliquid's specific parameters may differ.

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Backtest:** Post-ADL reversion positive in ≥70% of historical events with average return ≥2% after 0.5% slippage.
2. **Paper trading:** Minimum 5 live ADL events observed and traded on paper. Win rate ≥60% on paper trades.
3. **Execution test:** Confirm that API latency allows entry within 60-second post-ADL window. Run latency tests from intended server location. If median latency to Hyperliquid API exceeds 200ms, co-locate or use a closer server.
4. **Insurance fund monitoring:** Automated alert system operational and tested. Alert must fire within 30 seconds of fund crossing threshold.
5. **Legal review:** Confirm that monitoring on-chain data and trading on public markets does not constitute front-running under applicable jurisdiction's regulations. ADL is a public mechanism — this should be clean, but verify.

---

## Kill Criteria

Stop trading this strategy immediately if any of the following occur:

1. **Three consecutive post-ADL trades lose more than 2% each.** The reversion mechanism may have changed (e.g., Hyperliquid updated ADL parameters).
2. **Hyperliquid modifies ADL queue formula or insurance fund mechanics.** The structural edge is tied to specific protocol rules. Any protocol upgrade requires full re-evaluation before resuming.
3. **Insurance fund is permanently recapitalised to a level where ADL becomes structurally impossible** (e.g., fund > $50M with no path to depletion). Monitor Hyperliquid governance proposals.
4. **Post-ADL entry window shrinks below 30 seconds** due to increased competition from other monitors. If the market reprices within 30 seconds of ADL, our execution advantage is gone.
5. **Maximum drawdown on this strategy exceeds 5% of allocated capital** in any rolling 30-day period.

---

## Risks

### Risk 1: Cascade ADL (Highest Severity)
- **Description:** ADL in one market triggers liquidations in correlated markets, which deplete their insurance funds, triggering further ADL. The reversion trade becomes a falling knife.
- **Mitigation:** Do not enter post-ADL reversion trade if BTC or ETH has moved >3% in the same direction in the prior 30 minutes. Cascade risk is highest in correlated market stress.

### Risk 2: Protocol Rule Changes
- **Description:** Hyperliquid can update ADL parameters, insurance fund replenishment rules, or queue ranking formula via governance or team decision. The edge disappears without warning.
- **Mitigation:** Subscribe to Hyperliquid Discord and governance channels. Pause strategy immediately upon any announcement of protocol changes to liquidation or ADL mechanics.

### Risk 3: Thin Post-ADL Liquidity
- **Description:** Immediately after ADL, the order book is thin. A 2% position in a small-cap perpetual may move the market against us on entry.
- **Mitigation:** Only trade ADL events in markets with >$10M daily volume. Apply 0.5% slippage assumption in all sizing calculations. Use limit orders with 0.1% price improvement over mark price rather than market orders.

### Risk 4: False ADL Signal
- **Description:** Insurance fund drawdown and underwater positions are visible, but the position holder closes voluntarily or receives external capital before ADL fires. Pre-ADL trade loses on time stop.
- **Mitigation:** Pre-ADL position is sized at 0.5% maximum. Time stop at 4 hours limits loss. Accept this as the cost of the probabilistic leg.

### Risk 5: Regulatory Ambiguity
- **Description:** Monitoring on-chain state to anticipate forced liquidations could be characterised as front-running in some jurisdictions, even though all data is public.
- **Mitigation:** Obtain legal opinion before going live. The strategy uses only public data and trades on a public market — this is analogous to monitoring public order books, but confirm with counsel.

### Risk 6: Small Sample Size
- **Description:** With fewer than 60 historical ADL events, backtest results are not statistically robust. A 70% win rate on 20 events has wide confidence intervals.
- **Mitigation:** Treat this as a low-allocation, exploratory strategy. Maximum portfolio allocation is 2% per trade and 4% total. Do not scale until sample size from live trading exceeds 30 events.

---

## Data Sources

| Data Type | Source | Endpoint / Method | Latency |
|-----------|--------|-------------------|---------|
| Insurance fund balance | Hyperliquid public API | `POST /info` → `type: "meta"` | ~100ms |
| Mark price (real-time) | Hyperliquid WebSocket | `subscription: {"type": "allMids"}` | ~50ms |
| Open interest by market | Hyperliquid public API | `POST /info` → `type: "openInterest"` | ~100ms |
| Candlestick history | Hyperliquid public API | `POST /info` → `type: "candleSnapshot"` | ~200ms |
| ADL event confirmation | Hyperliquid WebSocket | `subscription: {"type": "trades"}` — look for ADL flag in trade metadata | ~50ms |
| Leaderboard / large accounts | Hyperliquid public API | `POST /info` → `type: "leaderboard"` | ~200ms |
| Historical on-chain state | Hyperliquid block explorer | `https://explorer.hyperliquid.xyz` | Batch |
| Cross-exchange ADL history (backtest supplement) | BitMEX API | `GET /trade` with `execType: "AdlFulfillment"` | Batch |

---

## Open Questions Before Backtest

1. Does Hyperliquid's API explicitly flag ADL events in the trade stream, or must they be inferred from mark price discontinuities? Confirm with API documentation and test on testnet.
2. What is the historical frequency of ADL events per market? If fewer than 10 events exist for any single market, cross-market pooling is required for the backtest.
3. Is the insurance fund balance updated in real time on-chain, or is there a reporting lag? A lag would reduce the pre-ADL signal's lead time.
4. Has Hyperliquid ever replenished the insurance fund externally (e.g., team injection)? If so, this creates a false-positive risk for the pre-ADL signal that must be modelled.
5. What is the minimum position size that can be entered and exited within the 60-second post-ADL window without moving the market by more than 0.3%? This determines the maximum viable trade size per event.

---

## Next Steps (Pipeline Stage 3)

1. Write data collection script to pull full Hyperliquid history and identify all ADL events. Estimated time: 3 days.
2. Manually verify each identified ADL event against Hyperliquid's public announcements and Discord history.
3. Run post-ADL reversion analysis on confirmed events. Produce distribution of returns at 5/15/30/60/120 minute horizons.
4. If post-ADL reversion shows ≥2% average return after slippage, proceed to pre-ADL signal backtest.
5. If post-ADL reversion is not present in historical data, **kill the strategy at step 3** — do not proceed to paper trading.
