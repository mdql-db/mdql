---
title: "DAI Savings Rate (DSR/SSR) Step-Change — Stablecoin Rotation Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 7
frequency: 2
composite: 420
categories:
  - stablecoin
  - defi-protocol
  - governance
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Maker/Sky governance votes to change the DAI Savings Rate (DSR) or USDS Savings Rate (SSR), a mechanically predictable capital rotation event is scheduled with a publicly visible execution timestamp. The governance spell enters a timelock (typically 24–72 hours) before activation. Rate-sensitive capital — yield aggregators, Morpho vaults, Spark Protocol, Aave sDAI collateral pools, DAO treasuries — does not rotate instantaneously; it migrates over a multi-day window as curators act, vault strategies rebalance, and treasury managers respond.

This creates a **known-direction, known-timing, uncertain-magnitude** flow event. The trade is to position ahead of the mechanical rotation, capture the demand/supply imbalance in sDAI or sUSDS pricing (primarily via Curve pool composition), and exit as the rotation completes.

**The edge is not prediction — it is reading a public schedule.**

---

## Structural Mechanism

### The Plumbing

```
Maker Governance Vote (public, on-chain)
        ↓
Executive Spell enters Timelock (24–72hr, readable at maker.blockanalitica.com)
        ↓
Spell executes → DSR/SSR rate changes (block-precise timestamp known in advance)
        ↓
sDAI/sUSDS yield changes immediately (ERC-4626 accrual rate updates)
        ↓
Yield aggregators (Yearn, Morpho, Spark, Aave) rebalance on their own schedules
        ↓
Curve pool composition shifts over 2–7 days as capital flows in/out
        ↓
sDAI Curve pool premium/discount normalises
```

### Why Capital Does NOT Move Instantly

1. **Vault curator latency**: Morpho and Aave curators (e.g., Block Analitica, Steakhouse) rebalance manually or on daily/weekly cadence — not on-chain triggers
2. **Gas cost friction**: Moving hundreds of millions requires batching; small rate changes don't justify immediate rebalancing for all participants
3. **Awareness lag**: Not all treasury managers monitor governance queues in real time
4. **Redemption queue mechanics**: Some protocols have their own internal timelocks before they can redeploy capital

### The Curve Pool Signal

The primary observable distortion is in the **sDAI/DAI Curve pool** (and sUSDS/USDS equivalent). When rate increases:
- sDAI becomes more attractive → buyers accumulate sDAI → pool skews toward DAI-heavy (sDAI being bought out)
- sDAI trades at a **premium** to NAV in the pool
- This premium is bounded above by the cost of minting sDAI directly (essentially zero slippage for large holders with DAI)

When rate decreases:
- sDAI holders exit → pool skews toward sDAI-heavy
- sDAI trades at a **discount** to NAV
- Discount is bounded below by the redemption path (burn sDAI → DAI, no timelock unlike sUSDe)

**Key structural difference from sUSDe**: sDAI has NO cooldown period. Redemption is instant. This means the discount/premium is purely a flow-timing arb, not a timelock-floor arb. Convergence is faster but the edge window is shorter.

---

## Entry Rules


### Variant A: Rate Increase — Buy sDAI Premium Play

**Entry Trigger:**
- Maker executive spell containing DSR/SSR increase enters timelock
- Rate increase magnitude: ≥ 50bps (smaller changes unlikely to move sufficient capital)
- sDAI is NOT already trading at premium >0.15% on Curve (pre-pricing check)

**Entry Action:**
- Buy sDAI on Curve (or mint directly if pool premium is <0.05%)
- Alternatively: provide DAI-side liquidity to sDAI/DAI Curve pool to capture fee income from incoming flow imbalance
- Position size: see sizing section

## Exit Rules

**Exit Trigger (first of):**
- sDAI Curve pool reaches >60% DAI composition (flow has arrived, premium likely peaked)
- 7 calendar days post spell execution (rotation window assumed complete)
- sDAI premium on Curve exceeds 0.20% (take profit — approaching mint-arb ceiling)
- Governance reversal signal (new spell reducing rate enters queue)

**Exit Action:**
- Sell sDAI on Curve back to DAI, or redeem directly via Maker (instant, no slippage for small sizes)

---

### Variant B: Rate Decrease — Short sDAI Premium / Curve LP Positioning

**Entry Trigger:**
- Maker executive spell containing DSR/SSR decrease enters timelock
- Rate decrease magnitude: ≥ 50bps
- sDAI is NOT already trading at discount >0.10% (pre-pricing check)

**Entry Action:**
- Primary: Provide sDAI-side liquidity to Curve pool (you hold sDAI, earn fees as outflows push pool toward sDAI-heavy)
- Secondary (if Curve LP is unavailable/undesirable): Sell sDAI to DAI on Curve before the outflow begins, redeploy DAI elsewhere, buy back sDAI after discount normalises
- Note: Direct shorting of sDAI is not straightforward — this variant is primarily a LP positioning or early-exit trade, not a leveraged short

**Exit Trigger (first of):**
- sDAI Curve pool reaches >60% sDAI composition (outflow has arrived)
- 7 calendar days post spell execution
- sDAI discount on Curve exceeds 0.15% (buy back — approaching redemption arb floor)

---

### Variant C: Hyperliquid Perp Overlay (DAI or USDS adjacent)

**Mechanism:** If DSR/SSR changes are large enough (>200bps), they affect the relative attractiveness of DAI vs USDC, potentially moving DAI/USDC spot rates on CEXs and affecting stablecoin perp funding rates.

**Entry:** Monitor DAI/USDC spot rate on Coinbase/Kraken. Large rate increase → DAI demand → DAI/USDC approaches 1.001+. If DAI perp funding on Hyperliquid goes negative (shorts being paid), that's a secondary signal.

**Assessment:** This variant is **speculative** and likely too thin to trade. Flag for monitoring only. Score: 4/10 standalone.

---

## Position Sizing

### Constraints
- This is a **low-volatility, low-return** trade. Expected edge per event: 0.05–0.25% on capital deployed
- Position must be sized to make the trade worthwhile given gas costs and opportunity cost
- Minimum viable position: ~$500k notional (at 0.10% edge = $500 gross, minus gas ~$50–200)
- Maximum position: limited by Curve pool depth — do not exceed 5% of pool TVL to avoid self-impact

### Sizing Formula
```
Position = min(
    0.05 × Curve_Pool_TVL,          # liquidity constraint
    0.20 × Available_Capital,        # portfolio risk constraint
    $5,000,000                       # absolute cap (thin edge, no concentration)
)
```

### Expected Return Per Event
| Rate Change | Historical Rotation Volume | Expected Pool Skew | Edge Estimate |
|-------------|---------------------------|-------------------|---------------|
| 50–100bps   | $50–200M                  | 2–5% pool shift   | 0.03–0.08%    |
| 100–200bps  | $200–500M                 | 5–15% pool shift  | 0.08–0.20%    |
| >200bps     | $500M+                    | 15–30% pool shift | 0.15–0.40%    |

*These are hypothesis-stage estimates. Backtest must validate.*

---

## Backtest Methodology

### Data Sources

| Data | Source | Notes |
|------|--------|-------|
| DSR/SSR change history | maker.blockanalitica.com, Dune Analytics | All historical rate changes with block timestamps |
| Spell execution timestamps | Etherscan, Maker governance portal | Exact block of execution |
| sDAI/DAI Curve pool composition | Curve subgraph (The Graph), Dune `@dune/curve` | Pool balance by block |
| sDAI NAV (exchange rate) | Maker MCD contracts, `pot.chi` accumulator | ERC-4626 `convertToAssets` |
| sDAI Curve price vs NAV | Derived: pool spot price ÷ NAV | Premium/discount time series |
| Morpho/Spark vault flows | Morpho subgraph, Spark on-chain events | Rebalancing timestamps |

### Backtest Events to Analyse

Key DSR change events (non-exhaustive, must be verified):

| Date (approx) | Change | Direction | Notes |
|---------------|--------|-----------|-------|
| Feb 2023 | 1% → 3.49% | +249bps | Major increase, sDAI launch era |
| Aug 2023 | 3.49% → 8% | +451bps | Largest single increase |
| Oct 2023 | 8% → 5% | -300bps | Large decrease |
| Jan 2024 | Various | Mixed | Multiple small adjustments |
| Aug 2024 | Sky rebrand | SSR introduced | New instrument |

*Full event list must be pulled from Dune before backtest begins.*

### Backtest Steps

1. **Pull all DSR/SSR change events** with spell execution block numbers
2. **For each event**, extract:
   - T-72hr to T+14d Curve pool composition (hourly)
   - sDAI premium/discount vs NAV (hourly)
   - Total pool TVL at time of event
3. **Simulate entry** at spell-enters-timelock timestamp (T-24hr to T-72hr before execution)
4. **Simulate exit** at each of the exit triggers, record which fires first
5. **Calculate P&L** net of:
   - Curve swap fees (0.04% on sDAI/DAI pool)
   - Gas costs (estimate $100–300 per round trip)
   - Opportunity cost (compare to holding DAI in DSR during same period)
6. **Segment results** by: rate change magnitude, direction (increase vs decrease), pre-existing pool composition at entry, whether market pre-priced during voting period

### Pre-Pricing Detection

Critical question: does the market price in the rate change **during the voting period** (before timelock), making the timelock-entry too late?

Measure: compare sDAI premium at:
- T-7d (before vote announced)
- T-vote (vote passes)
- T-timelock (spell enters timelock)
- T-execution (spell fires)
- T+3d, T+7d (post-execution)

If premium is already fully expressed at T-timelock, the entry signal is degraded. If premium continues to build post-execution (due to aggregator lag), the window is real.

---

## Go-Live Criteria

All of the following must be satisfied:

1. **Backtest shows positive expectancy** across ≥8 historical DSR/SSR change events, net of gas and fees
2. **Median edge per event ≥ 0.08%** on deployed capital (otherwise opportunity cost of capital is not justified)
3. **Pre-pricing analysis shows** that ≥50% of the pool movement occurs AFTER spell execution (confirming aggregator lag is real and exploitable)
4. **No single event shows loss >0.15%** (tail risk check — if a rate decrease causes a panic exit that overwhelms the pool, we need to know)
5. **Curve pool TVL ≥ $50M** at time of live deployment (liquidity floor for position sizing)
6. **Paper trade**: 2 live events observed and tracked in real time before capital deployment

---

## Kill Criteria

Abandon strategy if any of the following occur:

1. **Backtest fails**: fewer than 5 of 8+ events show positive net P&L
2. **Pre-pricing is systematic**: analysis shows >80% of pool movement occurs during voting period, before timelock — edge is front-run by governance watchers
3. **Aggregator automation**: Morpho/Spark/Aave deploy on-chain automation that rebalances within minutes of spell execution (eliminates the lag window)
4. **Pool TVL collapses**: sDAI/DAI Curve pool TVL falls below $20M (insufficient liquidity)
5. **Live paper trade shows**: two consecutive events with zero observable pool skew post-execution
6. **Maker deprecates DSR**: governance moves to a different yield mechanism without a Curve-tradeable instrument

---

## Risks

### Primary Risks

| Risk | Description | Severity | Mitigation |
|------|-------------|----------|------------|
| Pre-pricing | Market prices in rate change during vote, before timelock | HIGH | Backtest pre-pricing analysis; only enter if premium <50% of expected at timelock |
| Aggregator automation | Yield aggregators deploy bots that react within blocks | HIGH | Monitor Morpho/Spark rebalancing latency; kill if lag disappears |
| Governance reversal | Rate changed back before rotation completes | MEDIUM | Monitor governance queue continuously; exit on reversal signal |
| Curve pool migration | sDAI/DAI pool liquidity migrates to new venue (Uniswap v4, etc.) | MEDIUM | Track pool TVL; adjust venue if needed |
| Smart contract risk | Maker PSM, sDAI contract, or Curve pool exploit | LOW-MEDIUM | Use only established contracts; cap position size |
| Gas spike | Ethereum gas spike makes small positions uneconomical | LOW | Pre-calculate minimum viable position given current gas; skip event if uneconomical |
| Stablecoin depeg | DAI or USDS depegs during holding period | LOW | This is a DAI-denominated trade; depeg affects both sides equally in most scenarios |

### Structural Risk: The Edge May Already Be Gone

The most important risk is that this edge was real in 2022–2023 when DSR changes were novel and aggregators were slow, but has since been automated away. The backtest must specifically test **recent events (2024–2025)** separately from early events to detect edge decay.

---

## Data Sources

```
PRIMARY:
- Dune Analytics: @steakhouse/maker-dsr-history (DSR rate history)
- Dune Analytics: @dune/curve-pool-composition (pool balance time series)
- maker.blockanalitica.com: governance spell queue, execution timestamps
- The Graph (Curve subgraph): pool reserves by block
- Etherscan: MakerDAO MCD_POT contract events (Drip calls, file calls)

SECONDARY:
- Morpho subgraph: vault rebalancing events and timestamps
- Spark Protocol analytics: sDAI TVL time series
- DefiLlama: sDAI TVL across protocols (daily)
- Maker forum (forum.makerdao.com): governance discussion timing

MONITORING (for live trading):
- maker.blockanalitica.com/governance: spell queue with countdown timers
- Tenderly alerts: MCD_SPELL contract execution
- Curve pool monitor: custom Dune query or Curve API for real-time pool composition
```

---

## Implementation Notes

### Monitoring Setup (Pre-Live)
1. Set up Tenderly webhook on MakerDAO governance contracts to alert when new executive spell is submitted
2. Build Dune query: real-time sDAI Curve pool composition + NAV premium/discount
3. Build event log: every DSR/SSR change since sDAI launch with execution block

### Execution Path (Live)
- **Entry**: Curve UI or 1inch for sDAI purchase; direct Maker PSM mint for large sizes
- **Exit**: Curve UI for sDAI → DAI; direct Maker redemption for large sizes (instant, no fee)
- **No Hyperliquid perp required** for core variant — this is a spot/LP strategy
- Hyperliquid perp overlay (Variant C) is a separate, lower-conviction add-on

### Frequency Expectation
DSR/SSR changes occur approximately **4–12 times per year** in active governance periods. This is a low-frequency, event-driven strategy. It is not a continuous position.

---

## Open Questions for Backtest

1. What percentage of pool movement occurs before vs after spell execution? (Pre-pricing test)
2. Is there a minimum rate change threshold below which no observable pool movement occurs?
3. Do rate increases and decreases show asymmetric edge (exits faster than entries, or vice versa)?
4. Has the edge decayed in 2024–2025 vs 2022–2023?
5. Is the sUSDS/USDS Curve pool (Sky rebrand) showing similar dynamics to the original sDAI pool?
6. What is the correlation between rate change magnitude and pool skew magnitude?

---

## Next Steps

| Step | Action | Owner | Timeline |
|------|--------|-------|----------|
| 1 | Pull complete DSR/SSR change event log from Dune | Researcher | 1 day |
| 2 | Build sDAI Curve pool composition time series (hourly, 2022–present) | Researcher | 2 days |
| 3 | Align event timestamps with pool composition data | Researcher | 1 day |
| 4 | Run pre-pricing analysis (when does pool move relative to spell lifecycle?) | Researcher | 2 days |
| 5 | Simulate entry/exit P&L for all events | Researcher | 2 days |
| 6 | Segment by year to detect edge decay | Researcher | 1 day |
| 7 | Decision: proceed to paper trade or kill | Zunid | After step 6 |

**Estimated time to backtest decision: 7–10 working days**

---

*This document represents a hypothesis. No backtest has been run. No live trading should occur until go-live criteria are satisfied. The structural mechanism is sound in theory; whether the edge survives pre-pricing and aggregator automation is the central empirical question.*
