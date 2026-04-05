---
title: "Governance Timelock LP Migration — Destination Pool Liquidity Seeding"
status: HYPOTHESIS
mechanism: 6
implementation: 2
safety: 5
frequency: 3
composite: 180
categories:
  - governance
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DeFi protocol passes a governance proposal to migrate protocol-owned liquidity (POL) from Pool A to Pool B, the timelock delay (typically 2–7 days) creates a known, block-precise window before a large TVL injection into Pool B. By entering Pool B as an LP before the migration executes, we capture:

1. **Elevated fee revenue** during the post-migration period when Pool B handles the protocol's full trading volume on a temporarily shallow pre-migration TVL base (i.e., before the POL arrives, the pool is thin and fees per LP dollar are high)
2. **Spread widening arb** on Pool A as its liquidity shallows post-withdrawal — tradeable via perp short on the underlying token if Pool A was a primary price discovery venue

**Causal chain:**
```
Governance proposal passes
        ↓
Timelock countdown begins (block-precise end known)
        ↓
At execution block: protocol withdraws LP from Pool A
        ↓
Pool A TVL drops → slippage increases → price impact per trade rises
        ↓
Protocol deposits into Pool B
        ↓
Pool B TVL rises → fee revenue redistributes to existing LPs first
        ↓
Early LPs in Pool B earn outsized fees during the TVL ramp period
```

The edge is **not** that fees are permanently elevated — it is that there is a brief window (hours to days) where Pool B has incoming volume routed to it but our LP capital entered before the POL dilutes our share.

---

## Structural Mechanism — WHY This Must Happen

This is not pattern-based. The following are **contractually enforced**:

1. **Timelock execution is deterministic.** Once a proposal is queued in a timelock contract (e.g., OpenZeppelin `TimelockController`), the `eta` (earliest execution timestamp) is written on-chain. The transaction payload — including the exact LP withdrawal calldata — is readable in the queued transaction. There is no ambiguity about what will happen or when.

2. **Protocol-owned liquidity withdrawal is atomic.** The migration executes in a single transaction (or a batched multicall). Pool A loses the POL in one block. This is not a gradual drain — it is a cliff event.

3. **LP fee distribution is pro-rata by share at time of trade.** In Uniswap v2/v3 and Curve, fees accrue to LPs proportional to their pool share *at the time the swap occurs*. If we hold 10% of Pool B's pre-migration liquidity and the protocol's POL represents 90% of the incoming deposit, we earn 10% of all fees generated between our entry and the POL deposit — even if that window is only 6 hours.

4. **Volume follows liquidity migration.** Protocols that migrate POL typically update their router/aggregator routing simultaneously or within hours. Aggregators (1inch, Paraswap, CowSwap) re-route to Pool B once it has sufficient depth. This is mechanical — aggregators optimize for best execution and will route to the deeper pool.

**What is NOT guaranteed:** The magnitude of fee revenue. Volume may be low. The migration may be cancelled (governance guardian veto). MEV bots may sandwich our LP entry. These are risks, not structural failures.

---

## Entry/Exit Rules

### Trigger Conditions (all must be met)
- [ ] A timelock-queued transaction is identified that contains LP withdrawal calldata from Pool A AND LP deposit calldata into Pool B
- [ ] The protocol's POL in Pool A is ≥ $500K (smaller migrations not worth monitoring)
- [ ] Pool B exists and has ≥ $100K existing TVL (avoid bootstrapping a dead pool)
- [ ] Time remaining on timelock: ≥ 6 hours (enough time to enter safely), ≤ 7 days (don't hold LP for weeks)
- [ ] Pool B is NOT a Uniswap v3 concentrated liquidity pool with narrow range (IL risk too complex for this spec — exclude for now)
- [ ] No active governance guardian veto signal (check forum/Discord for cancellation discussion)

### Entry
- **Instrument:** LP position in Pool B (on-chain, not a perp)
- **Timing:** Enter LP position 2–6 hours before timelock `eta` timestamp
- **Why not earlier?** Holding LP for days pre-migration exposes unnecessary IL. The fee capture window is post-migration, not pre.
- **Why not at execution block?** MEV bots will front-run the execution block itself. Enter 2–6 hours before to avoid competing with atomic sandwich bots.

### Optional Directional Overlay (Pool A Short)
- If Pool A is a primary price discovery venue for the token (i.e., it is the largest pool by TVL for that pair), open a small perp short on the token via Hyperliquid
- Entry: same 2–6 hour window before execution
- Rationale: Pool A shallowing increases slippage and volatility; thin pools are more easily moved by sellers who know the migration is coming
- This is the **weaker** leg — treat as optional and size at 25% of total position risk

### Exit — LP Position
- **Primary exit:** 48–72 hours post-migration execution
- **Early exit trigger:** If Pool B TVL (post-POL deposit) exceeds 10x our LP position size, our fee share is diluted to noise — exit immediately
- **Late exit trigger:** If Pool B volume (24h) drops below $50K after migration, fees are not materializing — exit

### Exit — Perp Short (if held)
- Cover short within 24 hours of migration execution
- Stop loss: +8% adverse move on the token from entry price

---

## Position Sizing

**Capital allocation per trade:** 1–3% of total strategy capital

**Rationale for small size:**
- IL risk on LP positions is real and unhedged in this spec
- Individual migration events are infrequent and idiosyncratic
- This is a portfolio of small, uncorrelated bets — not a concentrated position

**LP position sizing:**
- Target: no more than 5% of Pool B's pre-migration TVL
- This ensures we are not ourselves moving the pool or creating IL for others
- Example: Pool B has $200K TVL pre-migration → max LP entry = $10K

**Perp short sizing (if used):**
- 25% of LP position notional
- Example: $10K LP → $2,500 short notional on Hyperliquid
- Max leverage: 2x (this is a structural trade, not a momentum trade — no need for leverage)

**Per-event max loss:** 2% of strategy capital (LP position + perp short combined)

---

## Backtest Methodology

### What We Are Testing
Not a price pattern — we are testing whether early LP entry into destination pools before governance-triggered migrations generates positive fee revenue net of IL and gas costs.

### Data Sources

| Data Type | Source | URL/Endpoint |
|---|---|---|
| Timelock queued transactions | Etherscan API | `https://api.etherscan.io/api?module=logs&action=getLogs` filtered by `CallScheduled` event (OpenZeppelin Timelock) |
| Protocol-owned liquidity | DefiLlama POL | `https://defillama.com/protocol/{protocol}` → "Protocol Owned Liquidity" tab |
| Pool TVL history | DefiLlama pools | `https://yields.llama.fi/pools` + `https://yields.llama.fi/chart/{pool_uuid}` |
| Pool fee revenue history | Uniswap v3 subgraph | `https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3` — `poolDayDatas` entity |
| Curve pool data | Curve API | `https://api.curve.fi/api/getPools/ethereum/main` |
| Historical governance proposals | Tally API | `https://api.tally.xyz/query` — GraphQL, free tier |
| Token price history | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |

### Historical Event Set Construction
1. Query Tally for all passed proposals (2021–present) on Uniswap, Curve, Aave, Compound, Balancer, Frax, MakerDAO
2. Filter for proposals containing keywords: "migrate", "liquidity", "pool", "redeploy", "deprecate"
3. Cross-reference with on-chain timelock `CallScheduled` events to get exact execution timestamps
4. Manually verify each event: confirm POL withdrawal from Pool A and deposit into Pool B
5. Target sample size: ≥ 20 confirmed migration events (expect ~30–50 across major protocols 2021–2024)

### Metrics to Calculate Per Event

**Fee revenue:**
- Pool B fee APR in the 72-hour window post-migration (annualized, then de-annualized to 3-day figure)
- Baseline: Pool B fee APR in the 72-hour window *before* migration (pre-migration baseline)
- Signal: Is post-migration fee APR > pre-migration fee APR? By how much?

**IL measurement:**
- Token price change from LP entry to exit (72h window)
- Calculate IL using standard formula: `IL = 2√(price_ratio)/(1+price_ratio) - 1`
- Net P&L = fee revenue - IL - gas costs (estimate gas at $20–50 per LP entry/exit on Ethereum mainnet)

**Pool A slippage widening:**
- Measure Pool A TVL drop at execution block
- Measure Pool A 24h volume in the 48h post-migration vs. 48h pre-migration
- If Pool A volume drops >50% post-migration, the perp short thesis is supported

### Baseline Comparison
- Compare fee revenue of "entering Pool B 2–6h before migration" vs. "entering Pool B 72h after migration"
- The structural claim is that early entry captures higher fee-per-dollar due to lower pool share dilution

### Key Metrics for Evaluation
- **Hit rate:** % of events where net P&L (fees - IL - gas) > 0
- **Average net P&L per event** (in USD and % of capital deployed)
- **Fee revenue uplift:** Average fee APR in 72h post-migration vs. 72h pre-migration in Pool B
- **IL frequency:** % of events where IL exceeded fee revenue
- **Cancellation rate:** % of queued proposals that were cancelled before execution (this is a pure loss — gas wasted on entry/exit)

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Sample size:** ≥ 15 confirmed migration events in backtest dataset
2. **Hit rate:** ≥ 60% of events show positive net P&L after fees, IL, and gas
3. **Average net P&L:** ≥ $200 per event at $10K position size (2% net return minimum — otherwise gas costs dominate)
4. **Fee uplift confirmed:** Post-migration Pool B fee APR is statistically higher than pre-migration baseline in ≥ 70% of events
5. **IL manageable:** Average IL across events < 1.5% (if IL routinely exceeds fees, the strategy is structurally broken)
6. **Cancellation rate:** < 20% of queued proposals cancelled (if governance cancels frequently, the signal is unreliable)

---

## Kill Criteria

Abandon the strategy (during paper trading or live) if:

- **3 consecutive losses** where IL > fee revenue, net P&L negative
- **MEV dominance confirmed:** On-chain analysis shows MEV bots entering Pool B within the same block as our target entry window, consistently capturing >80% of fee revenue in the first 24h
- **Governance cancellation rate rises above 30%** in live monitoring (protocol governance becoming more adversarial/unpredictable)
- **Gas costs exceed 50% of gross fee revenue** on average across 10 live events (Ethereum L1 gas spikes make this uneconomical — consider L2 migration events only if this triggers)
- **Regulatory/protocol change:** Uniswap or Curve introduces fee-switch or LP structure change that invalidates the fee accrual model

---

## Risks — Honest Assessment

### High Severity

**Impermanent Loss:** This is the dominant risk. If the token in Pool B moves >5% during the 72h hold, IL can exceed fee revenue entirely. This strategy has no IL hedge in its current form. Mitigation: prefer stablecoin/stablecoin pools or stablecoin/ETH pools where IL is lower.

**MEV Competition:** On Ethereum mainnet, MEV bots monitor the same timelock contracts. A bot with better infrastructure will enter Pool B in the same block as the timelock execution, capturing the first-mover fee advantage. Our 2–6h early entry is a partial mitigation but does not eliminate this risk. **This is the most likely reason the strategy underperforms in backtest.**

**Governance Cancellation:** OpenZeppelin timelocks have a guardian/canceller role. Proposals can be cancelled after queuing. If we enter LP and the migration is cancelled, we hold an LP position with no catalyst — pure IL exposure until we exit.

### Medium Severity

**Volume Doesn't Follow:** Aggregators may not immediately re-route to Pool B. If the protocol's frontend is slow to update or aggregators deprioritize Pool B (e.g., insufficient depth signals), volume stays on Pool A longer than expected and Pool B fees don't materialize.

**Small Event Size:** Most governance LP migrations involve <$1M POL. At $10K LP position size and 5% pool share, fee revenue on a $200K/day volume pool at 0.3% fee rate = $600/day total fees × 5% share = $30/day. Over 3 days = $90 gross. Gas costs on Ethereum = $40–80. Net = $10–50. This is not a scalable strategy on Ethereum L1 for small migrations.

**Concentrated Liquidity Complexity:** Uniswap v3 migrations involve range selection. If the protocol deposits POL in a narrow range that doesn't overlap with our range, we capture zero of the fee flow. This spec excludes v3 for now — but that excludes a large portion of the opportunity set.

### Low Severity

**Liquidity Lock:** Some LP positions have withdrawal delays (e.g., Curve gauge locks). Verify Pool B has no lock before entry.

**Smart Contract Risk:** Standard DeFi LP risk — pool contract exploit during hold period. Mitigate by only using audited, battle-tested pools (Uniswap v2, Curve main pools).

---

## Implementation Notes

### Monitoring Infrastructure Required
- Webhook or cron job polling timelock contracts for `CallScheduled` events every 15 minutes
- Contracts to monitor:
  - Uniswap Timelock: `0x1a9C8182C09F50C8318d769245beA52c32BE35BC`
  - Compound Timelock: `0x6d903f6003cca6255D85CcA4D3B5E5146dC33925`
  - Aave Executor (Short): `0xEE56e2B3D491590B5b31738cC34d5232F378a8D5`
  - Curve Ownership Admin: `0x40907540d8a6C65c637785e8f8B742ae6b0b9968`
- Parse `calldata` of queued transactions to identify LP-related function signatures (`removeLiquidity`, `withdraw`, `addLiquidity`, `deposit`)

### Manual Review Required
Each candidate event requires human review before capital deployment:
- Confirm the proposal text matches the calldata (governance forums: Tally, Snapshot, protocol forums)
- Confirm Pool B is the intended destination (not an intermediate step)
- Check for active cancellation discussion on governance forum

### L2 Consideration
Arbitrum and Optimism have lower gas costs ($0.10–1.00 per transaction vs. $20–80 on mainnet). If migration events on L2 protocols (e.g., GMX, Camelot, Velodrome) are included, the economics improve significantly. Recommend expanding scope to L2 in backtest phase 2.

---

## Summary Assessment

This strategy has a **real structural mechanism** (timelock execution is deterministic, fee accrual is pro-rata) but a **thin and execution-sensitive edge**. The primary value is as a **monitoring framework** — the timelock queue is a free, real-time feed of guaranteed future liquidity events. The LP seeding trade is one application; others (directional perp trades on Pool A shallowing, governance token trades) may prove more tractable in backtest.

**Recommended next step:** Build the historical event dataset from Tally + Etherscan before committing to backtest infrastructure. If fewer than 15 clean migration events exist in the 2021–2024 dataset, the strategy cannot be validated and should be parked.
