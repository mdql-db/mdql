---
title: "Post-Liquidation-Cascade Funding Carry"
status: HYPOTHESIS
mechanism: 6
implementation: 7
safety: 5
frequency: 3
composite: 630
categories:
  - funding-rates
  - liquidation
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large liquidation cascade wipes one side of a perpetual futures market, the surviving open interest becomes structurally skewed. Perpetual exchange funding formulas respond to this skew by mechanically producing extreme funding rates — often pinned at the cap — for 16–72 hours while new counterparty capital slowly re-enters to rebalance.

The trade is: immediately after a confirmed cascade, take the **funded side** of the market and collect multiple consecutive capped funding payments until OI rebalances, then exit.

This is not a directional bet on price. It is a **carry trade with a structural entry trigger and a mechanical exit condition**.

---

## Why It's an Edge

The edge is mechanical, not behavioural. Two structural facts create it:

**Fact 1: The funding formula must produce extreme rates after a cascade.**

Hyperliquid's funding rate is calculated as:

```
funding_rate = clamp(premium / funding_interval, -0.05%, +0.05%) per 8h
```

When a cascade wipes the dominant side, OI skew becomes severe. The premium between perpetual and spot widens. The formula arithmetically outputs a rate near or at the ±0.05% cap. This is not a tendency — it is a deterministic output of the formula given the input conditions.

**Fact 2: The rebalancing takes time.**

New capital cannot instantaneously enter the market to restore OI balance. Traders need to: observe the dislocation, decide to enter, fund their account, and place orders. This onboarding latency creates a window — typically 16–72 hours — during which the funded side pays the maximum rate every 8 hours. During this window the carry accrues mechanically to anyone willing to hold the funded position.

**Why this is different from the existing funding-rate-fade strategy:**

The existing strategy fades *sustained* high funding directionally (betting the crowded side will mean-revert). This strategy does not take a view on direction — it enters immediately *after* the cascade has already resolved the crowding (via forced liquidation), and collects the residual funding while the market mechanically clears. The trigger event (cascade + OI wipe) is the confirmation that the crowded side has been structurally removed, not an anticipation that it will be.

**Arithmetic of a clean execution:**

At 0.05% per 8h, 6 consecutive capped payments = 0.30% carry over 48 hours. Round-trip fees at Hyperliquid taker rates = ~0.09%. Net carry = ~0.21% over 48 hours before any adverse price move. Annualised, capped funding = 54.75%. The question is how often cascades occur, how long funding stays capped, and how much directional risk is incurred during the window.

**Why the market doesn't arbitrage this away immediately:**

- Cascades are sudden — capital cannot pre-position
- The stabilisation check (30-minute wait) is required before entry, meaning the funded side isn't obvious during the cascade itself
- The window is short (48h maximum hold), making it unattractive to large funds with higher minimum return thresholds
- Directional risk during the carry window deters pure carry traders who prefer delta-neutral structures

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Notes |
|---------|--------|-------------|-------|
| Open interest snapshots | Hyperliquid `/info` → `metaAndAssetCtxs` | 1-hour | Compute OI % change per candle |
| Funding rate history | Hyperliquid `/info` → `fundingHistory` | 8-hour | Per asset, per funding period |
| Liquidation volumes | Coinglass historical liquidations API | 1-hour | Confirms cascade direction |
| OHLCV price data | Hyperliquid or Binance public API | 1-hour | Entry/exit price, stop-loss tracking |

### Universe

Top 10 Hyperliquid perpetuals by average OI over the backtest period. Exclude assets where OI is regularly below $50M — liquidation cascades on thin books may be noise rather than structural events.

Suggested starting universe: BTC, ETH, SOL, ARB, SUI, APT, OP, DOGE, AVAX, LINK.

### Event Identification

Scan all 1-hour candles across the universe for:

1. **OI drop ≥20%** within the hour (primary trigger)
2. **Price move ≥5%** in the same direction as the OI drop (confirms a liquidation cascade, not just position closure)
3. **Funding rate in the subsequent 8h period ≥0.04%** (confirms the formula is producing near-capped rates)

For each identified event, record:
- Asset
- Cascade direction (long wipe = funding positive = shorts collect; short wipe = funding negative = longs collect)
- OI level at cascade bottom
- Price at cascade bottom
- Entry price (30 minutes after cascade bottom, first close above/below cascade extreme)

### Simulated Trade

For each event:

1. **Enter** the funded side at the close of the 30-minute stabilisation candle
2. **Simulate funding payments** collected every 8 hours using historical funding rate data
3. **Track exit condition** — first of:
   - OI recovers to ≥80% of pre-cascade level
   - Funding rate for a period drops below 0.02%
   - 48 hours elapsed from entry
   - Price moves 3% adverse from entry (stop loss)
4. **Record** total funding collected, price P&L, net P&L, and which exit condition triggered

### Metrics to Report

| Metric | Purpose |
|--------|---------|
| Number of qualifying events | Frequency — how often does this fire? |
| % events where funding stayed ≥0.04% for ≥3 consecutive periods | Duration of funding opportunity |
| Average funding collected per trade (% notional) | Carry yield per event |
| Average price P&L per trade | Directional drag or tailwind |
| Average net P&L (funding + price ± fees) | Total edge |
| % trades stopped out at 3% adverse | Directional risk materialisation rate |
| Sharpe ratio of net P&L distribution | Risk-adjusted edge |
| Average hold time (hours) | Turnover |

### Baseline Comparison

Compare each trade's net P&L against:
- **Flat carry baseline:** Simply holding the funded side for 48 hours from the cascade, ignoring OI/funding exit conditions — measures whether dynamic exit adds value
- **Random entry baseline:** Same trade structure entered on random dates (no cascade trigger) — measures whether the cascade event adds value beyond just collecting funding during high-funding periods

If the cascade-triggered entries outperform random high-funding entries, the cascade detection is adding genuine edge on top of the funding carry.

### Minimum Sample Size for Confidence

Target: ≥30 events across the universe and backtest period. If the universe produces fewer than 30 events, extend the backtest window or expand the universe before drawing conclusions.

---

## Entry Rules

All of the following must be true before entering:

1. **OI drop ≥20%** in any 1-hour candle on the target asset
2. **Price moved ≥5%** in the cascade direction during the same 1-hour window
3. **Funding rate for the upcoming 8h period is ≥0.04%** (per Hyperliquid's published next-period rate, visible in the API)
4. **30-minute stabilisation:** Price has not continued in the cascade direction for the 30 minutes following the OI drop (i.e., price has retraced ≥1% from the cascade extreme — confirming the cascade is over, not ongoing)
5. **OI still depressed:** OI at entry is still ≥15% below pre-cascade level (cascade has not already rebalanced)
6. **Asset is in the approved universe** (top 10 by OI — see above)

**Direction:** Determined by cascade type:
- Cascade wiped longs (price fell) → enter **short** (collect funding from remaining longs who pay)
- Cascade wiped shorts (price rose) → enter **long** (collect funding from remaining shorts who pay)

**Entry price:** Market order at open of the candle immediately following the 30-minute stabilisation period.

---

## Exit Rules

Exit on the **first** of these conditions:

1. **OI recovery:** OI recovers to ≥80% of pre-cascade level (rebalancing complete — funding will compress)
2. **Funding compression:** The next published funding rate drops below 0.02% per period (carry no longer justifies directional exposure)
3. **Time limit:** 48 hours elapsed from entry (hard stop on carry window)
4. **Stop loss:** Price moves 3% adverse from entry price (cascade has resumed — exit immediately at market)

**Exit price:** Market order at trigger. Do not use limit orders for stop-loss exits.

---

## Position Sizing

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Notional per trade | $500 | Paper trading phase; small enough to be irrelevant to book, large enough to generate meaningful signal |
| Leverage | 2x maximum | This is a carry trade; leverage beyond 2x turns a carry trade into a directional bet |
| Maximum concurrent positions | 2 | Cascades on correlated assets (e.g., BTC and ETH simultaneously) should not stack into a combined directional bet |
| Capital reserved per position | $250 margin at 2x | At $500 notional, $250 margin required |

**Correlation constraint:** If two cascade signals fire simultaneously on correlated assets (BTC + ETH, or two competing L1s), take only the one with the higher funding rate. Do not stack correlated positions.

---

## Go-Live Criteria

Deploy real capital when ALL of the following are met:

1. **Minimum events:** At least 5 paper trades have been triggered and closed (not just opened)
2. **Positive net P&L:** Net P&L across all closed paper trades is positive after fees and funding costs paid
3. **Stop-loss hit rate:** Fewer than 40% of paper trades stopped out at the 3% adverse stop (if stop-loss is triggering frequently, the entry stabilisation check is insufficient)
4. **Funding duration confirmed:** In at least 60% of paper trades, funding stayed ≥0.04% for ≥3 consecutive periods (confirming the mechanical carry window exists in live conditions)
5. **Backtest completed:** Full backtest across ≥30 historical events has been run and shows positive net expectancy
6. **Founder approval:** Hyperliquid wallet and USDC deposit confirmed operational (shared infrastructure with Strategy 001)

---

## Kill Criteria

| Condition | Action |
|-----------|--------|
| After 5 paper trades: net P&L negative | Kill or redesign entry stabilisation rules |
| Stop-loss triggered in >50% of paper trades | Kill — cascade continuation risk is not being filtered |
| Funding rate never reaches ≥0.04% for ≥2 consecutive periods on any event | Kill — carry window too short to justify directional exposure |
| After 10 paper trades: net P&L per trade < 0.05% after all costs | Kill — edge too thin for reliable deployment |
| Any single paper trade loses >5% of notional | Kill and review — stop-loss is not executing as specified |
| Hyperliquid changes its funding formula or caps | Reassess immediately — the mechanical basis of the edge changes |

---

## Risks

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Cascade continues after entry — price resumes in the original direction | High | Medium | Hard 3% stop loss; 30-minute stabilisation requirement before entry |
| OI rebalances within 1–2 funding periods (faster than expected) | Medium | Medium | OI recovery exit rule triggers early; carry collected is still positive net of fees if ≥2 periods collected |
| Funding rate does not pin at cap despite large OI drop | Medium | Low | Funding ≥0.04% is a required entry condition — if it's not there, no trade is taken |
| Correlated cascade across multiple assets creates concentrated directional exposure | High | Low | Maximum 2 concurrent positions; correlation filter prevents stacking |
| Cascade is a single large position closing (not a true liquidation wipe) | Medium | Medium | Require price move ≥5% in same direction as OI drop — voluntary closes do not move price this much |
| Hyperliquid funding formula or cap changes | Medium | Low | Monitor Hyperliquid documentation and Discord for parameter changes; kill strategy immediately if formula changes |
| Thin OI on smaller tokens — cascade may be artificial or manipulated | Medium | Medium | Restrict universe to top 10 assets by OI only |
| Post-cascade price gap (sudden continuation) bypasses stop loss | High | Low | Use market orders for stop exits; accept that gap risk cannot be fully eliminated on perpetuals |

---

## Data Sources

| Data | Source | Access Method | Cost |
|------|--------|---------------|------|
| Real-time and historical OI | Hyperliquid API — `POST /info` with `metaAndAssetCtxs` | REST, public, no auth required | Free |
| Funding rate history | Hyperliquid API — `POST /info` with `fundingHistory` | REST, public, no auth required | Free |
| Liquidation volume history | Coinglass — `GET /api/pro/v1/futures/liquidation/history` | REST, free tier available | Free tier |
| OHLCV (1-hour candles) | Hyperliquid API or Binance `GET /api/v3/klines` | REST, public | Free |
| Next-period funding rate (live signal) | Hyperliquid API — `metaAndAssetCtxs` returns predicted next-period funding | REST, public | Free |

**Shared infrastructure with Strategy 001:** Hyperliquid API access, wallet setup, USDC deposit, and GitHub Actions scheduling are all shared. This strategy can be added to the existing paper trading workflow with a new event type in `experiments/paper_state.json` and a cascade-detection module in `experiments/paper_trader.py`.

---

## Relationship to Existing Strategies

This strategy is **complementary to, not duplicative of**, the existing pipeline:

| Existing Strategy | Relationship |
|-------------------|--------------|
| Strategy 001 (Token Unlock Shorts) | Orthogonal — different trigger, different asset selection, different hold period |
| Funding rate fade (in pipeline) | Distinct — funding fade takes a directional bet on the crowded side unwinding; this strategy enters *after* the unwind and collects residual carry |
| Liquidation cascade momentum (in catalogue, 6.6) | Distinct — momentum strategy rides the cascade direction; this strategy enters *after* the cascade and bets on stabilisation |

The three strategies (unlock shorts, funding fade, and this cascade carry) form a natural triad: unlocks are supply-driven, funding fade is positioning-driven, and cascade carry is mechanics-driven. They should be largely uncorrelated because their triggers are independent.

---

## Future Improvements

- **Funding duration model:** Build a regression model predicting how many periods funding will stay above 0.04% as a function of: (a) OI drop magnitude, (b) pre-cascade funding level, (c) asset liquidity. This would allow dynamic position sizing — larger when carry window is predicted to be longer.
- **OI rebalancing speed tracker:** Monitor how quickly OI recovers post-cascade historically by asset. Some assets attract capital faster (BTC) than others (small L2 tokens). Use this to calibrate the 48-hour time limit per asset.
- **Cascade direction filter:** Test whether long-wipe cascades (price fell, short now funded) have better carry outcomes than short-wipe cascades (price rose, long now funded) — asymmetry may exist due to different capital re-entry speeds in bull vs. bear conditions.
- **Multi-exchange confirmation:** If the same cascade is visible on Binance and Bybit simultaneously, it confirms the event is market-wide rather than Hyperliquid-specific. Cross-exchange confirmation could be a higher-confidence filter.
