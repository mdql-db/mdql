---
title: "Repo-Equivalent — Crypto Collateral Upgrade/Downgrade Cascade"
status: HYPOTHESIS
mechanism: 7
implementation: 5
safety: 6
frequency: 2
composite: 420
categories:
  - liquidation
  - defi-protocol
  - governance
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DeFi lending protocol reduces the Loan-to-Value (LTV) ratio or liquidation threshold for a collateral asset via governance, all existing borrowers whose positions fall between the old and new parameter bands are **mechanically pushed toward liquidation** the moment the smart contract parameter change executes. This is structurally identical to a repo desk increasing haircuts on downgraded collateral: the rule change forces leveraged players to either post more collateral or unwind. The governance timelock (typically 24–72 hours between vote passage and execution) creates a **known, bounded window** in which the forced unwind is certain to occur but has not yet been priced. Shorting the collateral asset during this window captures the mechanical selling pressure.

The edge is not "LTV reductions tend to cause price drops." The edge is: **a specific, enumerable set of on-chain positions will be forced to sell or repay within a deterministic time window, and the size of that forced flow is calculable in advance.**

---

## Structural Mechanism

### 2.1 The Repo Haircut Analogy

In traditional repo markets:

1. Dealer holds sovereign bond as collateral against a cash loan
2. Rating agency downgrades the bond
3. Repo desk immediately applies a larger haircut (e.g., 5% → 10%)
4. Borrower must post additional collateral or repay principal
5. If borrower cannot top up, forced selling occurs
6. Forced selling depresses the bond price, triggering further margin calls on other holders — a cascade

The key structural feature: **the haircut change is not a market signal — it is a rule change enforced by contract.** The unwind is not probabilistic; it is mandatory for positions that cannot top up.

### 2.2 DeFi Equivalent

In Aave V3 (and Compound, Morpho, Euler):

- **LTV (Loan-to-Value):** Maximum borrow amount as a fraction of collateral value. Determines how much a user *can* borrow.
- **Liquidation Threshold (LT):** The collateral ratio at which a position *becomes liquidatable*. Always higher than LTV (e.g., LTV = 75%, LT = 80%).
- **Health Factor (HF):** `(Collateral × LT) / Debt`. Position is liquidatable when HF < 1.0.

When governance reduces the Liquidation Threshold from 80% to 75%:

- All positions with HF between 1.0 (calculated at 80% LT) and 1.0 (calculated at 75% LT) **instantly become liquidatable** at execution
- Positions with HF between 1.0 and ~1.07 (calculated at 80% LT) are pushed into the danger zone and face rational incentive to deleverage before execution
- Liquidators (bots) are watching the mempool; they will liquidate eligible positions within blocks of execution

The **timelock window** (governance passes → execution) is the trading window. During this window:
- Sophisticated borrowers who read governance will top up or repay (reducing forced selling)
- Unsophisticated borrowers will not (creating forced selling at execution)
- The net forced flow is the difference — estimable from on-chain position data

### 2.3 Why This Is Uncrowded

- Governance proposal readers and DeFi position analysts are different communities; few people do both
- The TradFi repo analogy is not how crypto governance is discussed — proposals are framed as "risk parameter updates," not "forced liquidation triggers"
- Systematic monitoring of governance timelocks as trading signals does not exist as a known strategy
- The opportunity is irregular (not daily) and requires manual governance monitoring — most quant funds skip it

---

## Universe and Scope

**Protocols in scope:**
- Aave V3 (Ethereum, Arbitrum, Optimism, Polygon, Base)
- Compound V3
- Morpho Blue
- Spark Protocol (MakerDAO's lending arm)

**Assets in scope:** Any collateral asset with a liquid perpetual futures market on Hyperliquid or a major CEX. Priority assets: ETH, wBTC, stETH/wstETH, cbBTC, ARB, OP, LINK, UNI, AAVE.

**Parameter changes in scope:**
1. Liquidation Threshold reduction (primary — directly forces liquidations)
2. LTV reduction (secondary — forces deleveraging for new borrows, creates rational unwind incentive for existing positions near max leverage)
3. Liquidation penalty increase (tertiary — increases cost of being liquidated, incentivizes preemptive unwind)
4. Supply cap reduction (if existing positions exceed new cap, forced partial unwind)

**Out of scope:** LTV *increases* (these relax constraints, no forced unwind), interest rate changes (gradual, not forced).

---

## Signal Generation

### 4.1 Governance Monitoring

**Step 1 — Proposal Detection**

Monitor the following for LT/LTV reduction proposals:
- Aave governance forum: `governance.aave.com` (forum posts tagged "ARFC" and "AIP")
- Aave on-chain governance contract: `AaveGovernanceV2` / `GovernanceV3` on Ethereum
- Compound Governor Bravo contract
- Morpho governance (Snapshot + on-chain)
- Chaos Labs and Gauntlet risk parameter recommendation threads (these are the primary proposers — monitoring their posts gives 1–2 week advance notice before formal vote)

**Step 2 — Vote Passage Confirmation**

Signal activates when:
- Quorum is met AND
- Vote passes (majority for) AND
- Timelock begins (on-chain event emitted)

Do NOT enter on proposal submission alone — proposals fail. Enter only on confirmed passage.

**Step 3 — Timelock Duration**

| Protocol | Typical Timelock |
|----------|-----------------|
| Aave V3 | 24 hours (short executor) |
| Compound V3 | 48 hours |
| Morpho Blue | 24–72 hours (configurable per market) |
| Spark | 48 hours |

Timelock duration is readable from the governance contract at signal time.

### 4.2 Position Impact Calculation

Immediately upon signal activation, query the protocol's subgraph to estimate forced flow:

**Query:** All positions using the affected asset as collateral, filtered by health factor in the danger zone.

**Danger zone definition:**
- **Immediate liquidation zone:** Positions with HF that will be < 1.0 under new LT (these are liquidated at execution)
- **Rational unwind zone:** Positions with HF between 1.0 and 1.10 under new LT (these face strong incentive to deleverage during timelock)

**Forced flow estimate:**

```
Immediate_Liquidation_USD = Σ (collateral_value × liquidation_penalty) 
                             for all positions where HF_new < 1.0

Rational_Unwind_USD = Σ (debt_value × estimated_deleverage_fraction)
                       for all positions where 1.0 < HF_new < 1.10
```

**Entry threshold:** Proceed only if `Immediate_Liquidation_USD > $5M` OR `(Immediate + Rational) > $15M`.

These thresholds are initial estimates — calibrate against historical events during backtesting.

### 4.3 Market Impact Adjustment

Raw forced flow overstates price impact. Apply adjustments:

1. **Top-up escape valve:** Historically, ~30–50% of at-risk borrowers top up collateral during the timelock window (estimate from historical Aave liquidation data vs. at-risk positions). Reduce forced flow estimate by 40% as base case.
2. **Liquidity adjustment:** Divide adjusted forced flow by 24-hour average DEX + CEX volume for the asset. If ratio > 2%, expect meaningful price impact. If < 0.5%, skip.
3. **Existing short interest:** Check Hyperliquid open interest and funding rate. If funding is already deeply negative (market already short), edge is partially priced — reduce position size.

---

## Entry Rules

### 5.1 Entry Trigger

**Primary entry:** Immediately upon on-chain confirmation that governance vote has passed and timelock has begun.

**Entry price:** Market order on Hyperliquid perpetual for the collateral asset. Use TWAP over 15 minutes to avoid moving the market on entry (position sizes will typically be small relative to market depth).

### 5.2 Entry Conditions (All Must Be Met)

1. Governance vote confirmed passed on-chain (not just forum announcement)
2. Affected asset has liquid perp on Hyperliquid (>$5M daily volume)
3. Estimated adjusted forced flow > $5M immediate OR > $15M combined
4. Funding rate on Hyperliquid perp is not already below -0.05% per 8 hours (already crowded short)
5. No conflicting positive catalyst in the next 48 hours (e.g., major protocol upgrade, token unlock for a different reason that creates buy pressure — manual check)
6. Timelock duration ≥ 12 hours (shorter timelocks don't give enough setup time)

### 5.3 Position Direction

**Short** the collateral asset on Hyperliquid perpetual futures.

Rationale: Forced selling of the collateral asset (to repay debt or avoid liquidation) creates net sell pressure on that asset. Liquidators who receive collateral at discount also typically sell immediately.

---

## Exit Rules

### 6.1 Primary Exit

**Exit at:** T + 24 hours after parameter change execution on-chain.

Rationale: Liquidation bots execute within blocks of parameter change. Rational unwinders complete within hours. By 24 hours post-execution, the mechanical selling is exhausted. Holding longer introduces unrelated market risk.

### 6.2 Secondary Exits

**Take profit:** If position is up >3% before execution, take 50% off. The pre-execution move means sophisticated borrowers are already unwinding — the post-execution move may be smaller than expected.

**Stop loss:** If position is down >2% at any point before execution, exit entirely. A move against us before execution suggests a positive catalyst has emerged that overrides the forced selling (e.g., protocol announces emergency pause, or large whale is buying the dip aggressively). The structural thesis is intact but market conditions have changed.

**Execution day exit:** If the parameter change is delayed (governance emergency, multisig veto), exit immediately at market. The timelock is the edge; delay invalidates the timing thesis.

### 6.3 Exit Mechanics

Market order on Hyperliquid. Do not use limit orders for exit — the goal is to be out within 30 minutes of the 24-hour mark, not to optimize exit price.

---

## Position Sizing

### 7.1 Base Sizing Formula

```
Position_Size_USD = min(
    Account_Risk_Budget × Kelly_Fraction,
    Forced_Flow_Estimate_Adjusted × Participation_Cap,
    Max_Position_Cap
)
```

**Parameters (initial, calibrate post-backtest):**

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Account_Risk_Budget | 2% of AUM per trade | Standard single-trade risk limit |
| Kelly_Fraction | 0.25 (quarter Kelly) | Conservative; edge magnitude uncertain |
| Participation_Cap | 5% of estimated forced flow | Avoid being the market |
| Max_Position_Cap | $500K notional | Liquidity constraint on Hyperliquid for mid-caps |

### 7.2 Leverage

Maximum 3x leverage. The edge is directional but not high-conviction on magnitude — excess leverage risks stop-out before the event executes.

### 7.3 Scaling by Signal Strength

| Forced Flow Estimate | Position Scale |
|---------------------|----------------|
| $5M–$15M | 50% of base size |
| $15M–$50M | 100% of base size |
| >$50M | 150% of base size (cap at Max_Position_Cap) |

---

## Backtest Methodology

### 8.1 Historical Event Set

Compile all historical Aave V3 LT/LTV reductions since V3 launch (January 2022). Preliminary list of known events:

- Aave V3 Ethereum: Multiple LT reductions for CRV, BAL, 1INCH, SNX, ENS (2022–2023 risk parameter tightening wave)
- Aave V3 Ethereum: stETH LT adjustments (multiple, 2022–2024)
- Aave V3 Polygon: MATIC, LINK, WBTC parameter reductions
- Compound V3: COMP, LINK, UNI collateral factor reductions
- Estimated total: 25–40 qualifying events across protocols

**Data collection:**
1. Pull all governance proposals from Tally API and Aave governance subgraph
2. Filter for LT/LTV reduction proposals that passed
3. Record: proposal pass timestamp, execution timestamp, old LT, new LT, affected asset
4. For each event, pull Aave subgraph snapshot at proposal pass time to estimate at-risk positions
5. Pull price data for affected asset: 7 days before proposal pass through 7 days after execution

### 8.2 Backtest Metrics

For each historical event, calculate:

- **Entry price:** Close price at proposal pass timestamp
- **Exit price:** Close price 24 hours after execution
- **Raw return:** (Entry - Exit) / Entry (short position)
- **Adjusted return:** Raw return minus estimated funding cost during hold period
- **Forced flow accuracy:** Compare estimated at-risk positions to actual liquidations (Aave liquidation events on-chain)

**Aggregate metrics:**
- Win rate (% of events where short was profitable)
- Average return per event
- Sharpe ratio (annualized, using event-level returns)
- Maximum drawdown per event
- Correlation between forced flow estimate and actual return

### 8.3 Confounds to Control For

1. **Broad market moves:** Subtract BTC/ETH return during the same window to isolate asset-specific effect
2. **Pre-announcement drift:** Check if price moved before proposal pass (information leakage from forum posts)
3. **Asset-specific events:** Flag events where another major catalyst occurred in the same window (manual review)
4. **Proposal failure rate:** Track how many proposals were submitted but failed — these are false signals that should not have triggered entry (confirm entry rule of "passed vote only" is correct)

### 8.4 Subgraph Query Template

```graphql
# Aave V3 — positions at risk for LT reduction
{
  userReserves(
    where: {
      reserve: "0x[ASSET_ADDRESS]",
      currentATokenBalance_gt: "0"
    }
    first: 1000
  ) {
    user {
      id
      healthFactor
      totalCollateralBase
      totalDebtBase
    }
    currentATokenBalance
    reserve {
      liquidationThreshold
      reserveLiquidationBonus
    }
  }
}
```

Note: Health factor must be recalculated with new LT applied — the subgraph returns current HF, not hypothetical HF under new parameters. Build a local recalculation script.

---

## Go-Live Criteria

The strategy moves from backtest to paper trading when:

1. **Historical event set ≥ 20 qualifying events** identified and priced
2. **Win rate ≥ 55%** on historical events (adjusted for market beta)
3. **Average return per event ≥ 0.8%** (net of estimated funding costs)
4. **Forced flow estimate correlates with return magnitude** (Pearson r > 0.3) — this validates the on-chain position scanner is actually predictive
5. **No single event accounts for >40% of total backtest P&L** (not a one-event wonder)

Paper trading period: Minimum 5 live events before real capital deployment. Paper trade with full position sizing simulation including funding cost tracking.

**Real capital deployment criteria (additional):**
- Paper trade win rate ≥ 50% across ≥ 5 events
- No systematic error found in position scanner (compare paper trade estimates to actual liquidations post-event)

---

## Kill Criteria

**Immediate kill (stop all new positions):**
- Two consecutive losses > 3% each (suggests systematic error in thesis or changed market conditions)
- Funding rate on Hyperliquid perps for target assets becomes structurally negative (market has learned the signal — edge is priced)
- Aave/Compound governance moves to on-chain execution without timelock (removes the advance window)
- A governance proposal is front-run by a large player who publicly announces the trade before our entry (crowding signal)

**Review and possible kill:**
- Win rate drops below 45% over trailing 10 events
- Average return per event drops below 0.3% (net of funding) — edge exists but not worth operational overhead
- Forced flow estimate accuracy degrades significantly (r < 0.15) — scanner is broken or market structure changed

**Structural kill:**
- Aave V3 migrates to a model where LT changes are gradual (e.g., linear reduction over 7 days) rather than step-change — this eliminates the instantaneous forced liquidation mechanic
- Hyperliquid removes perpetuals for the relevant assets

---

## Risks

### 11.1 Escape Valve Risk (Primary Risk)

**Risk:** Borrowers top up collateral during the timelock window, eliminating forced selling.

**Magnitude:** If 80%+ of at-risk positions top up, the forced flow is negligible and the trade loses to funding costs.

**Mitigation:** 
- Only enter when immediate liquidation zone (HF_new < 1.0) is material — these positions cannot escape by topping up unless they add significant capital
- Monitor collateral top-up activity during timelock; if >60% of at-risk positions have topped up before execution, exit early
- The rational unwind zone (1.0 < HF_new < 1.10) is the escape-valve-sensitive portion — weight immediate zone more heavily in sizing

### 11.2 Governance Delay / Veto Risk

**Risk:** After vote passes, a multisig guardian vetoes the proposal or an emergency pause is triggered. Timelock does not execute.

**Magnitude:** Position is held for 24–72 hours with no catalyst, exposed to market risk.

**Mitigation:** 
- Stop loss at -2% handles most scenarios
- Monitor Aave Guardian multisig activity during timelock; any veto transaction triggers immediate exit
- Historical veto rate on Aave is very low (<5% of passed proposals) — acceptable base rate

### 11.3 Pre-Pricing Risk

**Risk:** Governance forum posts (which precede on-chain vote by days) are read by other traders who short in advance, leaving no edge at our entry point.

**Magnitude:** If price has already moved 3–5% by the time the vote passes, the remaining expected move is small.

**Mitigation:**
- Track price action from forum post through vote passage — if asset is already down >2% from pre-forum price, reduce position size by 50%
- This is measurable in backtest: check how much of the total move occurs before vs. after vote passage

### 11.4 Liquidation Cascade Containment

**Risk:** Aave's liquidation mechanism has built-in protections (liquidation caps, close factor limits) that slow the cascade and spread it over multiple blocks/hours, reducing immediate price impact.

**Magnitude:** Aave V3 has a 50% close factor (liquidators can only liquidate 50% of a position per transaction) and liquidation caps per asset per block. This means large liquidations are spread over time, reducing instantaneous price impact.

**Mitigation:**
- Model liquidation spread in position impact calculator — a $20M liquidation may take 4–6 hours to fully execute under Aave's caps
- This actually helps the trade: it means the selling pressure is sustained over the 24-hour exit window, not front-loaded into the first block

### 11.5 Liquidity Risk on Hyperliquid

**Risk:** For mid-cap assets (ARB, OP, LINK), Hyperliquid perp liquidity may be insufficient to enter/exit $200–500K positions without significant slippage.

**Mitigation:**
- Hard cap: do not enter if 24-hour Hyperliquid perp volume < $3M for the asset
- Use TWAP entry over 15 minutes
- For very illiquid assets, consider spot short via borrowing on a CEX instead

### 11.6 Correlated Market Risk

**Risk:** During a broad market rally, the forced selling is overwhelmed by buy pressure and the short loses despite the mechanical selling occurring.

**Mitigation:**
- Stop loss at -2% handles this
- Consider adding a market regime filter: do not enter if BTC is up >3% in the 4 hours before entry (strong bull momentum overrides mechanical selling)
- In backtest, measure win rate conditional on market direction during event window

### 11.7 Smart Contract / Oracle Risk

**Risk:** The parameter change triggers an unexpected protocol behavior (oracle manipulation, reentrancy, etc.) that causes a protocol exploit rather than an orderly liquidation. This could cause the collateral asset to spike (if exploit is detected and positions are frozen) or crash (if exploit drains the protocol).

**Magnitude:** Low probability but high impact. Not a systematic risk — idiosyncratic.

**Mitigation:** This is not hedgeable. Accept as tail risk. Position sizing (2% AUM max) limits damage.

---

## Data Sources

| Data Type | Source | Access | Cost |
|-----------|--------|--------|------|
| Aave governance proposals | governance.aave.com forum | Public RSS/scrape | Free |
| Aave on-chain votes | Tally API, Aave governance subgraph | API | Free tier sufficient |
| Aave position data | The Graph — Aave V3 subgraph | GraphQL API | Free (rate limited) |
| Compound governance | Tally API, compound.finance/governance | API | Free |
| Morpho governance | Snapshot API, Morpho docs | API | Free |
| Chaos Labs / Gauntlet recommendations | chaos.xyz, gauntlet.xyz blogs | Public | Free |
| Historical liquidation events | Aave V3 subgraph (LiquidationCall events) | GraphQL | Free |
| Price data (backtest) | Coingecko API, Kaiko | API | Free / paid |
| Hyperliquid perp data | Hyperliquid API | API | Free |
| Funding rates | Hyperliquid API, Coinglass | API | Free |

**Key subgraph endpoints:**
- Aave V3 Ethereum: `https://api.thegraph.com/subgraphs/name/aave/protocol-v3`
- Aave V3 Arbitrum: `https://api.thegraph.com/subgraphs/name/aave/protocol-v3-arbitrum`

---

## Operational Requirements

### 13.1 Monitoring Infrastructure

**Minimum viable setup:**
1. **Governance watcher script:** Polls Tally API and Aave governance contract every 15 minutes for new proposal state changes (specifically: `Queued` state = timelock started)
2. **Position scanner:** On trigger, queries Aave subgraph and recalculates health factors under new parameters. Outputs: immediate liquidation USD, rational unwind USD, top-up activity tracker
3. **Alert system:** Telegram/Discord bot sends alert with: asset, old LT, new LT, estimated forced flow, timelock expiry, recommended position size
4. **Manual review step:** Human confirms entry before execution (this is not a fully automated strategy — governance context matters)

### 13.2 Time Commitment

- Governance monitoring: automated (15-minute polling)
- Per-event analysis: 30–60 minutes (position scanner + manual review)
- Trade management: 15 minutes/day during active trade
- Expected event frequency: 1–3 qualifying events per month across all protocols

### 13.3 Execution

- Platform: Hyperliquid perpetuals (primary)
- Backup: dYdX, Bybit perps if Hyperliquid liquidity insufficient
- Order type: Market order with 15-minute TWAP for entry; market order for exit

---

## Open Questions for Backtest Phase

1. **What fraction of at-risk positions historically topped up vs. were liquidated?** This determines the escape valve discount factor.
2. **How much of the price move occurs before vote passage (forum leak) vs. after?** This determines whether our entry timing is correct.
3. **Is the forced flow estimate (from subgraph) actually correlated with price impact?** This is the core validation question.
4. **Do smaller LT reductions (e.g., 80% → 79%) produce measurable effects, or is there a minimum threshold?** Helps filter noise events.
5. **Is the effect stronger for assets with concentrated borrower bases (few large positions) vs. distributed (many small positions)?** Large positions are more likely to be sophisticated (top up); small positions more likely to be liquidated.
6. **What is the optimal exit timing — 12 hours, 24 hours, or 48 hours post-execution?** Liquidation cascade duration varies by asset liquidity.

---

## Strategy Summary Card

| Field | Value |
|-------|-------|
| **Type** | Event-driven short |
| **Direction** | Short collateral asset |
| **Trigger** | DeFi governance LT/LTV reduction — timelock start |
| **Hold period** | ~24–72 hours (timelock) + 24 hours post-execution |
| **Expected frequency** | 1–3 trades/month |
| **Max position size** | $500K notional / 2% AUM |
| **Max leverage** | 3x |
| **Stop loss** | -2% |
| **Take profit** | Partial at +3%, full exit 24h post-execution |
| **Score** | 7/10 |
| **Primary risk** | Escape valve (borrower top-up) |
| **Structural guarantee** | Smart contract parameter change is certain; price impact is probabilistic |
| **Next step** | Build governance watcher + position scanner; compile 20+ historical events for backtest |

---

*This document represents a hypothesis requiring empirical validation. No backtest has been run. Do not allocate real capital until go-live criteria in Section 9 are met.*
