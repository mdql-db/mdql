---
title: "ERC-4626 Vault Share Price Ratchet — Yield Accrual Arbitrage"
status: HYPOTHESIS
mechanism: 3
implementation: 2
safety: 2
frequency: 3
composite: 36
categories:
  - defi-protocol
  - lending
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

ERC-4626 vault tokens trading on secondary DEX markets periodically misprice relative to their on-chain NAV because the vault's share price updates every block (monotonically, by smart contract invariant) while DEX pool prices update only on trade. When underlying lending utilization spikes past the interest rate kink, the per-block NAV increment accelerates, widening the gap faster than passive DEX liquidity can absorb. The arb is: redeem vault tokens at contract-enforced NAV, receive underlying, sell underlying at market. Convergence is guaranteed by the redemption function itself — not by market sentiment.

---

## Structural Mechanism

### The Invariant

ERC-4626 defines `convertToAssets(shares)` as the authoritative exchange rate between vault shares and underlying assets. For yield-bearing vaults (Morpho MetaMorpho, Yearn v3, Aave v3 wrapped vaults), this function returns a value that **can only increase or stay flat between fee harvests** — it cannot decrease unless the vault suffers a bad debt event. This is enforced at the smart contract level: yield accrues by inflating `totalAssets` while `totalSupply` remains constant between mints/burns.

### The Gap Creation Mechanism

```
Block N:   NAV = 1.00412 USDC per share   DEX price = 1.00410 USDC per share   Gap = 0.002%
Block N+500 (utilization spike):
           NAV = 1.00431 USDC per share   DEX price = 1.00411 USDC per share   Gap = 0.020%
```

The DEX pool (Curve StableSwap or Uniswap v3 concentrated liquidity) holds a static ratio of vault tokens to underlying until a trader moves it. During a utilization spike, the vault's `totalAssets` grows faster per block because the borrow rate has jumped from, say, 8% APY to 40% APY above the kink. The DEX price lags because no one has traded the pool yet. The gap is the arb.

### Why Convergence Is Guaranteed

The redemption path is: call `redeem(shares, receiver, owner)` on the vault contract → receive `convertToAssets(shares)` worth of underlying → the contract enforces this rate regardless of DEX price. There is no counterparty risk on the convergence leg. The only execution risk is gas cost and the time between submitting the transaction and it landing on-chain.

### Utilization Kink Mechanics

Aave v3 and Morpho use a two-slope interest rate model. Below the optimal utilization (typically 80–92%), rates are low and flat. Above it, rates increase steeply — often 10x steeper. When a large borrower draws down a pool or a liquidation cascade consumes liquidity, utilization can jump from 85% to 97% within a single block, spiking the borrow rate from ~8% APY to ~60% APY. At 60% APY, the per-block NAV increment is approximately `0.60 / (365 * 7200) ≈ 0.0000228%` per block (assuming ~12s blocks). Over 500 blocks (~100 minutes) of sustained high utilization, NAV grows by ~0.011% relative to a DEX price that hasn't moved. This is the window.

---

## Opportunity Taxonomy

Three distinct sub-opportunities exist within this mechanism, ordered by execution complexity:

| Sub-type | Frequency | Gap Size | Execution Speed Required | Competition Level |
|---|---|---|---|---|
| **A: Utilization spike arb** | 2–8x/month per major vault | 0.02–0.15% | Automated, <30s | High (bots present) |
| **B: Fee harvest timing** | Daily per vault | 0.005–0.02% | Automated, <5 min | Medium |
| **C: New vault bootstrapping** | Per new vault launch | 0.1–2.0% | Manual OK, hours | Low |

**Sub-type C is Zunid's primary target.** When a new ERC-4626 vault launches and its token is listed on a DEX, the initial DEX price is set by the first liquidity provider, who may misprice it relative to the vault's already-accrued NAV. This window can last hours to days before sophisticated arbers notice. No speed advantage required — just monitoring new vault deployments.

**Sub-type B is the secondary target.** Fee harvests temporarily reset the share price trajectory. In the blocks immediately after a harvest, the NAV-to-DEX gap is at its minimum. The gap then widens monotonically until the next harvest. Entering a long vault token position on DEX immediately after harvest and redeeming at the next harvest captures the full accrual period with minimal competition.

---

## Entry Rules

### Sub-type C (New Vault Launch Arb)

1. Monitor ERC-4626 vault factory contracts on Ethereum mainnet, Arbitrum, Base, and Optimism for `CreateVault` or equivalent events using a free subgraph or event listener.
2. When a new vault token appears on a DEX (detect via Uniswap v3 `PoolCreated` or Curve `AddLiquidity` events), immediately query `convertToAssets(1e18)` from the vault contract.
3. Calculate: `NAV = convertToAssets(1e18) / 1e18` (in underlying units). Query DEX spot price for the same pair.
4. **Entry trigger:** `(NAV - DEX_price) / DEX_price > 0.10%` AND vault has passed a basic security check (see Risk section).
5. Execute: Buy vault tokens on DEX → call `redeem()` on vault contract → receive underlying at NAV rate.
6. If vault token is not yet redeemable (timelock on new vaults), enter only if the expected yield accrual during the lock period exceeds the gap risk.

### Sub-type B (Post-Harvest Accumulation)

1. Track `fee_harvest` or `report()` events on target vaults (Yearn v3 strategy reports, Morpho MetaMorpho `updateWithdrawQueue` events).
2. Immediately after a harvest event, query current NAV and DEX price.
3. **Entry trigger:** `(NAV - DEX_price) / DEX_price < 0.005%` (gap is near minimum, post-harvest) AND current underlying APY > 5% annualized (gap will widen meaningfully before next harvest).
4. Buy vault tokens on DEX. Size position to be redeemable within one harvest cycle (typically 24 hours for Yearn, variable for Morpho).
5. Hold until either: (a) next harvest event fires, or (b) gap widens to `> 0.05%` intraday.

### Sub-type A (Utilization Spike Arb) — Lower Priority

1. Monitor Aave v3 and Morpho pool utilization rates via subgraph or direct RPC polling every 60 seconds.
2. **Entry trigger:** Utilization crosses above kink threshold (e.g., 90% for USDC pools) AND `(NAV - DEX_price) / DEX_price > 0.03%`.
3. Execute buy-and-redeem atomically if possible via a custom contract, or sequentially if gap is large enough to survive two separate transactions.
4. **Do not enter Sub-type A manually** — the window is too short. Only pursue if automated execution infrastructure is in place.

---

## Exit Rules

### Primary Exit: Redemption

The primary exit is always `redeem(shares, receiver, owner)` on the vault contract at the contract-enforced NAV rate. This is not a market exit — it is a guaranteed-rate exit. Execute immediately upon entry for Sub-type C and Sub-type A. For Sub-type B, execute at the next harvest event or when gap exceeds 0.05%.

### Secondary Exit: DEX Sale

If the vault has a withdrawal queue delay (e.g., Morpho MetaMorpho can have multi-day queues during high utilization), sell vault tokens back on DEX if: (a) the gap has closed to near zero, or (b) a security concern arises. Accept slippage on this path — it is the emergency exit only.

### Stop-Loss

Exit via DEX sale immediately if: (a) vault contract emits a `Paused` event, (b) underlying asset price drops >5% (bad debt risk), or (c) vault NAV decreases (impossible under normal conditions — treat as exploit signal).

---

## Position Sizing

### Per-Trade Sizing

- **Sub-type C:** Size up to `min(0.5% of vault TVL, $50,000)` per trade. Larger sizes risk moving the DEX price against entry and may exceed vault redemption liquidity.
- **Sub-type B:** Size up to `min(1% of vault TVL, $25,000)`. Smaller because the gap is tighter and gas costs must be covered.
- **Sub-type A:** Size up to `min(0.2% of vault TVL, $10,000)`. Smallest because execution risk is highest.

### Portfolio-Level Sizing

- Allocate no more than 20% of total capital to this strategy at any time.
- Maintain a minimum 30% cash buffer in stablecoins to fund gas and rapid entries.
- Never hold more than 3 open vault positions simultaneously — monitoring overhead degrades execution quality.

### Gas Cost Breakeven

Calculate minimum viable gap before every trade:

```
min_gap = (gas_cost_entry_tx + gas_cost_redeem_tx) / position_size_USD
```

At Ethereum mainnet gas prices of 20 gwei and ETH at $2,500:
- Entry swap: ~150,000 gas → ~$7.50
- Redeem call: ~80,000 gas → ~$4.00
- Total: ~$11.50 per round trip

For a $10,000 position: `min_gap = $11.50 / $10,000 = 0.115%`

**On Arbitrum/Base:** Gas costs drop to ~$0.10–$0.50 per transaction, reducing min_gap to ~0.005% for a $10,000 position. **Prioritize L2 vaults.**

---

## Backtest Methodology

### Data Collection

1. **Vault NAV history:** Query `convertToAssets(1e18)` at every block (or every 10 blocks for efficiency) for target vaults using Alchemy/Infura archive node or The Graph subgraph. Target vaults: Morpho USDC MetaMorpho (Ethereum), Yearn USDC v3 (Ethereum + Arbitrum), Aave v3 USDC wrapped vault (Arbitrum).
2. **DEX price history:** Pull Uniswap v3 and Curve pool swap events for vault token pairs. Reconstruct price at each block from swap event data. Use The Graph `uniswap-v3` subgraph (free tier available).
3. **Gas price history:** Pull `baseFeePerGas` from block headers. Available via Etherscan API (free) or archive node.
4. **Utilization history:** Pull Aave v3 `ReserveDataUpdated` events and Morpho `AccrueInterest` events to reconstruct utilization and borrow rate at each block.

### Simulation Logic

```python
for each block in backtest_range:
    nav = query_convert_to_assets(block)
    dex_price = reconstruct_dex_price(block)
    gas_cost = estimate_gas_cost(block)
    gap = (nav - dex_price) / dex_price
    min_viable_gap = (gas_cost * 2) / position_size
    
    if gap > min_viable_gap and gap > entry_threshold:
        # Simulate entry: buy vault tokens at dex_price + 0.05% slippage
        # Simulate exit: redeem at nav - 0.01% (redemption slippage)
        pnl = (nav * (1 - 0.0001)) - (dex_price * (1 + 0.0005)) - gas_cost
        record_trade(block, gap, pnl)
```

### Backtest Period

- **Primary:** January 2023 – December 2025 (covers multiple rate cycles, USDC depeg event, Aave utilization spikes).
- **Stress test:** March 2023 (USDC depeg), November 2022 (FTX contagion), August 2023 (Curve exploit period).

### Key Metrics to Measure

- Number of qualifying opportunities per month per vault.
- Average gap size at entry.
- Average PnL per trade after gas.
- Maximum drawdown (should be near zero — this is arb, not directional).
- Percentage of opportunities where gap closed via redemption vs. DEX convergence.
- Withdrawal queue delay distribution for Morpho vaults.

### Honest Limitation

Historical DEX price data for vault tokens is sparse before mid-2023 for most vaults. The backtest will have incomplete coverage for early periods. Flag any period where DEX liquidity was below $100,000 TVL as unreliable — thin pools make price reconstruction noisy.

---

## Go-Live Criteria

All five criteria must be met before deploying real capital:

1. **Backtest shows ≥ 20 qualifying trades** across the test period with positive expected value after gas on L2.
2. **Paper trade for 30 days** on mainnet/L2 with simulated entries logged in real time — minimum 5 paper trades executed.
3. **Withdrawal queue analysis complete:** For each target vault, document the maximum observed withdrawal queue delay and confirm it never exceeded 7 days historically.
4. **Smart contract security review:** For each target vault, confirm: (a) audited by a reputable firm, (b) no admin key can pause redemptions without timelock, (c) no history of NAV manipulation or exploit.
5. **Automated monitoring live:** Event listener running for harvest events and utilization spikes, with Telegram/Discord alert firing within 60 seconds of a qualifying gap.

---

## Kill Criteria

Pause the strategy immediately if any of the following occur:

1. **Any vault position experiences a NAV decrease** — this signals bad debt or exploit. Exit all positions in that vault via DEX immediately, accept slippage.
2. **Withdrawal queue exceeds 72 hours** on any active position — reassess whether DEX exit is preferable.
3. **Three consecutive trades lose money after gas** — indicates the competitive landscape has shifted (more bots, thinner gaps) or gas cost assumptions are wrong. Re-run gas breakeven analysis.
4. **Underlying asset (USDC, WETH, etc.) depegs >0.5%** — the arb math breaks down when the underlying itself is mispriced.
5. **Vault contract is paused or upgraded** — exit all positions, halt new entries until contract review is complete.

---

## Risks

### Risk 1: Smart Contract Exploit (HIGH SEVERITY, LOW-MEDIUM PROBABILITY)
A vault exploit could cause NAV to drop to zero. Mitigation: Only trade audited vaults with >$10M TVL and >6 months of live operation. Never hold a vault position overnight without automated monitoring. Maximum single-vault exposure: $50,000.

### Risk 2: Withdrawal Queue Delay (MEDIUM SEVERITY, MEDIUM PROBABILITY)
Morpho MetaMorpho vaults can have withdrawal queues that extend to days during high utilization. If capital is locked in the queue, it cannot be redeployed. Mitigation: Monitor queue depth before entry. Do not enter if current queue depth implies >48-hour wait. Size positions so that locked capital does not exceed 5% of total portfolio.

### Risk 3: Gas Cost Spike (LOW SEVERITY, HIGH PROBABILITY)
Ethereum mainnet gas spikes during congestion can make small-gap trades unprofitable. Mitigation: Prioritize L2 deployments (Arbitrum, Base) where gas is near-zero. On mainnet, only execute when gap exceeds 0.15%.

### Risk 4: DEX Liquidity Thin on Entry (MEDIUM SEVERITY, MEDIUM PROBABILITY)
Buying vault tokens on a thin DEX pool moves the price against entry, reducing or eliminating the gap. Mitigation: Check pool depth before entry. Do not enter if position size exceeds 0.5% of pool TVL. Use limit orders on Uniswap v3 where possible.

### Risk 5: Bot Competition (LOW SEVERITY, HIGH PROBABILITY)
Well-capitalized arb bots monitor the same gaps. For Sub-types A and B, Zunid will frequently be outcompeted. Mitigation: Focus on Sub-type C (new vault launches) where monitoring is manual and bots are slower to deploy. Accept that Sub-types A and B are secondary opportunities.

### Risk 6: Fee Harvest Manipulation (LOW SEVERITY, LOW PROBABILITY)
A vault manager could delay fee harvests to artificially widen the gap, then harvest at an unfavorable time. Mitigation: Only trade vaults where harvest timing is governed by a smart contract rule (e.g., Yearn's `tend()` callable by anyone), not by a centralized keeper.

### Risk 7: Regulatory/Compliance (LOW SEVERITY, UNKNOWN PROBABILITY)
Interacting with DeFi vault contracts carries regulatory uncertainty in some jurisdictions. Mitigation: Consult legal counsel before deploying significant capital. This is not legal advice.

---

## Data Sources

| Data Type | Source | Cost | Reliability |
|---|---|---|---|
| Vault NAV (historical) | The Graph — Morpho subgraph, Yearn subgraph | Free | High |
| Vault NAV (live) | Direct RPC call to vault contract `convertToAssets()` | Free (Alchemy free tier) | High |
| DEX price (historical) | The Graph — Uniswap v3 subgraph, Curve subgraph | Free | Medium (gaps in thin pools) |
| DEX price (live) | Uniswap v3 `slot0()` on-chain query | Free | High |
| Gas price (historical) | Etherscan API `eth_getBlockByNumber` | Free | High |
| Utilization (historical) | Aave v3 subgraph `ReserveDataUpdated` events | Free | High |
| Utilization (live) | Aave v3 `getReserveData()` on-chain | Free | High |
| New vault deployments | Factory contract event listener (custom) | Free (RPC costs only) | High |
| Withdrawal queue depth | Morpho `withdrawQueue()` on-chain | Free | High |

---

## Implementation Checklist

- [ ] Deploy event listener for vault factory contracts on Ethereum, Arbitrum, Base
- [ ] Build NAV vs. DEX price gap calculator (runs every 60 seconds)
- [ ] Build gas breakeven calculator (updates with each block)
- [ ] Pull historical NAV data for 5 target vaults (Morpho USDC, Morpho WETH, Yearn USDC v3, Yearn WETH v3, Aave v3 USDC Arbitrum)
- [ ] Pull historical DEX price data for same vaults
- [ ] Run backtest simulation with realistic gas and slippage assumptions
- [ ] Document withdrawal queue delay distribution for Morpho vaults
- [ ] Complete security checklist for each target vault
- [ ] Set up paper trading log with real-time gap alerts
- [ ] Review go-live criteria after 30-day paper trade period

---

## Relationship to Zunid's Existing Strategies

This strategy is structurally similar to Zunid's token unlock shorts in one key way: **the edge is a guaranteed mechanical event** (NAV accrual) rather than a pattern. It differs in that the convergence mechanism is a smart contract redemption function rather than market price discovery. The two strategies are uncorrelated — token unlock shorts are directional and macro-sensitive; this strategy is delta-neutral and rate-sensitive. Running both simultaneously provides diversification across the "artificial flow blocker" thesis.

The Sub-type C variant (new vault launch arb) is the most Zunid-appropriate because it requires monitoring and judgment rather than speed — consistent with Zunid's non-HFT mandate.
