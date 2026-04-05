---
title: "Options Vault Theta Decay Rebalance — Ribbon/Thetanuts Weekly Roll Pressure"
status: HYPOTHESIS
mechanism: 5
implementation: 2
safety: 4
frequency: 6
composite: 240
categories:
  - defi-protocol
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Structured options vaults (Ribbon Finance, Thetanuts, Friktion) operate on mechanically fixed weekly cycles governed by smart contract logic. Every Friday, these vaults execute two sequential forced transactions at predictable timestamps:

1. **Buy-back leg:** Purchase expiring short options (calls or puts) to close the prior week's position
2. **Sell-to-open leg:** Sell new ATM/OTM options for the following week to re-establish yield

These are not discretionary trades — they are protocol-enforced, on-chain, and visible in advance. The concentrated, one-directional flow at a known timestamp creates a mechanical IV distortion on the Deribit options surface:

- **Short-dated IV spikes** briefly as vaults buy back expiring options (demand shock on near-zero-DTE options)
- **Next-week IV is suppressed** as vaults dump new supply into the market (supply shock on 7-DTE options)

A trader who positions *ahead of* the sell-to-open leg (buying next-week options before vault supply hits) and *exits after* the roll completes captures the IV compression as a loss avoided — or alternatively, buys the short-dated options ahead of the buy-back spike.

**The edge is not "IV tends to move on Fridays." The edge is: a known, on-chain, contractually scheduled actor will execute a known directional trade at a known time, and that trade is large enough relative to altcoin options market depth to temporarily distort the vol surface.**

---

## Structural Mechanism

### Why This Must Happen (Not Just Tends To)

Ribbon Finance and Thetanuts vaults are governed by smart contracts with hardcoded weekly epoch structures. The vault lifecycle is:

```
Monday–Thursday:  Options position held, theta accruing
Thursday ~23:00 UTC: Vault enters "withdrawal window" — no new deposits
Friday ~08:00–12:00 UTC: Vault calls commitAndClose() → closes expiring options
Friday ~12:00–16:00 UTC: Vault calls rollToNextOption() → sells new weekly options
```

These function calls are:
- **Publicly visible** in the mempool before execution
- **Timestamped** by on-chain block data historically
- **Deterministic** — the vault cannot skip the roll without breaking the epoch

### The Flow Mechanics

At peak TVL (~$300M across Ribbon ETH covered call vault alone), the vault was selling approximately **$15–30M notional in weekly ETH calls** in a single transaction window. Deribit's ETH options market has meaningful depth, but a single $20M vega dump in a 2-hour window on a specific strike is observable in the IV surface.

The distortion is more pronounced for:
- **Altcoin vaults** (smaller underlying options markets, less depth)
- **Periods of low overall options market activity** (weekends, low-vol regimes)
- **ATM strikes** where vault concentration is highest

### Why the Edge Persists

1. Most options traders are not monitoring on-chain vault transactions
2. The distortion window is short (2–4 hours) — too short for weekly/monthly options strategies to exploit
3. The trade requires simultaneous on-chain monitoring + options execution — operationally complex for most participants
4. Vault TVL has declined from peak, reducing the signal, which paradoxically means fewer arb traders are watching

---

## Entry Rules

### Trade A: Pre-Roll Long Vol (Primary)

**Objective:** Buy next-week options before vault supply depresses IV

| Parameter | Specification |
|-----------|--------------|
| **Trigger** | On-chain detection of vault `commitAndClose()` call OR Thursday 22:00 UTC (whichever comes first) |
| **Instrument** | Deribit ATM weekly calls (or puts for put-selling vaults) on ETH or BTC |
| **Expiry** | Next Friday expiry (7 DTE at entry) |
| **Strike** | Nearest ATM strike |
| **Entry window** | Thursday 22:00 UTC → Friday 10:00 UTC |
| **Entry condition** | Current 7-DTE IV must be within 2 vol points of 30-day average (no entry if IV already elevated) |

### Trade B: Buy-Back Spike Capture (Secondary)

**Objective:** Buy expiring same-day options ahead of vault buy-back demand

| Parameter | Specification |
|-----------|--------------|
| **Trigger** | On-chain mempool detection of vault `rollToNextOption()` pending transaction |
| **Instrument** | Deribit 0-DTE calls (expiring same Friday) |
| **Strike** | ATM or 1-strike OTM |
| **Entry window** | Friday 07:00–09:00 UTC (before typical roll window) |
| **Entry condition** | 0-DTE IV must be below prior 4-week Friday average at same time |

**Note:** Trade B is higher risk — 0-DTE options have extreme gamma, and if the vault roll is delayed or the options expire worthless before buy-back, loss is total. Trade A is the primary focus.

---

## Exit Rules

| Scenario | Action |
|----------|--------|
| **Roll confirmed on-chain** (vault `rollToNextOption()` mined) | Exit Trade A within 60 minutes of confirmation |
| **IV expands ≥ 2 vol points** from entry on Trade A | Exit immediately, take profit |
| **IV compresses ≥ 3 vol points** from entry (vault selling harder than expected) | Stop loss — exit Trade A |
| **4 hours elapsed** from entry, no roll confirmation | Exit regardless (time stop) |
| **Trade B:** Options reach 50% of premium paid | Exit Trade B (theta destruction accelerating) |
| **Trade B:** Roll confirmed | Exit Trade B within 30 minutes |

**Maximum hold time:** 6 hours from entry for Trade A, 2 hours for Trade B.

---

## Position Sizing

### Sizing Philosophy

Options premium is the maximum loss. Size so that a total loss on the position is a defined, small fraction of portfolio.

| Parameter | Rule |
|-----------|------|
| **Max premium at risk per trade** | 0.5% of total portfolio |
| **Max concurrent vault roll trades** | 2 (one BTC, one ETH — never two on same underlying) |
| **Max weekly exposure** | 1.0% of portfolio in premium |
| **Notional vega limit** | Size such that a 5-vol-point adverse move = 0.5% portfolio loss |

### Sizing Formula

```
Max Premium = Portfolio × 0.005
Contracts = Max Premium / Option Price per Contract
Verify: Contracts × Vega × 5 ≤ Portfolio × 0.005
Use the binding constraint (whichever gives fewer contracts)
```

### Scaling Rules

- **Scale up** only after 20+ confirmed trades with positive expectancy
- **Scale down by 50%** if 3 consecutive losses occur
- **Pause entirely** if vault TVL drops below $50M (insufficient flow to move IV)

---

## Backtest Methodology

### Data Requirements

| Data Type | Source | Availability |
|-----------|--------|-------------|
| Vault on-chain transaction history | Etherscan, The Graph (Ribbon subgraph) | ✅ Public |
| Vault epoch timestamps (historical) | Ribbon/Thetanuts GitHub, on-chain events | ✅ Public |
| Deribit historical IV surface | Deribit API (historical data endpoint) | ✅ Public (some paid tiers) |
| Deribit options OHLCV | Tardis.dev | ✅ Paid, comprehensive |
| Vault TVL history | DeFiLlama | ✅ Public |

### Backtest Steps

**Step 1: Extract vault roll timestamps**
- Pull all `commitAndClose()` and `rollToNextOption()` events from Ribbon ETH Covered Call Vault (0x25751853Eab4D0eB3652B5eB6ecB102A2789644) and Thetanuts equivalents
- Record exact block timestamp for each event
- Date range: January 2022 – present (covers peak TVL and decline)

**Step 2: Construct IV surface snapshots**
- For each roll event, extract Deribit IV for:
  - ATM 0-DTE options: T-24h, T-4h, T-2h, T-1h, T+1h, T+2h, T+4h
  - ATM 7-DTE options: T-24h, T-4h, T-2h, T-1h, T+1h, T+2h, T+4h
- Calculate IV delta across these windows

**Step 3: Measure signal**
- Primary metric: Does 7-DTE IV compress in the 2-hour window after `rollToNextOption()`?
- Secondary metric: Does 0-DTE IV spike in the 2-hour window before `rollToNextOption()`?
- Segment by: vault TVL quartile, underlying (ETH/BTC/altcoin), market vol regime

**Step 4: Simulate P&L**
- For Trade A: Model buying ATM straddle at T-2h, selling at T+2h using Deribit mid prices
- Account for: bid-ask spread (use 0.5× spread as cost), Deribit fees (0.03% of notional)
- For Trade B: Model buying 0-DTE ATM call at T-4h, selling at T+30min post-roll

**Step 5: Stratify results**
- P&L by TVL bucket (>$100M, $50–100M, <$50M)
- P&L by vol regime (VIX proxy: ETH 30-day realized vol quartile)
- P&L by day-of-week (confirm Friday concentration)

### Key Backtest Questions

1. Is the IV compression on 7-DTE options statistically significant vs. random Friday IV moves?
2. What is the minimum vault TVL for a detectable signal?
3. Does the signal persist after transaction costs?
4. Is the signal stronger on altcoin vaults (smaller options markets)?

---

## Go-Live Criteria

All of the following must be satisfied before committing real capital:

| Criterion | Threshold |
|-----------|-----------|
| Backtest sample size | ≥ 40 vault roll events |
| Backtest Sharpe (annualized) | ≥ 1.5 |
| Win rate | ≥ 55% (options premium decay means wins must be larger than losses) |
| Average win / average loss | ≥ 1.8× |
| Signal present at current TVL levels | Must show positive expectancy in TVL < $100M subsample |
| Paper trade confirmation | ≥ 8 consecutive weeks of paper trading with positive expectancy |
| On-chain monitoring operational | Automated alert system for vault transactions live and tested |
| Maximum drawdown in backtest | ≤ 15% of strategy allocation |

---

## Kill Criteria

Abandon or pause the strategy immediately if:

| Trigger | Action |
|---------|--------|
| Vault TVL drops below $30M across all monitored vaults | **Pause** — insufficient flow |
| 5 consecutive losing trades in live trading | **Pause** — review mechanism |
| Cumulative live drawdown exceeds 20% of strategy allocation | **Stop** — structural review required |
| Ribbon/Thetanuts vaults change roll timing or frequency | **Stop** — re-validate entire thesis |
| Deribit changes fee structure materially | **Re-evaluate** — cost basis changes |
| Evidence that other systematic players are front-running the same signal | **Stop** — edge is competed away |
| Vault protocols sunset or migrate to new contracts | **Stop** — re-validate on new contracts |

---

## Risks

### Risk 1: TVL Decline (HIGH — Current)
Ribbon and Thetanuts TVL has declined significantly from 2022 peaks. At current TVL levels, vault flow may be insufficient to move Deribit IV meaningfully. **Mitigation:** Only trade when combined vault TVL > $50M; monitor DeFiLlama daily.

### Risk 2: Roll Timing Drift (MEDIUM)
Vault operators can adjust roll timing within the epoch window. Smart contract allows some flexibility in when the keeper calls the roll function. **Mitigation:** Use on-chain mempool monitoring rather than fixed-time entry; enter only after `commitAndClose()` is confirmed.

### Risk 3: Options Illiquidity (MEDIUM)
Deribit bid-ask spreads on altcoin options can be wide (5–10 vol points). Transaction costs may consume the entire IV edge. **Mitigation:** Only trade ETH and BTC options where spreads are tightest; model realistic spread costs in backtest.

### Risk 4: Gamma Risk on Trade B (HIGH)
0-DTE options have extreme convexity. If the vault roll is delayed by even 1–2 hours, the options can expire worthless before the buy-back demand materializes. **Mitigation:** Trade B is secondary, sized at 25% of Trade A allocation; strict time stop.

### Risk 5: Competing Arb Traders (MEDIUM)
If this edge becomes known, other traders will front-run the front-run, compressing or eliminating the signal. **Mitigation:** Monitor IV behavior in the T-4h window; if IV is already moving before vault roll, the edge is competed away.

### Risk 6: Protocol Risk (LOW-MEDIUM)
Smart contract bugs, vault pauses, or governance changes could disrupt the roll schedule. **Mitigation:** Monitor vault governance forums; maintain kill switch.

### Risk 7: Correlation to Spot Market (MEDIUM)
Large spot moves on Friday can overwhelm the IV distortion signal. A 5%+ spot move will dominate any vault-driven IV effect. **Mitigation:** Do not enter if spot has moved >3% in the prior 4 hours; use straddles rather than directional options to isolate vol exposure.

---

## Data Sources

| Source | Data | URL | Cost |
|--------|------|-----|------|
| Etherscan / Alchemy | Vault on-chain events, transaction timestamps | etherscan.io | Free / Paid |
| The Graph — Ribbon subgraph | Structured vault epoch data | thegraph.com | Free |
| Deribit API | Live and historical IV surface, options OHLCV | docs.deribit.com | Free (limited history) |
| Tardis.dev | Full historical Deribit options tick data | tardis.dev | Paid (~$500/mo) |
| DeFiLlama | Vault TVL history | defillama.com | Free |
| Ribbon Finance GitHub | Vault contract addresses, ABI, epoch logic | github.com/ribbon-finance | Free |
| Thetanuts Finance docs | Vault parameters, roll schedule | thetanuts.finance | Free |

---

## Monitoring & Operational Requirements

### Pre-Trade Checklist (Every Thursday)
- [ ] Check combined vault TVL on DeFiLlama (must be > $50M)
- [ ] Check ETH/BTC 4-hour spot move (must be < 3%)
- [ ] Check 7-DTE IV vs. 30-day average (must be within 2 vol points)
- [ ] Confirm vault epoch is active (no governance pause)
- [ ] Verify on-chain monitoring alert system is live

### On-Chain Monitoring Setup
- Subscribe to vault contract events via Alchemy/Infura webhook
- Alert on: `commitAndClose()` call, `rollToNextOption()` call
- Alert latency target: < 30 seconds from block confirmation
- Backup: Manual check of vault contract on Etherscan every 2 hours on Fridays

### Post-Trade Review (Every Friday)
- Record: Entry IV, exit IV, IV delta, P&L, vault TVL at time of trade
- Log: Was roll timing as expected? Any anomalies?
- Update: Running expectancy calculation

---

## Open Research Questions

1. **Altcoin vaults:** Thetanuts runs vaults on AVAX, MATIC, SOL options. Are these markets thin enough that even $5M TVL creates a detectable IV distortion? Needs separate analysis.
2. **Put-selling vaults:** The mechanism should work symmetrically for cash-secured put vaults (Ribbon ETH Put Selling). Does the IV distortion appear on the put side of the surface?
3. **Competitor vaults:** Friktion (Solana, now defunct), Katana (Solana), Cega — do any active vaults on other chains create similar dynamics on their respective options markets?
4. **Vault TVL recovery:** If DeFi structured products TVL recovers (e.g., in a bull market), does the signal strength recover proportionally? This would make TVL a leading indicator for strategy activation.
5. **Cross-asset:** Does the ETH vault roll create any detectable effect on BTC IV (correlated vol surfaces)? Unlikely but worth checking.

---

## Summary Assessment

| Dimension | Assessment |
|-----------|-----------|
| **Structural mechanism** | ✅ Real — on-chain, contractually scheduled, observable |
| **Causal logic** | ✅ Clear — forced supply/demand at known time |
| **Current signal strength** | ⚠️ Reduced — TVL down from peak |
| **Data availability** | ✅ Good — all sources public or accessible |
| **Operational complexity** | ⚠️ Medium-high — requires on-chain monitoring + options execution |
| **Edge durability** | ⚠️ Moderate — depends on vault TVL recovery and lack of competing arb |
| **Recommended next step** | Pull Ribbon vault roll timestamps + Deribit IV history, run Step 2–3 of backtest methodology |

**Bottom line:** The mechanism is real and the data exists to test it. The primary risk is that current vault TVL is too low to generate a detectable signal. The backtest will answer this definitively. Do not allocate capital until TVL-stratified backtest confirms positive expectancy at current TVL levels.
