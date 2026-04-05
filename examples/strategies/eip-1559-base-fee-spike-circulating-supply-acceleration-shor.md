---
title: "EIP-1559 Burn Spike — Net Issuance Inversion Long (ETH)"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 6
frequency: 5
composite: 720
categories:
  - token-supply
  - funding-rates
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Ethereum's real-time burn rate exceeds validator issuance for a sustained window (≥2 hours), a net supply contraction is occurring that is mechanically verifiable on-chain but not yet reflected in ETH spot or perpetual prices. The causal chain:

1. Network congestion drives `baseFeePerGas` above ~40 gwei (the approximate threshold where burn > issuance at full blocks)
2. Every block burns `baseFeePerGas × gasUsed` ETH — this is hardcoded, non-discretionary, irreversible
3. Burn rate exceeds the fixed validator issuance of ~2,300 ETH/day (~0.96 ETH/block at 12s block times)
4. Net circulating supply is contracting in real-time, visible via public RPC or block explorers
5. Most market participants monitor price feeds and social signals, not mempool/block data — creating a 1–3 hour information lag
6. ETH spot and perp prices adjust upward as the supply shock becomes visible to broader market participants
7. Funding rates on ETH perps lag spot by an additional 30–60 minutes, creating a secondary signal

**Null hypothesis to disprove:** Price moves cause congestion (reverse causality), meaning the burn signal is a lagging indicator of a price move already in progress, not a leading indicator of one yet to come.

---

## Structural Mechanism

**Why this MUST happen (the mechanical part):**

EIP-1559 (live since August 2021, block 12,965,000) hardcodes the following:

```
burn_per_block = baseFeePerGas × gasUsed
```

This is enforced at the protocol level — no miner/validator discretion, no governance override without a hard fork. The burn is permanent and irreversible.

Validator issuance is also fixed by protocol:

```
issuance_per_epoch ≈ 940 ETH (32 ETH × ~1,700 active validators per epoch, scaled by participation)
```

At current validator counts (~1M validators, ~32M ETH staked), issuance is approximately 2,300 ETH/day = ~0.96 ETH per 12-second block.

**The crossover point:** Net deflation occurs when:

```
baseFeePerGas × gasUsed_per_block > 0.96 ETH
```

At full blocks (gasUsed = 15M gas, the target; up to 30M at the cap):
- 15M gas block: baseFee must exceed ~64 gwei for net deflation
- 30M gas block (full): baseFee must exceed ~32 gwei for net deflation

This crossover is mechanically calculable from public data with zero ambiguity.

**What is NOT guaranteed (the probabilistic part):**

The supply contraction is guaranteed. The price response is not. The strategy bets that:
- The information lag is real and persistent (not already arbitraged away)
- The lag is long enough (>1 hour) to be exploitable without HFT infrastructure
- The supply shock is large enough to move price within the 12-hour hold window

---

## Entry/Exit Rules

### Data inputs (polled every 5 minutes)

```
burn_rate_eth_per_day = mean(baseFeePerGas_i × gasUsed_i) × (86400 / 12) 
                        [averaged over last 60 blocks = ~12 minutes]

net_issuance_eth_per_day = burn_rate_eth_per_day - 2300

eth_price_change_2h = (current_price - price_120min_ago) / price_120min_ago
```

### Entry conditions (ALL must be true simultaneously)

| Condition | Threshold | Rationale |
|-----------|-----------|-----------|
| Net issuance | < 0 ETH/day (burn > 2,300 ETH/day) | Confirms deflationary window |
| Duration | Sustained ≥ 2 hours (24 consecutive 5-min polls) | Filters transient spikes |
| ETH price move (prior 2h) | < +3% | Filters events already priced in |
| ETH price move (prior 2h) | > -5% | Avoids entering into crash-driven congestion (e.g., liquidation cascades causing gas spikes) |
| Funding rate (ETH perp) | < +0.05% per 8h | Avoids entering when longs already crowded |

**Entry instrument:** ETH perpetual on Hyperliquid (primary) or ETH spot (secondary if funding is elevated).

**Entry execution:** Market order at next 5-minute poll after all conditions met. No limit orders — the edge is time-sensitive.

### Exit conditions (first triggered wins)

| Exit trigger | Action |
|--------------|--------|
| Base fee < 20 gwei sustained for ≥ 60 minutes (12 consecutive polls) | Close full position — congestion window over |
| ETH price +5% from entry | Close full position — take profit |
| ETH price -3% from entry | Close full position — stop loss |
| 12 hours elapsed since entry | Close full position — time stop |
| Net issuance turns positive for ≥ 90 minutes | Close full position — structural signal reversed |

### Re-entry

No re-entry within 4 hours of a closed position. One active position at a time.

---

## Position Sizing

**Base size:** 2% of portfolio per trade.

**Rationale:** The causal direction is ambiguous (see Risks). This is a hypothesis-stage strategy. 2% allows meaningful backtest signal without catastrophic drawdown if the null hypothesis is correct.

**Leverage:** 2x maximum on perp. The edge (if real) comes from information lag, not leverage. Higher leverage introduces liquidation risk during the volatile congestion events that trigger entry.

**Scaling rule:** After 50 live trades with Sharpe > 1.0, increase to 4% base size. Do not scale before this threshold.

**No pyramiding** during a single congestion event.

---

## Backtest Methodology

### Data sources

| Data | Source | Endpoint/URL |
|------|--------|--------------|
| Per-block baseFee + gasUsed | Etherscan API | `https://api.etherscan.io/api?module=proxy&action=eth_getBlockByNumber` |
| Historical base fee (bulk) | Dune Analytics | Query: `ethereum.blocks` table, columns `base_fee_per_gas`, `gas_used`, `time` |
| ETH/USD OHLCV (hourly) | Binance API | `GET /api/v3/klines?symbol=ETHUSDT&interval=1h` |
| ETH perp funding rate history | Binance/Bybit | Binance: `GET /fapi/v1/fundingRate?symbol=ETHUSDT` |
| Validator issuance | beaconcha.in | `https://beaconcha.in/api/v1/epoch/{epoch}` — field `totalvalidatorbalancechange` |

### Time range

**Primary:** August 5, 2021 (EIP-1559 activation, block 12,965,000) → present.

**Key congestion events to verify coverage:**
- Otherside NFT mint: May 1, 2022 (base fee peaked ~8,000 gwei)
- BAYC mint congestion: April 2022
- Merge transition: September 15, 2022 (issuance structure changed — verify issuance rate recalibration)
- Blur/NFT season: Q1 2023
- Memecoin frenzies: May 2023, March 2024

### Backtest construction (Dune-first approach)

**Step 1 — Build burn rate series:**
```sql
-- Dune Analytics: ethereum.blocks
SELECT
  date_trunc('hour', time) AS hour,
  SUM(CAST(base_fee_per_gas AS DOUBLE) * gas_used) / 1e18 AS eth_burned,
  COUNT(*) AS block_count
FROM ethereum.blocks
WHERE time >= TIMESTAMP '2021-08-05'
GROUP BY 1
ORDER BY 1
```

**Step 2 — Compute net issuance per hour:**
```
net_issuance_per_hour = eth_burned_per_hour - (2300 / 24)
```
Note: Adjust issuance rate for pre/post-Merge (pre-Merge PoW issuance was ~13,000 ETH/day — strategy is only valid post-Merge for current issuance rate; or recalibrate for PoW era separately).

**Step 3 — Identify entry signals:**
Flag hours where rolling 2-hour net issuance < 0 AND ETH price change over prior 2 hours is between -5% and +3%.

**Step 4 — Simulate trades:**
For each entry signal:
- Record entry price (ETH/USD close of signal hour)
- Scan forward for first exit condition hit
- Record exit price, hold duration, P&L

**Step 5 — Metrics to compute:**

| Metric | Minimum acceptable | Target |
|--------|-------------------|--------|
| Total trades | ≥ 30 | ≥ 50 |
| Win rate | > 50% | > 60% |
| Average win / average loss | > 1.5 | > 2.0 |
| Sharpe ratio (annualised) | > 0.8 | > 1.5 |
| Max drawdown | < 15% | < 8% |
| % trades where price moved >1% in direction within 6h | > 40% | > 55% |

**Step 6 — Reverse causality test (critical):**
For each entry signal, check: did ETH price move >2% in the 2 hours BEFORE the burn signal triggered? If >40% of signals are preceded by a price move, the signal is likely lagging, not leading. This is the primary validity test.

**Step 7 — Baseline comparison:**
Compare strategy returns against: (a) buy-and-hold ETH, (b) random entry with same hold duration and exit rules, (c) entry triggered by price momentum alone (>2% move in prior 2h, no burn filter). Strategy must outperform all three baselines on Sharpe.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **≥ 30 historical trade signals** identified in backtest period
2. **Win rate ≥ 52%** (statistically above 50% at p < 0.10 with 30 trades)
3. **Average R:R ≥ 1.5** (average win ≥ 1.5× average loss)
4. **Reverse causality test passes:** < 35% of signals preceded by >2% price move in prior 2 hours
5. **Sharpe > 1.0** on out-of-sample period (hold out 2023–present as OOS, train on 2021–2022)
6. **No single event accounts for >30% of total strategy P&L** (concentration risk — e.g., if Otherside mint alone drives all returns, the strategy is not repeatable)

---

## Kill Criteria

Abandon strategy (stop paper trading, do not go live) if:

| Condition | Action |
|-----------|--------|
| Backtest shows < 25 signals in 3.5 years | Insufficient frequency — not worth infrastructure cost |
| Reverse causality test fails (>40% signals lag price) | Null hypothesis confirmed — signal is not leading |
| Out-of-sample Sharpe < 0.5 | In-sample results are overfit |
| Paper trading: 10 consecutive losses | Structural change in market microstructure — halt and review |
| Paper trading: drawdown > 8% of paper portfolio | Risk parameters violated — halt and review |
| ETH transitions to a gas model that removes base fee burning | Structural mechanism no longer exists — immediate kill |

---

## Risks

### 1. Reverse causality (HIGH — primary risk)
Price pumps cause congestion (users rush to mint/trade), not the other way around. If a major NFT launch drives both price and gas simultaneously, the burn signal is a coincident or lagging indicator. The backtest reverse causality test is the single most important validation step. **If this test fails, the strategy has no edge.**

### 2. Congestion events are already anticipated (MEDIUM)
Scheduled events (known NFT mint dates, protocol launches) may be priced in before the burn signal triggers. The 2-hour price filter (< +3% prior move) partially addresses this but won't catch slow pre-positioning.

### 3. Signal frequency may be too low (MEDIUM)
Post-Merge, ETH has been deflationary only during specific congestion windows. In low-activity markets (2023 bear market), the signal may fire < 5 times per year — insufficient for statistical confidence or to justify infrastructure.

### 4. Infrastructure requirement (LOW-MEDIUM)
Requires a persistent bot polling Ethereum RPC or Etherscan API every 5 minutes. Etherscan free tier: 5 calls/second, 100k calls/day — sufficient. Requires uptime monitoring. Not HFT, but not a manual strategy.

### 5. Congestion-driven volatility (MEDIUM)
The same events that trigger the burn signal (NFT frenzies, protocol launches) often cause extreme ETH price volatility in both directions. The -3% stop loss may be hit frequently during these windows, even if the directional thesis is correct over longer timeframes.

### 6. Post-Dencun structural change (LOW but real)
EIP-4844 (Dencun upgrade, March 2024) introduced blob transactions for L2s, reducing L1 gas demand from rollups. This structurally reduced base fee levels. The burn rate signal may fire less frequently post-Dencun. **Backtest must be segmented pre/post Dencun to check for regime change.**

### 7. Funding rate crowding (LOW)
If this signal becomes widely known, the 1–3 hour lag will compress. Monitor for this by tracking whether the lag shortens over time in live trading.

---

## Data Sources

| Source | URL | Notes |
|--------|-----|-------|
| Etherscan Blocks API | `https://api.etherscan.io/api?module=proxy&action=eth_getBlockByNumber&tag={hex_block}&boolean=true&apikey={KEY}` | Free tier sufficient; 5 req/s |
| Dune Analytics | `https://dune.com/queries` — table `ethereum.blocks` | Best for bulk historical pull; free tier available |
| Beaconcha.in API | `https://beaconcha.in/api/v1/epoch/latest` | Validator issuance; free, no auth required |
| Binance Spot OHLCV | `https://api.binance.com/api/v3/klines?symbol=ETHUSDT&interval=5m&limit=1000` | Free, no auth for public endpoints |
| Binance Perp Funding | `https://fapi.binance.com/fapi/v1/fundingRate?symbol=ETHUSDT&limit=1000` | Historical funding rates |
| Hyperliquid Perp Data | `https://api.hyperliquid.xyz/info` — POST `{"type": "fundingHistory", "coin": "ETH"}` | For live trading venue |
| ultrasound.money | `https://ultrasound.money` | Visual burn rate dashboard; useful for sanity-checking signal |
| Etherscan Gas Tracker | `https://api.etherscan.io/api?module=gastracker&action=gasoracle&apikey={KEY}` | Real-time base fee; use for live signal |

---

## Implementation Notes

**Minimum viable backtest:** Pull `ethereum.blocks` from Dune for the full post-Merge period (September 15, 2022 → present), compute hourly burn rates, join with Binance hourly OHLCV, run signal logic in Python/pandas. Estimated build time: 2–3 days for a competent analyst.

**Priority validation question:** Before building anything, run the reverse causality check manually on 5–10 of the largest historical congestion events (Otherside mint, PEPE launch May 2023, etc.) and check whether ETH price moved before or after the burn signal. This is a 2-hour manual check that will determine whether the full backtest is worth building.

**Post-Dencun regime:** Treat pre-Dencun (Aug 2021 – Mar 2024) and post-Dencun (Mar 2024 – present) as separate regimes. If signal frequency drops to < 3/year post-Dencun, the strategy is effectively dead in the current environment regardless of historical backtest results.
