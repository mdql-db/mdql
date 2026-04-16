---
title: "Stablecoin PSM (Peg Stability Module) Arbitrage — Daily Flow Direction Signal"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 6
frequency: 3
composite: 450
categories:
  - stablecoin
  - governance
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When MakerDAO's PSM USDC reserve falls below 20% of module capacity, the system is under structural DAI demand pressure that the governance apparatus has historically resolved by raising the DAI Savings Rate (DSR) within 24–72 hours. The governance spell that executes the DSR change is subject to a mandatory timelock delay (minimum 24 hours, typically 48 hours), creating a window where the on-chain signal is visible but the market impact has not yet been priced into DAI yield positions, Curve pool weights, or DAI borrow rates on Aave. The trade is: observe the on-chain imbalance signal, enter before the governance spell executes, exit after execution.

The edge is **not** that PSM mechanics are predictive in a statistical sense — it is that the governance response, when it comes, is announced publicly on the forum and then locked into a timelock contract before execution, giving a hard deadline for the trade to resolve.

---

## Structural Mechanism

### Layer 1 — The PSM as a pressure gauge (contractually enforced)

The PSM is a smart contract that allows 1:1 conversion between USDC and DAI at a fixed fee (currently 0 basis points in, 0 basis points out for most vaults). The USDC reserve balance is a direct, real-time, on-chain measure of net DAI demand: when users want DAI, they deposit USDC and drain the reserve; when they want USDC, they deposit DAI and fill the reserve. This is not a proxy signal — it is the actual flow, recorded in the contract state at every block.

### Layer 2 — The governance response loop (probabilistic but historically consistent)

MakerDAO's risk mandate explicitly tasks the Stability Fee and DSR with maintaining DAI peg stability. A depleted PSM reserve signals that DAI is trading at or above peg and that organic demand exceeds supply — the exact condition the DSR is designed to address by attracting more DAI holders. The governance forum posts a rate change proposal, which then enters the GSM (Governance Security Module) timelock — a **smart contract enforced delay** of at minimum 24 hours before the spell can be cast. This timelock is the mechanical edge: the market knows the change is coming, but the yield adjustment does not take effect until the spell executes.

### Layer 3 — The price impact channel

A DSR increase does three things simultaneously:
1. Raises the floor yield on holding DAI, attracting capital from USDC holders into DAI (Curve pool rebalancing).
2. Compresses the spread between DAI borrow rates on Aave and the DSR (reducing the incentive to borrow DAI for yield arbitrage).
3. Increases demand for DAI-denominated positions, putting upward pressure on DAI/USDC in Curve pools.

Each of these creates a measurable, directional price move that can be captured between signal observation and spell execution.

### Why this is not pure pattern-matching

The causal chain is: **PSM reserve depletion → governance mandate to act → timelock announcement → spell execution → yield/price adjustment**. Steps 1, 3, and 4 are smart contract enforced. Step 2 is the probabilistic link — governance could choose not to act, or could act with a different instrument (e.g., raising the PSM fee instead of the DSR). The 6/10 score reflects this single probabilistic link in an otherwise mechanical chain.

---

## Market Context and Current Applicability

**Important caveat as of 2024–2025:** MakerDAO rebranded to Sky Protocol and introduced significant changes to the DSR and PSM architecture. The USDC PSM has been partially replaced by the Spark Protocol and RWA (Real World Asset) vaults as primary peg management tools. Backtesting must account for the regime change circa Q3 2023 (DSR raised to 5% during the USDC depeg aftermath) and the Sky migration in 2024. The strategy specification below applies to the historical MakerDAO regime (2020–2024) as the primary backtest window, with a separate forward-test regime for Sky Protocol mechanics.

---

## Entry Rules

### Signal conditions (all must be true simultaneously)

| Condition | Threshold | Data source |
|---|---|---|
| PSM USDC reserve | < 20% of module cap | Makerburn.com or direct contract read |
| PSM reserve trend | Declining for ≥ 3 consecutive days | Makerburn.com historical chart |
| DSR unchanged | No DSR change in prior 7 days | MakerDAO governance portal |
| Governance forum | Active proposal or signal thread for DSR increase | forum.makerdao.com |
| GSM timelock | Spell queued but not yet executed | Etherscan spell contract |

**Entry trigger:** All five conditions met. Enter at close of the UTC day on which the spell is queued in the GSM timelock contract (i.e., the 24–48 hour window before execution is the trade window).

**Do not enter on forum signal alone** — wait for the spell to be queued in the GSM, which is the hard on-chain confirmation that execution is imminent. This eliminates trades where governance discusses but does not act.

### Instruments and direction

**Primary trade (highest conviction):**
- LONG DAI/USDC on Curve 3pool or DAI/USDC Curve stable pool
- Mechanism: DSR increase pulls DAI demand, rebalances Curve pool weights toward DAI, DAI trades at slight premium to USDC in pool

**Secondary trade (yield capture):**
- Deposit DAI into DSR contract (sDAI/ERC-4626 wrapper) immediately after spell execution
- Capture the higher yield rate from execution block forward
- This is not a price trade — it is a yield step-up trade

**Tertiary trade (rate compression):**
- SHORT DAI borrow rate on Aave via rate-sensitive position (borrow DAI at variable rate before DSR raise, repay after — captures the spread compression as Aave rates adjust to new DSR floor)
- This is the most complex leg and should only be added after primary trade is validated

**Avoid:** DAI perpetual futures on Hyperliquid — liquidity is insufficient for meaningful size and the funding rate mechanism does not cleanly reflect DSR changes.

---

## Exit Rules

### Primary exit

Exit the DAI/USDC Curve position within **6 hours of spell execution** on-chain. The price impact of the DSR change is front-loaded into the execution block and the subsequent 2–4 hours as yield arbitrageurs rebalance. Holding beyond 6 hours exposes the position to mean reversion as the market fully digests the new rate.

### Secondary exit (yield trade)

Hold the sDAI position for a minimum of **7 days** to capture meaningful yield at the new rate, then reassess. Exit if DSR is subsequently lowered or if PSM reserve refills above 80% of capacity (signal that the demand pressure has resolved and a rate cut may follow).

### Stop-loss

If the governance spell is cancelled or delayed beyond 72 hours of queuing without execution, exit all positions immediately. A cancelled spell means governance has reversed course — the causal chain is broken.

### Time-based stop

If the spell has not executed within **96 hours** of queuing, exit regardless of P&L. Governance delays beyond 96 hours are anomalous and indicate a contested vote or emergency veto — do not hold through governance uncertainty.

---

## Position Sizing

### Sizing rationale

This is a low-volatility, stablecoin-denominated trade. The price move in DAI/USDC is measured in basis points (typically 5–30 bps), not percent. Position size must be large enough to generate meaningful dollar returns on a small price move.

### Sizing formula

```
Position size = (Target dollar return per trade) / (Expected price move in bps × 0.0001)

Example:
Target return: $500 per trade
Expected move: 15 bps = 0.0015
Required position: $500 / 0.0015 = $333,000 notional
```

**Starting allocation:** $100,000–$500,000 notional in DAI/USDC Curve position. At 15 bps average move, this generates $150–$750 gross per trade before gas and fees.

**Gas cost budget:** Ethereum mainnet gas for Curve entry + exit + GSM monitoring ≈ $20–$80 per round trip at 20–50 gwei. This is a meaningful cost at small sizes — minimum viable position is approximately $200,000 notional for the trade to be gas-positive at 10 bps move.

**Maximum allocation:** 15% of total strategy capital per trade. This is a stablecoin trade with limited downside but also limited upside — do not over-allocate.

**Scaling rule:** After 10 validated trades with positive expectancy in backtest, scale to 25% of strategy capital per trade.

---

## Backtest Methodology

### Data collection

**Step 1 — Build the PSM reserve time series**
- Source: Makerburn.com historical data (free, downloadable) or direct Ethereum archive node queries to the PSM contract (`MCD_JOIN_USDC_A` and `MCD_PSM_USDC_A`)
- Frequency: Daily snapshots, 2020-11-01 (PSM launch) to present
- Fields required: `gem` (USDC balance), `Art` (DAI minted via PSM), `line` (debt ceiling = capacity cap)
- Derived field: `utilisation_pct = gem / line`

**Step 2 — Build the DSR change event log**
- Source: MakerDAO governance portal (vote.makerdao.com), forum.makerdao.com, and Etherscan spell execution timestamps
- Fields required: spell address, queue timestamp, execution timestamp, old DSR, new DSR
- Expected event count: approximately 15–25 DSR changes between 2020 and 2024

**Step 3 — Build the Curve DAI/USDC price series**
- Source: The Graph (Curve subgraph), Dune Analytics (free), or direct pool contract reads
- Frequency: Hourly for the 7-day window around each DSR change event
- Fields required: DAI virtual price, pool balances, implied DAI/USDC rate

**Step 4 — Build the Aave DAI borrow rate series**
- Source: Aave subgraph on The Graph, or Aave's published rate history
- Frequency: Daily
- Fields required: variable borrow rate for DAI, utilisation rate

### Event study design

For each DSR change event (n ≈ 15–25):

1. Define **T=0** as the block in which the governance spell is queued in the GSM timelock
2. Record PSM utilisation at T=0 — classify as "signal present" (< 20% reserve) or "signal absent" (≥ 20% reserve)
3. Measure DAI/USDC Curve price at T=0, T+6h, T+12h, T+24h (spell execution), T+48h
4. Calculate return = (price at T+6h after execution) − (price at T=0 entry)
5. Subtract estimated gas cost ($40 average) and Curve swap fee (0.04% per side)

### Hypothesis test

**Null hypothesis:** DAI/USDC price move in the 24-hour window around DSR spell execution is not statistically different from zero.

**Alternative hypothesis:** DAI/USDC price move is positive and statistically significant when PSM reserve < 20% at time of spell queuing.

**Minimum sample for significance:** 15 events with consistent direction (binomial test, p < 0.05 requires 12/15 in the same direction).

**Segmentation:** Separate analysis for (a) DSR increases vs. decreases, (b) PSM reserve < 20% vs. 20–50% vs. > 50% at signal time, (c) pre-2023 regime vs. post-2023 high-rate regime.

### Expected findings (pre-backtest priors)

- DSR increases with PSM reserve < 20%: expect positive DAI/USDC move of 5–25 bps in execution window
- DSR increases with PSM reserve > 50%: expect smaller or no move (governance acting pre-emptively, not reactively)
- DSR decreases: expect negative DAI/USDC move — this is the mirror trade (SHORT DAI/USDC when PSM reserve > 80% and DSR decrease spell queued)

---

## Go-Live Criteria

The strategy moves from hypothesis to paper trading when ALL of the following are met:

1. **Event study complete:** ≥ 15 historical DSR change events analysed with full data
2. **Positive expectancy confirmed:** Mean net return per trade > $0 after gas and fees, with ≥ 60% of trades directionally correct
3. **Signal specificity validated:** "Signal present" trades (PSM < 20%) outperform "signal absent" trades by a statistically meaningful margin — if the signal adds no predictive value over simply trading every DSR change, the strategy collapses to a simpler (and less differentiated) rule
4. **Monitoring infrastructure built:** Automated daily PSM reserve check with alert when reserve crosses 20% threshold; Etherscan webhook or polling for GSM spell queue events
5. **Gas cost model validated:** Confirmed that $200,000+ notional positions are gas-positive at the observed average price move

The strategy moves from paper trading to live trading when:

6. **3 paper trades completed** with documented entry/exit timestamps and P&L matching backtest expectations within 50%
7. **Liquidity confirmed:** Curve pool depth ≥ $10M at time of entry (sufficient to absorb $500K position without self-causing the move)

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

| Kill condition | Rationale |
|---|---|
| MakerDAO/Sky eliminates the PSM mechanism | The pressure gauge no longer exists |
| GSM timelock is reduced to < 6 hours | Trade window too short to execute without HFT infrastructure |
| 5 consecutive losing trades after go-live | Strategy has stopped working in current regime |
| Curve DAI/USDC pool depth falls below $5M | Insufficient liquidity for meaningful position size |
| DSR is replaced by a non-governance-controlled mechanism (e.g., algorithmic rate) | The probabilistic governance link is eliminated — the edge disappears |
| Sky Protocol migration makes historical PSM data non-comparable | Regime break invalidates backtest; restart event study from scratch |

**Pause (not kill) criteria:**
- Governance is in a contested vote or emergency shutdown mode — pause all entries until resolved
- Gas prices above 100 gwei — pause entries as gas costs exceed expected returns at standard position sizes

---

## Risks

### Risk 1 — Governance inaction (primary risk, score impact)
**Description:** The PSM reserve falls below 20% but governance chooses to use a different tool (PSM fee increase, new collateral onboarding, RWA yield adjustment) instead of raising the DSR. The causal chain breaks at the probabilistic link.
**Mitigation:** Only enter after the spell is queued in the GSM — do not enter on forum signal alone. A queued spell is a hard commitment; governance can still cancel it but this is rare and typically accompanied by a public emergency post.
**Residual risk:** Spell cancellation after queuing has occurred (low historical frequency, < 5% of queued spells).

### Risk 2 — Front-running by larger players
**Description:** The PSM reserve data is public and the governance forum is public. Larger players (Curve LPs, Aave rate traders) may already be pricing in the DSR change before the spell is queued.
**Mitigation:** The backtest will reveal whether the price move occurs before or after spell queuing. If the move is fully front-run before queuing, the entry rule must be moved earlier (to forum signal) — but this increases exposure to governance inaction risk.
**Assessment:** Likely partial front-running; the question is whether residual move after queuing is still positive expectancy.

### Risk 3 — Regime change (Sky Protocol migration)
**Description:** MakerDAO's migration to Sky Protocol in 2024 changed the DSR to the SSR (Sky Savings Rate) and altered PSM mechanics. Historical backtest data may not be representative of forward returns.
**Mitigation:** Segment backtest by regime. Build a separate forward-test framework for Sky Protocol mechanics. Do not apply pre-2024 parameters to post-2024 live trading without re-validation.

### Risk 4 — Smart contract risk
**Description:** Curve pool exploit, PSM contract bug, or Aave liquidation cascade could cause losses unrelated to the strategy thesis.
**Mitigation:** Use only audited, battle-tested contracts (Curve 3pool, official MakerDAO PSM). Do not use leveraged positions for the primary trade. Maintain position size limits.

### Risk 5 — Gas cost erosion
**Description:** At small position sizes, gas costs consume the entire expected return. Ethereum gas spikes during high-activity periods (NFT mints, airdrops, market volatility) can make the trade uneconomical.
**Mitigation:** Enforce minimum position size of $200,000 notional. Set a gas price ceiling of 50 gwei for entry; abort trade if gas exceeds ceiling at time of spell execution.

### Risk 6 — Liquidity impact (self-fulfilling at scale)
**Description:** If the strategy is scaled to $1M+ notional, the entry itself moves the Curve pool price, reducing the available edge.
**Mitigation:** Cap position at 5% of Curve pool depth at time of entry. At $10M pool depth, maximum position is $500K notional.

---

## Data Sources

| Data | Source | Cost | Update frequency |
|---|---|---|---|
| PSM USDC reserve balance | Makerburn.com (historical charts + CSV export) | Free | Daily |
| PSM reserve (real-time) | Direct Ethereum node call to `MCD_PSM_USDC_A` contract | Free (public RPC) | Per block |
| DSR change history | MakerDAO governance portal (vote.makerdao.com) | Free | Per governance action |
| Governance spell queue/execution timestamps | Etherscan (search `MCD_PAUSE` contract events) | Free | Per block |
| Governance forum proposals | forum.makerdao.com (public, searchable) | Free | Per post |
| Curve DAI/USDC historical prices | Dune Analytics (query Curve pool events) | Free tier available | Per block |
| Aave DAI borrow rate history | Aave subgraph on The Graph | Free | Per block |
| Ethereum gas prices | Etherscan Gas Tracker, historical CSV | Free | Per block |
| Sky Protocol SSR/PSM data | sky.money analytics, Makerburn (updated) | Free | Daily |

### Monitoring stack (for live trading)

- **PSM reserve alert:** Python script polling `gem` balance on `MCD_PSM_USDC_A` every 6 hours; Telegram alert when reserve < 25% (early warning) and < 20% (signal threshold)
- **GSM spell queue alert:** Etherscan webhook on `MCD_PAUSE` contract `plot()` event (spell queuing event); immediate Telegram alert on trigger
- **Spell execution alert:** Etherscan webhook on `MCD_PAUSE` contract `exec()` event; triggers exit timer (exit within 6 hours of this event)
- **Gas price monitor:** Check ETH gas price before any transaction; abort if > 50 gwei

---

## Next Steps (ordered)

1. **Pull PSM reserve history** from Makerburn.com for 2020-11-01 to 2024-12-31 — target: daily CSV with `gem`, `line`, `utilisation_pct`
2. **Build DSR change event log** — compile all DSR changes with spell queue timestamp and execution timestamp from Etherscan and governance portal
3. **Pull Curve DAI/USDC hourly prices** around each event using Dune Analytics query
4. **Run event study** — calculate mean return, hit rate, and Sharpe for "signal present" vs. "signal absent" subsets
5. **Assess regime break** — compare pre-2023 vs. 2023–2024 results; if regime break is severe, narrow the live trading framework to post-2023 mechanics only
6. **Build monitoring scripts** — PSM reserve poller and GSM spell queue webhook
7. **Paper trade next 3 qualifying events** — document entry/exit with timestamps
8. **Decision gate:** Go-live or kill based on paper trade results vs. backtest expectations

---

*This document is a strategy hypothesis. No live capital should be deployed until the backtest in step 4 above is complete and the go-live criteria in this specification are met. All historical references to DSR changes and PSM mechanics are based on publicly available governance records and have not been independently verified against on-chain data as of this writing.*
