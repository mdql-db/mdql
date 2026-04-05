---
title: "MakerDAO OSM Delay Front-Run"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 3
composite: 540
categories:
  - liquidation
  - defi-protocol
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When ETH spot price drops >3% in <30 minutes, a subset of MakerDAO Vault positions cross below their liquidation ratio at *market price* but remain above it at the *OSM price* — creating a guaranteed deferred liquidation window of up to 60 minutes. The at-risk collateral is visible on-chain in real time. A short position on ETH-USDC perp entered during this window captures the incremental selling pressure when the OSM updates and liquidation bots execute.

**Causal chain:**

1. ETH spot drops >3% in <30 min on CEX/DEX
2. MakerDAO OSM still reflects the price from ≤60 min ago — Vaults that are now insolvent at market price are not yet liquidatable on-chain
3. On-chain subgraph shows Vault collateral ratios calculated against *current* ETH price — at-risk positions are identifiable before the protocol can act
4. OSM updates on the next scheduled tick (every hour, on the hour, triggered by `poke()` call)
5. Post-update: Keeper bots call `bite()` on undercollateralised Vaults, forcing collateral liquidation auctions
6. Auction collateral (ETH) is sold into the market, adding real sell-side pressure
7. Short position profits from this incremental sell pressure; exit after cascade resolves

---

## Structural Mechanism — Why This MUST Happen

The OSM delay is not a bug or a tendency — it is a **hard-coded protocol parameter** in the `OSM.sol` contract deployed on Ethereum mainnet. Specifically:

- `OSM.sol` stores two price slots: `cur` (current, used by the protocol) and `nxt` (next, sourced from the underlying oracle)
- `poke()` can only be called if `block.timestamp >= last + hop`, where `hop` is set to **3600 seconds (1 hour)**
- Until `poke()` is called, `cur` does not update — the protocol *cannot* liquidate based on a newer price regardless of market conditions
- Liquidation eligibility check (`bite()`) uses `cur`, not spot price
- Therefore: any Vault that becomes undercollateralised between OSM updates is **legally unliquidatable** for up to 60 minutes

This is not probabilistic. The smart contract enforces it. The only escape valve is Emergency Shutdown (governance multisig), which has never been triggered for a routine price drop.

The liquidation cascade direction is probabilistic in magnitude but **certain in direction**: when OSM updates, Keeper bots (which are watching the same on-chain state) will immediately call `bite()` on all eligible Vaults. This is a competitive, well-incentivised keeper ecosystem — the liquidation execution itself is near-instant post-OSM-update.

---

## Entry / Exit Rules

### Pre-conditions (must ALL be true before monitoring)
- ETH-USD OSM `cur` price is available and timestamp is known (fetch from contract)
- MakerDAO total ETH-A + ETH-B + ETH-C Vault collateral at risk (calculated below) exceeds **$10M USD** at current market price
- Time to next OSM update is **>15 minutes** (enough time to enter and hold)

### Entry Signal
| Parameter | Value |
|-----------|-------|
| Trigger | ETH spot price drops >3% from its level at the last OSM update, within any rolling 30-min window |
| At-risk threshold | Sum of collateral in Vaults with collateral ratio <115% at *current* spot price exceeds $10M |
| Timing constraint | Entry only if >15 min remain before next OSM `poke()` is due |
| Instrument | ETH-USDC perpetual on Hyperliquid |
| Direction | Short |

**Entry price:** Market order at signal confirmation. Do not use limit orders — the window is time-sensitive.

### Exit Rules
| Condition | Action |
|-----------|--------|
| Primary exit | OSM `poke()` confirmed on-chain + 10 minutes elapsed | Close short at market |
| Stop loss | ETH spot recovers to within 1% of the OSM `cur` price (liquidation risk evaporates) | Close short at market |
| Time stop | 75 minutes after entry with no OSM update (anomaly — keeper failure) | Close 50% immediately, reassess |
| Cascade exhaustion | ETH price stabilises or reverses >1.5% after OSM update | Close remaining position |

### Do Not Enter If
- ETH has already dropped >8% (liquidation cascade may already be partially priced in, and recovery risk is higher)
- OSM update is <15 min away (insufficient holding window)
- Total at-risk collateral <$10M (insufficient expected sell pressure to overcome friction)
- Funding rate on ETH perp is >0.05% per 8h in the short direction (crowded short — edge is already priced)

---

## Position Sizing

**Base size:** 0.5% of portfolio per trade.

**Scaling rule:** Scale linearly with at-risk collateral:
- $10M–$25M at risk → 0.5% of portfolio
- $25M–$75M at risk → 1.0% of portfolio
- >$75M at risk → 1.5% of portfolio (hard cap)

**Rationale:** The at-risk collateral is the direct driver of expected sell pressure. Larger at-risk pools justify larger positions. Hard cap at 1.5% because cascade magnitude is still probabilistic — keeper efficiency, auction mechanics, and secondary market depth all affect realised impact.

**Leverage:** 3x maximum. The holding window is short (30–75 min) and the stop loss is tight. Higher leverage is not warranted given the probabilistic magnitude.

**Max concurrent positions:** 1. This strategy fires on the same asset (ETH) and the same mechanism — running multiple simultaneous positions means you've entered the same trade twice.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity |
|---------|--------|-------------|
| ETH spot price | Binance REST API (`/api/v3/klines`, ETH-USDT, 1m) | 1-minute OHLCV |
| OSM `poke()` call timestamps | Ethereum event logs, `LogValue` event on `0x81FE72B5A8d1A857d176C3E7d5Bd2679A9B85763` (ETH-A OSM) | Per-block |
| OSM `cur` price at each update | Same contract, `peek()` return value at each `poke()` block | Per OSM update |
| Vault collateral ratios | MakerDAO Subgraph (The Graph) — `Vault` entity with `collateralAmount`, `debtAmount`, `ilk` | Per block or per OSM update |
| ETH-USDC perp prices | Hyperliquid historical data API or reconstruct from Binance ETH-USDT perp | 1-minute |
| Funding rates | Hyperliquid or Binance perp funding history | Per 8h period |

### Backtest Period
- **Primary:** January 2022 – December 2024 (covers multiple high-volatility regimes including LUNA crash, FTX collapse, 2022 bear)
- **Out-of-sample validation:** January 2025 – present

### Signal Construction (per bar)

1. For each 1-minute bar, check if ETH spot has dropped >3% vs. the OSM `cur` price at the most recent `poke()`
2. If yes, query Vault subgraph for total collateral in Vaults with `(collateralAmount * spot_price) / debtAmount < 1.15` (i.e., collateral ratio <115% at spot)
3. If at-risk collateral >$10M and time to next OSM update >15 min → **signal = 1**
4. Record entry price (1-min close), OSM update timestamp, ETH price at OSM update + 10 min, stop-loss hit (if any)

### Metrics to Calculate

| Metric | Target | Minimum Acceptable |
|--------|--------|--------------------|
| Win rate | >55% | >50% |
| Average P&L per trade (bps) | >15 bps net of fees | >8 bps |
| Sharpe ratio (annualised) | >1.5 | >1.0 |
| Max drawdown | <15% | <25% |
| Number of qualifying signals | >30 | >20 (statistical validity) |
| Average holding time | 30–75 min | — |
| P&L correlation with at-risk collateral size | Positive | — |

### Baseline Comparison
- **Null hypothesis:** Short ETH on any >3% drop in 30 min, regardless of OSM state or Vault data, hold for 60 min. If the OSM-aware strategy does not outperform this baseline, the OSM mechanism is not adding signal.
- **Fee assumption:** 0.035% taker fee each way on Hyperliquid (0.07% round trip), plus 0.01% slippage assumption.

### Key Subgroup Analyses
- P&L segmented by at-risk collateral bucket ($10–25M, $25–75M, >$75M)
- P&L segmented by time-of-day (OSM updates on the hour — does the 55-min window vs. 20-min window matter?)
- P&L in high-volatility vs. low-volatility regimes (VIX proxy: ETH 30-day realised vol >80% vs. <80%)

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **≥30 qualifying signals** in backtest period (statistical minimum)
2. **Win rate ≥52%** on primary backtest period
3. **Average net P&L ≥10 bps** per trade after fees and slippage
4. **Sharpe ≥1.2** on primary period
5. **Positive P&L correlation** with at-risk collateral size (confirms the mechanism, not noise)
6. **OSM-aware strategy beats null hypothesis baseline** by ≥5 bps average per trade
7. **Out-of-sample period (2025) is not deeply negative** — acceptable to be flat, not acceptable to be -5 bps or worse average

If criteria 1–6 pass but criterion 7 fails, flag for review before paper trading — do not auto-proceed.

---

## Kill Criteria

Abandon the strategy (paper or live) if any of the following occur:

| Trigger | Action |
|---------|--------|
| MakerDAO governance votes to reduce OSM `hop` below 3600s | Kill immediately — structural mechanism changes |
| MakerDAO governance votes to change liquidation ratio calculation to use spot price | Kill immediately |
| ETH-A + ETH-B + ETH-C TVL drops below $500M total | Suspend — insufficient at-risk collateral pool to generate meaningful signals |
| 20 consecutive paper trades with average loss >5 bps | Kill — mechanism may be fully arbitraged away |
| Funding rate on ETH perp shorts persistently >0.03% per 8h | Suspend — carry cost erodes edge |
| Hyperliquid ETH-USDC perp average daily volume drops below $50M | Suspend — execution risk too high |
| Spark Protocol absorbs >80% of former MakerDAO ETH vault TVL without equivalent OSM mechanism | Reassess — may need to migrate to Spark equivalent |

---

## Risks — Honest Assessment

### High Severity

**Keeper efficiency has improved dramatically.** In 2020–2021, keeper bots were slower and less competitive. By 2024, the keeper ecosystem is highly optimised — the liquidation cascade may execute so efficiently that the price impact is absorbed within seconds of the OSM update, not minutes. If the cascade resolves in <2 minutes, a market-order entry and exit strategy cannot capture it without HFT infrastructure. *Mitigation: backtest will reveal if holding window is too short; if so, strategy is dead.*

**MakerDAO TVL is declining.** As Spark Protocol absorbs lending activity, the pool of at-risk Vault collateral shrinks. Fewer qualifying signals, smaller cascades. *Mitigation: monitor TVL threshold; kill criteria address this.*

**OSM update is not automatic — it requires a keeper to call `poke()`.** If no keeper calls `poke()` promptly at the hour mark, the update is delayed. In practice this is rare (keepers are incentivised), but it adds timing uncertainty to the exit. *Mitigation: monitor `poke()` calls directly; use time stop at 75 min.*

### Medium Severity

**The cascade is already partially priced in.** Sophisticated market participants (MEV bots, large funds) may be running the same trade. If the short is crowded, the entry price already reflects the expected cascade, and you're paying for a known event. *Mitigation: check funding rate pre-entry; skip if shorts are crowded.*

**Auction mechanics dampen spot impact.** MakerDAO uses Collateral Auction (`Clipper`) which sells collateral via Dutch auction, not immediate market dumps. The spot impact may be more gradual than a direct liquidation. *Mitigation: backtest will quantify actual price impact; adjust exit timing accordingly.*

**ETH recovery before OSM update.** If ETH bounces sharply before the OSM update, the at-risk Vaults become safe again and no liquidations occur. The stop loss handles this but it will generate losing trades. *Mitigation: tight stop at 1% recovery; position sizing limits damage.*

### Low Severity

**Emergency Shutdown.** Governance can trigger Emergency Shutdown, which freezes the OSM and all liquidations. This has never happened for a routine price drop and would require an extraordinary governance action. *Mitigation: monitor governance forums; not a realistic risk for normal operations.*

**Gas costs for on-chain monitoring.** Reading on-chain state (Vault ratios, OSM price) requires RPC calls but not transactions — no gas cost for monitoring. *Mitigation: use a free/paid Ethereum RPC endpoint (Alchemy, Infura, or self-hosted node).*

---

## Data Sources

| Data | URL / Endpoint |
|------|---------------|
| ETH-A OSM contract | `0x81FE72B5A8d1A857d176C3E7d5Bd2679A9B85763` on Ethereum mainnet |
| ETH-B OSM contract | `0xe0de8f5E63f7a746f444b5e4E5F3B4b4c3b4b4c3` — verify on Etherscan |
| OSM ABI / `peek()` call | [Etherscan OSM contract](https://etherscan.io/address/0x81FE72B5A8d1A857d176C3E7d5Bd2679A9B85763#readContract) |
| MakerDAO Subgraph (Vaults) | `https://api.thegraph.com/subgraphs/name/protofire/maker-protocol` |
| MakerDAO official API | `https://api.makerdao.com/` (check current availability) |
| DefiLlama MakerDAO TVL | `https://api.llama.fi/protocol/makerdao` |
| Binance ETH spot (1m OHLCV) | `https://api.binance.com/api/v3/klines?symbol=ETHUSDT&interval=1m` |
| Hyperliquid historical data | `https://app.hyperliquid.xyz/api` (check docs for historical fills) |
| Ethereum event logs (historical) | Alchemy / Infura archive node, or [Dune Analytics](https://dune.com) — query `ethereum.logs` for OSM `LogValue` events |
| Dune OSM dashboard (pre-built) | Search Dune for "MakerDAO OSM" — several community dashboards exist |
| MakerDAO governance forum | `https://forum.makerdao.com` — monitor for OSM parameter change proposals |

**Recommended implementation stack for backtest:**
- Python + `web3.py` for on-chain data fetching
- The Graph client for Vault subgraph queries
- Dune Analytics SQL for historical OSM event reconstruction (faster than raw RPC for historical data)
- Pandas for signal construction and P&L calculation

---

*This specification is sufficient to build a backtest. The critical unknown is whether the price impact of the liquidation cascade is large enough and slow enough to be captured without HFT infrastructure. The backtest will answer this. If average trade duration needs to be <5 minutes to capture the edge, the strategy is not executable for Zunid and should be killed.*
