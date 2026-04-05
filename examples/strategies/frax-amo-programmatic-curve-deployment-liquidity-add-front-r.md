---
title: "Frax AMO Programmatic Curve Deployment — Liquidity Add Front-Run"
status: HYPOTHESIS
mechanism: 5
implementation: 3
safety: 4
frequency: 3
composite: 180
categories:
  - defi-protocol
  - stablecoin
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## 1. One-Line Summary

When Frax AMO contracts approach their programmatic deployment thresholds — readable from on-chain state — position in the destination Curve pool's paired asset or LP tokens before the liquidity addition reprices pool composition, then exit once the AMO flow completes and pool depth stabilises.

---

## 2. Hypothesis

Frax's AMO contracts are rule-based autonomous systems that mint and deploy FRAX into Curve pools when FRAX trades above peg, and withdraw when below. These are not discretionary decisions — they are deterministic smart contract executions triggered by observable on-chain parameters. The trigger conditions (collateral ratio, peg deviation threshold, AMO utilisation caps) are readable from contract state before execution occurs. A non-HFT participant with automated monitoring can observe the approach to trigger thresholds and position ahead of the flow. The AMO flow itself is the edge — not a prediction about price, but a known mechanical event that will alter pool composition in a predictable direction.

**Null hypothesis to disprove:** AMO flows are either (a) too small relative to pool depth to move LP composition meaningfully, or (b) already priced in by the time any non-MEV participant can act.

---

## 3. Structural Mechanism — Detailed

### 3.1 What the AMO Does

Frax's Curve AMO (deployed at documented contract addresses on Ethereum mainnet) operates as follows:

- **Mint-and-deploy mode (FRAX > $1.000 + threshold):** AMO mints new FRAX and deposits it into designated Curve metapools (historically frax3CRV, fraxBP). This increases pool depth, reduces FRAX's premium by increasing FRAX supply in the pool, and shifts pool composition toward FRAX (reducing the share of USDC/USDT/3CRV).
- **Withdraw-and-burn mode (FRAX < $1.000 - threshold):** AMO removes FRAX liquidity from Curve pools, reducing depth, and burns withdrawn FRAX. This thins the pool, increases slippage for FRAX sellers, and mechanically reduces USDC/paired-asset share in the pool.

### 3.2 Why This Creates a Tradeable Distortion

When the AMO adds FRAX to a Curve pool:

1. Pool composition shifts: FRAX proportion increases, paired asset (USDC) proportion decreases relative to total pool value.
2. LP token holders who entered before the AMO addition now hold a pool with a higher FRAX concentration — their effective USDC exposure has decreased.
3. The Curve invariant means that as FRAX is added, the marginal price of FRAX within the pool decreases slightly — this is the peg-restoration mechanism.
4. **The tradeable moment:** In the window between "AMO threshold is approaching" (readable from state) and "AMO transaction executes," the pool composition has not yet shifted. A participant who adds USDC liquidity to the pool before the AMO addition will find their LP position has a higher USDC weight post-AMO (because AMO dilutes the FRAX side, not the USDC side). They can exit LP with a higher USDC proportion than they entered with — a mechanical, pool-math-driven gain.

Alternatively, on the withdrawal side: before AMO withdraws (peg stress), the pool is about to thin. Exiting LP before withdrawal avoids the impermanent loss from pool thinning and avoids being the liquidity that absorbs FRAX selling pressure.

### 3.3 The Information Asymmetry

The AMO trigger parameters are on-chain. Specifically:
- `collateral_ratio` — readable from FraxlendPairCore or Frax main contract
- `amo_minted_frax` — tracks how much FRAX the AMO has deployed
- `frax_price` — Chainlink oracle feed used by the protocol
- AMO utilisation caps — maximum FRAX the AMO can deploy, readable from AMO contract

The gap between "threshold approaching" and "transaction confirmed" is not milliseconds — AMO operations are governance-rate or keeper-triggered, not block-by-block. This creates a window measurable in minutes to hours, not microseconds.

---

## 4. Variants

| Variant | Action | Trigger | Exit |
|---|---|---|---|
| **4A — LP Front-Run (Add)** | Add USDC to fraxBP Curve pool | AMO approaching mint-deploy threshold | Exit LP after AMO tx confirmed + pool stabilises |
| **4B — LP Front-Run (Remove)** | Remove LP / avoid adding | AMO approaching withdrawal threshold | Re-enter LP after AMO withdrawal completes |
| **4C — FRAX Spot Short** | Short FRAX on available venue | FRAX > $1.001, AMO deployment imminent | Cover when FRAX returns to $1.000 ± 0.001 |
| **4D — Paired Asset Long** | Long USDC/FRAX on Curve or spot | AMO withdrawal imminent (FRAX < peg) | Exit when AMO withdrawal confirmed |

**Primary focus for initial backtest: Variant 4A and 4C.** These have the clearest causal chain and most measurable outcomes.

---

## 5. Entry Rules

### 5.1 Trigger Conditions (must ALL be true)

**For Variant 4A (LP Add Front-Run):**

1. FRAX spot price > $1.0008 (above peg, approaching AMO mint threshold) — sourced from Chainlink oracle used by Frax protocol
2. AMO minted FRAX balance is below its deployment cap by less than 20% (i.e., AMO has room to deploy and is likely to do so)
3. `collateral_ratio` is above minimum threshold (AMO is authorised to mint)
4. No governance proposal active that would pause AMO operations (check Frax governance forum/snapshot)
5. Pool depth in target Curve pool is below 30-day average (AMO addition will have larger compositional impact in shallower pools)

**For Variant 4C (FRAX Spot Short):**

1. FRAX spot price > $1.0010 on at least two independent venues
2. AMO deployment transaction has not yet been broadcast (mempool check)
3. Borrowing cost for FRAX short is below 5% annualised (check lending markets)

### 5.2 Entry Execution

- **4A:** Add USDC to fraxBP pool via Curve UI or direct contract call. No speed requirement — this is a minutes-to-hours window.
- **4C:** Open short FRAX position on available spot lending market (Fraxlend, Aave) or perp if available. Size limited by borrow availability.

---

## 6. Exit Rules

### 6.1 Normal Exit

**4A:**
- Exit LP position within 2 hours of confirmed AMO deployment transaction
- Or exit when pool FRAX/USDC ratio returns to within 2% of 30-day average composition
- Whichever comes first

**4C:**
- Cover short when FRAX price returns to $1.0000 ± 0.0005
- Hard stop: cover if FRAX price moves to $1.0030 (AMO failed to deploy or additional demand absorbed supply)

### 6.2 Time-Based Exit (No AMO Action)

If AMO transaction does not execute within **4 hours** of entry trigger, exit position regardless of P&L. The trigger read was wrong or AMO was paused — do not hold a position based on a stale thesis.

### 6.3 Stop Loss

- **4A:** Exit LP if pool composition moves adversely by more than 3% (FRAX proportion increases beyond expected AMO impact — suggests AMO is larger than anticipated or additional FRAX selling is occurring)
- **4C:** Hard stop at FRAX = $1.003 (loss of ~0.2% on short, acceptable given expected gain of 0.05–0.10%)

---

## 7. Position Sizing

### 7.1 Constraints

- Maximum single trade size: **$50,000 notional** — AMO flows are in the millions; individual position must be small enough not to move the pool before the AMO does
- Maximum portfolio allocation to this strategy: **5% of total capital**
- Do not size based on conviction — size based on pool depth. Position should be no more than **0.5% of current pool TVL** to avoid self-defeating the edge

### 7.2 Sizing Formula

```
Position Size = MIN(
    $50,000,
    Pool TVL × 0.005,
    Available Capital × 0.05
)
```

### 7.3 Expected P&L Per Trade

This is a **low-magnitude, high-frequency** edge. Expected gross gain per trade:
- 4A (LP): 0.02–0.08% on capital deployed (pool composition shift + LP fees earned during hold)
- 4C (FRAX short): 0.05–0.15% on notional (peg restoration from $1.001 to $1.000)

These are small. The strategy is viable only if:
- Trade frequency is high enough (multiple AMO events per month)
- Execution costs (gas, LP entry/exit fees, borrow costs) are below expected gain
- **Gas cost must be modelled explicitly** — Ethereum mainnet gas at 20 gwei makes small positions uneconomical

---

## 8. Backtest Methodology

### 8.1 Data Requirements

| Data Source | What to Pull | Where |
|---|---|---|
| Frax AMO contract events | All `AMOMinted`, `AMOBurned`, `CurveAMODeposit` events | Ethereum mainnet, contract: `0x49ee75278820f409ecd67063D47C5e9E4Cca2B4` (verify current address) |
| Curve fraxBP pool state | Pool reserves at each block, LP token supply | Curve subgraph, The Graph |
| FRAX price | Chainlink FRAX/USD feed | Chainlink historical data, Dune Analytics |
| Frax collateral ratio | `globalCollateralRatio()` historical calls | Ethereum archive node or Dune |
| Gas prices | Historical gwei by block | Etherscan gas tracker, Dune |
| AMO parameters | `amo_minted_frax`, deployment caps | Archive node state calls |

### 8.2 Backtest Period

- **Primary:** January 2022 – December 2023 (covers FRAX peg stress events including March 2023 USDC depeg, which is a critical stress test)
- **Secondary:** 2024 to present (post-FRAX v3 changes — verify AMO mechanics unchanged)

### 8.3 Backtest Procedure

**Step 1 — Event Identification**
Extract all historical AMO deployment and withdrawal transactions. For each event, record:
- Block number of transaction
- FRAX price at block N-100, N-50, N-10, N-1 (approach to trigger)
- Pool composition at block N-1 (pre-AMO)
- Pool composition at block N+1, N+10, N+100 (post-AMO)
- AMO transaction size (FRAX minted/deployed)

**Step 2 — Signal Reconstruction**
For each AMO event, reconstruct what the on-chain state looked like 1 hour, 2 hours, 4 hours before the transaction. Determine whether the trigger conditions (Section 5.1) would have been met. This identifies the "signal window."

**Step 3 — P&L Calculation**
For each identified signal:
- Simulate LP entry at pool state T-1hr (or T-2hr, T-4hr — test all)
- Simulate LP exit at T+2hr post-AMO
- Calculate: LP fees earned + composition shift gain/loss - gas costs - slippage
- For 4C: simulate FRAX short entry at T-1hr, exit at peg restoration, net of borrow cost and gas

**Step 4 — Sensitivity Analysis**
- Vary entry timing (T-30min, T-1hr, T-2hr, T-4hr) — does earlier entry improve or worsen results?
- Vary position size — does larger size move the pool and reduce edge?
- Separate results by AMO event size (small vs. large deployments)
- Separate results by pool depth at time of event

**Step 5 — Cost Accounting**
Apply realistic costs:
- Gas: 200,000–400,000 gas units for LP add/remove × historical gwei
- Curve LP fee: 0.04% on entry/exit
- FRAX borrow rate for short (if applicable)
- Slippage on LP entry (function of position size / pool depth)

### 8.4 Key Metrics to Report

- Number of qualifying AMO events in backtest period
- Hit rate (% of events where strategy was profitable net of costs)
- Average gross P&L per event
- Average net P&L per event (after gas and fees)
- Sharpe ratio (annualised)
- Maximum drawdown
- Breakdown by variant (4A vs 4C)
- Breakdown by AMO event size quartile

---

## 9. Go-Live Criteria

All of the following must be satisfied before live deployment:

1. **Backtest shows positive net P&L** in at least 60% of qualifying events after full cost accounting
2. **Minimum 30 qualifying events** in backtest period (strategy must have enough occurrences to be statistically meaningful)
3. **Average net P&L per trade > $50** at $10,000 position size (minimum economic viability given operational overhead)
4. **Signal lead time confirmed:** Backtest must show that trigger conditions were observable at least 30 minutes before AMO transaction in >70% of cases
5. **Stress test passed:** Strategy must not show catastrophic loss during March 2023 USDC depeg event (FRAX went significantly below peg — this is the worst-case scenario for Variant 4A)
6. **Gas model validated:** At current mainnet gas prices, net P&L remains positive
7. **AMO mechanics verified unchanged:** Confirm current AMO contract logic matches backtest-period logic (Frax has upgraded contracts — check deployment dates)
8. **Paper trade period:** 30 days of paper trading with at least 5 live events observed

---

## 10. Kill Criteria

Immediately pause and review if any of the following occur:

| Trigger | Action |
|---|---|
| Two consecutive trades with net loss > 0.5% of position | Pause, review signal quality |
| AMO contract upgraded or paused by governance | Halt all positions, re-verify mechanics |
| FRAX collateral ratio drops below 80% | Halt — systemic risk, AMO behaviour unpredictable |
| Gas costs exceed 50% of gross P&L over trailing 10 trades | Halt — strategy is uneconomical at current gas |
| Signal lead time drops below 15 minutes in 3 consecutive events | Halt — window has closed, possibly due to MEV bots |
| Frax governance votes to modify AMO trigger thresholds | Halt — recalibrate trigger conditions |
| Cumulative strategy drawdown exceeds 2% of allocated capital | Full stop, mandatory review |

---

## 11. Risks

### 11.1 Critical Risks

**R1 — MEV Extraction (HIGH)**
The most serious risk. MEV bots monitor the same on-chain state. If the signal window is being front-run by searchers, the edge is captured before any non-MEV participant can act. Mitigation: backtest must confirm that pool composition shifts are still observable and profitable after accounting for MEV front-running. If the signal window is consistently less than 1 block, the strategy is dead.

**R2 — AMO Governance Changes (MEDIUM-HIGH)**
Frax governance can modify AMO parameters, pause AMOs, or upgrade contracts. A governance vote can eliminate the edge overnight. Mitigation: monitor Frax governance forum and Snapshot continuously. Kill switch on any AMO-related proposal.

**R3 — FRAX Depeg Event (HIGH for 4A, MEDIUM for 4C)**
If FRAX depegs significantly (as in March 2023 when USDC depegged and FRAX followed), Variant 4A LP positions suffer impermanent loss and the AMO may behave unexpectedly. Mitigation: hard position size limits, mandatory exit on collateral ratio breach, never hold LP through a known systemic stress event.

**R4 — Pool Depth Changes (MEDIUM)**
If Curve pool TVL grows significantly, AMO flows become proportionally smaller and the compositional impact diminishes. The edge may decay as pools deepen. Mitigation: monitor pool TVL trend; if TVL grows 5× from backtest baseline, re-evaluate edge magnitude.

**R5 — Gas Cost Erosion (MEDIUM)**
At high gas prices, small position sizes become uneconomical. The strategy requires Ethereum mainnet execution. Mitigation: gas price ceiling — do not execute if gas > 50 gwei (recalibrate based on backtest cost analysis).

**R6 — Oracle Lag (LOW-MEDIUM)**
The AMO uses Chainlink oracle prices, not real-time DEX prices. If Chainlink lags real-time FRAX price, the trigger condition read may be stale. Mitigation: cross-reference Chainlink feed with Curve pool spot price before entry.

**R7 — Frax Protocol Obsolescence (LOW)**
Frax v3 has shifted toward a more fully-collateralised model. AMO activity may be reduced compared to the fractional-reserve era. Mitigation: verify current AMO activity levels before committing to backtest effort.

### 11.2 Risk Summary Table

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| MEV front-running | High | Strategy-killing | Backtest must show >15min window |
| Governance change | Medium | Strategy-killing | Continuous monitoring |
| FRAX depeg | Low-Medium | High loss | Hard stops, size limits |
| Pool depth growth | Medium | Edge decay | TVL monitoring |
| Gas erosion | Medium | Uneconomical | Gas ceiling |
| Oracle lag | Low | Missed signal | Dual price check |

---

## 12. Data Sources

| Source | URL / Access Method | Data |
|---|---|---|
| Ethereum Archive Node | Alchemy/Infura archive tier | Historical contract state calls |
| Dune Analytics | dune.com | AMO events, pool composition, FRAX price history |
| The Graph — Curve Subgraph | thegraph.com/explorer | Curve pool reserves by block |
| Chainlink Historical Feeds | data.chain.link | FRAX/USD historical oracle prices |
| Frax GitHub | github.com/FraxFinance | AMO contract source code, parameter documentation |
| Frax Governance | gov.frax.finance | Active and historical governance proposals |
| Etherscan | etherscan.io | AMO contract event logs, transaction history |
| Frax Analytics | facts.frax.finance | Protocol-level AMO statistics (verify availability) |

---

## 13. Open Questions Before Backtest

These must be answered during data collection phase:

1. **What is the exact current AMO trigger threshold?** Is it a fixed peg deviation or dynamic based on collateral ratio? Has it changed across protocol versions?
2. **How frequently did AMO events occur historically?** If fewer than 30 events exist in the backtest window, the strategy lacks statistical power.
3. **What was the typical signal lead time?** How many blocks/minutes before AMO execution were trigger conditions clearly met?
4. **Has Frax v3 reduced AMO activity?** If the protocol is now fully collateralised, mint-and-deploy AMO operations may be rare or eliminated.
5. **Are there competing MEV strategies already running on this signal?** Check MEV-Explore and Flashbots data for patterns around historical AMO transactions.
6. **What is the minimum viable position size given current gas costs?** Calculate break-even position size at 30-day average gas price.

---

## 14. Researcher Notes

This strategy sits at the intersection of DeFi protocol mechanics and LP position management. The structural mechanism is genuine — AMO flows are deterministic and observable. The honest uncertainty is whether the signal window is wide enough for non-MEV participants and whether the P&L per trade justifies the operational complexity.

**The March 2023 USDC depeg is the critical historical test.** During that event, FRAX dropped significantly below peg, AMO withdrawal mechanics were stress-tested, and pool dynamics were extreme. Any backtest that does not include this period is incomplete.

**Frax v3 migration is a potential strategy-killer.** If the protocol has moved away from fractional-reserve mechanics, the AMO mint-and-deploy cycle may be structurally reduced. This must be verified before significant backtest effort is invested.

**Recommended first step:** Pull all historical AMO contract events from Dune Analytics and count qualifying events by year. If fewer than 10 events per year exist in recent data, deprioritise this strategy and reallocate research effort.

---

*Strategy authored by Zunid Research. All claims are hypotheses pending backtest validation. No live capital should be deployed based on this document alone.*
