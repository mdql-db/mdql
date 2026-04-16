---
title: "Airdrop Cliff Sell — Claim-and-Dump Window Mapping"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 1
composite: 125
categories:
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Airdrop claim deadlines create a contractually enforced concentration of sell pressure in a narrow time window. Recipients who intend to sell but procrastinate are forced to act in the final 48–72 hours before unclaimed tokens are permanently forfeited or returned to treasury. This behavioral clustering — caused by the deadline mechanism, not sentiment — produces a measurable, repeatable price dip. After the deadline, unclaimed tokens are removed from circulating supply, creating a deflationary snap-back. Both legs (pre-deadline dump and post-deadline contraction) are tradeable.

**Null hypothesis to disprove:** Claim-and-sell activity is uniformly distributed across the claim window, producing no concentrated price impact near the deadline.

---

## Structural Mechanism

### Why this is not just a pattern

The deadline is written into the airdrop smart contract. The forfeiture of unclaimed tokens at expiry is a deterministic on-chain event — it is not probabilistic. What is probabilistic is the *magnitude* of the sell pressure, which depends on:

1. **Unclaimed balance at T-72h** — the larger the unclaimed pool, the larger the potential dump
2. **Recipient composition** — wallets that claimed but haven't sold vs. wallets that haven't claimed at all
3. **Token liquidity** — thin order books amplify the impact

### The causal chain

```
Deadline approaches (T-72h)
        ↓
Procrastinating recipients receive deadline reminders
(protocol announcements, community posts, wallet notifications)
        ↓
Claim volume spikes — on-chain observable in real time
        ↓
Claimers immediately sell (airdrop recipients have near-zero cost basis)
        ↓
Concentrated sell pressure in 48–72h window → price dip
        ↓
Deadline passes → unclaimed tokens forfeited
        ↓
Circulating supply contracts by [unclaimed %]
        ↓
Supply shock absorbed → price recovers (snap-back)
```

### Why the snap-back is real

If 10% of total airdrop allocation goes unclaimed, that 10% is permanently removed from circulating supply. This is economically equivalent to a 10% token burn relative to the airdrop tranche. The market frequently fails to price this in advance because unclaimed % is not known until the deadline passes — it is an information asymmetry that resolves at a known time.

### Why this edge persists

- Most traders focus on the *initial* airdrop distribution event, not the deadline
- Unclaimed % data requires active on-chain monitoring — most participants don't track it
- The deadline is often buried in documentation, not prominently marketed
- The snap-back leg is counterintuitive (buying after a dump requires conviction)

---

## Universe & Event Filters

Apply **all** of the following filters before trading any event:

| Filter | Threshold | Rationale |
|---|---|---|
| Unclaimed balance at T-72h | ≥ 5% of total airdrop allocation | Below this, supply impact is noise |
| Airdrop allocation as % of total supply | ≥ 2% | Small allocations produce negligible price impact |
| Token has liquid perp market | Open interest ≥ $5M | Required for short leg execution |
| Time since initial airdrop distribution | ≥ 14 days | Avoids overlap with initial distribution volatility |
| Claim deadline is on-chain verifiable | Required | No trading on rumored or unconfirmed deadlines |
| Token not in active governance vote | Required | Governance events create confounding flows |

**Expected qualifying events:** 4–8 per year based on major protocol airdrops (Arbitrum, Optimism, Uniswap, ENS, dYdX, Starknet, Eigenlayer scale events).

---

## Entry Rules

### Leg 1: Pre-Deadline Short

**Entry trigger:** Unclaimed balance at T-72h exceeds 5% of total airdrop allocation AND claim volume begins accelerating (on-chain observable as rising transaction rate to claim contract).

**Entry timing:** Open short position at T-60h (60 hours before deadline). Do not enter at T-72h — claim volume spike confirmation takes ~12 hours to validate.

**Entry execution:** Market order on perp (Hyperliquid). Do not use limit orders — the entry window is time-sensitive and slippage at entry is acceptable given expected move magnitude.

**Entry price check:** Do not enter if token has already declined >8% in the prior 24h — the move may be partially priced in and risk/reward deteriorates.

### Leg 2: Post-Deadline Long (Optional, Independent Position)

**Entry trigger:** Deadline has passed AND final unclaimed % confirmed on-chain ≥ 8% of total airdrop allocation.

**Entry timing:** T+6h to T+12h after deadline. Allow 6 hours for on-chain confirmation of forfeiture transaction and for initial post-deadline volatility to settle.

**Entry execution:** Limit order 1–2% below current market price. The snap-back is not urgent — patient entry improves risk/reward.

**Leg 2 is independent:** Do not enter Leg 2 simply because Leg 1 was profitable. Evaluate unclaimed % independently.

---

## Exit Rules

### Leg 1 Exit

**Primary exit:** Cover short at T+6h after deadline (6 hours post-deadline). This captures the dump window and exits before snap-back begins.

**Accelerated exit:** If price drops >12% from entry before deadline, cover 50% of position immediately (take partial profit, reduce risk of snap-back catching the short).

**Stop loss:** If price rises >5% from entry at any point before T-12h, close entire position. The mechanism has failed or been front-run.

**Time stop:** If deadline is extended by protocol (rare but possible), close position immediately at market — the structural constraint has been removed.

### Leg 2 Exit

**Primary exit:** Close long 48–72h after entry. The snap-back is a short-duration effect; holding longer introduces unrelated market risk.

**Stop loss:** If price falls >6% from Leg 2 entry, close position. The supply contraction thesis has failed to materialize.

**Profit target:** 8–15% gain from entry. Do not hold for larger moves — this is a mechanical event trade, not a trend position.

---

## Position Sizing

### Base sizing formula

```
Position size = (Account risk per trade) / (Stop loss distance in %)

Account risk per trade = 1.5% of total account
Stop loss distance = 5% (Leg 1), 6% (Leg 2)

Example: $100,000 account
Leg 1 size = ($1,500) / (0.05) = $30,000 notional
Leg 2 size = ($1,500) / (0.06) = $25,000 notional
```

### Leverage cap

Maximum 3x leverage on either leg. Airdrop events can produce unexpected volatility spikes; higher leverage creates unacceptable liquidation risk.

### Concentration cap

No single event to exceed 3% of total account in combined risk across both legs. If Leg 1 and Leg 2 are both active simultaneously, total risk = 3% of account, not 1.5% each.

### Scaling rule

If unclaimed % at T-72h exceeds 15% of total airdrop allocation, scale position to 1.75x base size. This is the only permitted scaling trigger — do not scale based on conviction or recent P&L.

---

## Backtest Methodology

### Step 1: Build the event database

Compile all major protocol airdrops from 2020–present with confirmed claim deadlines. Minimum dataset: 30 events. Sources:
- Dune Analytics: query claim contract addresses for transfer-to-zero events (forfeiture) and cumulative claim rates over time
- Etherscan: token transfer logs to/from claim contract addresses
- Protocol documentation and governance forums for deadline confirmation
- CoinGecko/CoinMarketCap: hourly price data around each deadline

Target events include (non-exhaustive): UNI (2020), ENS (2022), OP (multiple rounds), ARB (2023), dYdX (2021), BLUR (2023), STRK (2024), EIGEN (2024).

### Step 2: Construct the unclaimed % time series

For each event, calculate unclaimed balance as % of total airdrop allocation at T-72h, T-48h, T-24h, and T=0 (deadline). This is the primary independent variable.

### Step 3: Measure price impact

Calculate token price return from T-60h entry to T+6h exit for Leg 1. Calculate token return from T+6h to T+72h for Leg 2. Normalize against BTC return over the same window to isolate token-specific moves from market beta.

### Step 4: Regression analysis

Regress price impact (Leg 1 and Leg 2 separately) against:
- Unclaimed % at T-72h
- Airdrop allocation as % of total supply
- Token liquidity (average daily volume / market cap)
- Days since initial distribution

Identify which filters produce the strongest signal. Adjust filter thresholds based on regression output, not intuition.

### Step 5: Simulate execution

Apply 0.1% slippage on entry and exit for perp trades. Apply 0.05% funding rate cost per 8-hour period held. Calculate net P&L per event after costs.

### Step 6: Evaluate

Minimum acceptable backtest results to proceed to paper trading:
- Win rate ≥ 55% on Leg 1
- Average win/loss ratio ≥ 1.5 on Leg 1
- Positive expected value on Leg 2 independently
- No single event loss exceeds 2x average win

---

## Go-Live Criteria

All of the following must be satisfied before live trading:

1. **Backtest complete** on ≥ 20 qualifying events (post-filter) with positive expected value
2. **Paper trade** ≥ 3 live events with documented entry/exit timestamps and on-chain claim data
3. **Monitoring infrastructure live:** automated Dune query or on-chain alert for claim rate acceleration and unclaimed balance at T-72h
4. **Execution infrastructure live:** Hyperliquid API access with pre-configured order templates for short entry and cover
5. **Deadline calendar maintained:** rolling 90-day forward calendar of known airdrop deadlines, reviewed weekly

---

## Kill Criteria

Suspend strategy immediately if any of the following occur:

| Trigger | Action |
|---|---|
| 3 consecutive losing events on Leg 1 | Halt, review mechanism, do not trade until root cause identified |
| Single event loss > 3x average backtest loss | Halt, review position sizing and stop loss rules |
| Unclaimed % filter produces < 2 qualifying events in 6 months | Review filter thresholds — universe may have shrunk |
| Protocol begins extending deadlines routinely | Mechanism is broken — structural constraint no longer reliable |
| Perp funding rate on short exceeds 0.15% per 8h at entry | Skip event — carry cost destroys expected value |

---

## Risks

### Risk 1: Deadline extension (HIGH IMPACT, LOW PROBABILITY)
Protocols occasionally extend claim deadlines due to community pressure or technical issues. This removes the structural constraint entirely. **Mitigation:** Monitor governance forums and protocol announcements in the 7 days before deadline. Close position immediately if extension is announced.

### Risk 2: Front-running by sophisticated actors (MEDIUM IMPACT, MEDIUM PROBABILITY)
If this strategy becomes widely known, the dump may occur earlier than T-60h, making the entry stale. **Mitigation:** Monitor claim acceleration starting at T-96h. If claim volume spikes unusually early, adjust entry to T-84h or skip the event.

### Risk 3: Thin perp liquidity (HIGH IMPACT, MEDIUM PROBABILITY)
Smaller tokens with qualifying unclaimed balances may not have liquid perp markets, making entry/exit costly. **Mitigation:** The $5M open interest filter is mandatory. Do not waive it.

### Risk 4: Correlated market crash (MEDIUM IMPACT, LOW PROBABILITY)
A broad market crash during the trade window will overwhelm the token-specific signal. **Mitigation:** Hedge with a small BTC short (25% of notional) during Leg 1 to neutralize market beta. Close BTC hedge at same time as Leg 1 cover.

### Risk 5: Unclaimed % data lag (LOW IMPACT, HIGH PROBABILITY)
On-chain data may lag real-time by 15–30 minutes depending on indexer. **Mitigation:** Use direct RPC calls to claim contract for real-time balance queries, not third-party dashboards, for live trading decisions.

### Risk 6: Snap-back fails to materialize (MEDIUM IMPACT, MEDIUM PROBABILITY)
The market may not reprice the supply contraction, especially if unclaimed % is small or widely anticipated. **Mitigation:** Leg 2 is optional and independently evaluated. Never enter Leg 2 below the 8% unclaimed threshold.

---

## Data Sources

| Data type | Source | Cost | Latency |
|---|---|---|---|
| Claim contract events | Etherscan API, direct RPC | Free / $50/mo | 15–30 min |
| Unclaimed balance time series | Dune Analytics (custom query) | Free tier sufficient | 1–2h |
| Token price (hourly) | CoinGecko API | Free | 5 min |
| Airdrop deadline calendar | Protocol docs, governance forums, Airdrops.io | Free | Manual |
| Perp open interest | Hyperliquid API | Free | Real-time |
| Funding rates | Hyperliquid API, Coinglass | Free | Real-time |
| BTC price (hedge) | Hyperliquid API | Free | Real-time |

**Dune query template (pseudocode):**
```sql
SELECT
  date_trunc('hour', block_time) as hour,
  COUNT(*) as claim_txns,
  SUM(amount) as tokens_claimed,
  (total_allocation - SUM(amount) OVER (ORDER BY block_time)) as unclaimed_balance
FROM claim_contract_events
WHERE contract_address = '0x...'
  AND block_time >= (deadline - INTERVAL '7 days')
GROUP BY 1
ORDER BY 1
```

---

## Open Questions for Backtest Phase

1. Does claim volume acceleration (rate of change) predict price impact better than absolute unclaimed balance?
2. Is the Leg 2 snap-back stronger when unclaimed tokens are burned vs. returned to treasury? (Burn is permanent supply reduction; treasury return is not.)
3. Do tokens with higher % of airdrop recipients who are known "farmers" (wallets that claim and immediately sell across multiple airdrops) show stronger Leg 1 signal?
4. What is the optimal entry timing — T-60h as specified, or does earlier entry (T-84h) capture more of the move with less competition?
5. Does the strategy perform differently in bull vs. bear market regimes?

---

## Next Actions

| Action | Owner | Deadline |
|---|---|---|
| Build Dune query for 10 historical airdrop claim contracts | Researcher | 1 week |
| Compile event database with deadlines and unclaimed % | Researcher | 2 weeks |
| Run regression on price impact vs. unclaimed % | Researcher | 3 weeks |
| Evaluate Leg 2 (snap-back) independently | Researcher | 3 weeks |
| Present backtest results for go/no-go decision | Researcher | 4 weeks |
