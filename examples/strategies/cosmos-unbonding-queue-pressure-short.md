---
title: "Cosmos Unbonding Queue Pressure Short"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 3
composite: 540
categories:
  - token-supply
  - calendar-seasonal
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large unbonding event (≥0.3% of circulating supply) completes on a Cosmos-SDK chain, a meaningful fraction of that newly liquid stake will be sold within 48–72 hours of release. The 21-day unbonding period is protocol-enforced and the release timestamp is deterministic from the moment the undelegate transaction is broadcast. This creates a predictable, front-runnable supply event. We short the token 3–5 days before the release date and close the position within 24–48 hours after release, capturing the sell pressure window.

The causal mechanism is structural, not statistical: unbonded tokens are **illiquid during the 21-day window** and become **instantly liquid at a known block height**. Holders who initiated unbonding have already signalled intent to exit the staking position; the question is only whether they sell immediately or hold. The base rate of selling after unbonding is higher than random because: (a) opportunity cost of 21-day lockup selects for motivated sellers, (b) large unbonders are often institutions or whales rebalancing, and (c) there is no friction cost to selling once tokens are liquid.

---

## Structural Mechanism

1. **Protocol enforcement:** Cosmos SDK `staking` module enforces unbonding duration at the consensus layer. There is no way to accelerate or cancel an unbonding once initiated. Release block is calculable as: `release_block ≈ undelegate_block + (unbonding_days × avg_blocks_per_day)`.

2. **On-chain visibility:** Undelegate transactions are broadcast publicly and indexed by explorers (Mintscan, Numia, BigDipper) the moment they land on-chain. There is zero information asymmetry about *when* tokens unlock — only about *whether* they will be sold.

3. **Supply shock mechanics:** A single unbonding event ≥0.3% of circulating supply represents material new sell-side liquidity. For ATOM (~1.1B circulating), 0.3% = ~3.3M ATOM ≈ $20M+ at current prices. This is large relative to typical daily DEX + CEX volume.

4. **Staking yield forgone:** During the 21-day window, the unbonder earns zero staking rewards. This self-imposed cost signals conviction to exit, not a passive holder who forgot to redelegate.

5. **No re-entry friction:** Unlike token unlock cliffs (where insiders may hold for tax or strategic reasons), unbonded tokens have no lock-up, vesting schedule, or reputational cost to sell. The holder is already "out" of the staking relationship.

**Why this is NOT purely pattern-based:** The release date is a smart-contract-equivalent guarantee. The supply event WILL occur at a known time. The uncertainty is only in price impact magnitude, not in whether the event happens — hence score 7 rather than 8+.

---

## Universe

| Token | Chain | Hyperliquid Perp | Circulating Supply (approx) | 0.3% Threshold |
|-------|-------|------------------|-----------------------------|----------------|
| ATOM  | Cosmos Hub | ATOM-USDC | ~1.1B | ~3.3M ATOM |
| TIA   | Celestia | TIA-USDC | ~1.5B | ~4.5M TIA |
| INJ   | Injective | INJ-USDC | ~100M | ~300K INJ |

Expand to OSMO, DYDX, SEI if backtest shows sufficient signal. Exclude chains where Hyperliquid perp liquidity (open interest) is below $5M — slippage will eat the edge.

---

## Entry Rules

### Trigger Conditions (ALL must be met)
1. A single undelegate transaction (or cluster of transactions from the same address within a 6-hour window) totals ≥0.3% of circulating supply.
2. The calculated release date is 3–5 calendar days in the future at time of detection.
3. Hyperliquid 24h volume for the perp is ≥$10M (liquidity filter).
4. No major protocol upgrade, airdrop snapshot, or governance vote scheduled within the unbonding window that could create artificial buy pressure.
5. Current funding rate on Hyperliquid is not more negative than −0.10% per 8h (avoid paying excessive funding to be short).

### Entry Execution
- **Entry window:** Open short position within 4 hours of detecting the qualifying undelegate transaction.
- **Entry price:** Market order or limit order within 0.15% of mid-price. Do not chase if spread widens.
- **Entry timing rationale:** 3–5 days pre-release gives time for informed participants to front-run, which itself creates downward pressure before the actual release.

### Position Direction
- **Primary:** Short perpetual futures on Hyperliquid.
- **Alternative (if perp unavailable or funding too negative):** Spot short via margin on a CEX with sufficient liquidity (Binance, OKX).

---

## Exit Rules

### Primary Exit (Time-Based)
- Close 100% of position at **T+48h after release** (release = the block height at which unbonding completes).
- Rationale: Sell pressure is front-loaded. If the unbonder sells, they sell within 24–48h of receiving liquid tokens. Holding longer introduces unrelated market risk.

### Stop Loss
- Hard stop: **+8% adverse move** from entry price (i.e., if token rallies 8% after entry, close immediately).
- Rationale: An 8% rally against a short signals that either (a) the unbonder is not selling, (b) a countervailing catalyst has emerged, or (c) the thesis is wrong for this event. Do not average down.

### Take Profit (Optional Partial)
- If position is +5% in profit before release date, take 50% off the table. Let remaining 50% run through the release window.
- Rationale: Lock in gains from front-running while maintaining exposure to the actual release event.

### Funding Rate Override
- If cumulative funding paid exceeds 0.5% of notional while in the trade, close regardless of P&L. Funding drag can exceed the expected edge on smaller moves.

---

## Position Sizing

### Base Sizing
- **Per-trade risk:** 1.5% of total portfolio NAV.
- **Stop distance:** 8% from entry.
- **Position size formula:** `Size = (Portfolio NAV × 0.015) / 0.08`
- **Example:** $100,000 portfolio → risk $1,500 → position size = $1,500 / 0.08 = $18,750 notional.

### Leverage
- Maximum 3× leverage on Hyperliquid. At $18,750 notional with $100K portfolio, this is 0.19× portfolio — well within limits.
- Do not use leverage >3× regardless of conviction. Cosmos tokens are volatile; a short squeeze can move 20%+ in hours.

### Concentration Cap
- Maximum 2 concurrent positions in this strategy at any time.
- Maximum 30% of portfolio NAV allocated to this strategy in aggregate.

### Event Size Scaling
- Scale position size linearly with event size above threshold:
  - 0.3%–0.5% supply: 1.0× base size
  - 0.5%–1.0% supply: 1.25× base size
  - >1.0% supply: 1.5× base size (hard cap)

---

## Backtest Methodology

### Data Collection

**Step 1 — Unbonding event database**
- Source: Mintscan API (`https://api.mintscan.io/v1/{chain}/staking/unbonding`) and Numia Data (`https://docs.numia.xyz`) for historical unbonding transactions.
- Pull all undelegate transactions for ATOM (from genesis ~2019), TIA (from launch Oct 2023), INJ (from mainnet 2020).
- Filter: single-address events ≥0.3% circulating supply at time of event.
- Calculate release timestamp for each event.
- Store: `{chain, address, undelegate_block, undelegate_timestamp, amount, pct_supply, release_timestamp}`.

**Step 2 — Price data**
- Source: CoinGecko historical OHLCV (`https://api.coingecko.com/api/v3/coins/{id}/ohlcv`) at 1h granularity.
- Align price series to unbonding event timestamps.

**Step 3 — Funding rate data**
- Source: Hyperliquid historical funding (`https://api.hyperliquid.xyz/info` — `fundingHistory` endpoint).
- Note: Hyperliquid launched ~2023; for earlier ATOM events, use Binance perpetual funding history as proxy.

### Backtest Logic

For each qualifying event:
1. Record entry price = close of the 4h candle after detection.
2. Simulate short position held until T+48h post-release OR stop hit OR take-profit triggered.
3. Apply funding rate costs (sum of 8h funding rates during holding period).
4. Apply estimated slippage: 0.10% entry + 0.10% exit (conservative for $20K notional).
5. Record: gross P&L, net P&L (after funding + slippage), holding period, max adverse excursion.

### Metrics to Report
- Win rate (% of trades profitable net of costs)
- Average net P&L per trade (in %)
- Sharpe ratio (annualised, using trade-level returns)
- Maximum drawdown (consecutive losing trades)
- Average holding period
- P&L breakdown: pre-release vs. post-release (to identify where the edge concentrates)
- Subgroup analysis: by chain, by event size bucket, by market regime (bull/bear/sideways)

### Minimum Sample Size
- Require ≥30 qualifying events per chain before drawing conclusions. If fewer exist, treat as insufficient data and do not go live on that chain.

### Benchmark
- Compare against: (a) random short entry same duration, (b) short entered at release date (not before). If our pre-release entry does not outperform random, the front-running thesis is wrong.

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Backtest net Sharpe ≥ 1.0** across all qualifying events (not cherry-picked subset).
2. **Win rate ≥ 52%** net of costs (edge must be positive expectancy, not just a few large winners).
3. **Average net P&L per trade ≥ +1.5%** (must exceed realistic transaction costs with margin).
4. **No single chain drives >70% of total backtest P&L** (diversification of mechanism, not concentration).
5. **Manual review of 10 most recent qualifying events** to confirm on-chain data pipeline is working correctly and events are being detected within the 4-hour entry window.
6. **Paper trade for minimum 4 qualifying events** with simulated fills before live capital. Paper trade must show ≥2 of 4 events profitable.

---

## Kill Criteria

Suspend strategy immediately if any of the following occur:

1. **Live drawdown exceeds 10% of strategy allocation** (not portfolio NAV — strategy-level drawdown).
2. **5 consecutive losing trades** in live trading.
3. **Win rate drops below 40%** over any rolling 20-trade window in live trading.
4. **Mintscan/Numia API becomes unreliable** or introduces data delays >6 hours (destroys entry timing).
5. **Hyperliquid removes perpetual** for a chain we are trading (liquidity risk).
6. **A Cosmos SDK upgrade changes unbonding mechanics** (e.g., introduces instant unbonding or liquid staking at protocol level that eliminates the supply shock dynamic).
7. **Funding rates structurally negative** (market persistently short) — means the edge is crowded and cost of carry eliminates profit.

---

## Risks

### Primary Risks

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Unbonder does not sell (redelegates or holds) | High | Medium | Stop loss at 8%; position sizing limits loss |
| Countervailing catalyst (airdrop, partnership) | High | Low-Medium | Pre-trade catalyst check (Rule 4 in entry) |
| Funding rate drag exceeds edge | Medium | Medium | Funding rate filter at entry; funding override exit |
| On-chain data delay (miss entry window) | Medium | Low | Monitor multiple sources; set automated alerts |
| Short squeeze / low liquidity | High | Low | Leverage cap at 3×; liquidity filter ($10M daily vol) |
| Strategy crowding (others front-run the front-run) | Medium | Medium | Monitor: if entry-to-release drift disappears in backtest, edge is gone |

### Structural Risk (Most Important)
The rise of **liquid staking derivatives** (stATOM, milkTIA, etc.) partially neutralises this edge. If a large holder unbonds via a liquid staking protocol rather than directly, the protocol itself absorbs the sell pressure through its own treasury management. **Monitor LST market share per chain** — if LST TVL exceeds 40% of total staked supply, the direct unbonding signal becomes noisier and position sizes should be reduced by 50%.

### Tail Risk
A governance proposal to reduce unbonding period (e.g., ATOM governance has discussed 7-day unbonding) would shrink the front-running window. Monitor governance forums (`https://forum.cosmos.network`) for relevant proposals.

---

## Data Sources

| Source | URL | Use |
|--------|-----|-----|
| Mintscan Explorer | `https://mintscan.io` | Manual event monitoring |
| Mintscan API | `https://api.mintscan.io/v1/{chain}/staking/unbonding` | Programmatic unbonding data |
| Numia Data | `https://docs.numia.xyz` | Historical Cosmos chain data warehouse |
| BigDipper Explorer | `https://bigdipper.live` | Cross-chain backup explorer |
| CoinGecko OHLCV | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` | Historical price data |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | Funding rates, OI, trade execution |
| Binance Futures API | `https://fapi.binance.com/fapi/v1/fundingRate` | Historical funding (pre-Hyperliquid) |
| Cosmos Hub Governance | `https://forum.cosmos.network` | Monitor unbonding period proposals |
| Smart Stake Analytics | `https://smartstake.io` | Validator-level unbonding dashboards |

---

## Implementation Notes

### Detection Pipeline (Minimum Viable)
1. Set up a cron job (every 30 minutes) querying Mintscan API for new undelegate transactions on ATOM, TIA, INJ.
2. Filter for events ≥0.3% circulating supply.
3. Calculate release timestamp.
4. If release is 3–5 days out AND all entry conditions met → send alert to Telegram/Slack with: token, amount, % supply, release timestamp, suggested entry price range.
5. Human reviews alert and executes manually on Hyperliquid.

This is intentionally manual at first. Automate only after go-live criteria are met and the pipeline has been validated over ≥10 live events.

### Known Data Gap
Mintscan does not expose a clean "top unbonding events by size" API endpoint as of early 2026. The backtest data collection step will likely require querying raw transaction data from Numia's SQL interface or running a local node archive query. Budget 2–3 days of engineering time for data collection before backtest can begin.

---

*Next step: Assign to data engineer to build unbonding event database (Step 3 of 9). Target: backtest results within 3 weeks.*
