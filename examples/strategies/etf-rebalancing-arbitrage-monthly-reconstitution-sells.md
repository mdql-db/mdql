---
title: "Leveraged Crypto ETF Rebalancing Front-Run"
status: KILLED
mechanism: 7
implementation: 7
safety: 6
frequency: 3
composite: 882
categories:
  - index-rebalance
  - basis-trade
created: "2025-01-31"
pipeline_stage: "Killed at backtest (step 4 of 9)"
killed: "2026-04-05"
kill_reason: "Backtest: 109 trades, 25.7% win rate, -0.29%/trade. Post-BITX period (65 trades) worse than pre-BITX control (18.5% vs 36.4% WR). Larger moves produce worse results — market already fades the rebalancing. Edge vs baseline is negative (-0.26%)."
---

## Hypothesis

When BTC or ETH moves more than 5% intraday, leveraged crypto ETFs (e.g., BITX 2x BTC, ETHU 2x ETH) are contractually obligated to rebalance their futures exposure near the daily close to restore their stated leverage multiple. This rebalance is a predictable, directional, size-calculable flow that must be executed in a narrow time window. A trader who enters a same-direction position in BTC or ETH perps before this window and exits during it captures the price impact of the forced flow.

**Causal chain:**

1. BTC rises 6% by 3:00 PM ET
2. BITX (2x long BTC) now has an effective leverage ratio below 2x — its BTC exposure has grown in dollar terms but not proportionally to NAV
3. To restore 2x leverage, BITX must *buy* additional BTC futures exposure equal to approximately `(L − 1) × r × AUM` where L = 2, r = daily return, AUM = fund NAV at open
4. This buy order hits CME BTC futures between approximately 3:45–4:15 PM ET
5. CME BTC futures price is pulled up; Hyperliquid BTC perp tracks via arbitrage within seconds to minutes
6. Trader who is long BTC perp from 3:00 PM ET exits into this flow at 4:00–4:15 PM ET, capturing the impact

The mechanism is not "ETFs tend to move prices" — it is "this specific fund must execute this specific notional in this specific window or it will misrepresent its leverage multiple to investors, violating its prospectus."

---

## Structural Mechanism (Why This MUST Happen)

Leveraged ETFs are registered investment products with SEC-mandated daily rebalancing obligations. The prospectus of BITX explicitly states it seeks 200% of the *daily* return of BTC. To deliver this, the fund must reset its exposure every trading day.

**The math:**

Let:
- `NAV₀` = fund NAV at prior close
- `r` = BTC return from prior close to current time
- `L` = stated leverage (2 for BITX)

Current gross exposure = `NAV₀ × L × (1 + r)`  
Current NAV = `NAV₀ × (1 + L × r)`  
Required exposure for next day = `Current NAV × L`  
**Rebalance delta = Required − Current = NAV₀ × L × (L − 1) × r`**

For BITX with ~$3B AUM, L=2, r=6%:  
Rebalance = $3B × 2 × 1 × 0.06 = **$360M notional BTC futures to buy**

This is not optional. Failure to rebalance means the fund delivers a leverage ratio other than 2x the next day, which is a material misstatement of the fund's investment objective. The fund's authorized participants and compliance function enforce this daily.

**Why crypto specifically may still have edge:**
- CME BTC futures open interest is ~$15–25B; a $360M forced buy is ~1.5–2.5% of OI — non-trivial
- Crypto perp markets are thinner than equity index futures; price impact propagates faster
- Crypto ETF AUM is growing but the front-running community is smaller than in equity ETF arb

---

## Entry Rules


### Trigger Conditions
- Asset: BTC or ETH (run separately, do not combine into one position)
- Trigger: Spot/perp price has moved ≥ 5% from prior day's 4:00 PM ET close, measured at exactly 3:00 PM ET
- Direction: Long if move is positive; Short if move is negative
- No trade if move is between -5% and +5% at 3:00 PM ET

### Entry
- **Instrument:** BTC-PERP or ETH-PERP on Hyperliquid
- **Entry time:** 3:00 PM ET (market order, or limit within 0.1% of mid)
- **Entry price:** Record for P&L calculation

## Exit Rules

### Exit
- **Primary exit:** 4:10 PM ET market order (10 minutes into the rebalance window, capturing peak impact)
- **Stop exit:** If position moves against entry by 2% from entry price at any point before 4:10 PM ET, exit immediately (hard stop)
- **No overnight holds** — this is a pure intraday flow trade

### Rebalance Size Estimation (for context, not position sizing)
Pull BITX and ETHU AUM from ETF.com or Bloomberg daily before 3:00 PM ET:

```
Estimated rebalance notional = AUM × (L − 1) × L × daily_return_at_3pm
```

Track this number to validate that large estimated rebalances correspond to larger price moves in the exit window.

---

## Position Sizing

- **Base size:** 0.5% of portfolio NAV per trade
- **Scale up:** If estimated rebalance notional > $200M (BITX) or > $50M (ETHU), increase to 1.0% of NAV
- **Maximum:** 1.5% of NAV in any single rebalance trade
- **Rationale:** This is a short-duration, high-frequency-of-occurrence trade with moderate win rate. Small size per trade, high trade count. Do not size up based on conviction — the edge is thin and crowded.
- **Leverage:** 2–3x on the perp position (i.e., notional = 1–3% of NAV). Do not exceed 3x — the stop is only 2% and you cannot afford a liquidation cascade.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Frequency | Notes |
|---|---|---|---|
| BTC spot price | Coinbase API or Kaiko | 1-minute OHLCV | 2022–present |
| ETH spot price | Coinbase API or Kaiko | 1-minute OHLCV | 2022–present |
| BITX daily NAV and AUM | ETF.com or SEC EDGAR N-CEN filings | Daily | BITX launched June 2023 |
| ETHU daily NAV and AUM | ETF.com or SEC EDGAR | Daily | Check launch date |
| CME BTC futures (front month) | CME DataMine or Quandl | 1-minute | For rebalance window price action |
| Hyperliquid BTC-PERP | Hyperliquid API | 1-minute | For execution simulation |

**API endpoints:**
- Coinbase: `https://api.exchange.coinbase.com/products/BTC-USD/candles`
- Hyperliquid: `https://api.hyperliquid.xyz/info` (candleSnapshot endpoint)
- ETF.com AUM: scrape `https://www.etf.com/BITX` daily AUM field
- SEC EDGAR: `https://efts.sec.gov/LATEST/search-index?q=%22BITX%22&dateRange=custom`

### Backtest Period
- Primary: June 2023 (BITX launch) to present — ~18 months of live data
- Extended: Use ProShares BITO (launched Oct 2021) as proxy for pre-BITX period, noting BITO is 1x not 2x

### Simulation Rules
1. At 3:00 PM ET each day, check if |BTC return from prior close| ≥ 5%
2. If yes, record entry price (use 3:00 PM 1-minute close)
3. Record exit price at 4:10 PM ET 1-minute close
4. Apply 0.05% slippage each way (conservative for Hyperliquid)
5. Apply 0.01% funding cost (8hr rate prorated to 1hr)
6. Apply hard stop: if price moves 2% against entry between 3:00–4:10 PM ET, exit at stop price

### Metrics to Calculate
- Win rate (% of trades profitable)
- Average return per trade (net of costs)
- Sharpe ratio (annualized, using daily trade returns)
- Maximum drawdown (consecutive losing trades)
- **Key diagnostic:** Correlation between estimated rebalance notional and trade P&L — if the structural mechanism is real, larger estimated rebalances should produce larger profits
- **Segmentation:** Separate results for BTC up days, BTC down days, ETH up days, ETH down days — the mechanism may be asymmetric

### Baseline Comparison
- Compare against: random entry at 3:00 PM ET, exit at 4:10 PM ET (same direction as daily move, no ETF filter) — this isolates whether the ETF rebalance adds anything beyond simple momentum in the last hour

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Win rate ≥ 55%** on trades where estimated rebalance notional > $100M
2. **Net Sharpe ≥ 1.0** (after slippage and funding) on the full backtest period
3. **Positive correlation** (r > 0.2) between estimated rebalance notional and trade P&L — the structural mechanism must be detectable in the data
4. **Outperforms baseline** (random 3–4 PM momentum) by ≥ 0.3% average return per trade
5. **Maximum drawdown < 8%** of allocated capital across the backtest period
6. **Minimum 30 qualifying trade events** in the backtest — if BTC only moved 5%+ on 15 days since BITX launch, the sample is too small and we wait for more data before going live

---

## Kill Criteria

Abandon the strategy (in paper trade or live) if:

1. **10 consecutive losing trades** — suggests the front-running community has fully arbitraged the signal
2. **Win rate drops below 45%** over any rolling 30-trade window
3. **BITX or ETHU AUM drops below $500M** — rebalance notional becomes too small to move the market
4. **CME introduces extended trading hours** that spread the rebalance window — destroys the predictable time window
5. **Correlation between rebalance notional and P&L turns negative** over 20+ trades — the mechanism has inverted (market now fades the rebalance)
6. **Regulatory change** requiring intraday rebalancing or changing the rebalance window timing

---

## Risks

### Primary Risks

**1. Crowding (highest risk)**
This strategy is well-documented in equity ETF literature (Cheng & Madhavan 2009, "The Dynamics of Leveraged and Inverse Exchange-Traded Funds"). Every quant fund with an ETF desk knows this. The crypto version is less crowded but growing. The edge may already be fully priced in by the time BITX AUM is large enough to matter.
*Mitigation:* The correlation diagnostic in the backtest will reveal if the signal is already dead.

**2. CME vs. Perp Basis Risk**
BITX rebalances in CME BTC futures, not Hyperliquid perps. The arb between CME and perps is fast but not instantaneous. During volatile periods, the basis can widen and the perp may not fully capture the CME price move.
*Mitigation:* Track CME-perp basis during the 3:00–4:15 PM ET window in the backtest. If basis regularly widens > 0.3% during rebalance windows, the strategy needs to trade CME futures directly (requires different account setup).

**3. AUM Estimation Lag**
ETF.com AUM figures are T+1 — you see yesterday's AUM, not today's. If there are large inflows/outflows on the trigger day, your rebalance size estimate is wrong.
*Mitigation:* Use a conservative AUM estimate (prior day × 0.9 as floor). The direction of the rebalance is still correct even if the size estimate is off.

**4. Stop-Out Risk on Volatile Days**
The 2% hard stop is designed to prevent catastrophic loss, but on days with 5%+ moves, intraday volatility is high. The position may stop out and then the rebalance move happens anyway — you miss the profit and take the loss.
*Mitigation:* Analyze stop-out frequency in backtest. If stops trigger on > 20% of trades, widen to 2.5% or narrow the entry window to 3:30 PM ET (less time for adverse moves before rebalance).

**5. Rebalance Window Uncertainty**
The ETF prospectus does not specify an exact rebalance time — it says "near the close." In practice, BITX uses CME 4:00 PM ET settlement, but the actual order flow may be spread across 3:45–4:15 PM ET. The exit at 4:10 PM ET may be too early or too late.
*Mitigation:* In the backtest, test exit times at 3:50, 4:00, 4:10, and 4:15 PM ET separately and select the optimal window. Document which window is used and monitor for drift.

**6. Small Sample Size**
BTC has moved ≥ 5% intraday on roughly 15–25% of trading days historically, but since BITX launched (June 2023), the market has been relatively calm. The backtest may have only 40–60 qualifying events — borderline for statistical significance.
*Mitigation:* Apply bootstrap resampling to estimate confidence intervals on Sharpe and win rate. Do not go live if confidence interval on win rate includes 50%.

---

## Data Sources

| Source | URL | What to Pull |
|---|---|---|
| Hyperliquid API | `https://api.hyperliquid.xyz/info` | BTC/ETH perp 1-min candles via `candleSnapshot` |
| Coinbase Exchange API | `https://api.exchange.coinbase.com/products/BTC-USD/candles` | BTC spot 1-min OHLCV |
| ETF.com BITX page | `https://www.etf.com/BITX` | Daily AUM (scrape or manual) |
| ETF.com ETHU page | `https://www.etf.com/ETHU` | Daily AUM |
| SEC EDGAR full-text search | `https://efts.sec.gov/LATEST/search-index?q=BITX` | N-CEN and 497 filings for AUM confirmation |
| ProShares BITX fact sheet | `https://www.proshares.com/our-etfs/leveraged-and-inverse/bitx` | Prospectus rebalance methodology |
| CME Group DataMine | `https://www.cmegroup.com/market-data/datamine-historical-data.html` | CME BTC futures 1-min (paid, ~$50/month) |
| Kaiko (alternative) | `https://www.kaiko.com` | BTC/ETH OHLCV across exchanges (paid) |
| Coin Metrics Community | `https://community-api.coinmetrics.io/v4` | Free BTC/ETH reference rates |

**Priority data pull order:**
1. Hyperliquid API (free, direct execution venue)
2. Coinbase API (free, liquid spot reference)
3. ETF.com scrape for BITX AUM (free, T+1 lag acceptable)
4. CME DataMine only if basis risk analysis requires it (paid)

---

## Open Questions for Researcher

1. Does BITX use a single market-on-close order or VWAP execution for its rebalance? The answer changes the optimal exit time significantly.
2. Has anyone published realized price impact data for BITX rebalances specifically? Check Bloomberg ETFS function or academic preprints on SSRN.
3. Is there a crypto-native leveraged ETF (not futures-based) that rebalances on-chain — e.g., Index Coop's ETH2x-FLI on Ethereum? If so, the rebalance is fully transparent on-chain and the edge may be cleaner (though gas costs complicate execution).
4. What is the current BITX AUM trend? If it's growing toward $5B+, the rebalance notional becomes large enough to be genuinely market-moving and the strategy becomes more attractive.
