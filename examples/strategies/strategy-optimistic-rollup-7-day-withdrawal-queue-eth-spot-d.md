---
title: "Optimistic Rollup 7-Day Withdrawal Queue — ETH Spot Discount Arb"
status: HYPOTHESIS
mechanism: 5
implementation: 4
safety: 6
frequency: 3
composite: 360
categories:
  - basis-trade
  - funding-rates
  - defi-protocol
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## 1. One-Line Summary

When fast-exit bridge fees for ETH L2→L1 withdrawals are cheaper than 7-day ETH perp funding carry, collect the spread by paying the bridge fee, receiving L1 ETH immediately, and going long ETH perp to earn funding — a mechanical arbitrage between two prices for the same underlying constraint: the cost of time.

---

## 2. Hypothesis

The 7-day optimistic rollup challenge window is a hard protocol constraint. It creates a two-tier market for ETH liquidity:

- **Tier 1 (slow):** Wait 7 days, pay no fee, receive L1 ETH at par
- **Tier 2 (fast):** Pay a bridge LP fee (0.05–0.20%), receive L1 ETH immediately

Bridge LPs who provide Tier 2 liquidity are implicitly pricing the 7-day opportunity cost of their capital. Their fee floor is:

```
bridge_fee_floor = risk_free_rate × (7/365) + operational_risk_premium
```

When ETH perp funding rates are elevated, the opportunity cost of capital locked for 7 days rises — bridge LPs *should* raise fees. But if bridge fees lag funding rate changes (due to LP inertia, competition dynamics, or fee update latency), a window opens where:

```
bridge_fee_annualised < eth_perp_funding_rate_annualised
```

In that window, an arbitrageur can:
1. Pay the bridge fee to receive L1 ETH immediately
2. Deploy that ETH as collateral to go long ETH perp
3. Collect funding payments for the duration
4. Net profit = funding_collected - bridge_fee_paid - execution_costs

**This is not a bet on ETH price direction.** The perp long hedges spot ETH exposure. The trade is purely a carry capture on the spread between two prices for the same 7-day time value.

---

## 3. Structural Mechanism

### Why the constraint is real

The 7-day challenge window is enforced at the smart contract level on Ethereum L1. No user, sequencer, or bridge can bypass it without a trusted intermediary absorbing the delay. This is not a convention — it is a cryptographic and economic security requirement of the optimistic fraud proof system. As long as Arbitrum, Optimism, and Base operate as optimistic rollups, this constraint exists.

### Why the mispricing can occur

Bridge LP fees are not continuously updated by an automated market maker responding to funding rates. They are set by:

- **Hop Protocol:** Fee parameters governed by DAO or set by LP pools with discrete update cycles
- **Across Protocol:** Fees determined by a relayer network with capital utilisation-based pricing, updated per-quote but with smoothing

Neither system is directly wired to ETH perp funding rates. The connection is indirect: LPs notice their opportunity cost rising when funding is high and eventually reprice — but this repricing lags. The lag is the edge.

### Why the edge is bounded and self-limiting

Once the spread is large enough to attract capital, LPs raise fees or new LPs enter, compressing the spread. This means:

- The edge is episodic, not permanent
- It is largest during sudden funding rate spikes before LP repricing catches up
- It self-corrects, which is a feature (it means the mechanism is real) not a bug

### Structural analogy

This is the same structure as covered interest rate parity violations in FX: the forward rate *should* price the interest rate differential, but when it doesn't, carry traders close the gap. Here, the bridge fee *should* price the 7-day funding carry, but when it doesn't, this trade closes the gap.

---

## 4. Entry Rules

### Signal conditions (all must be true simultaneously)

| Condition | Threshold | Data Source |
|---|---|---|
| ETH perp funding rate (annualised) | > 15% APR | Hyperliquid funding API |
| Fast-exit bridge fee (annualised) | < ETH perp funding rate − 5% APR buffer | Hop/Across fee API |
| Bridge fee quote is executable | Minimum size met ($5k+) | Live quote from bridge UI/API |
| ETH perp funding rate trend | Positive or flat for prior 24h | Hyperliquid historical funding |
| No active bridge security incidents | Manual check | Bridge status pages / Twitter |

### Spread calculation

```
gross_spread_annualised = eth_perp_funding_annualised - bridge_fee_annualised
net_spread = gross_spread - gas_costs_annualised - execution_slippage_estimate

Signal fires when: net_spread > 3% APR (minimum viable after costs)
```

### Entry execution sequence

1. **Quote bridge fee** — pull live quote from Hop and Across APIs for target ETH amount; use the cheaper of the two
2. **Check funding rate** — confirm current 8h funding rate on Hyperliquid ETH-USDC perp; annualise it
3. **Compute net spread** — if net_spread > 3% APR, proceed
4. **Execute bridge withdrawal** — initiate L2→L1 fast-exit on the cheaper bridge; confirm receipt of L1 ETH
5. **Open perp long** — immediately after L1 ETH receipt, open ETH-USDC long on Hyperliquid equal to ETH received; this hedges spot price exposure
6. **Log entry** — record: bridge_fee_paid, eth_amount, perp_entry_price, funding_rate_at_entry, timestamp

---

## 5. Exit Rules

### Primary exit: funding rate normalisation

- **Trigger:** ETH perp funding rate (annualised) drops below bridge_fee_annualised + 2% buffer
- **Action:** Close perp long; hold L1 ETH (already received); trade complete
- **Rationale:** The carry has compressed; no reason to maintain the hedge

### Secondary exit: time-based

- **Trigger:** 7 calendar days elapsed since bridge withdrawal initiated
- **Action:** Close perp long regardless of funding rate
- **Rationale:** The original 7-day window has passed; the bridge LP has now claimed their L1 ETH; the structural context of the trade has resolved

### Emergency exit: funding rate reversal

- **Trigger:** ETH perp funding rate turns negative (shorts paying longs)
- **Action:** Close perp long immediately; accept partial loss on bridge fee
- **Rationale:** Negative funding means the perp long is now paying funding, not receiving it; the carry has inverted

### Exit execution

1. Close ETH perp long on Hyperliquid at market
2. Record: perp_exit_price, total_funding_received, net_PnL
3. L1 ETH remains in wallet — redeploy or hold as desired

---

## 6. Position Sizing

### Constraints

- **Minimum size:** $5,000 ETH equivalent (bridge LP minimums; below this, gas costs dominate)
- **Maximum size:** $500,000 ETH equivalent per trade (above this, bridge liquidity may be insufficient for a single quote; split across Hop and Across)
- **Perp collateral:** Use received L1 ETH as collateral for the perp long (or USDC equivalent); leverage = 1× (delta-neutral intent)

### Sizing formula

```
position_size = min(
    available_bridge_liquidity × 0.25,   # don't consume >25% of LP pool
    max_perp_position_at_1x_leverage,
    $500,000
)
```

### Kelly-adjusted sizing

Until backtest data exists, use **fractional Kelly at 10%** of theoretical optimal. This is a new, unvalidated strategy — capital preservation takes priority over maximising expected value.

```
initial_allocation = total_risk_capital × 0.05
```

Scale up only after 10+ live trades with positive expectancy confirmed.

---

## 7. Backtest Methodology

### What we are testing

1. How often does the signal condition fire (net_spread > 3% APR)?
2. What is the average net spread when it fires?
3. How long does the spread persist before funding normalises?
4. What is the distribution of outcomes (funding collected minus bridge fee)?

### Data required

| Dataset | Source | Availability | Notes |
|---|---|---|---|
| ETH perp 8h funding rate history | Hyperliquid API | Free, full history | Annualise: rate × 3 × 365 |
| Hop Protocol bridge fee history | Hop API / on-chain events | Partial — fee quotes not stored historically | Reconstruct from LP pool state changes |
| Across Protocol fee history | Across API / Dune | Partial — relayer fee events on-chain | Dune dashboard exists |
| ETH gas costs (L1) | Etherscan gas oracle history | Free, full history | For cost adjustment |

### Data gap: historical bridge fees

Historical bridge fee quotes are not stored in a queryable API. **Workaround:**

1. Use Dune Analytics to reconstruct Hop/Across LP utilisation rates over time (utilisation is the primary driver of fees)
2. Use the fee formula published in Hop/Across documentation to back-calculate implied fees from utilisation
3. Cross-reference with any available fee event logs on-chain

This is imperfect but sufficient for hypothesis validation. Flag reconstructed data clearly in results.

### Backtest period

- **Primary:** January 2023 – present (post-Merge, post-Arbitrum launch maturity)
- **Focus periods:** Episodes of high ETH funding (bull runs, leverage spikes) — these are when the signal should fire

### Backtest steps

```
Step 1: Load hourly ETH perp funding rate data (Hyperliquid)
Step 2: Annualise each observation
Step 3: Load reconstructed bridge fee data (Hop/Across utilisation → fee)
Step 4: Compute hourly net_spread = funding_annualised - bridge_fee_annualised - cost_buffer
Step 5: Identify signal events: net_spread > 3% APR for 2+ consecutive hours
Step 6: For each signal event, simulate:
    - Pay bridge fee at signal time
    - Collect funding for min(days_until_funding_normalises, 7) days
    - Compute net PnL per trade
Step 7: Aggregate: win rate, average PnL, max drawdown, Sharpe
Step 8: Sensitivity analysis: vary the 3% threshold, 7-day cap, exit trigger
```

### Success criteria for backtest

| Metric | Minimum bar to proceed |
|---|---|
| Signal fires | ≥ 20 times in backtest period |
| Win rate | > 65% |
| Average net spread captured | > 2% APR after costs |
| Max single-trade loss | < bridge_fee_paid (i.e., worst case = fee lost, not more) |
| Sharpe (annualised) | > 1.0 |

---

## 8. Go-Live Criteria

Before allocating real capital beyond paper trading:

- [ ] Backtest completed with reconstructed data; meets success criteria above
- [ ] Live signal monitoring script operational (Hop API + Across API + Hyperliquid funding API, polling every 15 minutes)
- [ ] Manual bridge execution tested with $1,000 test trade on each of Hop and Across (confirm fee quotes match API, confirm L1 receipt timing)
- [ ] Perp execution tested: confirm 1× ETH long can be opened on Hyperliquid within 5 minutes of L1 ETH receipt
- [ ] Emergency exit procedure documented and tested (negative funding → close perp immediately)
- [ ] Bridge smart contract risk assessed: review audit reports for Hop and Across; confirm no active security advisories
- [ ] Paper trade: 5 signal events tracked in real time without capital; confirm signal fires and spread is real before live deployment

---

## 9. Kill Criteria

Abandon the strategy if any of the following are confirmed:

| Kill Condition | What it means |
|---|---|
| Signal fires < 5 times per year in backtest | Opportunity is too rare to build infrastructure for |
| Bridge fees consistently track funding rates within 1% APR | LPs have already automated repricing; lag is gone |
| Backtest win rate < 55% | Mechanism is real but costs eat the edge |
| Live paper trades show bridge fee quotes differ materially from API quotes | API data is not reliable for signal generation |
| ETH perp funding rate on Hyperliquid is structurally low (<5% APR) for >6 months | The carry environment has changed; revisit sizing |
| Hop or Across protocol suffers a security exploit | Bridge smart contract risk is unacceptable; suspend immediately |
| Optimistic rollup challenge window is reduced (e.g., via governance upgrade to 3 days or 1 day) | The structural constraint weakens; recalibrate bridge fee floor |

---

## 10. Risks

### Risk 1: Bridge smart contract exploit
**Severity:** Catastrophic (total loss of bridged capital)
**Mitigation:** Use only audited, battle-tested bridges (Hop, Across). Cap single-bridge exposure at $100k. Monitor bridge security channels actively. Never bridge more than 20% of total strategy capital simultaneously.

### Risk 2: Funding rate reversal mid-trade
**Severity:** Moderate (lose bridge fee + pay some funding instead of receiving it)
**Mitigation:** Emergency exit rule (close perp immediately on negative funding). Maximum loss on any trade is bounded: bridge_fee_paid + funding_paid_while_negative. With 1× leverage, no liquidation risk.

### Risk 3: Bridge fee API data is stale or wrong
**Severity:** Low-moderate (enter trade with incorrect spread calculation)
**Mitigation:** Cross-check API quote with manual UI quote before executing. Require both to agree within 0.02% before proceeding.

### Risk 4: L1 gas costs spike during bridge execution
**Severity:** Low (gas cost is a small fraction of trade size at $5k+)
**Mitigation:** Include a gas cost buffer in the net_spread calculation. If L1 gas > 50 gwei, add 0.05% to cost estimate.

### Risk 5: Bridge LP pool is depleted (can't get a quote)
**Severity:** Low (trade simply doesn't execute)
**Mitigation:** Check both Hop and Across. If neither has liquidity, skip the signal event. This is a capacity constraint, not a loss event.

### Risk 6: Regulatory risk on perp trading
**Severity:** Jurisdiction-dependent
**Mitigation:** Execute perps on Hyperliquid (decentralised). Standard operational risk for any perp strategy.

### Risk 7: The edge is too small to be worth the operational complexity
**Severity:** Medium (opportunity cost of time spent)
**Mitigation:** Kill condition: if net_spread < 3% APR consistently, the strategy doesn't meet the minimum return threshold for its complexity. Abandon and redeploy capital.

---

## 11. Data Sources

| Data | Source | Access | Update Frequency |
|---|---|---|---|
| ETH perp funding rate | Hyperliquid REST API (`/info` endpoint, `fundingHistory`) | Free, no auth | Every 8 hours (funding settlement) |
| Hop bridge fee quotes | Hop Protocol SDK / REST API | Free, no auth | Per-quote (real-time) |
| Across bridge fee quotes | Across Protocol API (`/suggested-fees`) | Free, no auth | Per-quote (real-time) |
| Hop historical LP utilisation | Dune Analytics (Hop dashboards) | Free with account | Daily |
| Across historical relayer fees | Dune Analytics (Across dashboards) | Free with account | Daily |
| ETH L1 gas prices | Etherscan Gas Oracle API | Free tier available | Real-time |
| Optimistic rollup challenge window status | L2Beat.com / Arbitrum/OP docs | Public | Manual check monthly |
| Bridge security advisories | Hop Discord / Across Discord / Twitter | Manual monitoring | As needed |

---

## 12. Implementation Notes

### Monitoring script (pseudocode)

```python
while True:
    funding_rate_8h = hyperliquid.get_funding_rate("ETH")
    funding_annualised = funding_rate_8h * 3 * 365  # 3 periods/day, 365 days

    hop_fee = hop_api.get_quote(amount=target_eth, from_chain="arbitrum", to_chain="mainnet")
    across_fee = across_api.get_quote(amount=target_eth, from_chain="arbitrum", to_chain="mainnet")
    best_bridge_fee = min(hop_fee.annualised, across_fee.annualised)

    gas_cost_buffer = estimate_gas_cost_annualised(target_eth)
    net_spread = funding_annualised - best_bridge_fee - gas_cost_buffer - 0.02  # 2% safety buffer

    if net_spread > 0.03:  # 3% APR minimum
        alert("SIGNAL: Bridge arb opportunity detected")
        log(funding_annualised, best_bridge_fee, net_spread, timestamp)

    sleep(900)  # poll every 15 minutes
```

### Key implementation constraint

This strategy requires **manual execution** for the bridge step (or a custom bridge integration). The perp leg can be automated via Hyperliquid API. The bridge leg requires either:
- Manual execution via bridge UI (acceptable for $50k+ trades where the spread justifies the time)
- Custom integration with Hop/Across SDK (recommended if signal fires frequently)

Do not automate the bridge leg without extensive testing — a bug in bridge automation could result in funds sent to wrong address or lost in a failed transaction.

---

## 13. Next Steps

1. **Data acquisition:** Pull Hyperliquid ETH funding rate history (full available history) and Dune bridge utilisation data
2. **Fee reconstruction:** Build the bridge_fee ≈ f(utilisation) model from Hop/Across documentation and validate against any available historical quotes
3. **Backtest execution:** Run the signal identification and PnL simulation per methodology in Section 7
4. **Review backtest results:** If success criteria met, proceed to paper trading phase
5. **Paper trade:** Monitor signal in real time for 30 days; log every signal event; confirm spread is real before live deployment
6. **Go/no-go decision:** After paper trading, review against go-live criteria in Section 8

**Estimated time to backtest completion:** 2–3 weeks (data acquisition is the bottleneck)
**Estimated time to paper trade completion:** 30–60 days (depends on signal frequency)
