---
title: "Aave Health Factor 1.0 Cliff — Pre-Liquidation Position Mapping Short"
status: HYPOTHESIS
mechanism: 7
implementation: 5
safety: 6
frequency: 5
composite: 1050
categories:
  - liquidation
  - defi-protocol
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When ETH or BTC spot price approaches a price level at which a large cluster of Aave borrowing positions reaches Health Factor (HF) = 1.0, protocol-mandated liquidation bots will execute forced collateral sales into the open market simultaneously. This is not a behavioural tendency — it is a smart contract rule. The forced selling creates a predictable, measurable, directional price impulse that can be front-run during the approach phase and faded after the cascade exhausts. Two distinct sub-strategies exist: (A) a momentum short entered before the cluster is breached, and (B) a mean-reversion long entered after the cascade overshoot. This document specifies both legs with independent entry, exit, and kill criteria.

---

## Structural Mechanism

### Why This Edge Exists

Aave's liquidation rule is encoded in the protocol's `LiquidationCall` function: any position with HF < 1.0 is immediately eligible for liquidation by any external caller who receives a liquidation bonus (currently 5–10% depending on asset). This creates a race condition among liquidation bots — the first caller captures the bonus, so bots monitor HF continuously and act within the same block that HF crosses 1.0.

The HF formula is deterministic and public:

```
HF = (Σ collateral_i × price_i × liquidation_threshold_i) / total_debt_USD
```

Given a known debt amount and collateral composition, the exact price level at which HF = 1.0 is calculable for every position on-chain. Aggregating across all positions produces a **liquidation depth map**: a histogram of notional collateral value at risk at each price level.

### The Cascade Mechanism

1. Price declines toward a cluster level.
2. Positions at the cluster hit HF = 1.0 simultaneously.
3. Liquidation bots sell collateral (ETH/BTC) into spot and perp markets to repay debt.
4. Selling pressure accelerates the price decline.
5. Lower prices push additional positions below HF = 1.0, triggering secondary cascades.
6. Cascade exhausts when no remaining clusters exist within the new price range.
7. Temporary overshoot below the final cluster level occurs due to bot execution overlap and panic selling.
8. Price mean-reverts as organic buyers absorb the discounted collateral.

### Why This Is Structural, Not Behavioural

The liquidation trigger is a protocol rule, not a human decision. Bots cannot choose to delay. The collateral composition of every position is publicly readable from chain state. The price feed used by Aave (Chainlink oracle) is also public. Therefore, the liquidation price for every position is calculable in advance with zero ambiguity. The only probabilistic elements are: (a) the magnitude of price impact per dollar of liquidation, and (b) the speed at which organic buyers absorb the selling.

---

## Market & Asset Scope

- **Primary assets:** ETH-USDC, WBTC-USDC collateral positions on Aave v3 Ethereum mainnet and Aave v3 Arbitrum.
- **Trading venue:** Hyperliquid perpetual futures (ETH-PERP, BTC-PERP) for execution; no spot execution required.
- **Cluster size threshold:** Only trade clusters with ≥ $20M notional collateral at risk within a ±1% price band. Below this threshold, price impact is insufficient to generate tradeable momentum.
- **Volatility filter:** Only activate when 24h realised volatility on ETH or BTC exceeds 3% (annualised equivalent ~57%). In low-volatility regimes, price rarely reaches cluster levels within a tradeable timeframe.

---

## Entry Rules


### Leg A — Momentum Short (Pre-Liquidation)

**Entry conditions (all must be true simultaneously):**

1. A liquidation cluster of ≥ $20M notional exists within 2.0% below current spot price, as read from the live liquidation depth map.
2. Price is moving toward the cluster: 15-minute close is below the 15-minute open, and the 1-hour price change is negative.
3. The cluster has not been breached in the prior 4 hours (prevents re-entry into an already-liquidated level).
4. Funding rate on Hyperliquid ETH-PERP or BTC-PERP is not more than +0.05% per 8 hours (avoids paying excessive funding against the short).
5. No Chainlink oracle freeze or deviation alert is active (oracle manipulation risk).

**Entry mechanics:**
- Enter market order on Hyperliquid perp short at the moment all five conditions are satisfied.
- Record the cluster level price as `P_cluster`.

## Exit Rules

**Exit conditions (first trigger wins):**

- **Target exit:** Price passes through `P_cluster` by 1.0% to the downside (i.e., price ≤ `P_cluster × 0.99`). Close entire position at market.
- **Stop loss:** Price moves 1.5% above entry price. Close entire position at market. This is a hard stop — no exceptions.
- **Time stop:** If price has not reached `P_cluster` within 6 hours of entry, close at market regardless of P&L. Cluster proximity without breach indicates the level is being defended; the setup has failed.
- **Cluster dissolution:** If on-chain data shows the cluster has been reduced below $10M (e.g., positions closed voluntarily or debt repaid), close immediately.

**Expected holding period:** 30 minutes to 6 hours.

---

### Leg B — Fade Long (Post-Cascade)

**Entry conditions (all must be true simultaneously):**

1. Leg A target exit has triggered (price passed through cluster by 1%), confirming cascade occurred.
2. On-chain liquidation volume in the prior 15 minutes exceeds $5M (confirms actual liquidation event, not just price movement). Source: Aave subgraph `LiquidationCall` events.
3. Price has stopped declining: two consecutive 5-minute candles with higher lows.
4. No additional cluster of ≥ $10M exists within 2% below current price (secondary cascade risk check).
5. Entry is taken no later than 30 minutes after the Leg A exit. If the two-candle stabilisation pattern has not formed within 30 minutes, skip Leg B entirely.

**Entry mechanics:**
- Enter market order long on Hyperliquid perp at the open of the candle following the two-candle stabilisation confirmation.

**Exit conditions (first trigger wins):**

- **Target exit:** Price retraces 50% of the cascade candle's range (from cascade start to cascade low). Calculate as: `cascade_start - (cascade_start - cascade_low) × 0.50`.
- **Stop loss:** Price makes a new low below the cascade low. Close immediately. A new low indicates a secondary cascade or genuine trend continuation — the fade thesis is invalidated.
- **Time stop:** Close at market after 4 hours regardless of P&L.

**Expected holding period:** 15 minutes to 4 hours.

---

## Position Sizing

### Base Sizing Formula

Position size is determined by the cluster notional and the account's risk budget, not by conviction alone.

```
Position size (USD notional) = Account equity × Risk per trade / Stop distance
```

Where:
- **Risk per trade** = 0.75% of account equity per leg (Leg A and Leg B are sized independently).
- **Stop distance** = distance from entry to stop loss in percentage terms.

**Example (Leg A):**
- Account equity: $100,000
- Risk per trade: $750
- Stop distance: 1.5%
- Position notional: $750 / 0.015 = $50,000

**Leverage cap:** Maximum 5× leverage on any single leg. If the formula produces a position requiring >5× leverage, cap at 5× and accept that the dollar risk is less than 0.75%.

### Cluster-Size Scaling

Scale position size linearly with cluster notional above the $20M threshold, capped at 2× base size:

| Cluster Notional | Size Multiplier |
|---|---|
| $20M–$50M | 1.0× |
| $50M–$100M | 1.5× |
| >$100M | 2.0× |

### Concurrent Position Limit

Run a maximum of two simultaneous Leg A positions (one ETH, one BTC). Never run Leg A and Leg B simultaneously on the same asset — they are sequential, not parallel.

---

## Data Sources

### Required Data Feeds

| Data | Source | Latency Requirement | Cost |
|---|---|---|---|
| Aave v3 position data (collateral, debt, liquidation threshold) | The Graph — Aave v3 subgraph | ≤ 5 minutes | Free |
| Real-time ETH/BTC price | Chainlink on-chain oracle or CoinGecko API | ≤ 30 seconds | Free |
| Liquidation event stream | Aave subgraph `LiquidationCall` entity | ≤ 2 minutes | Free |
| Perp OHLCV and funding rate | Hyperliquid REST API | Real-time | Free |
| Historical liquidation history | DeFiLlama liquidations endpoint | Batch | Free |
| Chainlink oracle health | Chainlink Data Feeds status page | ≤ 5 minutes | Free |

### Liquidation Depth Map Construction

Build the depth map as follows:

1. Query all active Aave v3 positions from the subgraph: for each position, retrieve `collateralAsset`, `collateralAmount`, `debtAsset`, `debtAmount`, `liquidationThreshold`.
2. For each position, solve for `P_liquidation` where HF = 1.0:
   ```
   P_liquidation = total_debt_USD / (collateral_amount × liquidation_threshold)
   ```
3. Bin positions into $100 price buckets. Sum notional collateral at risk per bucket.
4. Refresh the depth map every 5 minutes. Positions are added/removed as users open/close/repay.
5. Flag any bucket where cumulative notional within ±1% of current price exceeds $20M.

### Infrastructure Minimum

- One cloud server (2 vCPU, 4GB RAM) running the depth map builder continuously.
- Alert system (Telegram or PagerDuty) that fires when a cluster enters the 2% proximity window.
- Hyperliquid API key for order execution.
- No MEV infrastructure required — entry is during the approach phase, not at the liquidation block.

---

## Backtest Methodology

### Phase 1 — Historical Cluster Reconstruction (Months 1–2)

1. Download Aave v3 Ethereum mainnet position snapshots from The Graph for the period January 2023 – present. This covers multiple high-volatility episodes (March 2023, August 2023, January 2024, April 2024).
2. Replay position state block-by-block using archived Chainlink price feeds to reconstruct the liquidation depth map at each 15-minute interval.
3. Identify all historical instances where a cluster ≥ $20M existed within 2% of spot price and price was moving toward it.
4. Record: cluster level, cluster notional, price at 2% proximity, subsequent price path, actual liquidation volume (from `LiquidationCall` events), cascade low, and 4-hour post-cascade price.

### Phase 2 — Strategy Simulation (Month 2)

5. Apply Leg A entry and exit rules to each identified instance. Record P&L per trade assuming 0.05% slippage on entry and exit (conservative for Hyperliquid ETH/BTC perps).
6. Apply Leg B entry and exit rules to each confirmed cascade. Record P&L separately.
7. Compute: win rate, average R-multiple, maximum drawdown, Sharpe ratio, and trade frequency per month.

### Phase 3 — Sensitivity Analysis (Month 2)

8. Vary cluster threshold ($10M, $20M, $50M) and measure how win rate and frequency change.
9. Vary proximity trigger (1%, 2%, 3%) and measure false trigger rate (price approaches but does not breach).
10. Vary stop distance (1%, 1.5%, 2%) and measure impact on R-multiple distribution.

### Minimum Backtest Sample

Require ≥ 30 Leg A trades and ≥ 20 Leg B trades before drawing conclusions. If the historical period does not produce this sample, extend to Aave v2 data or include Arbitrum deployments.

### Backtest Honesty Constraints

- Do not use hindsight to select which clusters to trade. Apply the rules mechanically to every qualifying instance.
- Include all losing trades. Do not exclude "unusual market conditions" unless a specific kill criterion (defined below) would have been triggered.
- Report results separately for ETH and BTC — they may have different characteristics.

---

## Go-Live Criteria

All of the following must be satisfied before allocating real capital:

1. **Backtest Leg A:** Win rate ≥ 55% and average R-multiple ≥ 0.4 across ≥ 30 trades.
2. **Backtest Leg B:** Win rate ≥ 50% and average R-multiple ≥ 0.3 across ≥ 20 trades.
3. **Maximum backtest drawdown:** ≤ 15% of starting equity across the full backtest period.
4. **Paper trade period:** 30 calendar days of live paper trading with ≥ 5 real-time signals observed. Paper trade results must not show a win rate more than 15 percentage points below backtest win rate (indicates overfitting or data snooping).
5. **Infrastructure validation:** Depth map builder has run continuously for 14 days without data gaps exceeding 15 minutes. Alert system has been tested with simulated cluster events.
6. **Funding rate check:** Confirm that average funding rate during backtest periods was not systematically negative (which would have subsidised shorts and inflated backtest results).

---

## Kill Criteria

Stop trading and return to research immediately if any of the following occur:

1. **Live win rate:** Drops below 40% over any rolling 20-trade window (Leg A and Leg B combined).
2. **Live drawdown:** Account drawdown exceeds 10% from the strategy's peak equity.
3. **Structural change:** Aave governance votes to change liquidation thresholds, bonus structure, or oracle provider. Re-evaluate the mechanism before resuming.
4. **Oracle manipulation event:** Any confirmed Chainlink price feed manipulation on ETH or BTC. The entire mechanism depends on oracle integrity.
5. **Bot competition escalation:** If liquidation events begin occurring within 1–2 blocks of HF crossing 1.0 (detectable via on-chain timing analysis), the approach window may compress to the point where human-speed entry is no longer viable.
6. **Cluster dissolution rate:** If more than 60% of identified clusters dissolve (positions repaid voluntarily) before being breached, the signal is generating too many false setups. Raise the cluster threshold to $50M and re-evaluate.
7. **Consecutive loss streak:** Six consecutive losing trades on Leg A. Pause, audit each trade against the rules, and confirm no rule was violated before resuming.

---

## Risks

| Risk | Severity | Probability | Mitigation |
|---|---|---|---|
| Bot front-running: liquidation bots execute before our entry is filled | High | Medium | Entry is during approach phase (2% away), not at liquidation event. Bots operate at the liquidation block; we operate minutes earlier. |
| False cluster: large position repays debt voluntarily before breach | Medium | Medium | Cluster dissolution check every 5 minutes; exit immediately if cluster drops below $10M. |
| Secondary cascade: additional clusters below the first cause Leg B stop to trigger | High | Low-Medium | Leg B entry condition 4 explicitly checks for sub-clusters. Skip Leg B if secondary cluster exists. |
| Oracle manipulation: attacker manipulates Chainlink feed to trigger liquidations artificially | High | Very Low | Monitor Chainlink deviation alerts. Kill criterion 4 covers this. |
| Funding rate drag: persistent negative funding erodes Leg A returns | Medium | Low | Funding rate filter in entry condition 4. Monitor cumulative funding cost weekly. |
| Aave v3 protocol upgrade: liquidation mechanics change | High | Low | Monitor Aave governance forum. Kill criterion 3 covers this. |
| Subgraph data lag: The Graph returns stale position data | Medium | Medium | Cross-validate subgraph data against direct RPC calls to Aave's `getUserAccountData` function for top-10 positions by collateral size. |
| Liquidity on Hyperliquid: large position cannot be filled without significant slippage | Low | Low | Position sizing caps at $50,000 notional per leg at base size. ETH-PERP and BTC-PERP on Hyperliquid routinely handle $1M+ orders. |
| Correlated positions: ETH and BTC clusters breach simultaneously during market crash | High | Low | Concurrent position limit of one ETH + one BTC. Total risk exposure is 2 × 0.75% = 1.5% of equity simultaneously. |

---

## Open Research Questions

These questions must be answered during the backtest phase before go-live:

1. **What is the historical false trigger rate?** How often does price enter the 2% proximity window and then reverse without breaching the cluster? A false trigger rate above 50% would require tightening the proximity threshold to 1%.

2. **What is the average price impact per $1M of liquidations?** This determines whether the momentum leg generates enough movement to cover transaction costs. Hypothesis: $1M of ETH liquidations moves ETH price by approximately 0.05–0.15% in normal liquidity conditions.

3. **Do cascades produce measurable overshoot?** Specifically, does price consistently trade below the cluster level after breach, and by how much? If the median overshoot is less than 0.5%, Leg B is not viable after transaction costs.

4. **Is there a time-of-day effect?** Liquidation cascades during low-liquidity hours (02:00–06:00 UTC) may produce larger overshoots but also larger slippage. Analyse by hour.

5. **Does cluster notional predict cascade magnitude?** Test whether $50M+ clusters produce proportionally larger price moves than $20M clusters, or whether market depth absorbs larger liquidations efficiently.

6. **What is the lead time between cluster entry into the 2% window and breach?** This determines whether a 6-hour time stop is appropriate or whether it should be shorter.

---

## Relationship to Existing Zunid Strategies

This strategy is directionally correlated with token unlock shorts during market stress periods — both are short-biased and both are triggered by forced selling events. During a broad market downturn, both strategies may be active simultaneously, doubling short exposure. Monitor combined portfolio delta and reduce position sizes on this strategy if token unlock shorts are also active on ETH or BTC. The fade leg (Leg B) provides a natural hedge — it is long-biased and activates after the momentum leg closes, partially offsetting directional risk at the portfolio level.

---

*Next step: Build the depth map reconstruction script using Aave v3 subgraph data and run Phase 1 historical cluster identification. Target completion: 6 weeks. Assign to: data engineering.*
