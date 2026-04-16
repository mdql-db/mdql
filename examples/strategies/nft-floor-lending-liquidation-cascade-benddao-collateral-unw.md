---
title: "NFT Floor Lending Liquidation Cascade — BendDAO Collateral Unwind Short"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 5
frequency: 1
composite: 150
categories:
  - liquidation
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a significant portion of BendDAO's loan book for a single NFT collection crosses a critical health factor threshold, the protocol's smart contract **must** initiate a 48-hour liquidation auction. If no bidder emerges, the protocol absorbs the NFT and must sell it into the open market. Each forced sale depresses the floor price, which mechanically reduces the collateral value of all remaining loans in that collection, triggering the next wave of health factor breaches. This cascade is not probabilistic — it is a deterministic consequence of the protocol's liquidation math.

**Causal chain:**

1. NFT floor price drops (exogenous shock — market sell-off, whale exit, negative news)
2. BendDAO recalculates collateral value using floor price oracle (Chainlink + OpenSea floor feed)
3. Loans where `collateral_value / debt < 1.1` enter liquidation eligibility
4. Protocol initiates 48-hour Dutch auction for each eligible NFT
5. If auction clears: buyer absorbs NFT, loan repaid, floor may stabilize
6. If auction fails to clear: protocol takes NFT onto balance sheet, must sell → additional floor pressure
7. Floor drops further → next tranche of loans breaches health factor → repeat from step 3
8. BEND token holders face bad debt risk → BEND sells off
9. APE (if BAYC is the collateral collection) faces correlated selling pressure from leveraged holders unwinding

**Testable prediction:** Within 48–96 hours of ≥5% of a collection's BendDAO loan book hitting HF < 1.15, BEND price declines ≥10% and the NFT floor declines a further ≥5% from the trigger point.

---

## Structural Mechanism — WHY This MUST Happen

This is not a tendency — it is a protocol rule encoded in BendDAO's smart contracts:

- **Health factor formula:** `HF = (floor_price × liquidation_threshold) / total_debt`. When HF < 1.0, the position is liquidatable. BendDAO uses 1.1 as the soft trigger for auction initiation.
- **Auction mechanic:** Once triggered, the 48-hour auction window is non-discretionary. The protocol cannot pause it without a governance vote (which takes days). The liquidation bot network is permissionless — anyone can call the liquidation function and earn a bonus.
- **Oracle dependency:** BendDAO's floor price oracle updates on a defined cadence (historically every ~30 minutes via Chainlink). Each update is a potential trigger event. The oracle update schedule is public.
- **Cascade math:** If a collection has 100 loans and 10 breach HF < 1.1, the resulting auction sales (even partial) reduce the floor, which may push the next 15 loans below threshold. The cascade amplification factor depends on loan concentration — calculable from on-chain data.
- **BEND token exposure:** BEND holders are the residual claimants on bad debt. If auctions clear below loan value, the shortfall hits the protocol's reserve fund, which is denominated in BEND. This is a direct, mechanical link between cascade severity and BEND price.

**What is NOT guaranteed:** The magnitude of APE price impact. APE is a large-cap token; BendDAO's BAYC collateral positions are a small fraction of APE's float. The APE leg is probabilistic, not structural. BEND is the higher-conviction short.

---

## Entry Rules


### Trigger Conditions (all must be met)

| Condition | Threshold | Data Source |
|-----------|-----------|-------------|
| Collection health factor concentration | ≥5% of loan book by USD value at HF < 1.15 | BendDAO subgraph |
| Floor price trend | Floor down ≥8% in prior 24h | Reservoir API |
| BEND funding rate | Not already deeply negative (avoid crowded short) | Hyperliquid / Binance |
| Protocol reserve ratio | Reserve fund < 150% of at-risk debt | BendDAO dashboard |

### Entry

- **Primary:** Short BEND perpetual on Hyperliquid or Binance at market open of next 1-hour candle after trigger confirmation
- **Secondary (optional, lower conviction):** Short APE perpetual if BAYC is the at-risk collection AND BAYC loans represent ≥$2M in BendDAO book
- **Entry size:** Full position entered in one tranche (not scaled — the trigger is binary and time-sensitive)

## Exit Rules

### Exit Rules

**Take profit:**
- BEND down ≥25% from entry → close 50% of position
- BEND down ≥40% from entry → close remaining position
- Alternatively: floor price stabilizes (less than 2% move in 12h) AND auction queue clears below 10 active liquidations → close full position

**Stop loss:**
- BEND up ≥12% from entry → close full position (cascade failed to materialize or was absorbed)
- 96 hours elapsed from entry with no ≥10% BEND move → close full position (time stop)

**Forced exit:**
- BendDAO governance posts emergency pause proposal → close immediately (protocol intervention breaks the cascade mechanic)

---

## Position Sizing

- **Maximum allocation per trade:** 3% of portfolio NAV
- **BEND leg:** 2% of NAV (primary, higher conviction)
- **APE leg:** 1% of NAV (secondary, only if BAYC trigger)
- **Leverage:** 2–3x maximum on BEND perp (BEND is illiquid; higher leverage risks liquidation on a short squeeze)
- **Rationale for small size:** BEND perp open interest is thin. Large positions will move the market against entry. 2% NAV at 2x = 4% NAV notional — check this against BEND perp OI before entry; position should not exceed 5% of 24h BEND perp volume.

---

## Backtest Methodology

### Target Period
- **Primary:** June 2022 – December 2023 (BendDAO crisis period, multiple cascade events)
- **Secondary:** January 2024 – present (lower TVL regime, test if edge persists)

### Data Required

| Dataset | Source | Format |
|---------|--------|--------|
| BendDAO loan book snapshots | BendDAO subgraph (The Graph) | GraphQL → JSON |
| NFT floor prices (BAYC, CryptoPunks, Azuki) | Reservoir API (`/collections/v7`) | REST → JSON |
| BEND OHLCV | CoinGecko API (`/coins/bend-dao/market_chart`) | REST → JSON |
| APE OHLCV | CoinGecko or Binance REST API | REST → JSON |
| BendDAO liquidation events | Etherscan event logs (`LiquidateNFT` event) or subgraph | GraphQL |
| BEND perp funding rate | Binance historical funding (`/fapi/v1/fundingRate`) | REST → JSON |

### Event Identification
1. Pull all `LiquidateNFT` events from BendDAO contract (Ethereum mainnet: `0x70b97A0da65C15dfb0FFA02aEE6FA36e507C2762`)
2. For each liquidation cluster (≥3 liquidations within 48h for same collection), mark T=0
3. Check if pre-trigger condition (HF < 1.15 on ≥5% of book) was observable 6–24h before T=0

### Metrics to Compute

- **BEND return:** T+0 to T+96h from trigger
- **APE return:** T+0 to T+96h from trigger (BAYC events only)
- **Floor return:** T+0 to T+96h from trigger
- **Hit rate:** % of triggers where BEND declines ≥10% within 96h
- **Average return per trigger**
- **Max drawdown per trade** (for stop-loss calibration)
- **Cascade amplification factor:** (loans liquidated at T+48h) / (loans at HF < 1.15 at T=0)

### Baseline Comparison
- Compare BEND returns on trigger days vs. random non-trigger days (same lookback window)
- Compare to simply shorting BEND on any day floor drops ≥8% (test whether the HF concentration filter adds value)

### Minimum Event Count
- Need ≥8 distinct trigger events to draw any conclusions. If fewer exist in the historical record, the strategy cannot be backtested with statistical confidence — flag as "insufficient data."

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **Hit rate ≥ 60%** on the primary BEND short (BEND down ≥10% within 96h of trigger)
2. **Average return per trade ≥ 8%** on the BEND leg (gross, before fees)
3. **Max single-trade drawdown < 15%** (validates stop-loss placement)
4. **Cascade amplification factor > 1.5x** in ≥50% of events (confirms the cascade mechanic is real, not just correlated selling)
5. **HF filter adds value:** Trigger hit rate must be ≥15 percentage points higher than the naive "short on 8% floor drop" baseline
6. **Minimum 6 qualifying events** in backtest period (if fewer, do not go live — sample too small)

---

## Kill Criteria

Abandon the strategy (do not paper trade or go live) if any of the following:

1. **BendDAO TVL falls below $5M** — cascade events will be too small to move BEND price meaningfully
2. **BEND perp delisted** from Hyperliquid and Binance — no executable short instrument
3. **BendDAO deploys circuit breaker** (governance-approved pause mechanism) — structural mechanic is broken
4. **Backtest hit rate < 50%** — mechanism is not reliably triggering price impact
5. **No qualifying trigger events in 6 months of live monitoring** — protocol is dormant; opportunity cost too high
6. **New NFT lending protocol (e.g., Arcade, NFTfi) absorbs majority of BAYC loan book** — BendDAO-specific trigger no longer representative of collection-wide stress

---

## Risks

### Structural Risks (affect the mechanism itself)

| Risk | Severity | Mitigation |
|------|----------|------------|
| BendDAO governance pauses liquidations mid-cascade | High | Monitor governance forum; exit immediately on pause proposal |
| Oracle manipulation (floor price feed gamed upward) | Medium | Cross-check Reservoir floor vs. Blur floor; if divergence >10%, do not enter |
| Protocol upgrades change HF formula or thresholds | Medium | Re-read contract on each new deployment; re-validate trigger logic |
| BendDAO v2 / migration changes liquidation mechanic | High | Treat any major upgrade as a kill event until re-analyzed |

### Market Risks (affect P&L but not the mechanism)

| Risk | Severity | Mitigation |
|------|----------|------------|
| BEND short squeeze (thin OI, coordinated buy) | Medium | Hard 12% stop-loss; max 3x leverage |
| APE price driven by macro/Yuga news unrelated to cascade | High | Size APE leg at 1% NAV only; treat as optional |
| Cascade absorbed by a single whale bidder (no cascade) | Medium | This is the primary reason hit rate won't be 100%; sized accordingly |
| Funding rate on BEND short becomes very negative | Low-Medium | Check funding before entry; if annualized cost >50%, reduce size or skip |

### Operational Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Subgraph data lag (The Graph can be 15–30 min behind) | Medium | Use direct Etherscan event polling as backup; accept some signal delay |
| Reservoir API rate limits | Low | Cache floor data locally; use paid tier if needed |
| BEND perp liquidity too thin for exit | Medium | Pre-check OI; size position to exit within 3 market orders |

### Honest Assessment of Edge Decay

The strongest cascade events occurred in **June–August 2022** when BendDAO's BAYC loan book was >$100M. Current TVL is a fraction of that. The BEND short is still mechanically valid but the **APE leg is likely dead** — BendDAO's BAYC exposure is no longer large enough relative to APE's market cap to move the price. The strategy is worth monitoring for a **TVL revival** (new NFT bull market cycle) rather than aggressive deployment now. Current expected value is low due to infrequent triggers, not because the mechanism is broken.

---

## Data Sources

| Resource | URL / Endpoint |
|----------|---------------|
| BendDAO subgraph (The Graph) | `https://api.thegraph.com/subgraphs/name/benddao/bend-protocol` |
| BendDAO contract (Ethereum) | `0x70b97A0da65C15dfb0FFA02aEE6FA36e507C2762` (verify on Etherscan) |
| Reservoir floor price API | `https://api.reservoir.tools/collections/v7?id={collection_address}` |
| Blur floor price | `https://blur.io/api/v1/collections/{slug}` (unofficial; scrape carefully) |
| CoinGecko BEND history | `https://api.coingecko.com/api/v3/coins/bend-dao/market_chart?vs_currency=usd&days=365` |
| Binance BEND perp funding | `https://fapi.binance.com/fapi/v1/fundingRate?symbol=BENDUSDT&limit=1000` |
| Hyperliquid BEND perp | `https://app.hyperliquid.xyz/trade/BEND` (check if listed; OI via API) |
| BendDAO dashboard (live HF) | `https://www.benddao.xyz/en/liquidity/` |
| Etherscan event logs | `https://api.etherscan.io/api?module=logs&action=getLogs&address=0x70b97...&topic0={LiquidateNFT_sig}` |

---

*This document is a hypothesis specification. No backtest has been run. All thresholds (5% loan book, HF 1.15, 8% floor drop) are initial estimates requiring calibration against historical data. Do not trade this strategy until backtest go-live criteria are met.*
