---
title: "Sui/Aptos New Perp Listing Funding Squeeze"
status: HYPOTHESIS
mechanism: 6
implementation: 3
safety: 5
frequency: 3
composite: 270
categories:
  - funding-rates
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

New perpetual futures listings on immature DEX venues (Bluefin on Sui, Merkle Trade on Aptos) attract directional retail long flow before market makers establish balanced open interest. This creates mechanically elevated funding rates that must be paid from longs to shorts. The edge is not that funding *tends* to be elevated — it is that the structural immaturity of the venue (thin MM competition, no systematic arbitrage bots, no cross-venue hedging infrastructure) means the imbalance persists for days rather than hours. Collecting funding while delta-neutral (short perp + long spot hedge) converts this structural imbalance into a carry trade with a defined decay curve.

**Null hypothesis to disprove:** Funding rates on new Bluefin/Merkle listings are not systematically elevated relative to Hyperliquid equivalents in the first 7 days post-listing, or the elevated funding is offset by execution costs and bridging friction.

---

## Structural Mechanism

**Why this must happen (not just tends to happen):**

1. **Retail flow is directionally biased at listing.** New token perp listings are marketing events. Retail participants arrive to express bullish views. There is no equivalent structural force creating short interest at listing — shorts require conviction against the narrative, while longs are momentum-driven.

2. **Market maker bootstrapping lag.** Professional MMs on Bluefin and Merkle Trade face higher operational overhead than on Hyperliquid: separate RPC infrastructure for Sui/Aptos, separate smart contract integrations, separate risk systems. This means the first 24–72 hours of a new listing have fewer competing market makers absorbing the OI imbalance.

3. **Funding rate is a mechanical transfer, not a prediction.** When OI is long-skewed, the funding mechanism *contractually* transfers payment from longs to shorts every 8 hours. This is not probabilistic — it is a protocol rule. The only uncertainty is how long the imbalance persists, not whether it pays while it exists.

4. **No cross-venue arbitrage bots.** Hyperliquid funding arb bots are well-documented. Bluefin and Merkle Trade have no equivalent systematic monitoring infrastructure. The friction of bridging capital to Sui/Aptos creates a moat that keeps the opportunity open longer.

5. **Venue immaturity amplifies the effect.** With <$50M daily volume on Bluefin and <$20M on Merkle Trade, even modest retail flow creates large OI skew. A $500K net long position on Merkle Trade represents 2.5% of daily volume — enough to push funding to extreme levels.

**Why it decays:** As funding rates remain elevated, rational capital eventually bridges in to collect the carry. The decay curve is the exit signal.

---

## Entry Rules

### Signal Detection (must satisfy ALL conditions)

| Condition | Threshold | Rationale |
|-----------|-----------|-----------|
| Listing age | ≤ 7 days since first trade on venue | Imbalance is freshest; MM competition lowest |
| Funding rate | > 0.5% per 8h (annualised: >547%) | Filters noise; covers execution costs with margin |
| Funding persistence | ≥ 2 consecutive 8h periods above threshold | Eliminates single-period spikes from liquidation cascades |
| Spot hedge availability | Spot or perp on Hyperliquid/Binance exists for hedge leg | Delta-neutral structure is non-negotiable |
| OI concentration check | Strategy size ≤ 5% of venue OI | Prevents self-defeating market impact |
| Smart contract age | Protocol deployed > 90 days on mainnet | Filters highest-risk new protocol deployments |

### Entry Execution

1. **Leg 1 (short perp):** Enter short on Bluefin or Merkle Trade at market. Use limit orders within 0.1% of mid to reduce slippage. Do not chase — if fill requires >0.2% slippage, abort.
2. **Leg 2 (long hedge):** Simultaneously enter long spot or long perp on Hyperliquid (preferred) or Binance. Match notional value of Leg 1 exactly. Hyperliquid perp is preferred over spot because it avoids bridging capital to Sui/Aptos for the hedge.
3. **Timing:** Enter within 30 minutes of signal confirmation. Funding accrues at fixed 8h intervals — entering just after a funding settlement maximises time-in-trade for the next payment.
4. **Record entry funding rate, entry OI, and entry timestamp** for exit trigger calculation.

---

## Exit Rules

### Primary Exit Triggers (first condition met closes position)

| Trigger | Action | Rationale |
|---------|--------|-----------|
| Funding rate < 0.2% per 8h for 2 consecutive periods | Close both legs at market | Carry no longer compensates for execution risk |
| 7-day time stop | Close both legs at market | Structural imbalance has had time to normalise; tail risk increases |
| Venue OI drops > 40% in 24h | Close both legs immediately | Signals mass exit; liquidity deteriorating |
| Spot-perp basis on hedge leg > 0.5% adverse | Close both legs | Hedge leg is bleeding; net carry is negative |
| Smart contract exploit or protocol pause | Close both legs immediately | Binary risk event |

### Exit Execution

1. Close Leg 1 (short perp) first to eliminate directional exposure.
2. Close Leg 2 (long hedge) within 5 minutes of Leg 1 close.
3. Use limit orders with 10-minute patience before switching to market orders.
4. Do not hold a single leg overnight — partial fills must be resolved same session.

---

## Position Sizing

### Hard Limits

- **Maximum per trade:** 2% of total portfolio NAV
- **Maximum concurrent Sui/Aptos exposure:** 6% of total portfolio NAV (3 simultaneous positions maximum)
- **Maximum single-venue exposure:** 4% of portfolio NAV (concentration risk on one smart contract)
- **OI cap:** 5% of venue OI at entry — recalculate at each 8h funding period; reduce if OI has shrunk

### Sizing Formula

```
Position size = MIN(
  0.02 × Portfolio NAV,
  0.05 × Venue OI at entry,
  Liquidity limit: max size where entry slippage < 0.15%
)
```

### Expected Return Calculation (pre-trade filter)

```
Expected gross carry = Funding rate (per 8h) × Expected holding periods
Expected holding periods = 7 days ÷ 8h = 21 periods (maximum)
Expected net carry = Gross carry − Entry slippage − Exit slippage − Bridge fees − Hedge leg funding cost
Minimum acceptable: Net carry > 3% over expected hold period
```

If expected net carry < 3%, do not enter regardless of signal.

---

## Backtest Methodology

### Data Collection

**Bluefin (Sui):**
- Endpoint: `https://dapi.api.sui-prod.bluefin.io/fundingRate` — returns historical funding rates per market
- Collect: All markets listed since Bluefin mainnet launch (2023-Q3), funding rate every 8h, OI every 8h, 24h volume
- Script: Poll API every 8h, store in local Postgres. Free, no auth required.

**Merkle Trade (Aptos):**
- Source: Aptos RPC — query `MerkleTrade::FundingRate` events on-chain
- Node: Use public Aptos fullnode (`https://fullnode.mainnet.aptoslabs.com`) or Alchemy Aptos
- Collect: All historical funding rate events since Merkle Trade mainnet (2023-Q4)
- Fallback: Merkle Trade subgraph if available

**Hedge leg (Hyperliquid):**
- Source: Hyperliquid public API — historical funding rates and mark prices
- Match by asset ticker; note that not all Bluefin/Merkle assets trade on Hyperliquid

**Spot prices:**
- CoinGecko API (free tier) for assets not on major venues
- Binance historical klines for assets that trade there

### Backtest Logic

```python
# Pseudocode — implement in Python

for each new_listing in bluefin_listings + merkle_listings:
    listing_date = new_listing.first_trade_timestamp
    
    for each 8h_period in range(listing_date, listing_date + 7_days):
        funding_rate = get_funding_rate(new_listing.asset, period)
        
        if funding_rate > 0.005:  # 0.5% per 8h
            consecutive_count += 1
        else:
            consecutive_count = 0
        
        if consecutive_count >= 2:
            # Entry signal
            entry_price = get_mark_price(new_listing.asset, period.end)
            hedge_price = get_hedge_price(new_listing.asset, period.end)
            
            # Simulate carry collection
            for each subsequent_period:
                collect funding_rate × position_size
                deduct hedge_leg_funding_cost
                
                if exit_condition_met:
                    record_pnl(carry_collected - entry_slippage - exit_slippage)
                    break
```

### Backtest Metrics to Report

| Metric | Minimum acceptable | Target |
|--------|--------------------|--------|
| Number of qualifying events | ≥ 20 | ≥ 40 |
| Win rate (net positive carry) | > 65% | > 75% |
| Average net carry per trade | > 3% | > 6% |
| Max drawdown per trade | < 5% | < 3% |
| Sharpe ratio (annualised) | > 1.5 | > 2.5 |
| Average holding period | Report actual | — |

### Known Backtest Limitations

1. **Slippage is estimated, not observed.** Historical order book depth is not available for Bluefin/Merkle. Use 0.2% round-trip as conservative estimate; sensitivity-test at 0.5%.
2. **Bridge fees are variable.** Use $15 flat per round-trip as estimate for Sui/Aptos bridging; verify against current Wormhole/LayerZero rates.
3. **Survivorship bias.** Some new listings may have been delisted — ensure data collection captures failed listings.
4. **OI data may be sparse.** If OI history is unavailable, use volume as proxy for sizing constraint.

---

## Paper Trading Protocol

### Duration
Minimum 30 days or 10 qualifying events, whichever comes later.

### Execution Simulation
- Log every signal in real time with timestamp
- Record what fill price would have been (use mark price + 0.15% slippage estimate)
- Record actual funding rates collected each 8h period
- Compare simulated PnL to theoretical PnL daily

### Paper Trade Tracking Sheet (minimum fields)

```
Date | Venue | Asset | Entry funding rate | Entry OI | Simulated entry price | 
Hedge venue | Hedge entry price | Periods held | Funding collected | 
Exit trigger | Simulated exit price | Net PnL | Notes
```

### Go-Live Criteria (ALL must be satisfied)

| Criterion | Threshold |
|-----------|-----------|
| Paper trade win rate | > 65% over ≥ 10 events |
| Paper trade average net carry | > 3% per trade |
| Backtest results confirmed | Paper trade within 30% of backtest expectation |
| Operational readiness | Automated signal detection running; manual execution checklist complete |
| Smart contract audit reviewed | Bluefin and Merkle Trade audits read; known risks documented |
| Bridge infrastructure tested | At least 2 live test bridges completed with small amounts |
| Legal/compliance check | Sui/Aptos DEX access confirmed for operating jurisdiction |

---

## Kill Criteria

**Immediate kill (same day):**
- Any smart contract exploit on Bluefin or Merkle Trade, regardless of whether current positions are affected
- Funding mechanism paused or modified by protocol governance
- Bridge used for hedging suffers exploit or extended downtime

**Strategy review (pause and reassess within 5 days):**
- 3 consecutive losing trades (net negative carry after costs)
- Average net carry drops below 1.5% per trade over trailing 10 trades
- Venue OI grows > 10x (strategy may become crowded; edge may compress)
- Competing systematic bots detected (evidence: funding rates normalise within 1–2 periods of listing rather than 2–7 days)

**Permanent kill:**
- Backtest shows < 55% win rate after full data collection
- Regulatory action against Sui/Aptos DEX access in operating jurisdiction
- Net carry consistently below cost of capital over 60-day live period

---

## Risk Register

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Smart contract exploit (Bluefin/Merkle) | Critical | Low-Medium | Hard cap 2% NAV per trade; audit review pre-go-live; monitor protocol security channels |
| Bridge exploit (Wormhole/LayerZero) | Critical | Low | Minimise time capital sits in bridge; use battle-tested bridges only; consider native Sui/Aptos spot as hedge instead |
| Liquidity crunch on exit | High | Medium | 5% OI cap; limit orders with patience; never market-order exit unless kill trigger |
| Hedge leg basis risk | Medium | Medium | Monitor spot-perp basis on hedge leg every 8h; exit if basis > 0.5% adverse |
| Funding rate manipulation | Medium | Low | Require 2 consecutive periods above threshold; large single-period spikes are excluded |
| Regulatory access loss | Medium | Low | Monitor Sui/Aptos regulatory status; maintain jurisdiction-specific access log |
| Protocol governance changes funding formula | Medium | Low | Monitor governance forums for Bluefin and Merkle Trade; subscribe to Discord/governance alerts |
| Counterparty risk (venue insolvency) | High | Very Low | Position size limits; diversify across Bluefin and Merkle rather than concentrating |
| Network congestion on Sui/Aptos | Low | Medium | Test execution during high-congestion periods in paper trading; set gas limits appropriately |

---

## Data Sources

| Data | Source | Access | Cost |
|------|--------|--------|------|
| Bluefin funding rates | `https://dapi.api.sui-prod.bluefin.io/fundingRate` | Public REST API | Free |
| Bluefin OI and volume | `https://dapi.api.sui-prod.bluefin.io/marketData` | Public REST API | Free |
| Merkle Trade funding rates | Aptos RPC — `MerkleTrade` module events | Public RPC | Free |
| Aptos RPC | `https://fullnode.mainnet.aptoslabs.com` or Alchemy | Public / Alchemy free tier | Free / $0 |
| Hyperliquid funding rates (hedge) | `https://api.hyperliquid.xyz/info` | Public REST API | Free |
| Hyperliquid mark prices | Same endpoint | Public REST API | Free |
| Spot prices | CoinGecko API v3 | Public | Free (rate limited) |
| Binance klines (hedge prices) | `https://api.binance.com/api/v3/klines` | Public REST API | Free |
| Bridge fee estimates | Wormhole/LayerZero documentation | Public | Free |
| Protocol audit reports | Bluefin docs, Merkle Trade docs, Certik/Ottersec | Public | Free |
| Governance alerts | Bluefin Discord, Merkle Trade Discord | Manual monitoring | Free |

---

## Differentiation from Hyperliquid New Listing Strategy

| Dimension | Hyperliquid Version | Sui/Aptos Version |
|-----------|--------------------|--------------------|
| Funding rate magnitude | 0.3–0.5% per 8h | 1–3% per 8h (hypothesis) |
| Competition | Moderate (bots exist) | Low (no systematic bots observed) |
| Execution complexity | Low | High (bridging, multi-chain ops) |
| Smart contract risk | Low (Hyperliquid audited, battle-tested) | Medium (newer protocols) |
| Hedge leg availability | Easy (same venue or Binance) | Medium (asset may not trade elsewhere) |
| Expected edge duration | 1–3 days | 3–7 days (hypothesis) |
| Position size ceiling | Higher (deeper liquidity) | Lower (thin markets) |
| Recommended allocation | Core satellite | Satellite only |

**Operational dependency:** This strategy should only be activated after the Hyperliquid new listing strategy is live and operational. The Sui/Aptos version shares the same conceptual framework but requires separate infrastructure, separate risk monitoring, and separate smart contract risk tolerance. Do not run both simultaneously until each is independently validated.

---

## Next Steps (ordered)

1. **Week 1:** Write data collection scripts for Bluefin API and Merkle Trade on-chain events. Store 90 days of historical funding rate data minimum.
2. **Week 2:** Run backtest on all qualifying historical listings. Report metrics against thresholds above.
3. **Week 3:** If backtest passes, begin paper trading with real-time signal detection.
4. **Week 4–6:** Paper trade for minimum 30 days. Log every signal regardless of whether it meets entry criteria.
5. **Week 7:** Review paper trade results against go-live criteria. Decision: go live, extend paper trading, or kill.
6. **Ongoing:** Monitor governance forums and security channels for both protocols weekly.
