---
title: "Crypto ETF Authorized Participant Creation/Redemption Premium Arb"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 7
composite: 1260
categories:
  - basis-trade
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When spot Bitcoin ETFs (IBIT, FBTC, ARKB, etc.) trade at a measurable premium or discount to their intraday NAV, Authorized Participants are mechanically incentivized to arbitrage the gap. AP creation flow (ETF premium) requires buying BTC spot; AP redemption flow (ETF discount) requires selling BTC spot. This flow is not random — it is triggered by a contractual profit opportunity and must occur before the premium fully compresses. Zunid cannot execute the AP arb directly, but can anticipate the directional BTC spot/perp flow that the AP arb mechanically generates, entering before or alongside that flow and exiting as the premium compresses.

The edge is **not** that Zunid captures the spread itself. The edge is that AP flow is **predictable in direction and approximate timing** once the premium signal is observable, and that this flow is large enough (APs transact in creation unit blocks, typically 10,000–50,000 shares ≈ $400K–$2M+ per unit) to move BTC spot price measurably in the short term.

**Null hypothesis to disprove:** ETF premium/discount has no predictive relationship with BTC spot returns over the subsequent 5–60 minutes.

---

## Structural Mechanism

### Why APs Must Act

ETF APs hold contractual agreements with issuers (BlackRock, Fidelity, ARK) granting the exclusive right to create and redeem ETF shares in large blocks. The mechanism:

**Creation (ETF at premium):**
1. AP observes ETF market price > iNAV by more than their all-in transaction cost
2. AP buys BTC on spot market (Coinbase Prime, etc.)
3. AP delivers BTC to ETF custodian (Coinbase Custody for IBIT)
4. AP receives newly created ETF shares at NAV
5. AP sells ETF shares at market price, capturing the spread
6. Net effect: **mechanical BTC spot buying + ETF share selling**

**Redemption (ETF at discount):**
1. AP observes ETF market price < iNAV by more than transaction cost
2. AP buys ETF shares at market price
3. AP delivers ETF shares to issuer
4. AP receives BTC at NAV
5. AP sells BTC on spot market
6. Net effect: **ETF share buying + mechanical BTC spot selling**

### Why the Bound Is Real

This is not a tendency — it is a risk-free arbitrage for APs (modulo execution risk and custodian settlement lag). Any premium above AP all-in costs (~10–20bps for institutional players with Coinbase Prime access) will be arbitraged. The only question is **when** within the trading day the AP executes, and whether they pre-hedge with futures before the spot leg settles.

### The Settlement Lag Creates the Window

BTC ETF creation/redemption settles T+1 or T+2 (depending on issuer). APs typically hedge their BTC exposure immediately in spot or futures markets on the day of the premium signal, **before** the creation unit is formally processed. This means the BTC spot buying/selling pressure occurs **intraday**, often within 15–90 minutes of the premium breaching the AP threshold. This lag is Zunid's entry window.

### Why This Is Uncrowded

- Crypto-native traders do not monitor ETF iNAV spreads; their data infrastructure is exchange-centric
- TradFi ETF arbitrageurs do not trade BTC perps on Hyperliquid
- The signal requires combining two data streams (ETF tick data + BTC spot) that live in separate ecosystems
- The trade is too slow for HFT (minutes, not microseconds) and too fast/manual for most systematic funds

---

## Market Scope

**Primary instruments:**
- IBIT (iShares Bitcoin Trust) — largest AUM, highest liquidity, tightest spreads
- FBTC (Fidelity Wise Origin Bitcoin Fund) — second largest, independent custodian
- ARKB (ARK 21Shares Bitcoin ETF) — smaller but active AP ecosystem

**Execution instrument:**
- BTC-USDC perpetual futures on Hyperliquid (primary)
- BTC spot on Hyperliquid or Coinbase as alternative

**ETH ETFs (ETHA, FETH):** Secondary candidates. Include in backtest but treat as lower priority — ETH ETF AUM is smaller and AP activity less frequent.

---

## Signal Construction

### Step 1: Calculate iNAV

```
iNAV(t) = (BTC_spot_price(t) × BTC_per_share) + cash_per_share - fees_accrued
```

- BTC_per_share: published daily by issuer (e.g., IBIT ≈ 0.000957 BTC/share as of early 2025, declining slowly due to fee accrual)
- BTC_spot_price(t): Coinbase BTC/USD mid-price (1-minute bars)
- cash_per_share and fees_accrued: small, update daily from issuer filings

**Simplified approximation for backtesting:**
```
iNAV(t) ≈ ETF_NAV_previous_close × (BTC_spot(t) / BTC_spot_previous_close)
```
This introduces minor error but is sufficient for signal detection at >10bps thresholds.

### Step 2: Calculate Premium/Discount

```
Premium(t) = (ETF_market_price(t) / iNAV(t)) - 1
```

Expressed in basis points: `Premium_bps(t) = Premium(t) × 10,000`

### Step 3: Signal Threshold

| Signal | Condition | Direction |
|--------|-----------|-----------|
| Long BTC | Premium_bps > +15 | AP creation flow expected → BTC spot bid |
| Short BTC | Premium_bps < -15 | AP redemption flow expected → BTC spot offer |
| No signal | -15 ≤ Premium_bps ≤ +15 | Within AP cost band, no forced flow |

**Threshold rationale:** 15bps chosen as conservative estimate of AP all-in cost (Coinbase Prime spread ~5bps, custody/settlement ~5bps, operational overhead ~5bps). APs with better infrastructure may act at 10bps. Backtest should sweep thresholds from 8–25bps.

### Step 4: Signal Confirmation (Optional Filter)

To reduce false signals from stale ETF quotes at open/close:
- Only trade between 10:00–15:30 ET (avoid open auction noise and closing imbalance)
- Require premium to persist for ≥2 consecutive 1-minute bars before entry
- Require ETF volume in the bar to be above 20-period average (confirms active trading, not stale quotes)

---

## Entry Rules

**Long BTC (ETF Premium Signal):**
1. Premium_bps crosses above +15 on 1-minute bar close
2. Signal persists for 2 consecutive bars (confirmation)
3. ETF bar volume > 20-bar average volume
4. Time is between 10:00–15:30 ET
5. **Enter:** Market buy BTC-USDC perp on Hyperliquid at next bar open

**Short BTC (ETF Discount Signal):**
1. Premium_bps crosses below -15 on 1-minute bar close
2. Signal persists for 2 consecutive bars
3. ETF bar volume > 20-bar average volume
4. Time is between 10:00–15:30 ET
5. **Enter:** Market sell BTC-USDC perp on Hyperliquid at next bar open

**Maximum concurrent positions:** 1 (one directional BTC position at a time; no pyramiding until edge is validated)

---

## Exit Rules

**Primary exit — premium compression:**
- Long: Exit when Premium_bps falls below +5 (AP arb complete, flow exhausted)
- Short: Exit when Premium_bps rises above -5

**Secondary exit — time stop:**
- Exit any position held for >60 minutes regardless of premium level
- Rationale: If AP flow hasn't materialized in 60 minutes, either the AP pre-hedged in futures (not spot), the premium was a data artifact, or the AP is waiting for better execution. The thesis has failed for this instance.

**Hard stop loss:**
- Exit if BTC moves >50bps against position from entry
- Rationale: AP flow at 15bps premium should not require absorbing 50bps adverse move; if this happens, a larger market force is overriding the AP flow

**End-of-day exit:**
- Close all positions by 15:45 ET regardless of status
- Do not hold overnight; ETF premium/discount resets at next open

---

## Position Sizing

**Base position:** 0.5% of portfolio NAV per trade

**Rationale for small size:**
- This is a flow-anticipation trade, not direct convergence; outcome is probabilistic
- AP may pre-hedge in CME futures rather than spot/perps, meaning Hyperliquid sees no flow
- Premium can persist or widen before compressing (AP queue management)
- Until backtest validates, treat as exploratory

**Scaling rule (post-validation only):**
- If backtest Sharpe > 1.5 and win rate > 55%: scale to 1.0% NAV
- If backtest Sharpe > 2.0 and win rate > 60%: scale to 2.0% NAV
- Hard cap: 3% NAV per trade (this is a supporting strategy, not a core position)

**Leverage:** 2x maximum on Hyperliquid perp. The edge is in direction, not magnitude; excessive leverage introduces liquidation risk that overwhelms the small expected edge per trade.

---

## Backtest Methodology

### Data Requirements

| Dataset | Source | Cost | Notes |
|---------|--------|------|-------|
| IBIT 1-min OHLCV | Polygon.io (free tier) or Yahoo Finance | Free | Available from Jan 2024 (ETF launch) |
| FBTC 1-min OHLCV | Polygon.io | Free | Available from Jan 2024 |
| BTC/USD 1-min | Coinbase via CCXT or Kaiko | Free/paid | Use Coinbase as it's the primary AP execution venue |
| BTC perp 1-min | Hyperliquid public API | Free | For execution simulation |
| ETF shares-per-BTC ratio | SEC N-CEN filings / issuer websites | Free | Update monthly |

**Minimum backtest window:** January 2024 – present (~15 months). ETF launched January 11, 2024.

**Note on data quality:** Yahoo Finance 1-minute data has gaps and occasional bad ticks. Polygon.io free tier is more reliable. Cross-validate iNAV calculation against published end-of-day NAV from issuer — if daily error > 5bps, recalibrate the BTC_per_share ratio.

### Backtest Steps

1. **Reconstruct iNAV** at 1-minute frequency using Coinbase BTC mid-price and published BTC_per_share ratios
2. **Calculate Premium_bps** for IBIT and FBTC at each 1-minute bar
3. **Apply signal logic** with 2-bar confirmation filter
4. **Simulate entries** at next-bar open on Hyperliquid BTC perp (use actual Hyperliquid 1-min data for execution price)
5. **Apply exits** in priority order: premium compression → time stop (60 min) → hard stop (50bps) → EOD
6. **Costs:** Deduct 3bps per side (Hyperliquid taker fee ~2bps + slippage estimate 1bps)
7. **Record:** Entry time, exit time, exit reason, P&L in bps, premium at entry, premium at exit

### Parameter Sweep

Test all combinations of:
- Entry threshold: 8, 10, 12, 15, 20, 25 bps
- Confirmation bars: 1, 2, 3
- Time stop: 30, 45, 60, 90 minutes
- Hard stop: 30, 50, 75 bps

Report: number of trades, win rate, average P&L per trade, Sharpe ratio, max drawdown per parameter set. Flag overfitting risk if optimal parameters are extreme outliers.

### Key Metrics to Examine

- **Signal frequency:** How often does premium breach 15bps? (Hypothesis: 2–8 times per trading day across IBIT+FBTC)
- **Premium persistence:** After breaching 15bps, how long does it take to compress to <5bps? Distribution of compression times.
- **BTC spot correlation:** Does BTC spot move in the predicted direction during premium compression windows? (Core causal test)
- **AP activity correlation:** Cross-reference with published ETF creation/redemption data (available daily from issuer websites with 1-day lag) — do high-premium days correlate with large creation units?

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

1. **Backtest Sharpe ratio ≥ 1.5** on out-of-sample data (last 3 months of backtest window held out)
2. **Win rate ≥ 52%** (above coin-flip with meaningful margin given small per-trade edge)
3. **Average trade P&L ≥ 5bps** after costs (must exceed 6bps round-trip cost with buffer)
4. **Minimum 100 trades** in backtest (statistical significance; if signal is too rare, revisit thresholds)
5. **No single parameter dominates:** Results must be reasonably stable across ±20% parameter perturbation
6. **Manual review of 20 random trades:** Confirm signal logic is firing correctly, not on data artifacts
7. **Paper trade for 3 weeks** with full signal logging before any live capital

---

## Kill Criteria

**Immediate kill (same day):**
- Live trade loses >100bps in a single position (data error or model failure)
- Signal fires during known ETF NAV calculation errors (e.g., Coinbase outage)
- Discovery that AP hedging has migrated entirely to CME futures (making spot/perp flow prediction invalid)

**Review and likely kill (within 1 week):**
- 3-week paper trade Sharpe < 0.5
- Win rate in live/paper trading < 45% over ≥30 trades
- Average trade duration consistently hitting time stop (>50% of trades) — suggests AP flow is not materializing in perp markets

**Structural kill:**
- ETF structure changes (e.g., in-kind redemption replaced by cash redemption) — would alter AP flow mechanics
- Hyperliquid BTC perp liquidity degrades significantly (spread > 5bps consistently)
- A major AP (Jane Street, Virtu, Citadel) publicly discloses they hedge exclusively via CME — invalidates the spot/perp flow anticipation thesis

---

## Risks

### Risk 1: AP Pre-Hedges in CME Futures, Not Spot
**Description:** APs may hedge their BTC exposure using CME Bitcoin futures rather than spot or perps. If so, the flow Zunid anticipates never hits BTC spot/Hyperliquid perps.
**Severity:** High — would invalidate the core mechanism
**Mitigant:** Backtest will reveal this: if BTC spot shows no directional move during premium compression windows, this risk is confirmed. Also monitor CME open interest changes on high-premium days.
**Detection:** If CME futures basis widens on premium days (APs buying CME futures), the flow is going to CME, not spot.

### Risk 2: Premium Persists or Widens (AP Queue Delay)
**Description:** APs may batch creation units, executing once per day at close rather than intraday. Premium could persist for hours before compression.
**Severity:** Medium — time stop handles this but at cost of missed P&L
**Mitigant:** Time stop at 60 minutes limits exposure. Backtest will show distribution of compression times.

### Risk 3: iNAV Calculation Error
**Description:** If BTC_per_share ratio is stale or Coinbase price feed has latency, iNAV is miscalculated, generating false signals.
**Severity:** Medium — could cause systematic misfires
**Mitigant:** Daily recalibration of BTC_per_share from issuer website. Cross-check calculated iNAV against published end-of-day NAV. Alert if daily error > 3bps.

### Risk 4: ETF Market Hours vs. Crypto 24/7
**Description:** ETF trades 9:30–16:00 ET. BTC trades 24/7. Signal only exists during ETF hours, limiting trade frequency.
**Severity:** Low — this is a known constraint, not a risk
**Mitigant:** Accept limited trading hours. Do not attempt to extrapolate signal outside ETF hours.

### Risk 5: Crowding as ETF AUM Grows
**Description:** As more TradFi participants become aware of ETF premium signals, the window may compress.
**Severity:** Low-Medium — currently uncrowded but could change
**Mitigant:** Monitor signal frequency and average premium magnitude over time. If average premium at signal breach declines from 15bps toward 8bps, crowding is occurring. Adjust thresholds or retire strategy.

### Risk 6: Regulatory Changes to ETF Structure
**Description:** SEC could mandate changes to creation/redemption mechanics (e.g., mandatory cash creation).
**Severity:** Low probability, high impact
**Mitigant:** Monitor SEC filings. Cash creation would change AP flow mechanics but not eliminate them entirely (APs would still need to buy/sell BTC to hedge cash positions).

---

## Data Sources

| Data | Source | URL | Refresh | Notes |
|------|--------|-----|---------|-------|
| IBIT 1-min price | Polygon.io | polygon.io | Real-time | Free tier: 15-min delay; paid: real-time |
| FBTC 1-min price | Polygon.io | polygon.io | Real-time | Same as above |
| BTC/USD 1-min | Coinbase via CCXT | github.com/ccxt/ccxt | Real-time | Use `coinbasepro` exchange ID |
| BTC perp 1-min | Hyperliquid API | api.hyperliquid.xyz | Real-time | Public, no auth required |
| BTC_per_share ratio | iShares website | ishares.com/IBIT | Daily | Download fund holdings CSV |
| ETF creation/redemption activity | ETF issuer websites | ishares.com, fidelity.com | T+1 | Use for backtest validation only |
| CME BTC futures OI | CME Group | cmegroup.com | Daily | For AP hedging venue detection |

**Total data cost for backtesting:** $0 (free tier Polygon.io sufficient for historical 1-min data; Hyperliquid API is free)

**Total data cost for live trading:** ~$29/month (Polygon.io Starter plan for real-time ETF quotes) or $0 with 15-minute delay (insufficient for live trading — real-time required)

---

## Implementation Notes

### Signal Pipeline (Live)

```
Every 1 minute during 09:30–15:45 ET on US trading days:
  1. Fetch IBIT last trade price (Polygon.io WebSocket)
  2. Fetch BTC/USD mid-price (Coinbase WebSocket)
  3. Calculate iNAV = BTC_price × BTC_per_share
  4. Calculate Premium_bps = (IBIT_price / iNAV - 1) × 10000
  5. Check signal conditions (threshold, confirmation, volume, time)
  6. If signal: submit order to Hyperliquid via API
  7. Monitor exit conditions every 1 minute
  8. Log all signals (fired and unfired) for ongoing validation
```

### Monitoring Dashboard (Minimum Viable)

- Real-time premium/discount chart for IBIT and FBTC
- Signal log with entry/exit timestamps and P&L
- Rolling 20-trade win rate and average P&L
- Alert if iNAV calculation error > 3bps vs. prior close

---

## Open Questions for Backtest

1. What is the empirical distribution of premium compression times? Is 60 minutes the right time stop?
2. Does IBIT or FBTC generate better signals? (IBIT has higher AUM and more AP activity)
3. Is there a time-of-day pattern? (Hypothesis: premiums are larger at open and close due to ETF auction mechanics)
4. Does the signal work better on high-volume BTC days (more AP activity) vs. low-volume days?
5. Is there a lead/lag relationship between ETF premium and BTC spot price, or do they move simultaneously? (If simultaneous, entry is harder; if ETF premium leads by 1–2 minutes, entry is cleaner)
6. Do large creation/redemption days (published T+1) correspond to days with more frequent premium breaches? (Validates the AP flow hypothesis)

---

## Relationship to Existing Zunid Strategies

This strategy is **complementary** to the token unlock short strategy:
- Token unlock shorts are active during specific calendar windows
- ETF AP arb is active during US equity market hours (9:30–15:45 ET)
- Both are structural/mechanical, not pattern-based
- Correlation should be low (different triggers, different timeframes)
- Combined, they increase strategy count without adding correlated risk

**Portfolio allocation:** Treat as a separate sleeve with independent position sizing. Do not allow ETF AP arb positions to interact with token unlock positions in risk calculations.

---

*Next step: Build iNAV reconstruction script using Polygon.io + CCXT. Validate against published end-of-day NAV for accuracy before running full backtest signal sweep.*
