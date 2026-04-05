---
title: "MakerDAO Surplus Buffer Auction Front-Run (MKR Buyback & Burn)"
status: HYPOTHESIS
mechanism: 6
implementation: 6
safety: 6
frequency: 2
composite: 432
categories:
  - defi-protocol
  - governance
  - token-supply
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When MakerDAO's surplus buffer (`Vow.dai - Vow.Sin`) approaches the `hump` threshold, a programmatic MKR buyback-and-burn auction (`flap`) becomes contractually imminent. This creates a predictable, on-chain-readable buy pressure event. The hypothesis is that MKR price appreciates in the window between the buffer crossing 95% of `hump` and the first `flap` auction clearing, because:

1. The buy flow is non-discretionary — the smart contract will auction DAI for MKR regardless of market conditions
2. The event is observable on-chain before it executes, creating an information window
3. MKR supply permanently decreases post-burn, which is a hard mechanical fact

**Causal chain:**

```
Vow.dai - Vow.Sin > hump
        ↓
flap() becomes callable by any keeper
        ↓
Protocol sells DAI, buys MKR at market
        ↓
MKR burned → permanent supply reduction
        ↓
Rational anticipation of buy pressure → price appreciation pre-auction
```

The edge, if it exists, lives in the gap between the buffer crossing the threshold and the market fully pricing in the imminent auction. This gap may be minutes, hours, or days depending on keeper latency and market attention.

---

## Structural Mechanism (Why This MUST Happen)

The `flap` auction is enforced by the `Vow` contract on Ethereum mainnet (`0xA950524441892A31ebddF91d3cEEFa04Bf454466`). The relevant invariant:

```solidity
function flap() external returns (uint id) {
    require(dai(address(this)) >= add(add(Sin(), bump()), hump()), "Vow/insufficient-surplus");
    ...
}
```

- `hump`: minimum surplus buffer that must remain after auction (currently 50M DAI, set by governance)
- `bump`: lot size per auction (currently 30,000 DAI per flap)
- `Sin`: total bad debt queued for healing

**What is guaranteed:**
- Once `Vow.dai - Vow.Sin >= hump + bump`, any Ethereum address can call `flap()` and trigger an auction
- The auction sells exactly `bump` DAI for MKR via a decreasing-price Dutch auction
- Winning MKR is sent to `address(0)` — burned permanently
- This cannot be stopped by governance in the short term without an emergency shutdown (which has its own observable signals)

**What is NOT guaranteed:**
- The price impact of the auction on MKR spot/perp markets
- That the market hasn't already priced in the imminent auction
- The timing between buffer crossing threshold and keeper calling `flap()`

**Current parameters (verify before trading):**
- `hump`: 50,000,000 DAI (50M)
- `bump`: 30,000 DAI per auction lot
- Auction duration: 30 minutes per lot
- Source: [Makerburn.com](https://makerburn.com/#/runrate) or direct RPC call

---

## Entry / Exit Rules

### Entry Signal

**Condition:** `(Vow.dai - Vow.Sin) / hump >= 0.95` AND the ratio has been increasing over the prior 7 days (buffer trending toward threshold, not retreating)

**Confirmation check:**
- No active `Sin` spike in the last 48 hours (bad debt event would reset buffer)
- Governance has not submitted a `hump` increase proposal that is within 48 hours of execution (check [Maker governance portal](https://vote.makerdao.com/))
- MKR perp funding rate is not already extreme positive (>0.1% per 8h) — would indicate market already positioned

**Instrument:** MKR/USDC perp on Hyperliquid OR MKR spot on any liquid venue

**Entry:** Market order at next candle open after all conditions confirmed. Do not chase intraday.

### Exit Signal (take profit / event completion)

**Primary exit:** First `flap` auction clears on-chain (monitor `Vow` contract for `Flap` event log). Exit at market open of next candle after confirmation.

**Secondary exit (time stop):** If no `flap` auction fires within 14 calendar days of entry, exit regardless. Buffer may be stalling due to protocol revenue slowdown.

### Stop Loss

**Hard stop:** Buffer drops below 85% of `hump` after entry (bad debt event or governance `hump` raise has reset the setup). Exit immediately at market.

**Governance stop:** Any on-chain signal of Emergency Shutdown Module activation (`ESM.sum > ESM.min`). Exit immediately.

**Funding stop (perp only):** If 8h funding rate exceeds +0.15% for two consecutive periods, exit — the trade is crowded and carry cost destroys the edge.

---

## Position Sizing

- **Base allocation:** 1% of portfolio per trade
- **Maximum allocation:** 2% (never pyramid into this — it's a single event trade)
- **Rationale for small size:** MKR market cap ~$1.5B at time of writing. Each `flap` auction burns ~30,000 DAI worth of MKR. Even 100 consecutive auctions = $3M of buy pressure = ~0.2% of market cap. The mechanical buy pressure is small; the edge is purely in anticipation, not in the auction itself moving price materially.
- **Leverage:** 1x–2x maximum. This is not a high-conviction size-up trade.
- **Perp vs spot:** Prefer spot if funding is positive (you pay carry on perp). Use perp only if spot liquidity is insufficient or if you want defined leverage.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format |
|---|---|---|
| Historical `Vow.dai` and `Vow.Sin` values | Ethereum archive node or [The Graph — Maker subgraph](https://thegraph.com/hosted-service/subgraph/makerdao/maker-protocol) | Time-series, hourly |
| Historical `flap` auction timestamps and clearing prices | Maker subgraph query on `FlapAuction` entity OR [Makerburn API](https://makerburn.com/api/) | Event log with block timestamp |
| MKR/USD OHLCV | CoinGecko API, Binance historical data, or Kaiko | Daily and hourly candles |
| Historical `hump` parameter values | Maker governance forum + on-chain `LogNote` events on Vow contract | Point-in-time values |
| MKR perp funding rates (if testing perp variant) | Hyperliquid API or Binance historical funding | 8h intervals |

### Backtest Period

- **Start:** June 2020 (first `flap` auctions post-MCD launch)
- **End:** Present
- **Note:** The `hump` parameter has changed multiple times. Each backtest window must use the `hump` value that was active at that time, not the current value. Failure to do this will generate false signals.

### Signal Construction

For each historical date:
1. Compute `buffer_ratio = (Vow.dai - Vow.Sin) / hump` using the `hump` active at that date
2. Flag entry signal when `buffer_ratio >= 0.95` and 7-day trend is positive
3. Record entry price (MKR/USD close on signal date)
4. Record exit price (MKR/USD close on day of first subsequent `flap` auction)
5. Record time-stop exit if no auction within 14 days

### Metrics to Compute

- **Win rate:** % of trades where exit price > entry price
- **Average return per trade:** mean and median (log returns)
- **Average holding period:** days from entry to exit
- **Sharpe ratio:** annualised, using risk-free rate = 0 (crypto context)
- **Max drawdown per trade:** from entry to worst intra-trade price
- **False signal rate:** % of 95% crossings that never triggered a `flap` (buffer retreated)
- **Comparison baseline:** Buy-and-hold MKR over same periods (are we just capturing MKR beta?)

### Subgroup Analysis

- Segment by market regime: bull (BTC >200d MA), bear (BTC <200d MA)
- Segment by buffer approach speed: fast (>1% per day) vs slow (<0.5% per day)
- Segment by `bump` size relative to market cap at time of auction

---

## Go-Live Criteria

The backtest must show ALL of the following before moving to paper trading:

1. **Win rate ≥ 55%** across all historical signals (minimum 15 signals required for statistical validity — if fewer exist, this strategy cannot be validated statistically and should be held at hypothesis stage)
2. **Average return per trade ≥ 1.5%** (must exceed estimated transaction costs + slippage of ~0.3% round-trip)
3. **Sharpe ratio ≥ 0.8** annualised
4. **Alpha over MKR buy-and-hold:** Strategy returns must exceed a naive "always long MKR" benchmark over the same holding periods. If the strategy just captures MKR beta, it has no edge.
5. **False signal rate ≤ 30%:** No more than 30% of entry signals should result in the buffer retreating without a `flap` firing
6. **No single trade loss > 8%:** If any historical instance shows >8% drawdown from entry to stop, the stop rules need tightening before go-live

---

## Kill Criteria

Abandon the strategy (do not paper trade or go live) if ANY of the following are true:

1. **Fewer than 10 historical `flap` auction events** exist in the backtest period — insufficient sample for any conclusion
2. **Backtest win rate < 50%** — no edge over coin flip
3. **MakerDAO governance migrates to a new surplus mechanism** (e.g., Sky/Endgame protocol changes the `Vow` architecture — this is actively in progress as of 2025; verify current protocol state before backtesting)
4. **`flap` auctions have been deprecated** in the current protocol version (Sky/Endgame migration may replace `flap` with a different mechanism — check [sky.money](https://sky.money) documentation)
5. **Paper trading shows < 50% win rate over 5+ live signals** — live performance diverges from backtest

**Critical note on protocol migration:** MakerDAO is actively transitioning to "Sky" (formerly Endgame). The `Vow`/`flap` mechanism may be deprecated or replaced. Before building any backtest infrastructure, confirm that `flap` auctions are still active in the current deployed contracts. If deprecated, this strategy is dead and should be archived, not iterated on.

---

## Risks (Honest Assessment)

### High Severity

| Risk | Description | Mitigation |
|---|---|---|
| Protocol migration | Sky/Endgame may have already replaced `flap` auctions | Verify contract state before any work |
| `hump` governance raise | Governance can raise `hump` mid-setup, pushing the threshold further away | Check governance queue before entry |
| Bad debt spike | Large `Sin` event resets buffer to zero instantly | Monitor `Vow.Sin` daily; hard stop at 85% |
| Market already priced in | Makerburn.com is public; sophisticated participants monitor it continuously | Backtest will reveal if edge has been arbed away |

### Medium Severity

| Risk | Description | Mitigation |
|---|---|---|
| Small buy pressure | Each `flap` lot is only 30K DAI — tiny vs MKR market cap | Size position accordingly (1% portfolio) |
| Keeper latency variable | Time between buffer crossing and `flap()` call is unpredictable | Use 14-day time stop |
| Perp funding cost | If trade is crowded, positive funding erodes returns | Funding rate filter at entry; prefer spot |
| Liquidity | MKR perp on Hyperliquid may have wide spreads | Check OI and spread before entry; use limit orders |

### Low Severity

| Risk | Description | Mitigation |
|---|---|---|
| Oracle manipulation | Unlikely to affect `Vow` accounting | No mitigation needed |
| Smart contract bug | Theoretical | Not hedgeable; accept as tail risk |

---

## Data Sources

| Source | URL / Endpoint | What it provides |
|---|---|---|
| Makerburn | `https://makerburn.com/#/runrate` | Live surplus buffer, burn rate, historical charts |
| Makerburn API | `https://makerburn.com/api/mkr_burned` | Historical MKR burn events (verify endpoint availability) |
| The Graph — Maker | `https://thegraph.com/hosted-service/subgraph/makerdao/maker-protocol` | On-chain `Vow` state, `FlapAuction` events |
| Ethereum RPC (direct) | Any archive node (Alchemy, Infura, self-hosted) | Real-time `Vow.dai`, `Vow.Sin`, `hump`, `bump` via `eth_call` |
| Vow contract | `0xA950524441892A31ebddF91d3cEEFa04Bf454466` (Ethereum mainnet) | Source of truth for all buffer parameters |
| Maker governance | `https://vote.makerdao.com/` | Pending `hump` parameter changes |
| CoinGecko API | `https://api.coingecko.com/api/v3/coins/maker/market_chart` | MKR/USD historical OHLCV |
| Binance historical data | `https://data.binance.vision/` | MKR/USDT spot OHLCV (higher quality than CoinGecko) |
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | MKR perp funding rates, OI, mark price |
| Sky/Endgame docs | `https://sky.money` | Protocol migration status — check before starting |

### Key RPC Calls for Live Monitoring

```python
# Check current surplus buffer
vow = w3.eth.contract(address="0xA950524441892A31ebddF91d3cEEFa04Bf454466", abi=VOW_ABI)
dai_balance = vow.functions.dai().call()   # RAD units (divide by 1e45 for DAI)
sin_balance = vow.functions.Sin().call()   # RAD units
hump = vow.functions.hump().call()         # RAD units
bump = vow.functions.bump().call()         # RAD units

buffer_ratio = (dai_balance - sin_balance) / hump
# Entry signal: buffer_ratio >= 0.95
```

---

## Open Questions Before Backtest

1. **Is `flap` still active?** Confirm the current Sky/Endgame deployment still uses `Vow`/`flap` or identify the replacement mechanism.
2. **How many historical `flap` events exist?** Pull from The Graph before building infrastructure — if <15, statistical validation is impossible.
3. **What is the typical lead time?** How many hours/days does the buffer typically sit above 95% of `hump` before a keeper calls `flap()`? This determines whether there's a tradeable window at all.
4. **Is Makerburn monitoring widespread?** Survey crypto Twitter/Discord for evidence that this signal is widely watched — if it is, the edge may already be fully arbed.
