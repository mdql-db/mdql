---
title: "ERC-4626 Vault PT Launch Mispricing"
status: HYPOTHESIS
mechanism: 7
implementation: 3
safety: 6
frequency: 2
composite: 252
categories:
  - defi-protocol
  - options-derivatives
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a new Pendle or Spectra yield market is deployed for an ERC-4626 vault, the factory's seed implied APY is set manually by the deployer and is frequently stale relative to the vault's true on-chain yield at the moment of deployment. If the seeded implied APY exceeds the vault's actual current APY, the Principal Token (PT) is priced at a deeper discount to face value than it should be — meaning PT buyers lock in a yield-to-maturity that exceeds the vault's real yield. Since PT redemption at face value is enforced by the smart contract at maturity, this mispricing must converge to zero by expiry regardless of market conditions.

**Causal chain:**

1. Deployer manually estimates implied APY → seeds pool with this rate
2. Seeded rate diverges from vault's actual current APY (observable on-chain)
3. PT is priced at `face_value / (1 + seeded_APY)^T` where T = time to maturity in years
4. True fair value of PT = `face_value / (1 + actual_APY)^T`
5. If `seeded_APY > actual_APY`, PT is cheaper than fair value
6. Buyer of PT at launch locks in `seeded_APY` as yield-to-maturity
7. At maturity, smart contract redeems PT 1:1 for underlying asset — convergence is contractually guaranteed
8. Excess return = `seeded_APY - actual_APY`, annualised over remaining maturity

---

## Structural Mechanism (WHY This Must Happen)

**The guarantee:** Pendle and Spectra PT contracts enforce face-value redemption at maturity via immutable smart contract logic. There is no scenario in which a PT holder receives less than 1 unit of underlying per PT at expiry (barring smart contract exploit or underlying vault insolvency). This is the hard floor that makes the convergence structural, not probabilistic.

**The gap source:** Pool deployment on Pendle requires the deployer to specify an initial `lnFeeRateRoot` and implied rate parameter. This is not pulled from an oracle — it is a constructor argument passed at deployment time. Deployers typically estimate this from recent vault APY data, which may be hours or days stale, or may use a round-number approximation. On-chain evidence: examine Pendle factory deployment calldata on Etherscan for historical pools — the seeded rate is visible in the transaction input data.

**Why arbitrageurs don't instantly close it:** PT markets on Pendle have an AMM with concentrated liquidity. At launch, liquidity is thin and the AMM curve is steep. Large arb trades move the price significantly, meaning the full mispricing cannot be extracted in one transaction without slippage eating the edge. The window closes as liquidity providers seed the pool and secondary traders reprice — typically over minutes to hours, not seconds. This is not an HFT problem.

**Why this is not just "historical tendency":** The mispricing is not a statistical pattern — it is a direct consequence of the deployment mechanism. The seeded rate is a fixed number written into the transaction. The vault's actual APY is a different, independently observable number. The gap between them is arithmetic, not statistical inference.

---

## Entry/Exit Rules

### Monitoring
- Subscribe to `CreateNewMarket` (Pendle) or equivalent factory event on Ethereum mainnet and Arbitrum via WebSocket RPC or The Graph
- Pendle factory address (Ethereum): `0x27b1dAcd74688aF24a64BD3C9C1B143118740784`
- Pendle factory address (Arbitrum): `0x6fcf753f2C67b83f7B09746Bbc4FA0047b35D050`
- Spectra factory: check Spectra docs/Etherscan for current deployment

### At Pool Deployment (trigger: new market event)

**Step 1 — Read seeded implied APY:**
- Call `readState()` or `getMarketState()` on the newly deployed Pendle market contract
- Extract `lastLnImpliedRate` from market state; convert: `implied_APY = exp(lastLnImpliedRate) - 1`

**Step 2 — Read vault's actual current APY:**
- Identify the underlying ERC-4626 vault address from the market's `SY` (Standardised Yield) token
- Compute vault APY from on-chain `sharePrice` (also called `convertToAssets(1e18)`):
  - Method A (preferred): Read `sharePrice` at current block and at block ~7 days prior; annualise: `APY = (price_now / price_7d_ago)^(365/7) - 1`
  - Method B (fallback): Read vault's own `APY()` getter if exposed (not all vaults expose this)
  - For Aave-based vaults (aUSDC, etc.): read `getReserveData().currentLiquidityRate` from Aave's `IPool` contract directly

**Step 3 — Decision:**
- Compute `gap = seeded_APY - actual_APY`
- **If `gap > 200bps` (0.02):** Execute PT buy — proceed to sizing
- **If `gap < -200bps`:** YT is underpriced; pass (YT long requires active yield accrual monitoring — out of scope for this version)
- **If `|gap| < 200bps`:** No trade

### Entry Execution
- Buy PT via Pendle Router (`PendleRouterV3`) using `swapExactTokenForPt()` or equivalent
- Set slippage tolerance: accept up to 50bps slippage on entry (if slippage exceeds 50bps, abort — edge is consumed)
- Execute within 30 minutes of pool deployment; after 2 hours, re-check gap; if gap < 150bps, abort

### Exit
- **Primary:** Hold PT to maturity; redeem via `redeemPyToToken()` on Pendle router
- **Secondary (early exit):** If PT secondary market price implies yield < actual vault APY (i.e., PT is now fairly priced or overpriced), sell PT on secondary market to recycle capital faster
- **No stop-loss on PT long** — downside is capped at opportunity cost (you earn less than expected if vault APY rises above seeded rate post-entry, but you cannot lose principal in nominal terms assuming vault solvency)

---

## Position Sizing

- **Per-trade maximum:** 2% of total strategy capital
- **Rationale:** Low-frequency, low-liquidity markets; position size is constrained by pool depth at launch, not by risk model
- **Liquidity check:** Before entry, simulate trade via Pendle SDK's `swapExactTokenForPt` quote; if price impact > 50bps on intended size, reduce size until impact < 50bps
- **Minimum trade size:** $5,000 notional (below this, gas costs on Ethereum mainnet consume the edge; use Arbitrum for smaller sizes)
- **Capital allocation:** This strategy is capital-efficient only if maturities are short (< 6 months). For 6-month PT, $100k deployed earns ~$1k on a 200bps gap — acceptable only as a portfolio component, not standalone
- **Concentration limit:** No more than 20% of strategy capital locked in PT positions simultaneously (maturity mismatch risk)

---

## Backtest Methodology

### Data Sources
- **Pendle historical pool deployments:** Pendle subgraph on The Graph — `https://api.thegraph.com/subgraphs/name/pendle-finance/core-mainnet` — query `Market` entities with `createdAt` timestamp
- **Historical PT prices at launch:** Pendle subgraph `MarketDailySnapshot` or `Swap` events within first 24h of pool creation
- **ERC-4626 vault sharePrice history:** Direct RPC archive node calls (`eth_call` with block number parameter) to `convertToAssets(1e18)` on vault contracts; use Alchemy/Infura archive endpoints
- **Seeded implied rate at deployment:** Decode constructor calldata from factory deployment transactions on Etherscan; alternatively query `getMarketState()` at the deployment block via archive node
- **Vault APY at deployment time:** Compute from sharePrice at deployment block vs. sharePrice 7 days prior (archive node required)

### Backtest Universe
- All Pendle markets deployed on Ethereum mainnet and Arbitrum from **January 2023 to present**
- Filter to ERC-4626 underlying vaults only (exclude non-vault underlyings like stETH raw)
- Expected universe: ~40–80 pool launches meeting criteria

### Procedure
1. For each pool launch: extract seeded implied APY (from deployment calldata) and actual vault APY (from archive node sharePrice delta)
2. Compute `gap = seeded_APY - actual_APY` at T=0 (deployment block)
3. For all launches where `gap > 200bps`: record PT price at T=0, T=1h, T=2h, T=24h
4. Compute yield-to-maturity locked in at each entry time
5. Compare locked-in YTM against: (a) actual vault APY over the holding period, (b) risk-free rate (USDC lending rate on Aave)
6. Compute excess return = locked-in YTM minus actual vault APY realised over same period
7. Track how quickly the gap closes (half-life of mispricing) — this determines the entry window

### Key Metrics
- **Hit rate:** % of launches where `gap > 200bps` (tests how often the opportunity exists)
- **Mean excess return (bps annualised):** Average locked-in YTM minus actual vault APY
- **Entry window half-life:** Median time for gap to fall below 100bps (tests urgency of execution)
- **Slippage at entry:** Actual PT price vs. theoretical fair value at entry size
- **Sharpe ratio:** Annualised excess return / standard deviation of excess returns across trades
- **Baseline:** USDC deposited directly into the underlying vault (same capital, same duration)

### What to Measure
- Distribution of `gap` at launch across all historical pools (is 200bps threshold too high/low?)
- Correlation between gap size and time-to-close (do larger gaps persist longer?)
- Whether gap direction is random or systematically biased (do deployers consistently seed high or low?)

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **Opportunity frequency:** At least 8 qualifying launches (gap > 200bps) in the historical dataset — insufficient sample otherwise
2. **Positive excess return:** Mean excess return > 150bps annualised after estimated slippage (50bps) and gas costs
3. **Entry window:** Gap half-life > 30 minutes (if gap closes in < 10 minutes, execution is impractical without HFT infrastructure)
4. **No systematic negative skew:** No single trade should show a loss in excess of 300bps annualised (PT held to maturity cannot lose nominal value, so large losses would indicate a data/methodology error to investigate)
5. **Slippage is manageable:** At $25k position size, simulated price impact < 75bps in at least 70% of qualifying launches

---

## Kill Criteria

Abandon strategy if any of the following occur:

1. **Backtest shows < 5 qualifying launches** in full historical dataset — opportunity is too rare to build infrastructure for
2. **Mean excess return < 75bps annualised** after costs — not worth capital lockup vs. simply holding the vault
3. **Pendle updates factory to use oracle-based seeding** — structural gap is eliminated; monitor Pendle governance proposals and contract upgrades
4. **Entry window half-life < 15 minutes** in backtest — requires near-HFT execution, outside our operational model
5. **Live paper trading:** If first 5 live opportunities show mean gap at execution < 100bps (i.e., gap is closed before we can enter), kill and reassess
6. **Underlying vault exploit or depeg** on any live position — reassess vault selection criteria, not the strategy itself

---

## Risks

### Smart Contract Risk (HIGH — existential)
PT redemption guarantee is only as good as the Pendle contract and the underlying vault. A Pendle contract exploit or an ERC-4626 vault exploit (e.g., Aave hack, sDAI depeg) would cause PT to not redeem at face value. **Mitigation:** Only trade PT backed by battle-tested vaults (sDAI, aUSDC on Aave v3, weETH from established LST protocols). Avoid new/unaudited vaults. Cap per-vault exposure.

### Opportunity Scarcity (MEDIUM)
The strategy is event-driven and low-frequency. If Pendle improves its seeding methodology (e.g., pulls from an on-chain oracle at deployment), the structural gap disappears. **Mitigation:** Monitor Pendle governance; treat this as a finite-life opportunity.

### Liquidity / Slippage (MEDIUM)
At launch, pool liquidity is thin. Large entries move the PT price significantly, consuming the edge. **Mitigation:** Hard slippage cap of 50bps; size down aggressively if pool is shallow.

### Capital Lockup (LOW-MEDIUM)
PT held to maturity locks capital for the duration (up to 12+ months for some pools). Opportunity cost is real if better opportunities arise. **Mitigation:** Prefer short-dated pools (< 6 months); use secondary market exit if PT reprices to fair value early.

### Measurement Error in Vault APY (MEDIUM)
7-day trailing sharePrice delta may not reflect true forward APY if vault yield is volatile (e.g., variable-rate lending pools). A vault with temporarily elevated APY (due to a liquidation event) may show high trailing APY that reverts, making the seeded rate look low when it is actually fair. **Mitigation:** Cross-check vault APY against 30-day trailing and protocol's own rate display; flag high-volatility vault APYs as unreliable signals.

### Gas Costs (LOW on Arbitrum, MEDIUM on Mainnet)
Pendle router transactions on Ethereum mainnet cost $20–$80 in gas. On a $10k position with a 200bps edge, gas is ~$50 = 50bps drag. **Mitigation:** Use Arbitrum deployment; set minimum position size of $10k on Arbitrum, $25k on mainnet.

### Regulatory / Protocol Risk (LOW)
Pendle is a permissioned-deployment protocol; pool creation requires no KYC but protocol could be sanctioned or paused. Standard DeFi protocol risk — not strategy-specific.

---

## Data Sources

| Data | Source | Endpoint / Notes |
|---|---|---|
| Pendle pool deployments | The Graph (Pendle subgraph) | `https://api.thegraph.com/subgraphs/name/pendle-finance/core-mainnet` — query `Market { id, createdAt, expiry, SY }` |
| Pendle Arbitrum subgraph | The Graph | `https://api.thegraph.com/subgraphs/name/pendle-finance/core-arbitrum` |
| PT price history (swaps) | Pendle subgraph | `Swap` events on each `Market` entity, filtered to first 24h post-creation |
| Seeded implied rate | Etherscan / archive RPC | Decode factory `CreateNewMarket` calldata; or call `getMarketState()` at deployment block |
| ERC-4626 sharePrice history | Archive RPC node | `eth_call` to `convertToAssets(1e18)` at specific block numbers; use Alchemy/Infura archive |
| Aave lending rates | Aave subgraph / on-chain | `IPool.getReserveData(asset).currentLiquidityRate` — Aave v3 Ethereum: `0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2` |
| sDAI APY | MakerDAO on-chain | `Pot.dsr()` — DSR rate, convert from ray units |
| Pendle SDK (simulation) | Pendle SDK npm | `@pendle/sdk-v2` — use `Router.swapExactTokenForPt()` with `simulateOnly: true` for price impact |
| Pendle official API | Pendle REST API | `https://api-v2.pendle.finance/core/v1/markets` — lists active markets with implied APY (useful for monitoring, not for historical backtest) |
| Spectra factory events | Etherscan / Spectra docs | Spectra protocol: `https://spectra.finance` — factory address in their docs; similar event structure to Pendle |
