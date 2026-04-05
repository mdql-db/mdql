---
title: "Protocol Surplus Buyback Pressure Index (Cross-Protocol Buyback Race)"
status: HYPOTHESIS
mechanism: 7
implementation: 6
safety: 6
frequency: 3
composite: 756
categories:
  - defi-protocol
  - governance
  - token-supply
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DeFi protocol's on-chain surplus buffer approaches its programmatic buyback/burn trigger threshold, a contractually mandated buy order for the protocol's governance token is imminent. This buy pressure is:

1. **Forecastable** — buffer level and daily accrual rate are on-chain readable, allowing days-to-trigger estimation
2. **Mechanical** — the buyback fires automatically once the threshold is crossed (no human discretion required at execution time)
3. **Underpriced** — the market does not continuously monitor surplus buffer levels; the information is public but requires active on-chain querying to surface

**Causal chain:**

```
Protocol revenue accrues to surplus buffer
        ↓
Buffer level crosses programmatic threshold
        ↓
Smart contract initiates buyback auction / burn mechanism
        ↓
Governance token buy pressure enters market
        ↓
Token price responds to demand shock
        ↓
Informed positioning 48-72h before threshold crossing captures the move
```

The edge is **information asymmetry via data friction**, not speed. Most market participants do not monitor surplus buffer levels in real time. The signal is public but requires infrastructure to surface continuously.

---

## Structural Mechanism — WHY This Must Happen

### MakerDAO (MKR) — Best-Documented Case

The `Vow` contract (`0xA950524441892A31ebddF91d3cEEFa04Bf454466`) maintains a `surplus buffer` (`hump` parameter, currently ~50M DAI). When `dai.balanceOf(vow) - sin - ash > hump + bump`, the `flap()` function becomes callable. This triggers a **Flap Auction**: the protocol sells DAI surplus to buy MKR, which is then burned.

- `hump`: minimum surplus before auction can fire (governance-set, ~50M DAI)
- `bump`: lot size per auction (~30K DAI per flap)
- `flap()` is permissionlessly callable once conditions are met — **no governance vote required at execution**
- MKR burned per auction = bump / MKR market price

This is the gold standard: the buy is **contractually guaranteed** once the buffer threshold is crossed. The only discretion is in governance adjusting `hump` between now and threshold crossing.

### Aave (AAVE) — Collector + Safety Module

Aave's `Collector` contract accumulates protocol fees. The `AaveEcosystemReserveController` and governance proposals periodically authorize buybacks of AAVE for the Safety Module. Unlike MakerDAO, Aave buybacks require **governance execution** — reducing the mechanical certainty. However, Aave has moved toward more automated buyback programs (BGD Labs proposals). Score this sub-mechanism at 4/10 mechanical certainty.

### Frax Finance (FXS/FPI) — AMO Revenue

Frax AMOs generate yield that accrues to the protocol. FXS buybacks are triggered by CR (collateral ratio) mechanics and governance. Less mechanical than MakerDAO. Score: 3/10.

### Liquity V2 (LQTY) — Stability Pool Kickbacks

LQTY stakers earn ETH and LUSD from liquidations. The protocol itself does not do buybacks — staking rewards are the mechanism. **Exclude from this strategy** — wrong mechanism type.

**Conclusion:** Focus initial backtest exclusively on **MKR/MakerDAO Flap Auctions** as the only sufficiently mechanical case. Expand to other protocols only after validating the core thesis.

---

## Entry Rules


### Universe
- **Primary:** MKR (MakerDAO governance token)
- **Secondary (future):** AAVE, once automated buyback programs are confirmed on-chain

### Signal Construction

**Step 1: Read current surplus buffer state**
```
surplus = dai.balanceOf(vow) - vow.sin() - vow.ash()
net_surplus = surplus - vow.hump()
```
If `net_surplus > 0`, a flap auction can fire immediately. If `net_surplus < 0`, compute days to threshold:

**Step 2: Estimate daily accrual rate**
- Pull `surplus` readings at T and T-7 days
- `daily_accrual = (surplus_T - surplus_T7) / 7`
- Use 7-day rolling average to smooth volatility

**Step 3: Forecast days to threshold**
```
days_to_threshold = abs(net_surplus) / daily_accrual
```

**Step 4: Compute buy pressure score**
```
buyback_lot_size = vow.bump()  # DAI per auction
expected_auctions_per_day = daily_accrual / buyback_lot_size
daily_mkr_buy_pressure_usd = expected_auctions_per_day * buyback_lot_size
buy_pressure_ratio = daily_mkr_buy_pressure_usd / mkr_market_cap
```

### Entry Trigger
- `days_to_threshold <= 3` AND `days_to_threshold > 0`
- `daily_accrual > 0` (buffer is growing, not shrinking)
- No active governance proposal to raise `hump` within 72h (check Maker governance forum + on-chain spell queue)
- MKR 24h volume > $5M (minimum liquidity gate)

**Enter long MKR perpetual on Hyperliquid at next daily close after signal fires.**

## Exit Rules

### Exit Rules
- **Primary exit:** 24h after first confirmed `Flap` event (on-chain event log from Vow contract)
- **Time stop:** Exit at T+96h if no Flap event fires (threshold not crossed — accrual estimate was wrong)
- **Stop loss:** -4% from entry (hard stop, no exceptions)
- **Take profit:** No fixed TP — ride until primary exit trigger

### Position Direction
- **Long only.** The mechanism creates buy pressure, not sell pressure.
- Do not short MKR post-flap; burn reduces supply but price impact is typically already realized.

---

## Position Sizing

- **Base allocation:** 1% of portfolio per signal
- **Scaling:** Do not scale up based on buy_pressure_ratio alone — the ratio is small vs daily volume in most cases
- **Maximum concurrent positions:** 1 (MKR only in initial phase)
- **Leverage:** 1x-2x maximum. This is a low-conviction, short-duration trade. Do not use leverage > 2x.
- **Rationale for small sizing:** The buyback lot sizes (~$30K DAI per flap) are small relative to MKR's daily volume (~$20-50M). The edge is in anticipation, not in the buyback itself moving the market significantly. Position sizing must reflect this modest expected impact.

---

## Backtest Methodology

### Data Sources

| Data | Source | Endpoint |
|------|---------|----------|
| Vow contract state (hump, bump, sin, ash) | Ethereum RPC / Etherscan | `eth_call` to `0xA950524441892A31ebddF91d3cEEFa04Bf454466` |
| DAI balance of Vow | Ethereum RPC | `dai.balanceOf(vow)` at historical blocks |
| Historical Flap auction events | Dune Analytics | `maker.flap_auctions` table |
| MKR OHLCV | CoinGecko API / Kaiko | `/coins/maker/market_chart` |
| MKR perpetual price | Hyperliquid historical data | Hyperliquid API |
| Maker governance proposals | Maker governance portal | `vote.makerdao.com` + on-chain spell addresses |

### Historical Flap Auction Dataset

Dune query to extract all historical Flap events:
```sql
SELECT
  block_time,
  tx_hash,
  lot,  -- DAI sold
  bid   -- MKR bought and burned
FROM maker.flap_auctions
ORDER BY block_time ASC
```
Expected dataset: ~2019 to present. MakerDAO has run hundreds of flap auctions. This is the richest dataset available for this strategy type.

### Backtest Procedure

1. **Reconstruct signal history:** For each day from 2020-01-01 to present, compute `days_to_threshold` using historical block data (query Vow state at each day's block)
2. **Identify signal fires:** Days where `days_to_threshold` crossed from >3 to ≤3
3. **Simulate trades:** Enter at next-day open, exit per rules above
4. **Record:** Entry price, exit price, days held, whether Flap fired within 96h, P&L

### Metrics to Compute

| Metric | Target | Rationale |
|--------|--------|-----------|
| Win rate | >55% | Low bar given small expected moves |
| Average win / average loss | >1.5 | Must compensate for small wins |
| Sharpe ratio | >0.8 annualized | Minimum acceptable risk-adjusted return |
| % of signals where Flap fired within 96h | >70% | Validates accrual rate forecasting accuracy |
| Average MKR return T-3 to T+1 (Flap day) | >0% | Core hypothesis test |
| Average MKR return vs ETH (beta-adjusted) | >0% | Isolate protocol-specific effect |

### Baseline Comparison
- **Null hypothesis:** Buy MKR randomly on any day, hold 4 days. Compare Sharpe and win rate.
- **Beta control:** Subtract ETH return over same window to isolate MKR-specific effect from crypto market beta.

### Segmentation Analysis
- Split results by: bull market vs bear market periods, high vs low MKR volatility, large vs small `bump` size
- Check if edge degrades post-2022 (when MakerDAO became more widely followed)

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Flap forecast accuracy ≥ 70%:** At least 70% of signals result in a Flap auction within 96h of entry
2. **Beta-adjusted MKR return > 0%** on average over the T-3 to T+1 window
3. **Sharpe ratio ≥ 0.8** on simulated trades (after 0.1% per-side transaction cost)
4. **Minimum 30 signal events** in backtest (MakerDAO has enough history; if fewer than 30 clean signals exist, the strategy cannot be validated)
5. **No single governance event** (hump adjustment) accounts for more than 30% of losses — if governance interference is the dominant loss driver, the strategy is not viable

---

## Kill Criteria

Abandon the strategy if any of the following occur:

### At Backtest Stage
- Beta-adjusted return is negative or not statistically distinguishable from zero (p > 0.1)
- Flap forecast accuracy < 50% (accrual rate is too noisy to forecast threshold crossing)
- Edge is entirely concentrated in pre-2021 data and absent post-2022

### At Paper Trading Stage
- 10 consecutive paper trades with negative P&L
- Forecast accuracy drops below 60% in live conditions
- MakerDAO governance votes to eliminate or fundamentally restructure the Flap mechanism (this has been discussed — monitor)

### Structural Kill Triggers
- MakerDAO migrates to Spark/SubDAO structure that eliminates the Vow/Flap mechanism
- MKR is replaced by a new token (NewStable/NewGovToken migration — already partially underway as of 2024)
- Daily MKR volume drops below $3M (insufficient liquidity for clean entry/exit)

**Note:** MakerDAO's ongoing "Endgame" restructuring is a live risk. The Vow/Flap mechanism may be deprecated. Monitor `https://forum.makerdao.com` and on-chain spell queue continuously.

---

## Risks

### Risk 1: Governance Threshold Adjustment (HIGH probability, MEDIUM impact)
MakerDAO governance can raise `hump` at any time via executive spell. If `hump` is raised while a signal is active, the threshold moves away and the Flap may not fire. **Mitigation:** Check the on-chain spell queue and governance forum for pending `hump` changes before entry. If a spell touching `hump` is in the hat queue, do not enter.

### Risk 2: Small Buyback Size vs Market Volume (HIGH probability, LOW impact)
Each Flap auction is ~$30K DAI. MKR daily volume is ~$20-50M. The mechanical buy pressure per auction is <0.1% of daily volume. The edge, if it exists, is from **anticipation** not from the buyback itself moving the market. This means the edge is fragile — if more participants start monitoring the Vow contract, the anticipatory move gets front-run earlier and earlier until it disappears. **Mitigation:** Monitor whether the T-3 to T+1 window is compressing over time.

### Risk 3: Revenue Rate Volatility (MEDIUM probability, MEDIUM impact)
MakerDAO revenue depends on DSR utilization, stability fees, and RWA yields. A sudden drop in revenue (e.g., large USDC depeg event, RWA redemptions) can cause the accrual rate to drop or reverse, making the days-to-threshold forecast wrong. **Mitigation:** Use 7-day rolling accrual rate; if 1-day rate diverges >30% from 7-day rate, do not enter.

### Risk 4: MakerDAO Endgame Migration (MEDIUM probability, HIGH impact)
MakerDAO is restructuring into SubDAOs with a new token (MKR → NewGovToken). The Vow/Flap mechanism may be deprecated. If this happens, the entire strategy is void. **Mitigation:** This is a kill trigger. Monitor governance continuously.

### Risk 5: Market Beta Dominance (HIGH probability, LOW-MEDIUM impact)
MKR moves with the broader crypto market. A 4-day window is long enough for macro crypto moves to swamp the protocol-specific signal. **Mitigation:** Beta-adjust returns in backtest; consider hedging with short ETH or BTC to isolate the MKR-specific component. This adds complexity and cost.

### Risk 6: Information Leakage / Front-Running (LOW probability now, HIGH probability if strategy scales)
If this strategy becomes known, others will monitor the Vow contract and the anticipatory window will compress. The strategy has a natural capacity ceiling — probably $50K-$200K position size before self-defeating. **Mitigation:** Keep position sizes small; monitor whether the edge is degrading over time.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|--------|---------------|--------------|
| Ethereum RPC (Alchemy/Infura) | `https://eth-mainnet.g.alchemy.com/v2/{key}` | Historical Vow contract state via `eth_call` at block numbers |
| Vow contract | `0xA950524441892A31ebddF91d3cEEFa04Bf454466` | `sin()`, `ash()`, `hump()`, `bump()`, `Joy()` |
| DAI contract | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | `balanceOf(vow)` |
| Dune Analytics | `https://dune.com/queries` | `maker.flap_auctions` table — all historical Flap events with timestamps |
| Maker Governance Forum | `https://forum.makerdao.com` | Monitor for `hump` / `bump` adjustment proposals |
| Maker On-Chain Governance | `https://vote.makerdao.com/executive` | Spell queue — check for pending parameter changes |
| CoinGecko API | `https://api.coingecko.com/api/v3/coins/maker/market_chart` | MKR OHLCV historical |
| Kaiko (paid) | `https://www.kaiko.com` | Higher-quality MKR tick data if needed |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | MKR-USDC perpetual historical funding + price |
| The Graph — Maker subgraph | `https://thegraph.com/hosted-service/subgraph/makerdao/mcd-core` | Alternative to direct RPC for historical state |

### Key ABI Calls for Vow Contract
```python
# Pseudocode for signal computation
vow = w3.eth.contract(address=VOW_ADDRESS, abi=VOW_ABI)
dai = w3.eth.contract(address=DAI_ADDRESS, abi=ERC20_ABI)

surplus = dai.functions.balanceOf(VOW_ADDRESS).call(block_identifier=block_num)
sin = vow.functions.Sin().call(block_identifier=block_num)  # note: Sin in RAD units
ash = vow.functions.Ash().call(block_identifier=block_num)
hump = vow.functions.hump().call(block_identifier=block_num)
bump = vow.functions.bump().call(block_identifier=block_num)

# Convert RAD to WAD (divide by 1e27 for sin/ash, surplus already in WAD)
net_surplus_dai = (surplus - sin/1e27 - ash/1e27) - hump/1e27
```

---

## Implementation Priority

1. **Week 1:** Build Dune query to extract all historical Flap auction events with timestamps. Cross-reference with MKR price data. Run simple event study: MKR return T-5 to T+5 around each Flap event.
2. **Week 2:** Reconstruct historical Vow surplus levels using The Graph or direct RPC archive node. Validate that days-to-threshold forecasting would have been accurate.
3. **Week 3:** Full backtest simulation with entry/exit rules. Compute all metrics.
4. **Week 4:** Governance interference analysis — identify all historical `hump` changes and their impact on signals.
5. **Go/No-Go decision** based on backtest results against go-live criteria.

**Estimated backtest complexity:** Medium. The Dune data for Flap auctions is clean. The hard part is reconstructing historical Vow surplus levels at daily granularity — requires either an archive node or The Graph historical queries.
