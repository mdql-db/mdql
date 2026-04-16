---
title: "Morpho/Euler Interest Rate Reset — Borrow Queue Displacement Short"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 4
frequency: 3
composite: 300
categories:
  - lending
  - liquidation
  - defi-protocol
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When an isolated lending market on Morpho Blue or Euler v2 reaches 100% utilization, the smart contract's interest rate model mechanically forces the borrow APR to its maximum ceiling (typically 500–2000% APR depending on the IRM configuration). This rate spike creates an existential cost for existing borrowers who hold leveraged long positions via collateral-against-stablecoin loans. Borrowers face a binary choice: (a) repay the stablecoin debt by selling collateral, or (b) add fresh collateral capital. In small isolated markets with thin collateral token liquidity, the forced repayment flow creates outsized downward price pressure on the collateral token. We short the collateral token on Hyperliquid perpetuals when utilization crosses 95% and cover when utilization normalizes below 80%.

**The structural claim:** The rate spike is not probabilistic — it is deterministic. The IRM smart contract code guarantees the rate at 100% utilization. What is probabilistic is whether borrowers repay (vs. add collateral or accept the cost). The edge degrades if borrowers are well-capitalized and patient. The edge is strongest when: (1) the collateral token has thin spot liquidity, (2) the market has many distinct borrowers (not one whale who can easily recapitalize), and (3) no external liquidity injection is imminent.

---

## Structural Mechanism

### The Forcing Function (Deterministic Layer)

Morpho Blue uses a configurable Interest Rate Model (IRM) per market. The canonical `AdaptiveCurveIRM` adjusts rates based on utilization with a hard kink. At utilization = 100%:

```
Borrow APR = f(utilization) → ceiling rate (contract-defined, typically 500–2000% APR)
```

This is enforced by the smart contract on every `accrueInterest()` call. No human discretion. No governance vote required. The rate is applied per-block.

**At 1000% APR:** A borrower with $100k stablecoin debt accrues ~$2.74/day per $1k borrowed, or $274/day on $100k. Over 72 hours this is $822 in interest on a $100k position — roughly 0.8% of notional. This is painful but not immediately fatal for a well-capitalized borrower.

**At 2000% APR:** The same position accrues $1,644 over 72 hours — 1.6% of notional. For leveraged positions with thin equity buffers, this accelerates insolvency.

**Key insight:** The rate spike alone may not force repayment. The *combination* of rate spike + approaching liquidation threshold is the true forcing function. Monitor both utilization AND the health factor distribution of borrowers in the market.

### The Transmission Mechanism (Probabilistic Layer)

```
100% utilization
       ↓
Max borrow rate activates (deterministic)
       ↓
Borrower health factors deteriorate (rate accrual + any price drop)
       ↓
Borrowers repay stablecoin debt (requires selling collateral token)
       ↓
Sell pressure on collateral token (thin market = outsized impact)
       ↓
Price drop triggers further health factor deterioration
       ↓
Reflexive loop until utilization normalizes
```

The reflexive loop is the amplifier. Once collateral price drops, health factors worsen, triggering more repayments, triggering more price drops. This is structurally identical to a liquidation cascade but slower (hours to days vs. minutes).

### Why Isolated Markets Amplify the Effect

Morpho Blue's isolation model means each market has its own liquidity pool. A $15m TVL market with a $30m market cap collateral token means the Morpho borrowers represent a meaningful fraction of total token float. Compare to Aave where the same token might be in a shared pool with $500m TVL — the relative pressure is diluted.

---

## Market Selection Criteria

Apply ALL of the following filters before entering a trade:

| Filter | Threshold | Rationale |
|--------|-----------|-----------|
| Utilization | ≥ 95% | Approaching max rate territory |
| Market TVL | $2m – $100m | Large enough to matter, small enough to be isolated |
| Collateral token market cap | < $500m | Thin enough for Morpho repayments to move price |
| Collateral token listed on Hyperliquid | Required | Execution venue |
| Number of distinct borrowers | ≥ 5 | Avoids single-whale markets (one actor can self-rescue) |
| Collateral token spot 24h volume | < 10x the Morpho market's borrow outstanding | Ensures Morpho repayments are meaningful relative to normal flow |
| Protocol-owned liquidity as lender | < 50% of supply | POL can be injected instantly, killing the squeeze |
| Time since last utilization spike | > 7 days | Avoids re-entering a market that already flushed |

**Disqualifying conditions:**
- The market's sole lender is a DAO treasury or protocol multisig (they can inject liquidity in one transaction)
- The collateral token has a scheduled unlock event in the next 7 days (confounds the signal)
- The collateral token is a liquid staking token with a guaranteed redemption path (NAV floor limits downside)

---

## Entry Rules

### Signal Generation

**Step 1 — Utilization scan (run every 4 hours):**
Query Morpho Blue subgraph or API for all active markets:
```
GET https://blue-api.morpho.org/graphql
Query: markets where utilization > 0.90
Fields: id, collateralAsset, borrowAsset, utilization, totalBorrowAssets, totalSupplyAssets, borrowApy, uniqueBorrowerCount
```

**Step 2 — Apply market selection filters** (table above)

**Step 3 — Confirm rate is at or near ceiling:**
- Fetch current `borrowRate` from the market contract
- Confirm it is ≥ 80% of the IRM's configured ceiling rate
- This confirms the mechanical forcing function is active

**Step 4 — Check borrower health factor distribution:**
- Query borrower positions via subgraph
- If median health factor of borrowers < 1.3, the forcing function is near-critical
- If median health factor > 2.0, borrowers can absorb rate pressure longer — reduce position size by 50%

### Entry Execution

- **Instrument:** Hyperliquid perpetual for the collateral token
- **Entry timing:** Enter short within the next 4-hour candle close after signal confirmation
- **Do not chase:** If the collateral token has already dropped >15% since utilization crossed 95%, skip the trade (late entry, risk/reward degraded)
- **Order type:** Limit order within 0.3% of mid-price; if not filled within 30 minutes, use market order for up to 50% of intended size

---

## Exit Rules

### Primary Exit Triggers (check every 4 hours)

| Trigger | Action |
|---------|--------|
| Utilization drops below 80% | Close 100% of position at market |
| 72 hours elapsed since entry | Close 100% of position at market |
| Collateral token price drops >25% from entry | Close 50% (take profit), trail stop on remainder |
| New large lender deposit detected (>20% of current supply) | Close 100% immediately — squeeze is over |
| Borrower count drops to ≤ 2 | Close 100% — market is now a single-actor situation |

### Stop Loss

- **Hard stop:** +12% adverse move on the collateral token from entry price
- **Rationale:** A 12% move against the short suggests external buying pressure is overwhelming the Morpho repayment flow; the structural edge is not manifesting
- **No averaging down:** If the stop is hit, exit and re-evaluate; do not add to a losing short

### Partial Profit Taking

- At -10% move in collateral token price: close 33% of position
- At -18% move: close another 33%
- Remainder: hold until primary exit trigger

---

## Position Sizing

### Base Sizing Formula

```
Position size = min(
    Account_Risk_Per_Trade / Stop_Distance,
    Max_Position_Cap
)

Where:
    Account_Risk_Per_Trade = 1.5% of total account NAV
    Stop_Distance = 12% (hard stop from entry)
    Max_Position_Cap = 3% of collateral token's 24h spot volume
```

**Example:**
- Account NAV: $500,000
- Risk per trade: $7,500 (1.5%)
- Stop distance: 12%
- Max position from risk formula: $7,500 / 0.12 = $62,500 notional
- Collateral token 24h volume: $5,000,000 → 3% cap = $150,000
- **Final position: $62,500 notional**

### Adjustments

| Condition | Size Adjustment |
|-----------|----------------|
| Median borrower health factor < 1.2 | +25% (higher urgency) |
| Median borrower health factor > 1.8 | -50% (lower urgency) |
| Market TVL < $5m | -30% (too small, slippage risk on exit) |
| Utilization has been at 100% for >48h already | -50% (most forced sellers may have already acted) |
| Collateral token has Hyperliquid OI > $10m | +0% (no adjustment, sufficient liquidity) |
| Collateral token has Hyperliquid OI < $1m | -50% (execution risk on exit) |

### Maximum Concurrent Positions

- No more than 3 simultaneous positions in this strategy
- No two positions in collateral tokens with correlation > 0.7 (avoids sector-wide move confounding individual signals)
- Total strategy exposure cap: 8% of account NAV

---

## Backtest Methodology

### Data Requirements

| Dataset | Source | Cost | Availability |
|---------|--------|------|--------------|
| Morpho Blue market utilization history | Morpho Blue subgraph (TheGraph) | Free | From launch ~Aug 2023 |
| Per-market borrow APY history | Morpho API / subgraph | Free | From launch |
| Borrower position snapshots | Morpho Blue subgraph | Free | From launch |
| Collateral token OHLCV | Hyperliquid historical data / Binance API | Free | Varies by token |
| Lender deposit/withdrawal events | Morpho Blue subgraph (Supply/Withdraw events) | Free | From launch |

### Backtest Steps

**Step 1 — Event identification:**
Pull all historical instances where Morpho Blue market utilization crossed 95% threshold. Filter by market selection criteria. Expected sample size: 15–40 events across all markets since Aug 2023 (hypothesis — needs verification).

**Step 2 — Collateral token price alignment:**
For each event, extract collateral token price at the moment utilization crossed 95%, and at 24h, 48h, 72h, and 168h intervals. Also extract price at the moment utilization dropped below 80%.

**Step 3 — Outcome classification:**
Classify each event:
- **Type A:** Utilization resolved via repayment (borrowers sold collateral) — expect negative price return
- **Type B:** Utilization resolved via new lender deposit — expect neutral/positive price return
- **Type C:** Utilization remained elevated >72h — measure price drift

**Step 4 — Return calculation:**
For each Type A event, calculate:
- Return from entry (95% utilization cross) to exit (80% utilization or 72h)
- Maximum adverse excursion (MAE) — how far against the short did price move before resolving
- Maximum favorable excursion (MFE) — maximum profit available

**Step 5 — Filter validation:**
Test whether each filter in the Market Selection Criteria section improves the signal. Remove filters that don't improve Sharpe or win rate. Add filters if patterns emerge.

**Step 6 — Slippage modeling:**
Apply realistic slippage: 0.15% entry + 0.15% exit for Hyperliquid perps. Apply funding rate cost (assume 0.01%/8h for short positions as a conservative estimate).

### Minimum Backtest Acceptance Criteria

| Metric | Minimum Threshold |
|--------|------------------|
| Sample size | ≥ 15 qualifying events |
| Win rate | ≥ 55% |
| Average win / Average loss | ≥ 1.5 |
| Sharpe ratio (annualized) | ≥ 1.0 |
| Maximum drawdown | ≤ 20% of strategy allocation |
| % of events resolved via repayment (Type A) | ≥ 40% (validates the mechanism) |

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

1. **Backtest passes** all minimum acceptance criteria above
2. **Paper trade period:** Minimum 60 days or 5 qualifying events (whichever comes later), with paper trade results within 20% of backtest expectations
3. **Monitoring infrastructure live:** Automated 4-hour utilization scan running with alerting (PagerDuty or equivalent); no manual checking required
4. **Execution infrastructure live:** Hyperliquid API integration tested with limit and market orders; position sizing calculator automated
5. **Lender deposit detection live:** Webhook or polling system that detects large Supply events on monitored Morpho markets within 30 minutes
6. **Legal/compliance review:** Confirm short selling of relevant tokens is permissible in operating jurisdiction
7. **Correlation check:** Confirm strategy is not highly correlated (>0.6) with existing Zunid strategies in live deployment

---

## Kill Criteria

**Immediate suspension (same day):**
- Any single trade loses >3x the expected maximum loss (suggests model break)
- Monitoring infrastructure fails and cannot be restored within 24 hours
- Morpho Blue undergoes a smart contract upgrade that changes the IRM mechanics

**Strategy review (within 1 week):**
- 5 consecutive losing trades
- Win rate drops below 40% over any rolling 20-trade window
- Average loss exceeds average win over any rolling 20-trade window
- A structural change in Morpho's market design (e.g., cross-collateral pools replacing isolated markets) that invalidates the isolation assumption

**Permanent retirement:**
- Backtest re-run on updated data shows Sharpe < 0.5 over the full history
- The strategy's edge is publicly documented in a widely-read research report (edge likely to be arbitraged away)
- Morpho TVL in isolated markets drops below $50m total (insufficient opportunity set)

---

## Risks

### Risk 1: Lender Rescue (HIGH PROBABILITY)
**Description:** A single large lender (DAO, protocol, whale) deposits new liquidity into the market, instantly resolving utilization and eliminating the squeeze.
**Mitigation:** Filter out markets where a single entity controls >50% of supply. Monitor for large deposit transactions in real time. Exit immediately on detection.
**Residual risk:** Even with monitoring, a deposit can occur between polling intervals. Accept this as a known loss scenario.

### Risk 2: Borrower Patience (MEDIUM PROBABILITY)
**Description:** Borrowers are well-capitalized and accept the high rate cost, waiting for utilization to normalize organically. No forced selling occurs.
**Mitigation:** Health factor filter (exit if median HF > 2.0 at entry). 72-hour hard exit prevents prolonged capital tie-up.
**Residual risk:** Rate cost at 1000% APR over 72h is ~0.8% of notional — borrowers with strong conviction may absorb this.

### Risk 3: Collateral Token External Catalyst (MEDIUM PROBABILITY)
**Description:** A positive external catalyst (partnership announcement, exchange listing, airdrop) overwhelms the Morpho repayment sell pressure.
**Mitigation:** Hard stop at +12% adverse move. No averaging down.
**Residual risk:** Catalysts are by definition unpredictable. This is the primary source of large individual losses.

### Risk 4: Hyperliquid Liquidity / Funding (LOW-MEDIUM PROBABILITY)
**Description:** Small-cap tokens on Hyperliquid may have wide spreads, low OI, or punitive funding rates for shorts.
**Mitigation:** Minimum OI filter ($1m). Position size capped at 3% of 24h volume. Monitor funding rate; if short funding exceeds 0.05%/8h, reduce position by 50%.
**Residual risk:** Funding rate can spike unpredictably if many traders pile into the same short.

### Risk 5: Smart Contract Risk (LOW PROBABILITY)
**Description:** A bug or exploit in Morpho Blue causes abnormal market behavior unrelated to the utilization mechanism.
**Mitigation:** This strategy takes no direct protocol exposure (we short on Hyperliquid, not within Morpho). Indirect risk: an exploit could cause panic selling that benefits the short, or a rescue that harms it.
**Residual risk:** Low; we have no funds in Morpho.

### Risk 6: Small Sample Size / Overfitting (HIGH PROBABILITY during backtest)
**Description:** Morpho Blue launched in mid-2023. The total number of qualifying events may be 15–30, which is insufficient for robust statistical inference.
**Mitigation:** Apply conservative acceptance criteria. Do not optimize more than 3 parameters in the backtest. Require out-of-sample paper trading period.
**Residual risk:** The strategy may be data-mined. Treat the backtest as hypothesis validation, not proof.

---

## Data Sources

| Source | URL | Data | Update Frequency |
|--------|-----|------|-----------------|
| Morpho Blue Subgraph | `https://thegraph.com/explorer/subgraphs/morpho-blue` | Utilization, borrow APY, borrower positions, supply/withdraw events | Per block |
| Morpho Blue API | `https://blue-api.morpho.org/graphql` | Aggregated market stats, cleaner than raw subgraph | 5-minute cache |
| Morpho Blue Contract Events | Ethereum mainnet via Alchemy/Infura | Raw Supply, Borrow, Repay, Withdraw events | Per block |
| Hyperliquid Historical Data | `https://app.hyperliquid.xyz/data` | Perp OHLCV, OI, funding rates | Per candle |
| Binance/CoinGecko | Standard APIs | Spot price for tokens not on Hyperliquid | Per minute |
| DeFiLlama | `https://defillama.com/protocol/morpho-blue` | TVL trends, market-level breakdown | Hourly |

---

## Open Questions for Backtest Phase

1. **What fraction of 100% utilization events resolve via repayment vs. new deposits?** This determines the base rate of the mechanism firing. If >60% resolve via new deposits, the edge may be weak.
2. **What is the median price impact on the collateral token during Type A events?** Need to establish whether the move is large enough to overcome transaction costs and funding.
3. **Is there a lead time between utilization crossing 95% and the price move beginning?** If the market prices in the squeeze before we enter, the edge is front-run.
4. **Do Euler v2 isolated markets show the same pattern?** Euler v2 uses a similar IRM architecture; expanding the opportunity set would improve trade frequency.
5. **Is the effect stronger on Base/Arbitrum Morpho deployments vs. Ethereum mainnet?** Gas costs on mainnet may slow borrower repayment, extending the squeeze window.

---

## Next Steps

| Action | Owner | Deadline |
|--------|-------|----------|
| Pull all Morpho Blue utilization history via subgraph | Data team | T+5 days |
| Identify all events where utilization crossed 95% | Data team | T+7 days |
| Classify events as Type A/B/C | Research | T+10 days |
| Run backtest Steps 1–6 | Quant | T+21 days |
| Present backtest results vs. acceptance criteria | Research | T+25 days |
| Decision: proceed to paper trade or kill | PM | T+28 days |
