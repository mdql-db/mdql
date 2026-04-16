---
title: "Blur Bid Pool Vacuum — Season-End Snapshot Unwind"
status: HYPOTHESIS
mechanism: 6
implementation: 6
safety: 6
frequency: 1
composite: 216
categories:
  - defi-protocol
  - airdrop
  - calendar-seasonal
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Blur farming seasons create a **temporary, artificial bid wall** in NFT collections. Rational farmers park ETH in the bid pool to accumulate points, then have **zero mechanical incentive** to keep bids live once the season snapshot is taken. The causal chain:

1. Season snapshot locks in point allocations → bid pool capital is no longer "working"
2. Farmers withdraw ETH from bid pool within 24–72h post-snapshot (on-chain verifiable)
3. NFT floor support evaporates as the artificial bid wall drains
4. NFT floor prices drop or become illiquid, particularly for mid-tier collections where Blur bid pool represented a disproportionate share of total bids
5. BLUR token price declines in sympathy: (a) reduced protocol activity signal, (b) incoming sell pressure from farmers liquidating BLUR rewards, (c) sentiment shift as "season is over" narrative spreads

**The core bet:** BLUR token price drops in the 48h window post-snapshot, driven by the mechanical unwinding of farming capital — not by sentiment or TA.

**Null hypothesis to disprove:** BLUR price is uncorrelated with bid pool TVL changes at season boundaries.

---

## Structural Mechanism — WHY This Must Happen

This is **not** a "tends to happen" pattern. The mechanism is game-theoretic and rational:

**During season:** Every ETH in the bid pool earns points per block. Removing capital before snapshot = leaving points on the table. Rational farmers hold until the last possible moment.

**At snapshot:** Points are frozen. The opportunity cost of keeping ETH in the bid pool instantly becomes the full cost of capital (ETH yield foregone, liquidation risk on NFT bids, etc.) with **zero offsetting reward**. The rational action is immediate withdrawal.

This is structurally identical to:
- **Curve gauge vote deadlines**: veToken holders vote until the deadline, then votes have no marginal value until next epoch
- **Airdrop snapshot unwind**: wallets that held tokens purely for snapshot sell immediately after
- **Liquidity mining cliff ends**: LPs exit when emissions drop to zero

The difference from pure sentiment plays: the withdrawal is **mechanically forced by the incentive structure ending**, not by price action or news. The on-chain bid pool drain is the leading indicator, not a lagging one.

**Why BLUR token specifically?**
- BLUR token price is correlated with protocol activity (bid pool TVL is the primary activity metric)
- Season-end = peak TVL → post-snapshot TVL collapses → protocol looks "dead" until next season announcement
- Farmers who earned BLUR rewards during the season often sell at season end (same airdrop-unwind dynamic)
- Funding rates on BLUR perp tend to be elevated during active farming seasons (longs paying), which reverses post-snapshot

---

## Entry Rules


### Trigger Conditions (all must be met)
1. Blur season end date is **publicly confirmed** (Blur blog, official Discord, or on-chain season contract parameter)
2. Current bid pool TVL is **>5,000 ETH** (sufficient capital to create measurable drain)
3. BLUR perp is available on Hyperliquid with open interest >$2M (liquidity check)
4. Funding rate on BLUR perp is **positive** (longs paying shorts) — confirms crowded long side from farmers hedging or speculating

### Entry
- **Timing:** Open short position **6–12 hours before** confirmed snapshot time
- **Instrument:** BLUR perpetual futures on Hyperliquid (primary) or Binance (secondary)
- **Price:** Market order or limit within 0.3% of mid
- **Do not enter** if season has been extended in the past 7 days (protocol manipulation risk)

## Exit Rules

### Exit
- **Primary exit:** 48 hours post-snapshot, market order regardless of P&L
- **Early exit trigger 1:** Bid pool TVL stabilises (less than 10% drain from peak) within 24h post-snapshot — thesis failed, exit immediately
- **Early exit trigger 2:** Blur announces next season start date within 24h post-snapshot — capital may re-enter, exit immediately
- **Stop loss:** +12% adverse move from entry (hard stop, no exceptions)
- **Take profit:** -20% from entry (partial: take 50% off, trail remainder with 8% stop)

### Re-entry
- No re-entry on same season. Wait for next season cycle.

---

## Position Sizing

**Base position:** 2% of portfolio NAV per trade

**Rationale:** 
- Causal chain has one weak link (bid pool drain → BLUR token price), warranting below-standard sizing
- Season events are infrequent (2–4 per year historically), so this is a low-frequency strategy
- Max loss at stop = 2% NAV × 12% = 0.24% NAV per trade — acceptable for a hypothesis-stage strategy

**Scaling rule:** If backtest shows Sharpe >1.5 and max drawdown <15% across all seasons, increase to 4% NAV. Do not exceed 4% until live track record of 3+ seasons.

**Leverage:** 2–3x maximum. BLUR is a small-cap token; higher leverage creates liquidation risk from wick moves.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Endpoint/URL |
|---|---|---|
| Blur bid pool TVL (hourly) | Blur API | `https://core-api.prod.blur.io/v1/collections/{slug}/bid-stats` |
| Bid pool deposit/withdraw events | Ethereum mainnet | Filter `BlurPool` contract `0x0000000000A39bb272e79075ade125fd351887Ac` for `Deposit`/`Withdrawal` events |
| Season start/end dates | Blur blog + Wayback Machine | `https://blur.io/blog` |
| BLUR/USDT hourly OHLCV | Binance | `GET /api/v3/klines?symbol=BLURUSDT&interval=1h` |
| BLUR perp funding rate history | Hyperliquid | `https://api.hyperliquid.xyz/info` → `fundingHistory` |
| NFT floor prices (top 10 Blur collections) | Reservoir | `https://api.reservoir.tools/collections/v7` |

### Season History to Cover
Blur has run approximately 4–6 seasons since launch (late 2022 through 2024). Identify each season end date precisely. Minimum: 4 seasons for backtest validity.

### Metrics to Calculate Per Season

**Primary:**
- BLUR price return: entry (T-6h) to exit (T+48h) where T = snapshot time
- Max adverse excursion (MAE) during hold window
- Max favorable excursion (MFE) during hold window

**Secondary (mechanism validation):**
- Bid pool ETH drain: % of peak TVL withdrawn within 24h, 48h, 72h post-snapshot
- Correlation between bid pool drain rate and BLUR price decline (Pearson r)
- NFT floor price change for top 5 Blur collections in 48h post-snapshot

**Baseline comparison:**
- BLUR price return for random 54h windows (T-6h to T+48h) during non-season-end periods
- ETH price return over same windows (to strip out market beta)

### Statistical Tests
- t-test: mean season-end return vs. mean random-window return (need p < 0.10 given small N)
- Spearman rank correlation: bid pool drain % vs. BLUR return (mechanism test)
- Check for confounding: did any season end coincide with broad crypto drawdown? Adjust for ETH beta.

### What "Works" Looks Like
- Mean BLUR return in window: < -5% (net of ETH beta)
- Bid pool drain >30% within 48h post-snapshot in majority of seasons
- Positive correlation between drain magnitude and price decline

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show **all** of the following:

1. **Win rate ≥ 60%** across all backtested seasons (minimum 4 seasons)
2. **Mean return per trade ≤ -5%** (i.e., short profits ≥5% on average) net of ETH beta
3. **Bid pool drain confirmed** in on-chain data for ≥75% of seasons (mechanism must be real, not assumed)
4. **No season** shows a loss exceeding the 12% stop loss threshold (tail risk check)
5. **Funding rate was positive** at entry in ≥75% of seasons (confirms the setup was present)

If fewer than 4 seasons are available with clean data, escalate to "needs more data" status — do not go live on 2–3 data points.

---

## Kill Criteria

Abandon the strategy immediately if any of the following occur:

**Pre-backtest kills:**
- Fewer than 3 seasons with clean, verifiable on-chain bid pool data
- Bid pool drain is not observable in on-chain data (mechanism doesn't exist)

**Post-backtest kills:**
- Mean return is positive (shorts lose money on average)
- Bid pool drain and BLUR price are uncorrelated (r < 0.2) — causal chain is broken
- Blur announces permanent end to bid pool farming model

**During paper/live trading kills:**
- Two consecutive losing trades at full stop loss
- Blur changes season structure (e.g., continuous rolling seasons with no discrete snapshot)
- BLUR perp delisted or OI drops below $1M (liquidity insufficient)
- Blur team announces season extension within 12h of entry (protocol risk materialises)

---

## Risks

### Risk 1: Protocol Rule Changes (HIGH probability, HIGH impact)
Blur has extended seasons retroactively with little notice. A single extension announcement post-entry turns the trade from "snapshot in 6h" to "snapshot in 3 weeks" — the bid pool refills and the short bleeds. **Mitigation:** Hard stop at +12%. Do not enter if any extension has occurred in the current season.

### Risk 2: Weak Causal Link to Token Price (MEDIUM probability, HIGH impact)
The bid pool drain → BLUR token price mechanism has one speculative step. BLUR token price is driven by many factors (broader NFT market, team announcements, exchange listings). The bid pool drain may be fully priced in by sophisticated traders before the snapshot. **Mitigation:** Backtest will reveal if the link exists. If correlation is weak, kill the strategy.

### Risk 3: Small Sample Size (CERTAIN, MEDIUM impact)
4–6 seasons is not statistically robust. Any backtest result is directional evidence, not proof. **Mitigation:** Treat go-live as paper trading only until 3 live seasons are observed. Never size above 2% NAV until live track record exists.

### Risk 4: Crowded Trade (LOW-MEDIUM probability, MEDIUM impact)
If this mechanism is well-known, sophisticated traders will front-run the entry, compressing or eliminating the edge. **Check:** Look at BLUR perp short interest and funding rate in the 48h before snapshot. If funding is already negative (shorts paying), the trade is crowded — skip.

### Risk 5: NFT Market Beta (MEDIUM probability, MEDIUM impact)
A broad NFT market rally post-snapshot could overwhelm the bid pool drain effect. BLUR token is correlated with NFT market sentiment. **Mitigation:** Strip ETH beta from returns. Consider hedging with long ETH if NFT market is in strong uptrend.

### Risk 6: Liquidity / Slippage (LOW probability, LOW impact)
BLUR perp on Hyperliquid has moderate liquidity. At 2% NAV sizing, slippage should be minimal. Monitor OI before entry.

---

## Data Sources

| Source | URL | Notes |
|---|---|---|
| Blur bid pool contract | `0x0000000000A39bb272e79075ade125fd351887Ac` on Ethereum mainnet | Query via Etherscan or Alchemy |
| Blur API (collections) | `https://core-api.prod.blur.io/v1/collections/` | Rate-limited; may require headers |
| Reservoir API (NFT floors) | `https://api.reservoir.tools/collections/v7?sortBy=allTimeVolume` | Free tier available |
| Blur season announcements | `https://blur.io/blog` + Wayback Machine | Archive historical season dates |
| Blur Discord (season dates) | discord.gg/blur-io | Cross-reference with blog |
| BLUR OHLCV (Binance) | `https://api.binance.com/api/v3/klines?symbol=BLURUSDT&interval=1h` | Free, no auth |
| Hyperliquid funding history | `https://api.hyperliquid.xyz/info` → POST `{"type": "fundingHistory", "coin": "BLUR"}` | Free |
| Dune Analytics (bid pool TVL) | `https://dune.com/` — search "Blur bid pool" | Community dashboards exist; verify query logic |
| Alchemy / Infura (on-chain events) | `https://www.alchemy.com/` | Needed for event log queries on BlurPool contract |

---

## Implementation Notes

**Step 1 — Season date reconstruction:** Use Wayback Machine to archive Blur blog posts and Discord announcements. Build a table: Season N | Start Date | End Date | Snapshot Block | Bid Pool Peak TVL.

**Step 2 — On-chain drain measurement:** Query `Withdrawal` events from BlurPool contract in blocks T to T+7200 (approximately 24h at 12s/block). Sum ETH withdrawn. Express as % of peak TVL.

**Step 3 — Price alignment:** Align BLUR hourly OHLCV to snapshot block timestamp. Calculate return from T-6h open to T+48h close.

**Step 4 — Regression:** Run OLS: BLUR_return ~ bid_pool_drain_pct + ETH_return. If `bid_pool_drain_pct` coefficient is significant and negative, mechanism is confirmed.

**Step 5 — Decision:** Apply go-live criteria. If passed, paper trade next season with full position sizing simulation.
