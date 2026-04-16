---
title: "Symbiotic/Karak Slashing Condition — Pre-Slash Unwind Positioning"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 1
composite: 180
categories:
  - defi-protocol
  - liquidation
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a slashing event is initiated against a Symbiotic or Karak vault, the restaked collateral token faces a contractually guaranteed NAV reduction during the governance timelock window (typically 24–72 hours). This window creates an asymmetric short opportunity: the slash execution is mechanically certain absent a veto, the veto is observable on-chain, and the market has not yet priced the impairment because no systematic monitoring tooling exists for these protocols. The edge is the information asymmetry between on-chain event visibility and market pricing, combined with the mechanical certainty of NAV destruction upon execution.

---

## Structural Mechanism

### Why This Edge Must Exist

1. **Contractual NAV destruction:** Symbiotic's `ISlasher` contract reduces vault collateral by a defined slash amount upon execution. This is not probabilistic — the smart contract arithmetic is deterministic. If slash amount = X tokens, vault NAV decreases by exactly X tokens.

2. **Timelock delay creates a window:** Symbiotic implements a two-phase slash: `SlashingRequested` (initiation) → optional veto period → `Slashed` (execution). The delay is a protocol parameter, currently 24–72 hours depending on vault configuration. This delay exists to allow operators to dispute, but it also creates a guaranteed window where the outcome is known before the market prices it.

3. **Veto is the only escape valve:** The only mechanism that prevents execution is an explicit veto transaction from the vault's resolver. Veto is observable on-chain in real time. If no veto is observed within the timelock window, execution is guaranteed. This converts the trade from "probabilistic short" to "guaranteed convergence short" once the veto window expires without action.

4. **Market pricing lag is structural, not accidental:** Symbiotic and Karak have no equivalent of EigenLayer's third-party monitoring dashboards (e.g., EigenExplorer). Retail and most institutional participants have no alerting infrastructure on `ISlasher` events. The information exists publicly but requires active RPC monitoring to surface — a friction that creates persistent lag.

5. **Collateral tokens trade on liquid venues:** The restaked collateral (e.g., wstETH, WBTC, ETH) trades on Hyperliquid perpetuals with sufficient liquidity to absorb a position. The slash impairs the vault's collateral, creating selling pressure from vault depositors unwinding positions once the slash becomes public knowledge.

### Causal Chain

```
SlashingRequested emitted on-chain
        ↓
Vault NAV impairment is now mathematically certain (absent veto)
        ↓
Market has not priced this (no monitoring tooling, no alerts)
        ↓
Enter short on underlying collateral token
        ↓
[Branch A] Slash executes → collateral token sells off → close short at profit
[Branch B] Veto tx observed → stop out immediately, expected loss <5%
```

---

## Contract Addresses and Event Signatures

### Symbiotic (Ethereum Mainnet)

| Component | Address / Reference |
|---|---|
| `ISlasher` interface | Public on Symbiotic GitHub (`symbiotic-fi/core`) |
| `SlashingRequested` event | `event SlashingRequested(address indexed subnetwork, address indexed operator, uint256 slashAmount, uint48 captureTimestamp, uint256 vetoDeadline)` |
| `Slashed` event | `event Slashed(uint256 slashIndex, uint256 slashedAmount)` |
| `VetoSlash` event | `event VetoSlash(uint256 slashIndex, address indexed resolver)` |
| Resolver registry | `IVetoSlasher.resolverAt()` — identifies who can veto and their address |

### Karak (Ethereum Mainnet + L2s)

| Component | Reference |
|---|---|
| Slash events | Monitor `SlashingHandler` contract, `SlashRequested` event |
| Veto mechanism | `DSSSlashingHandler` — veto window configurable per DSS |
| Deployment | Ethereum mainnet + Arbitrum + Mantle — monitor all three |

**Action:** Deploy event listeners on all three chains via Alchemy/Infura free tier. Estimated setup time: 8–12 hours of engineering.

---

## Entry Rules

### Signal Detection

```python
# Pseudocode — implement as production monitor
def on_slash_requested(event):
    slash_amount = event['slashAmount']
    vault_tvl = get_vault_tvl(event['subnetwork'])
    slash_pct = slash_amount / vault_tvl
    
    if slash_pct > MINIMUM_SLASH_PCT:  # filter noise, set at 0.5%
        collateral_token = get_vault_collateral(event['subnetwork'])
        veto_deadline = event['vetoDeadline']
        alert_trader(collateral_token, slash_pct, veto_deadline)
```

### Entry Criteria (ALL must be met)

1. `SlashingRequested` event detected on `ISlasher` contract
2. Slash amount ≥ 0.5% of vault TVL (filters dust slashes with no price impact)
3. Collateral token has a liquid perpetual on Hyperliquid (ETH, BTC, wstETH proxy via ETH)
4. Veto deadline is ≥ 12 hours away (ensures sufficient time for position to work)
5. No `VetoSlash` event observed in the 60 minutes following detection
6. Entry executed within 60 minutes of `SlashingRequested` block confirmation

### Entry Execution

- **Instrument:** Hyperliquid perpetual on the collateral token (ETH-PERP, BTC-PERP, or closest proxy)
- **Entry type:** Market order — speed matters here; use limit order only if spread > 0.15%
- **Entry price:** Record entry price and block timestamp for P&L attribution

---

## Exit Rules

### Exit Scenario A — Slash Executes (Target Exit)

- **Trigger:** `Slashed` event observed on-chain
- **Action:** Close 100% of position at market within 15 minutes of event confirmation
- **Rationale:** Price impact from vault depositor unwinding typically occurs in the 1–4 hour window post-execution; close before the move fully dissipates

### Exit Scenario B — Veto Observed (Stop Out)

- **Trigger:** `VetoSlash` event observed on-chain at any point before execution
- **Action:** Close 100% of position at market immediately, target execution within 5 minutes
- **Expected loss:** 2–5% (veto is rare; market may have partially priced the slash already, creating a partial reversal)
- **Hard rule:** No holding through a veto event under any circumstances

### Exit Scenario C — Time Stop

- **Trigger:** Veto deadline passes + 2 hours with no `Slashed` event (anomalous state — contract bug or governance failure)
- **Action:** Close 50% immediately, hold remaining 50% for up to 6 additional hours, then close fully
- **Rationale:** Anomalous state introduces unknown risk; reduce exposure mechanically

### Exit Scenario D — Price Stop

- **Trigger:** Position moves against entry by 8% (collateral token rallies 8% post-entry)
- **Action:** Close 100% regardless of on-chain state
- **Rationale:** An 8% rally against a slash signal suggests either a veto is imminent or broader market is overriding the slash signal; structural edge is compromised

---

## Position Sizing

### Base Sizing Formula

```
Position Size = (Account Equity × Risk Per Trade) / (Stop Distance)

Where:
  Risk Per Trade = 2% of account equity (fixed)
  Stop Distance = 8% (price stop) or 5% (veto stop, use conservative estimate)
  
Conservative position size = Account Equity × 0.02 / 0.08 = 25% of equity at 1x leverage
```

### Leverage

- **Maximum leverage:** 3x on Hyperliquid perpetual
- **Recommended leverage:** 2x — slash events are rare enough that over-leveraging a single event is unnecessary; the edge is in the structural certainty, not in leverage amplification
- **Rationale for leverage cap:** Veto risk is binary and fast-moving; high leverage on a veto stop-out creates unacceptable drawdown

### Scaling Rules

- **Single event:** Deploy full position size (25% equity at 2x = 50% notional exposure)
- **Concurrent events:** If two slash events occur simultaneously on different collateral tokens, cap total notional at 60% of equity; size each position at 30% notional
- **Frequency cap:** Maximum 3 concurrent positions; beyond this, queue by slash percentage size (largest slash first)

---

## Backtest Methodology

### Challenge: Low Historical Event Frequency

Symbiotic launched Q3 2024, Karak Q1 2024. As of April 2026, estimated 0–6 slash events have occurred across both protocols. This sample size is insufficient for statistical significance. The backtest is therefore a **forensic reconstruction**, not a statistical backtest.

### Step 1 — Historical Event Reconstruction

1. Pull all `SlashingRequested` events from Symbiotic `ISlasher` contracts from genesis block to present using Alchemy Archive Node (free tier supports this)
2. Pull equivalent events from Karak `SlashingHandler` on Ethereum, Arbitrum, Mantle
3. Record: block timestamp, slash amount, vault TVL at time of event, collateral token, veto deadline, outcome (slashed vs. vetoed)
4. Cross-reference with `Slashed` and `VetoSlash` events to classify each event

### Step 2 — Price Impact Analysis

For each historical slash event:
1. Pull 1-minute OHLCV data for the collateral token from the event timestamp + 1 hour (entry) through execution timestamp + 4 hours (exit window)
2. Measure: price at entry, price at slash execution, price 1h/2h/4h post-execution
3. Calculate: max drawdown from entry, P&L at each exit point, veto stop-out P&L where applicable

### Step 3 — Proxy Analysis (Expand Sample)

Since Symbiotic/Karak slash history is thin, use EigenLayer AVS slashing events as a proxy:
1. Pull EigenLayer `SlashingRequested` events from EigenLayer contracts (longer history, more events)
2. Apply identical entry/exit rules
3. This tests whether the structural mechanism (slash initiation → price impact) holds in the broader restaking category
4. **Caveat:** EigenLayer has more monitoring tooling, so information asymmetry is lower; treat as a lower-bound estimate of edge

### Step 4 — Sensitivity Analysis

Test the following parameter variations:
- Entry delay: 30 min vs. 60 min vs. 2 hours post-event
- Minimum slash size: 0.1% vs. 0.5% vs. 1% of vault TVL
- Exit timing: at execution vs. 1h post-execution vs. 2h post-execution
- Stop distance: 5% vs. 8% vs. 12%

### Deliverable

A table with columns: `[Event Date | Protocol | Collateral | Slash% | Vetoed? | Entry Price | Exit Price | P&L% | Notes]`

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

1. **Monitor is live and tested:** Event listener has been running for ≥ 14 days with zero missed events (validate by replaying known historical events against the monitor)
2. **Alert latency < 5 minutes:** From block confirmation of `SlashingRequested` to trader notification, measured and logged
3. **Backtest complete:** Forensic reconstruction completed with ≥ 3 historical events analyzed (Symbiotic + EigenLayer proxy)
4. **Proxy backtest result:** EigenLayer proxy analysis shows positive expectancy (mean P&L per trade > 0) across ≥ 5 events
5. **Execution infrastructure tested:** Paper trade at least 1 full event cycle (entry → exit) on Hyperliquid testnet or with minimal real capital ($500 max)
6. **Veto resolver mapping complete:** For each active Symbiotic vault, the resolver address is identified and monitored separately so veto detection latency is minimized

---

## Kill Criteria

Abandon or pause the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| 3 consecutive veto stop-outs | Pause, investigate whether resolver behavior has changed; resume only after root cause identified |
| Slash event produces < 0.5% price move on collateral token | Reduce position size by 50%; re-evaluate minimum slash size threshold |
| Monitoring latency exceeds 15 minutes on any live event | Halt new entries until infrastructure is fixed; latency advantage is the entire edge |
| Symbiotic/Karak deploys a new slasher contract version | Halt immediately; re-audit new contract ABI before resuming |
| Competing monitoring tools become publicly available (e.g., Dune dashboard tracking ISlasher) | Re-score edge from 7/10 to 5/10; reduce position size by 50% to reflect reduced information asymmetry |
| Regulatory action against restaking protocols | Full halt; reassess protocol viability |

---

## Risks

### Risk 1 — Veto Risk (Primary)
**Description:** The resolver vetoes the slash, reversing the trade thesis.
**Probability:** Low but non-zero; veto exists specifically for disputed slashes.
**Mitigation:** Veto is observable on-chain in real time; stop-out is fast. Pre-map resolver addresses for each vault to minimize detection latency. Expected loss on veto stop-out: 2–5%.

### Risk 2 — Low Event Frequency
**Description:** 0–3 events per quarter means months may pass with no trades.
**Impact:** Strategy generates no revenue during quiet periods; monitoring infrastructure has ongoing cost (engineering time, RPC calls).
**Mitigation:** Run this as a satellite strategy alongside higher-frequency strategies. The per-event alpha justifies the infrastructure cost even at 4 events per year.

### Risk 3 — Collateral Token Illiquidity on Hyperliquid
**Description:** If the slashed collateral is a long-tail token not listed on Hyperliquid, the short cannot be executed directly.
**Mitigation:** Pre-map each active Symbiotic/Karak vault's collateral token to its Hyperliquid listing. For unlisted tokens, identify the closest correlated instrument (e.g., if collateral is rETH, short ETH-PERP as proxy). Document correlation coefficient for each proxy pair before going live.

### Risk 4 — Smart Contract Upgrade Risk
**Description:** Symbiotic or Karak upgrades the slasher contract, changing event signatures or adding new veto mechanisms not captured by the monitor.
**Mitigation:** Subscribe to Symbiotic/Karak GitHub release notifications and governance forums. Any contract upgrade triggers an immediate strategy halt and re-audit.

### Risk 5 — Market-Wide Correlation
**Description:** A slash event may coincide with a broad market rally (e.g., ETH pumps 15% on macro news simultaneously), overwhelming the slash-driven selling pressure.
**Mitigation:** The 8% price stop handles this mechanically. Do not override the stop based on conviction in the slash thesis — macro can dominate for days.

### Risk 6 — Front-Running by Validators/Operators
**Description:** The operator being slashed may have advance knowledge and hedge their own exposure, reducing the available price impact.
**Mitigation:** This is a risk to magnitude, not direction. The slash still executes; the price move may be smaller. Size conservatively and treat any positive P&L as confirmation.

### Risk 7 — Slash Amount Overestimation
**Description:** The `slashAmount` in `SlashingRequested` may be reduced at execution (partial slash).
**Mitigation:** Use the slash percentage of vault TVL as the sizing input, not the absolute amount. A partial slash still impairs NAV; the direction is correct even if magnitude is smaller.

---

## Data Sources

| Data Type | Source | Cost | Latency |
|---|---|---|---|
| Symbiotic `ISlasher` events | Alchemy/Infura RPC (free tier) | Free | ~2–5 seconds |
| Karak `SlashingHandler` events | Alchemy RPC (Ethereum + Arbitrum + Mantle endpoints) | Free | ~2–5 seconds |
| Vault TVL at event time | Symbiotic subgraph (The Graph, free) | Free | ~30 seconds |
| Collateral token price | Hyperliquid public API | Free | Real-time |
| Historical OHLCV for backtest | Hyperliquid historical data API | Free | N/A |
| EigenLayer proxy events | EigenExplorer API or direct RPC | Free | N/A |
| Resolver address mapping | `IVetoSlasher.resolverAt()` on-chain call | Free | On-demand |
| Contract ABI updates | Symbiotic GitHub (`symbiotic-fi/core`) | Free | Manual check |

**Total infrastructure cost:** $0 in data fees. Engineering time to build monitor: estimated 8–16 hours. Ongoing maintenance: 1–2 hours per week.

---

## Implementation Checklist

- [ ] Clone Symbiotic `core` repo; extract `ISlasher` and `IVetoSlasher` ABI
- [ ] Deploy event listener on Alchemy free tier for Ethereum mainnet
- [ ] Deploy event listener for Karak on Ethereum, Arbitrum, Mantle
- [ ] Build vault TVL lookup function using Symbiotic subgraph
- [ ] Map all active vault collateral tokens to Hyperliquid instruments
- [ ] Build veto resolver address registry (call `resolverAt()` for each active vault)
- [ ] Set up alerting (Telegram bot or PagerDuty) with < 5 minute latency target
- [ ] Replay historical `SlashingRequested` events against monitor to validate zero missed events
- [ ] Pull EigenLayer historical slash events for proxy backtest
- [ ] Complete forensic backtest table
- [ ] Paper trade first live event before deploying full capital

---

## Open Questions (Must Resolve Before Go-Live)

1. **What is the actual historical veto rate?** Pull all `SlashingRequested` vs. `VetoSlash` events from genesis to present. If veto rate > 30%, re-score the strategy downward.
2. **What is the median price impact per 1% of vault TVL slashed?** This determines whether the slash size threshold of 0.5% is appropriate or should be raised.
3. **Does Karak have a different timelock structure than Symbiotic?** Read `DSSSlashingHandler` source code; document the exact veto window per DSS configuration.
4. **Are there any slashing events that occurred before Hyperliquid listed the collateral token?** If so, what was the spot market impact, and can we use spot as a fallback execution venue?
5. **Is the Symbiotic subgraph reliable for TVL data, or does it lag?** If subgraph lag > 5 minutes, switch to direct on-chain `totalStake()` calls for real-time TVL.
