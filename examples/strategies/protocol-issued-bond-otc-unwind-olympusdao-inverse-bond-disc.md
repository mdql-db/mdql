---
title: "Protocol Bond Vesting Cliff Short (OlympusDAO-Style Discount Bond Convergence)"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 2
composite: 360
categories:
  - token-supply
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large tranche of OlympusDAO-style discount bonds approaches its vesting cliff, bond recipients who are in-profit (spot price > their effective cost basis) will sell the delivered tokens to realise the arbitrage gain. This selling is not behavioural — it is the *completion of the trade they explicitly entered*. Bond buyers are yield arbitrageurs who purchased a discounted forward delivery of the protocol token; their exit at vesting is the mechanical close of a known position. The causal chain:

1. Bond buyer pays asset (e.g. DAI, LP token) to protocol at T=0
2. Protocol commits to deliver X tokens at a discount D% to spot at T=0, vesting linearly or cliff-style over 5–7 days
3. Buyer's breakeven = `spot_at_purchase × (1 - D)` — this is fixed and on-chain
4. At vesting cliff T+N, if `spot_current > breakeven`, buyer is in profit and has strong incentive to sell immediately (no carry, no yield on held OHM, opportunity cost of capital)
5. Aggregate sell pressure = `Σ(bond_notional_in_profit)` across all vesting tranches in the window
6. This sell pressure is predictable 24h in advance from on-chain state alone

**Null hypothesis to disprove:** Vesting cliffs with >$500K in-profit bond notional produce no statistically significant negative return in the 24h window around the cliff.

---

## Structural Mechanism — WHY This Must Happen

This is not "tends to happen." The mechanism has three structural layers:

**Layer 1 — Contractual delivery is guaranteed.** The bond contract will deliver tokens at the vesting timestamp regardless of market conditions. There is no discretion on the protocol side. The token delivery is as guaranteed as a token unlock.

**Layer 2 — Recipient incentive is unambiguous.** Bond buyers are explicitly arbitrageurs, not long-term holders. They entered the bond to capture the discount. Unlike airdrop recipients (mixed motivations) or team token recipients (reputational constraints on selling), bond buyers have a single, stated objective: receive tokens at below-market price and sell. This makes their behaviour more predictable than standard unlock recipients.

**Layer 3 — No early exit mechanism.** Standard OlympusDAO bond implementations (V1, V2, and most forks) do not allow early redemption or secondary market transfer of the bond position. The buyer is locked in. This means all selling is concentrated at the vesting cliff rather than distributed — creating a sharper, more tradeable pressure event compared to linear vesting schedules.

**Why the discount/premium indicator adds signal:** The bond discount tells you the *minimum* price move required to make recipients indifferent to selling. If spot is 15% above breakeven, recipients have a 15% profit motive to sell. If spot is 5% below breakeven, recipients are underwater and selling is already partially priced in (they may have hedged or the market has already discounted the overhang). This gives a directional filter absent from standard token unlock strategies.

---

## Entry Rules


### Signal Construction

For each active bond contract on a protocol with a Hyperliquid-listed perp:

```
breakeven_price(bond_i) = spot_at_issuance × (1 - discount_rate_i)
bond_in_profit(bond_i) = 1 if spot_current > breakeven_price(bond_i)
notional_at_risk = Σ [ bond_tokens_i × spot_current ] for all bond_i where:
    - vesting_cliff_i is within [T+4h, T+28h]
    - bond_in_profit(bond_i) == 1
```

### Entry Conditions (ALL must be true)

1. `notional_at_risk > $500,000` (aggregate in-profit bonds vesting in next 24h)
2. `spot_current > breakeven_price` for the dominant tranche (largest single bond cohort)
3. `spot_current / breakeven_price > 1.03` — spot is at least 3% above breakeven (profit motive is real, not noise)
4. Hyperliquid perp for the token exists with >$1M open interest (liquidity check)
5. No concurrent positive catalyst in the next 48h (governance vote, major partnership announcement — manual check)

### Entry Execution

- **Instrument:** Hyperliquid perpetual future (short)
- **Entry timing:** T-20h before the vesting cliff (allows position to be established before any front-running, captures the full pre-cliff drift)
- **Entry price:** Market order or limit within 0.3% of mid — do not chase
- **Leverage:** 2x maximum (this is a probabilistic, not guaranteed, outcome)

## Exit Rules

### Exit Rules (first trigger wins)

| Condition | Action |
|-----------|--------|
| T+4h after vesting cliff | Close 100% at market |
| Adverse move ≥5% from entry | Stop loss, close 100% |
| Spot drops >8% below breakeven before cliff | Close 50% (thesis partially invalidated — recipients now underwater, sell pressure reduced) |
| Open interest on perp drops >40% (liquidity risk) | Close 100% |

### Position Sizing

See dedicated section below.

---

## Position Sizing

**Base size:** 1% of total portfolio per trade.

**Scaling rule:** Scale linearly from 0.5% to 2% based on the "profit overhang ratio":

```
profit_overhang_ratio = notional_at_risk / 30d_avg_daily_volume(token)
```

| Ratio | Position Size |
|-------|--------------|
| <0.05 | 0.5% (small relative to volume, limited impact) |
| 0.05–0.15 | 1.0% (base case) |
| 0.15–0.30 | 1.5% |
| >0.30 | 2.0% (large overhang relative to liquidity — high conviction) |

**Hard cap:** Never exceed 2% of portfolio on a single bond vesting event. Never hold more than 4% total across concurrent bond vesting shorts (protocol correlation risk).

**Funding cost adjustment:** Check Hyperliquid funding rate before entry. If annualised funding rate for the short side exceeds 50% (i.e., you pay >0.14% per 8h), reduce position size by 50% or skip — funding cost will erode edge over the 28h hold period.

---

## Backtest Methodology

### Data Sources

| Data | Source | Notes |
|------|--------|-------|
| OHM bond issuance events | The Graph — OlympusDAO subgraph (`https://api.thegraph.com/subgraphs/name/olympusdao/olympus-protocol-metrics`) | Query `BondCreated` events |
| Bond vesting timestamps | Etherscan — OlympusDAO Depository contract (`0x9025046c6fb25Fb39e720d97a8FD881ED69a1Ef6`) | Parse `BondCreated`, `BondRedeemed` logs |
| OHM spot price history | CoinGecko API (`https://api.coingecko.com/api/v3/coins/olympus/market_chart`) | Free tier, hourly OHLCV |
| OHM perp price/OI | Hyperliquid public API (`https://api.hyperliquid.xyz/info`) | `candleSnapshot` endpoint, hourly |
| Funding rates | Hyperliquid API — `fundingHistory` endpoint | 8h intervals |
| Bond discount rates | On-chain from Depository contract `bonds()` mapping | Requires Etherscan API or direct RPC call |

### Backtest Period

- **Primary:** January 2021 – December 2022 (peak OlympusDAO bond activity)
- **Secondary:** January 2023 – present (low activity, tests null hypothesis — should show no signal when mechanism is dormant)

### Event Identification

1. Pull all `BondCreated` events from Depository contract
2. For each event: record `(issuance_timestamp, vesting_cliff_timestamp, discount_rate, token_amount, payout_token)`
3. Group by vesting cliff date (bonds issued within same 24h window often share cliff)
4. At each cliff: compute `notional_at_risk` using spot price 24h before cliff
5. Apply entry filters — record which events qualify

### Metrics to Compute

For each qualifying event:

```
entry_price = OHM spot at T-20h before cliff
exit_price = OHM spot at T+4h after cliff (primary exit)
raw_return = (entry_price - exit_price) / entry_price  [short position]
funding_cost = Σ(8h_funding_rate) over hold period
net_return = raw_return - funding_cost - 0.05% (estimated slippage each way)
```

### Aggregate Statistics Required

- Win rate (% of qualifying events with positive net return)
- Mean net return per event
- Median net return per event
- Sharpe ratio (annualised, using event returns)
- Maximum drawdown across all events
- Return stratified by `profit_overhang_ratio` bucket
- Return stratified by `spot/breakeven ratio` at entry (3–5%, 5–10%, >10%)
- Comparison baseline: random 28h short windows in same period (same instrument, no signal)

### What the Backtest Must Show

See Go-Live Criteria below.

---

## Go-Live Criteria

The following thresholds must ALL be met before moving to paper trading:

| Metric | Minimum Threshold |
|--------|------------------|
| Number of qualifying events | ≥ 20 (statistical minimum) |
| Win rate | ≥ 55% |
| Mean net return per event | ≥ +0.8% (after funding + slippage) |
| Sharpe ratio (event-based) | ≥ 0.8 |
| Win rate vs. random baseline | Must exceed baseline by ≥ 15 percentage points |
| Max single-event loss | ≤ 8% (confirms stop loss is functioning) |
| Positive return in ≥ 2 of 3 profit_overhang_ratio buckets | Required (confirms scaling logic) |

If fewer than 20 qualifying events exist in the historical dataset, the strategy cannot be statistically validated on OHM alone. In this case, expand to OHM forks (Wonderland TIME, Klima, Spartacus) and re-run. If still <20 events, flag as **insufficient data — monitor only**.

---

## Kill Criteria

Abandon the strategy (stop paper trading, do not go live) if any of the following occur:

1. **Backtest fails go-live criteria** — any single threshold not met
2. **Mechanism is dead:** Fewer than 2 qualifying events per quarter across all monitored protocols for 2 consecutive quarters
3. **Paper trading underperformance:** After 10 paper trades, win rate <45% or mean return <0%
4. **Structural change:** A major protocol adopts transferable bond positions or early redemption (removes the concentration-at-cliff mechanic)
5. **Liquidity collapse:** Hyperliquid OI for relevant perps drops below $500K (cannot size meaningfully)
6. **Funding rate regime change:** Average funding cost for short positions exceeds 100% annualised for >30 days (carry destroys edge)

---

## Risks

### High Severity

**Mechanism is largely legacy (2021–2022).** OlympusDAO-style bonds peaked in 2021–2022. As of 2024–2025, active bond programs are rare. The opportunity set may be near-zero until a new protocol revives the mechanic. This is the primary risk — not that the strategy is wrong, but that there are no trades to take.

**OHM perp liquidity on Hyperliquid is thin.** OHM may not have sufficient OI to absorb even a 1% portfolio position without significant slippage. Verify current OI before any live deployment.

### Medium Severity

**Bond buyers may hedge during vesting.** Sophisticated arbitrageurs may short OHM on-chain or via perps during the vesting period, front-running the cliff. If this is widespread, the sell pressure at the cliff is already priced in by T-20h. The backtest will reveal whether T-20h entry still captures alpha or whether earlier entry is needed.

**Protocol buybacks can offset sell pressure.** OlympusDAO's treasury has historically conducted buybacks (RFV floor defense). A buyback concurrent with a vesting cliff would neutralise or reverse the short. Monitor governance forums for buyback signals.

**Discount rate may not reflect true cost basis.** If the bond was purchased with an LP token (not stablecoin), the "breakeven" calculation requires tracking the LP token's value at issuance, not just the discount rate. This adds data complexity and potential calculation error.

**Small sample size.** Even in peak periods, qualifying events (>$500K in-profit notional) may be rare. The strategy may be statistically unvalidatable on historical data alone.

### Low Severity

**Fork protocol data availability.** Wonderland, Klima, and other forks have less reliable subgraph data. Manual contract parsing may be required.

**Regulatory/protocol risk.** A protocol could pause bond redemptions via emergency governance. Unlikely but non-zero.

---

## Data Sources

```
# On-chain bond data
OlympusDAO Depository V1: 0x9025046c6fb25Fb39e720d97a8FD881ED69a1Ef6 (Ethereum)
OlympusDAO Depository V2: 0x007F7735baF391e207E3aA380bb53c4Bd9a5Fed1 (Ethereum)
Etherscan API: https://api.etherscan.io/api?module=logs&action=getLogs&address=<contract>
The Graph OlympusDAO: https://api.thegraph.com/subgraphs/name/olympusdao/olympus-protocol-metrics

# Price data
CoinGecko OHM: https://api.coingecko.com/api/v3/coins/olympus/market_chart?vs_currency=usd&days=max&interval=hourly
CoinGecko TIME: https://api.coingecko.com/api/v3/coins/wonderland/market_chart?vs_currency=usd&days=max&interval=hourly
CoinGecko KLIMA: https://api.coingecko.com/api/v3/coins/klima-dao/market_chart?vs_currency=usd&days=max&interval=hourly

# Hyperliquid perp data
Candles: POST https://api.hyperliquid.xyz/info {"type": "candleSnapshot", "req": {"coin": "OHM", "interval": "1h", "startTime": <unix_ms>, "endTime": <unix_ms>}}
Funding history: POST https://api.hyperliquid.xyz/info {"type": "fundingHistory", "coin": "OHM", "startTime": <unix_ms>}
Open interest: POST https://api.hyperliquid.xyz/info {"type": "metaAndAssetCtxs"}

# Fork monitoring (manual check quarterly)
Berachain bonds: Monitor https://app.berachain.com (no subgraph yet as of writing)
New protocol bonds: Monitor https://dune.com (search "bond protocol" dashboards)
Bond Protocol (generic bond infrastructure): https://app.bondprotocol.finance — check active markets
```

**Monitoring cadence:** Run on-chain scanner daily. Bond vesting cliffs are deterministic from issuance — build a calendar from `BondCreated` events and alert at T-48h for any qualifying cliff.

---

## Implementation Notes

**Minimum viable monitor:** A Python script that queries the Etherscan log API for `BondCreated` events daily, computes vesting cliffs, checks spot price against breakeven, and sends an alert if `notional_at_risk > $500K` within 24h. This is a ~200 line script, not a complex system.

**Expansion trigger:** If Bond Protocol (`bondprotocol.finance`) or a similar generic bond infrastructure gains traction with new protocols, re-evaluate the opportunity set. The mechanism is protocol-agnostic — any protocol issuing discount bonds with cliff vesting is a candidate.

**Cross-strategy note:** This strategy is structurally identical to the token unlock short (Zunid's core strategy). If the token unlock short is already live, the marginal infrastructure cost to add bond vesting monitoring is low. Treat as an extension of the same system, not a separate build.
