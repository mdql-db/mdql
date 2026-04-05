---
title: "Cross-Chain Price Dislocation Arb — Slow Bridge Premium Capture"
status: HYPOTHESIS
mechanism: 5
implementation: 4
safety: 5
frequency: 7
composite: 700
categories:
  - cross-chain
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When the same token trades at a persistent price premium on Chain A versus Chain B, and the fastest available bridge between those chains has a minimum latency of 10+ minutes, the spread cannot be closed by arbitrageurs faster than that latency floor. Market makers who pre-fund the arb absorb float cost and counterparty risk, meaning they require a minimum spread to participate. This creates a structural dead zone — spreads below the market maker's required return persist until organic bridge flow rebalances supply. The trade captures this dead zone by going long the discounted token on the cheaper chain and shorting equivalent exposure via Hyperliquid perpetual, collecting the spread as it compresses.

**Causal chain:**
1. Token supply on Chain B becomes temporarily scarce (e.g., net outflows via bridge, airdrop farming demand, protocol incentive spike)
2. Price on Chain B rises above Chain A by >0.5%
3. Arbitrageurs face a choice: bridge tokens (10–20 min CCTP delay, 7-day optimistic delay) or pre-fund via market making (requires capital float + risk premium)
4. The float cost for a 15-minute bridge round trip at current DeFi lending rates (~5–8% APY) on a 15-minute window ≈ 0.0014–0.0023% — trivially small
5. Therefore the binding constraint is NOT float cost — it is **execution risk during the bridge window**: price may revert before the bridged tokens arrive, leaving the arb leg unhedged
6. This execution risk is what keeps the spread alive for 15–120 minutes on smaller chains/tokens
7. We hedge the execution risk by using a Hyperliquid perp as the short leg — no bridge required, instant execution, no on-chain settlement delay

---

## Structural Mechanism (WHY This Must Happen)

This is not "tends to happen" — it is mechanically enforced by bridge architecture:

- **CCTP (Circle's Cross-Chain Transfer Protocol):** Requires attestation from Circle's off-chain attestation service. Minimum observed time: 10–20 minutes. This is a hard floor — no amount of gas payment accelerates it. Source: [Circle CCTP docs](https://developers.circle.com/stablecoins/docs/cctp-getting-started)
- **Across Protocol (fastest intent bridge):** Relayer pre-funds destination, but relayer reimbursement from origin chain still requires ~2–5 minutes of block finality. Relayer spread is typically 0.05–0.15% — this is the market's revealed minimum acceptable spread for instant cross-chain execution
- **Optimistic bridges (Arbitrum native, Optimism native):** 7-day challenge period. Completely non-viable for arb. Creates persistent one-way flow pressure that can sustain premiums for days
- **The ADR analogy is precise:** ADR arb requires FX settlement (T+2) and custody transfer. Here, bridge settlement is the equivalent friction. The spread is the cost of that friction, and it is bounded below by the relayer's required return

**Why this is NOT fully arbitraged away:**
- MEV bots operate within a single chain. Cross-chain MEV requires pre-funded relayers with capital deployed on both chains simultaneously
- Capital efficiency: a bot pre-funding $1M on every chain for every token is capital-intensive. Smaller tokens on less-trafficked chains are under-served
- The opportunity is largest during: (a) network congestion events, (b) token-specific demand spikes (airdrop snapshots, governance votes, protocol launches), (c) off-hours when fewer relayers are active

---

## Entry/Exit Rules

### Universe
Monitor these token/chain pairs (start narrow, expand after validation):
- USDC: Ethereum ↔ Arbitrum, Ethereum ↔ Base, Ethereum ↔ Optimism, Ethereum ↔ Polygon
- LINK: Ethereum ↔ Arbitrum, Ethereum ↔ Base
- OP: Optimism ↔ Arbitrum (via DEX pricing)
- ARB: Arbitrum ↔ Ethereum
- MATIC/POL: Polygon ↔ Ethereum, Polygon ↔ Arbitrum

Exclude tokens where Hyperliquid perp is not listed (no clean hedge leg available).

### Signal Detection
Every 5 minutes, compute:
```
spread_pct = (price_chain_B - price_chain_A) / price_chain_A * 100
net_spread = spread_pct - estimated_gas_cost_pct - bridge_fee_pct
```

Gas cost estimation: use current gas price × estimated gas units for the specific bridge contract, converted to % of trade size at target position size ($10,000 notional).

### Entry Conditions (ALL must be true)
1. `net_spread > 0.5%` on the same token across two chains
2. Spread has persisted for **≥15 minutes** (3 consecutive 5-minute checks above threshold) — eliminates fleeting noise
3. Hyperliquid perp for the token exists and has **24h volume > $5M** (ensures perp is liquid enough to short without excessive slippage)
4. DEX liquidity on the discounted chain: **≥$50,000 depth within 0.3% of mid** (check via 0x API quote for target size)
5. No known bridge exploit, pause, or congestion event in last 24h (manual check or automated alert from bridge status pages)
6. Perp funding rate on Hyperliquid is **not strongly negative** (i.e., funding rate > -0.05% per 8h) — negative funding means market is already short-heavy, adds cost

### Trade Construction
- **Leg 1 (Long):** Buy token on discounted chain via DEX aggregator (0x or Paraswap). Execute as limit order within 0.1% of quoted price. Chain: whichever chain shows lower price.
- **Leg 2 (Short):** Short equivalent USD notional on Hyperliquid perp. Use market order (perp liquidity is sufficient). Execute within 30 seconds of Leg 1 fill confirmation.
- **Hedge ratio:** 1:1 notional (delta-neutral). Do not adjust for basis unless perp consistently trades at >0.2% premium/discount to spot.

### Exit Conditions (first trigger wins)
1. **Spread compression:** `net_spread < 0.1%` — close both legs simultaneously. Leg 1: sell spot on same chain or bridge to better-priced chain. Leg 2: close perp short.
2. **Time stop:** 2 hours elapsed from entry — close regardless of spread. Rationale: if spread hasn't compressed in 2 hours, structural rebalancing is not occurring; risk of adverse move increases.
3. **Adverse spread widening:** spread widens to >1.5% (double the entry threshold) — suggests a structural break (bridge pause, chain issue) rather than temporary dislocation. Exit immediately.
4. **Funding rate deterioration:** Hyperliquid funding rate drops below -0.1% per 8h — short leg is becoming expensive; exit if spread < 0.8%.

### Execution Notes
- Do NOT use the bridge as part of the trade exit unless spread compression is confirmed AND bridge is fast (Across/CCTP only, never optimistic)
- Both legs must be closeable independently. The spot leg closes on the same chain it was opened (sell back to DEX). The perp leg closes on Hyperliquid. No cross-chain dependency at exit.

---

## Position Sizing

**Base position:** $10,000 notional per trade (both legs combined = $5,000 spot + $5,000 perp short equivalent)

**Rationale for $10,000:**
- Large enough to cover gas costs (Ethereum mainnet gas for a DEX swap ≈ $5–$30; on L2s ≈ $0.10–$2.00)
- Small enough to fit within DEX liquidity constraints on thin chains
- At 0.5% net spread, gross P&L = $50. After gas ($5 on L2), net ≈ $45. This is the minimum viable trade.

**Scaling rules:**
- Maximum single trade: $50,000 notional (beyond this, DEX slippage on thin chains likely exceeds spread)
- Scale to available liquidity: position size = min($50,000, 20% of DEX depth within 0.3% of mid)
- Maximum concurrent positions: 3 (capital constraint + monitoring overhead)
- No leverage on spot leg. Perp leg: 1x (no leverage — this is a hedge, not a directional bet)

**Kelly sizing:** Not applicable at this stage. Use fixed $10,000 until backtest establishes win rate and average P&L per trade.

---

## Backtest Methodology

### Data Sources

| Data Type | Source | URL | Cost |
|-----------|--------|-----|------|
| DEX prices by chain (historical) | DeFiLlama | `https://defillama.com/docs/api` | Free |
| DEX swap quotes (historical) | 0x API | `https://api.0x.org/swap/v1/quote` | Free tier |
| Bridge flow data | Li.Fi API | `https://apidocs.li.fi/` | Free |
| Bridge flow data | Socket API | `https://docs.socket.tech/socket-api` | Free |
| CCTP attestation times | Circle on-chain | Ethereum/Arbitrum event logs via Etherscan API | Free |
| Gas prices (historical) | Etherscan Gas Tracker API | `https://api.etherscan.io/api?module=gastracker` | Free |
| Hyperliquid perp prices + funding | Hyperliquid API | `https://api.hyperliquid.xyz/info` | Free |
| Across Protocol relayer spreads | Across API | `https://across.to/api/suggested-fees` | Free |

### Backtest Period
- **Primary:** January 2024 – December 2024 (full year, includes bull and bear phases)
- **Stress test:** March 2023 (USDC depeg event — extreme cross-chain dislocations), November 2022 (FTX collapse — chain-specific liquidity crises)

### Backtest Construction

**Step 1: Build price matrix**
For each token in universe, collect 5-minute OHLCV from DEX aggregators on each chain. Use the mid-price of the best available DEX pool (by TVL) as the reference price. DeFiLlama's `/coins/chart/{chain}:{address}` endpoint provides this.

**Step 2: Compute net spreads**
For each 5-minute interval and each chain pair:
```python
net_spread = abs(price_A - price_B) / min(price_A, price_B)
             - gas_cost_pct(chain_A, trade_size)
             - gas_cost_pct(chain_B, trade_size)
             - bridge_fee_pct(bridge_protocol)
```
Gas cost: use median gas price for that 5-minute window × estimated gas units (hardcode per chain: Ethereum ≈ 150,000 gas for DEX swap, Arbitrum ≈ 800,000 L2 gas units ≈ $0.20).

**Step 3: Apply entry/exit rules**
Simulate signal detection: flag entries where net_spread > 0.5% for 3 consecutive intervals. Apply exit rules in order. Record: entry time, entry spread, exit time, exit spread, P&L per trade.

**Step 4: Adjust for execution realism**
- Add 0.1% slippage to each DEX leg (conservative for $10k trades on liquid pools)
- Add 0.05% Hyperliquid taker fee per leg
- Add actual gas cost from historical gas data (not estimated)
- Assume 30-second delay between Leg 1 and Leg 2 execution (price may move)

### Key Metrics to Compute
- **Win rate:** % of trades where exit spread < entry spread
- **Average net P&L per trade** (after all costs)
- **Average hold time**
- **Sharpe ratio** (annualized, using daily P&L)
- **Maximum drawdown** (consecutive losing trades)
- **Opportunity frequency:** trades per month per token/chain pair
- **Spread decay curve:** how quickly does spread compress after entry? (plot spread vs. time-since-entry, averaged across all trades)
- **Chain/token breakdown:** which pairs generate the most opportunity?

### Baseline Comparison
Compare against: (a) random entry/exit at same frequency (null hypothesis), (b) always-short perp with no spot leg (directional baseline), (c) Across Protocol relayer spread as the "market rate" for this arb — if our net P&L < relayer spread, we're not adding value over just using the bridge.

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show ALL of the following:

1. **Win rate ≥ 60%** across all trades (spread compresses before time stop)
2. **Average net P&L per trade ≥ $20** at $10,000 notional (0.2% net after all costs) — below this, execution variance will dominate
3. **≥ 50 qualifying trades** in the backtest period (statistical significance floor)
4. **Sharpe ratio ≥ 1.0** on daily P&L series
5. **Spread decay curve shows:** median spread at T+30min is ≤ 50% of entry spread (confirms compression is structural, not random)
6. **At least 2 token/chain pairs** independently meet criteria 1–3 (avoids single-pair overfitting)
7. **Stress test:** strategy does not produce >3 consecutive losing trades during any 30-day window in the backtest

If backtest passes, paper trade for **30 days minimum** before live capital. Paper trade must show ≥ 10 qualifying signals with execution latency < 5 minutes from signal to both legs filled.

---

## Kill Criteria

Abandon the strategy (at any stage) if:

1. **Backtest shows win rate < 55%** — spread compression is not structural, it's noise
2. **Average hold time > 90 minutes** in backtest — suggests spreads are not mean-reverting within our window; execution risk too high
3. **Opportunity frequency < 5 trades/month** across entire universe — not worth the infrastructure cost
4. **MEV/bot competition evidence:** during paper trading, >50% of signals are gone (spread < 0.1%) by the time both legs are executable — indicates the window has closed for non-HFT
5. **Across Protocol relayer spread tightens to < 0.1%** on all major pairs — indicates the market has fully priced out this opportunity
6. **Live trading:** 3 consecutive months with Sharpe < 0.5 or negative cumulative P&L
7. **Structural change:** CCTP v2 or equivalent reduces attestation time to < 2 minutes — the latency floor that creates the dead zone disappears

---

## Risks

### Primary Risks (quantified where possible)

**1. MEV/Bot competition — HIGH probability, HIGH impact**
Dedicated cross-chain arb bots (Across Protocol relayers, Stargate LPs) are continuously monitoring these spreads. On Ethereum mainnet + Arbitrum/Base, spreads > 0.3% likely last < 60 seconds. Our 15-minute persistence filter may eliminate most opportunities on major chains. Mitigation: focus on Polygon, smaller L2s, and tokens with < $10M daily DEX volume.

**2. Execution leg mismatch — MEDIUM probability, HIGH impact**
Leg 1 (spot) fills but Leg 2 (perp) fails or fills at a significantly different price. This leaves a naked directional position. Mitigation: if Leg 2 cannot be filled within 60 seconds of Leg 1 at acceptable price, immediately close Leg 1 (accept the gas cost as a loss). Never hold an unhedged spot position.

**3. Bridge/chain failure during hold — LOW probability, HIGH impact**
If the discounted chain experiences an outage or the bridge is paused, the spot leg cannot be exited. The perp leg continues to accumulate funding. Mitigation: 2-hour time stop forces exit regardless. Monitor bridge status APIs. Do not enter if bridge has had any incident in past 24h.

**4. Perp-spot basis risk — MEDIUM probability, MEDIUM impact**
Hyperliquid perp may not track the spot price on the specific chain. If the perp tracks Ethereum mainnet price and we're long on Polygon, the hedge may be imperfect. Measure the perp-to-chain-spot basis historically before trading any pair.

**5. Gas cost spike — MEDIUM probability, MEDIUM impact**
Ethereum mainnet gas spikes (e.g., during NFT mints, major events) can turn a 0.5% spread into a net negative trade. Mitigation: real-time gas monitoring; do not enter if estimated gas > 0.3% of trade size. This is why L2-only pairs are preferred.

**6. Liquidity illusion — MEDIUM probability, MEDIUM impact**
DEX aggregator quotes at $10k may not reflect actual fill at $50k. Backtest must use actual fill simulation, not mid-price. Mitigation: cap position size at 20% of quoted depth.

### Honest Assessment of Edge Durability
This edge is **real but thin and contested**. The structural mechanism is sound — bridge latency floors are hard constraints. But the opportunity is actively competed for by well-capitalized relayers. The viable niche is: (a) tokens too small for major relayers to pre-fund, (b) chain pairs with low relayer coverage, (c) stress events where relayers pull back. This is a **niche, opportunistic strategy** — not a continuous alpha engine. Expected frequency: 10–30 trades/month across the full universe, concentrated in 2–3 chain pairs. Expected annual P&L at $10k base size: $2,000–$8,000 — only worth running if it scales to $100k+ notional, which requires solving the liquidity problem on thin chains.

---

## Data Sources (Complete Reference)

| Source | Endpoint | Data | Notes |
|--------|----------|------|-------|
| DeFiLlama Coins API | `https://coins.llama.fi/chart/{chain}:{token_address}?span=365&period=5m` | Historical token prices by chain | Free, 5-min granularity |
| 0x API (price quotes) | `https://api.0x.org/swap/v1/price?sellToken=...&buyToken=...&sellAmount=...` | Real-time DEX quotes with gas | Free tier: 1 req/sec |
| Paraswap API | `https://apiv5.paraswap.io/prices/?srcToken=...&destToken=...&amount=...&network={chainId}` | DEX aggregator quotes | Free |
| Li.Fi Bridge API | `https://li.quest/v1/quote?fromChain=...&toChain=...&fromToken=...&toToken=...&fromAmount=...` | Bridge quotes + fees | Free |
| Across Suggested Fees | `https://across.to/api/suggested-fees?token=...&destinationChainId=...&amount=...` | Relayer fee (market rate for instant arb) | Free |
| Hyperliquid Info API | `https://api.hyperliquid.xyz/info` (POST, `{"type": "candleSnapshot", ...}`) | Perp OHLCV + funding rates | Free |
| Etherscan Gas API | `https://api.etherscan.io/api?module=gastracker&action=gasoracle&apikey={key}` | Ethereum gas prices | Free tier |
| Arbiscan/Basescan | Same pattern as Etherscan with respective API keys | L2 gas prices | Free |
| Circle CCTP on-chain | Ethereum: `0xBd3fa81B58Ba92a82136038B25aDec7066af3155` (MessageTransmitter) — filter `MessageReceived` events | Actual CCTP attestation times | Requires Etherscan API or node |
| Socket Bridge API | `https://api.socket.tech/v2/quote?fromChainId=...&toChainId=...&fromTokenAddress=...&toTokenAddress=...&fromAmount=...` | Bridge route quotes | Free with API key |

**Implementation priority:** Start with DeFiLlama + Hyperliquid API for the backtest skeleton. Add 0x quotes for slippage simulation. Add Across fees as the "market rate" baseline. CCTP on-chain data is optional for the first backtest pass — use 15-minute flat assumption for attestation time.
