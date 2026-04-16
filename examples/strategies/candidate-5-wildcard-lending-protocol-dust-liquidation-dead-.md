---
title: "Governance Bad Debt Socialization Short (GBDS)"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 2
composite: 300
categories:
  - governance
  - token-supply
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DeFi lending protocol passes a governance vote to socialize or write off bad debt, the protocol's native token faces **mechanically forced sell pressure** during the timelock window between vote confirmation and on-chain execution. The causal chain:

1. Protocol accumulates bad debt (collateral < outstanding debt after liquidation)
2. Governance proposes bad debt socialization — covered by protocol reserves, safety module, or direct token liquidation
3. Vote passes on-chain (Snapshot or Governor Bravo/Alpha) — event is now **contractually scheduled**
4. Timelock delay (48h–7 days) creates a window where the outcome is certain but execution has not occurred
5. Treasury or safety module **must** liquidate protocol token reserves to cover the shortfall, or token holders face dilution via backstop mechanism
6. This is a **guaranteed supply event** — structurally identical to a token unlock, but sourced from governance rather than a vesting schedule

**Short the protocol token immediately after vote confirmation, before timelock execution.**

The edge is not "bad news causes selling." The edge is that a specific, quantified amount of token supply **must** enter the market within a known time window, and this event is visible on-chain before it executes.

---

## Structural Mechanism (WHY This MUST Happen)

The mechanism varies by protocol. Verify which applies before entering any trade:

### Aave (AAVE)
- Aave's **Safety Module** holds staked AAVE (stkAAVE). When bad debt exceeds protocol reserves, the Safety Module is slashed — up to 30% of staked AAVE is liquidated to cover the shortfall.
- The slash is executed by the **Aave Guardian** or governance after a vote, with a **10-day cooldown** on the Safety Module itself.
- The slashed AAVE is sold on-market or via OTC to cover the debt. This is a **contractual obligation** written into the Safety Module smart contract (`SlashingAdmin` role, `slash()` function on `StakedAaveV3`).
- Reference: Aave Safety Module docs, `0x25F2226B597E8F9514B3F68F00f494cF4f286491` (stkAAVE on Ethereum mainnet)
- **Aave Umbrella** (2024 upgrade) introduces automated bad debt coverage — monitor `UmbrellaController` contract for coverage events.

### Compound (COMP)
- Compound's **reserves** accumulate from the spread between borrow and supply rates. Bad debt is covered by reserves first.
- If reserves are insufficient, governance can vote to **mint COMP** or liquidate treasury COMP holdings.
- COMP treasury address: `0x2775b1c75658Be0F640272CCb8c72ac986009e38`
- Mechanism is less automatic than Aave — requires explicit governance action to liquidate treasury COMP, making the signal cleaner (vote = confirmed intent).

### MakerDAO/Sky (MKR/SKY)
- MakerDAO's **Debt Auction** mechanism is the most mechanical: when the system surplus buffer is insufficient to cover bad debt, the protocol automatically triggers `flop` auctions — **minting new MKR** and auctioning it for DAI to cover the deficit.
- This is **fully automated** by the `Vow` contract (`0xA950524441892A31ebddF91d3cEEFa04Bf454466` on Ethereum mainnet). No governance vote required — the `Vow.flop()` function triggers when `Sin` (bad debt) exceeds `Ash` (queued debt) and the surplus buffer is depleted.
- For MKR, the signal is the `Vow` contract state, not a governance vote. Monitor `Vow.Sin()` and `Vow.Ash()` and `Vow.Joy()` (surplus) — when `Sin - Ash > Joy + hump` (where `hump` is the surplus buffer floor), `flop` auctions are imminent.
- **This is the highest-conviction variant** — MKR minting is contractually automatic, not governance-dependent.

### Morpho (MORPHO)
- Morpho's bad debt socialization is handled at the vault level — losses are spread across vault depositors, not covered by MORPHO token liquidation.
- **MORPHO token is not at risk from bad debt socialization** — do not apply this strategy to MORPHO.
- Morpho is included only as a monitoring source for bad debt data, not as a trade target.

---

## Entry Rules


### Pre-Trade Checklist (must complete before entry)
- [ ] Confirm which coverage mechanism applies (Safety Module slash vs. treasury liquidation vs. debt auction)
- [ ] Quantify the bad debt amount in USD
- [ ] Calculate implied token sell pressure: `bad_debt_USD / token_price = tokens_to_be_sold`
- [ ] Compare tokens_to_be_sold against 30-day average daily volume — if < 0.5% of ADV, skip (impact too small)
- [ ] Confirm timelock duration (check `TimelockController.getMinDelay()` or protocol docs)
- [ ] Check whether bad debt was already disclosed in governance forum (if discussion phase > 14 days old, skip — likely priced in)

### Entry
- **Trigger:** On-chain vote confirmation (Governor Bravo `ProposalExecuted` event OR Snapshot vote finalized with quorum met AND on-chain execution queued in timelock)
- **Entry timing:** Within 2 hours of vote confirmation (before timelock execution)
- **Instrument:** Perpetual futures short on Hyperliquid (AAVE-PERP, COMP-PERP) or spot short via borrowing
- **Entry price:** Market order at open of next 1-hour candle after trigger confirmation

## Exit Rules

### Exit
- **Primary exit:** On-chain execution of the bad debt coverage transaction (monitor mempool/block explorer for `slash()` call or `flop` auction settlement) + 24 hours
- **Secondary exit:** If timelock expires and execution has not occurred within 48h of expected window, close position (governance may have been cancelled or delayed)
- **Profit target:** None — hold until execution + 24h regardless of interim P&L (the event hasn't happened yet)
- **Stop-loss:** 7% adverse move from entry price (not 5% — crypto vol will trigger 5% stops on noise)

### Position Management
- Do not add to position during timelock window
- If token drops > 15% before execution, take 50% profit and hold remainder to execution

---

## Position Sizing

**Base size:** 1% of portfolio per trade

**Scaling by conviction:**

| Condition | Size |
|---|---|
| MKR `Vow.flop()` imminent (automated, no governance) | 1.5% of portfolio |
| AAVE Safety Module slash confirmed by governance vote | 1.0% of portfolio |
| COMP treasury liquidation confirmed by governance vote | 0.75% of portfolio |
| Bad debt < 0.5% of token's 30-day ADV | Skip |
| Discussion phase > 14 days (likely priced in) | Skip |

**Maximum concurrent exposure:** 2 positions (2% of portfolio) — these events are rare; don't force concentration.

**Leverage:** 2–3x maximum. This is an event-driven trade with a defined timelock window, not a directional bet. Higher leverage increases liquidation risk during the timelock noise period.

---

## Backtest Methodology

### Data Sources
See Data Sources section below for URLs. Collect:
1. All governance proposals tagged "bad debt," "shortfall," "safety module slash," or "debt auction" from Aave, Compound, MakerDAO governance forums (2020–present)
2. On-chain vote confirmation timestamps (Governor Bravo event logs via Etherscan or The Graph)
3. Token price data: OHLCV at 1h resolution (Coingecko API, Kaiko, or Tardis.dev for historical perp data)
4. MKR `Vow` contract state history (Dune Analytics query or direct archive node)

### Event Universe (expected sample size)
- MakerDAO `flop` auctions: ~8–12 events (March 2020 Black Thursday, various 2021–2023 events)
- Aave Safety Module slashes: 0 actual slashes to date (as of 2025-01) — **Aave has never executed a Safety Module slash**. This means Aave backtest will have near-zero sample size. Do not rely on Aave for backtest validation.
- Compound bad debt governance votes: ~3–5 events (including the November 2022 COMP distribution bug)
- **Total usable events: ~12–18.** This is a small sample. Treat backtest results as directional, not statistically conclusive.

### Metrics to Calculate
For each event:
- `T0` = vote confirmation timestamp
- `T_exec` = on-chain execution timestamp
- `Return_window` = token return from `T0` to `T_exec + 24h`
- `Return_baseline` = BTC return over same window (market beta control)
- `Alpha` = `Return_window - Return_baseline`
- `Max_adverse_excursion` = maximum adverse move during `T0` to `T_exec` (for stop-loss calibration)
- `Pre_disclosure_drift` = token return from governance forum post to `T0` (measures how much is priced in during discussion)

### Baseline Comparison
- Compare alpha against: (a) BTC-adjusted return, (b) random 48h–7 day short windows on same token, (c) sector index (DeFi blue-chip basket)
- Null hypothesis: bad debt events produce no excess negative return vs. baseline

### Minimum Viable Backtest Output
- Mean alpha across all events (target: < -5% on average)
- Win rate (target: > 60%)
- Sharpe of the strategy (target: > 1.0 on event-adjusted basis)
- Pre-disclosure drift analysis — if > 70% of the move happens before `T0`, the strategy has no edge in the timelock window

### Backtest Caveats
- MakerDAO `flop` auctions in March 2020 occurred during extreme market dislocation — these events may not be representative of normal conditions. Run analysis with and without Black Thursday.
- Small sample size means any result has wide confidence intervals. Do not over-fit stop-loss or exit parameters to this dataset.

---

## Go-Live Criteria

All of the following must be met before paper trading:

1. **Mean alpha ≤ -5%** across all backtest events (token underperforms BTC by at least 5% during the trade window)
2. **Win rate ≥ 55%** (majority of events show negative alpha)
3. **Pre-disclosure drift < 50%** — at least half the move must occur after vote confirmation, not during discussion phase
4. **At least 8 usable events** in backtest (if fewer, extend to adjacent events like large reserve liquidations)
5. **Max adverse excursion analysis** supports 7% stop-loss (i.e., < 20% of events would have been stopped out before execution)
6. **MKR `Vow` variant tested separately** from governance-vote variants — if only MKR works, scope the live strategy to MKR only

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Backtest shows pre-disclosure drift > 70%** — the edge is in the discussion phase, not the timelock window. The strategy as specified cannot capture it without monitoring forums 24/7 weeks in advance.
2. **Win rate < 50% in backtest** — no edge over random
3. **First 3 live paper trades all lose** — re-examine mechanism assumptions
4. **Aave deploys Umbrella and eliminates Safety Module slash mechanism** — the structural mechanism for AAVE no longer exists (monitor Aave governance for Umbrella full deployment)
5. **MakerDAO eliminates `flop` auctions** via governance (Sky/Endgame restructuring may change this — monitor `sky.money` governance)
6. **Event frequency drops to < 1 per year** — not enough occurrences to maintain operational readiness

---

## Risks

### Mechanism Risk (High for Aave)
Aave has **never executed a Safety Module slash** despite having bad debt. The protocol has covered shortfalls via fee income and treasury reserves without slashing stkAAVE. If this pattern continues, the AAVE short thesis has no historical execution to validate. **Do not trade AAVE on this thesis until a slash actually occurs and can be studied.**

### Pre-Pricing Risk (Medium)
Governance discussions are public weeks before votes. Sophisticated participants (Gauntlet, Chaos Labs, large token holders) monitor these forums. By the time a vote passes, the market may have already priced the event. The backtest's pre-disclosure drift metric will quantify this — if drift is high, the strategy window must shift to the discussion phase, which requires different monitoring infrastructure.

### Mechanism Substitution Risk (Medium)
Protocols may cover bad debt via fee income, insurance funds, or off-market OTC deals rather than open-market token liquidation. In these cases, there is no market sell pressure despite the governance event. **Always verify the specific coverage mechanism before entry** — this is the most important pre-trade checklist item.

### Small Sample Risk (High)
12–18 historical events is not enough to draw statistically robust conclusions. Any backtest result should be treated as a directional signal, not a validated edge. The strategy requires ongoing live monitoring to build sample size.

### Timelock Cancellation Risk (Low)
Governance timelocks can be cancelled by the Guardian multisig in most protocols. If a proposal is cancelled after the short is entered, the supply event does not occur. Monitor the timelock contract for cancellation events during the trade window.

### Liquidity Risk (Medium for COMP)
COMP perpetual futures have thin liquidity on Hyperliquid. Large positions will move the market. Size accordingly — check open interest and daily volume before entry. AAVE-PERP has better liquidity. MKR-PERP liquidity is moderate.

### Regulatory/Protocol Upgrade Risk (Low, ongoing)
MakerDAO's transition to Sky (SKY token) and Endgame restructuring may alter the `Vow`/`flop` mechanism. Monitor `sky.money` governance for changes to the debt auction system.

---

## Data Sources

### Governance Monitoring
- **Aave Governance:** `https://governance.aave.com` (forum) + `https://app.aave.com/governance` (on-chain votes)
- **Aave Governor Bravo contract:** `0xEC568fffba86c094cf06b22134B23074DFE2252c` on Ethereum — query `ProposalQueued` and `ProposalExecuted` events
- **Compound Governance:** `https://compound.finance/governance` + Governor Bravo at `0xc0Da02939E1441F497fd74F78cE7Decb17B66529`
- **MakerDAO/Sky Governance:** `https://vote.makerdao.com` + `https://forum.makerdao.com`
- **Snapshot (all protocols):** `https://snapshot.org/#/aave.eth` — GraphQL API: `https://hub.snapshot.org/graphql`

### Bad Debt Dashboards
- **Chaos Labs Risk Dashboard:** `https://community.chaoslabs.xyz/aave/risk/overview` — tracks Aave bad debt in real time
- **Gauntlet Risk Reports:** `https://gauntlet.network/reports` — historical bad debt reports for Aave, Compound
- **B.Protocol Bad Debt Tracker:** `https://bad-debt.riskdao.org` — tracks bad debt across Aave, Compound, Euler, Morpho (JSON API available)
- **Dune Analytics — MakerDAO Vow state:** Search "MakerDAO Vow flop auctions" on `https://dune.com` — multiple community dashboards track `Sin`, `Ash`, `Joy`

### MakerDAO Contract Monitoring
- **Vow contract:** `0xA950524441892A31ebddF91d3cEEFa04Bf454466` on Ethereum mainnet
- **Key functions to monitor:** `Vow.Sin()` (bad debt), `Vow.Ash()` (queued debt), `Vow.Joy()` (surplus), `Vow.hump()` (surplus buffer floor)
- **Flop auction contract:** `0xa41B6EF151E06da0e34B009B86E828308986736D` — monitor `Kick` events for new debt auctions
- **Etherscan API:** `https://api.etherscan.io/api?module=logs&action=getLogs&address=0xA950524441892A31ebddF91d3cEEFa04Bf454466`

### Price Data
- **Coingecko API (free):** `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=max&interval=hourly`
  - AAVE: `id=aave`, COMP: `id=compound-governance-token`, MKR: `id=maker`
- **Tardis.dev (historical perp data, paid):** `https://tardis.dev` — Hyperliquid perpetual OHLCV, funding rates
- **Kaiko (institutional, paid):** `https://www.kaiko.com` — spot and derivatives historical data

### Aave Safety Module
- **stkAAVE contract:** `0x4da27a545c0c5B758a6BA100e3a049001de870f5` on Ethereum mainnet
- **Slash function:** `slash(address destination, uint256 amount)` — monitor for `Slashed` events
- **Aave Umbrella (new):** Monitor `https://governance.aave.com` for Umbrella deployment announcements and `UmbrellaController` contract address (TBD post-deployment)

### Alerting Infrastructure
- **Tenderly Alerts:** `https://tenderly.co` — set contract event alerts on Vow `Kick`, stkAAVE `Slashed`, Governor Bravo `ProposalQueued`
- **OpenZeppelin Defender:** `https://defender.openzeppelin.com` — alternative for contract monitoring
- **Governance alert bots:** `https://boardroom.io` aggregates governance activity across protocols with API access

---

*This document is a hypothesis specification. No backtest has been run. All claims about mechanism are based on smart contract documentation and protocol design — verify against current deployed contract code before trading. Protocol upgrades may have altered mechanisms described above.*
