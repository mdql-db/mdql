---
title: "Closed-End Crypto Fund Discount-to-NAV Collapse (ETF Conversion Arbitrage)"
status: HYPOTHESIS
mechanism: 8
implementation: 5
safety: 7
frequency: 1
composite: 280
categories:
  - basis-trade
  - regulatory
  - exchange-structure
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a closed-end crypto fund (CEF) trades at a persistent discount to NAV due to the absence of a redemption mechanism, and a credible ETF conversion event is pending, the discount **must** converge to zero upon conversion because arbitrageurs gain the contractual right to redeem shares at NAV. The trade is: long the discounted CEF, short equivalent crypto notional on perps to isolate the discount-convergence return from directional crypto exposure.

**Causal chain:**

1. CEF structure prohibits redemptions → discount persists because supply can enter but not exit at NAV
2. Regulatory body approves ETF conversion → redemption mechanism switches ON at a specific date
3. Any discount > transaction costs becomes immediately riskless arbitrage for authorized participants (APs)
4. APs buy discounted shares, redeem at NAV, pocket spread → discount collapses to near zero
5. Long CEF / short crypto position captures the spread as pure P&L, delta-neutral to crypto price

The edge is not "discounts tend to close" — it is "discounts **must** close because a contractual redemption right now exists." The mechanism is identical to why closed-end fund discounts collapse at open-ending events in traditional finance.

---

## Structural Mechanism (WHY This MUST Happen)

**The lock-in problem and its removal:**

Closed-end crypto funds (GBTC pre-2024, ETHE pre-2024, Purpose Bitcoin ETF in Canada, 3iQ funds, etc.) issue shares via private placement or public offering but provide **no redemption window**. Secondary market price is set by supply/demand among retail investors, not by NAV. When sentiment is negative or the fund is out of favor, shares trade below NAV with no mechanism to force convergence.

**The conversion trigger:**

ETF conversion grants APs the right to create/redeem shares at NAV daily. This is a **binary structural change**:

- Pre-conversion: No arbitrage possible. Discount can widen indefinitely.
- Post-conversion: Any discount > AP transaction costs (~10–30 bps) is immediately arbitraged away by APs who buy shares on exchange and redeem for underlying crypto at NAV.

This is not probabilistic. The AP redemption mechanism is a **contractual right** embedded in the ETF prospectus. Discounts above transaction costs cannot persist because they represent free money for APs with redemption access.

**Why the discount doesn't fully close pre-conversion:**

The market prices in conversion probability, not certainty. A fund at a 15% discount with 60% conversion probability might trade at a 9% discount (15% × 40% residual risk). As conversion probability approaches 1.0, the discount must approach zero. The final convergence is guaranteed; the timing is not.

**Historical confirmation (not the edge, but validation):**

- GBTC: Traded at discounts as wide as 49% (December 2022). Closed to ~0% within weeks of January 2024 ETF conversion approval.
- ETHE: Traded at ~24% discount pre-conversion. Closed to ~0% post-conversion (July 2024).
- These are not backtests — they are proof-of-mechanism events.

---

## Target Universe — Finding the Next Candidate

Priority screening criteria (in order):

1. **Structure:** Must be a closed-end vehicle (no existing redemption mechanism). Trusts, grantor trusts, closed-end funds — not open-end funds.
2. **Discount:** Currently trading >5% below NAV. Tighter discounts reduce risk/reward.
3. **Regulatory credibility:** Active filing with a credible regulator (SEC, OSC, SFC, FCA, BaFin). Not a rumor — a docketed application.
4. **Underlying asset:** BTC or ETH preferred (liquid perp markets for hedging). Altcoin funds are harder to hedge cleanly.
5. **Liquidity:** Minimum $50M AUM. Illiquid CEFs have wide bid/ask that eats the discount.

**Current candidates to monitor (as of early 2025):**

| Vehicle | Jurisdiction | Regulator | Discount (approx) | Status |
|---|---|---|---|---|
| Grayscale Bitcoin Mini Trust | US | SEC | ~0% (already ETF) | Closed |
| Osprey Bitcoin Trust (OBTC) | US | SEC | Monitor | Pending/uncertain |
| 3iQ Bitcoin ETF (QBTC) | Canada | OSC | Near 0% | Already converted |
| CoinShares Physical Bitcoin | Europe | Various | Near 0% | ETP structure |
| Hong Kong Bitcoin/ETH ETFs | HK | SFC | Monitor spot vs NAV | New, monitor |
| Any new Grayscale altcoin trusts | US | SEC | Check EDGAR | Active pipeline |

**Immediate action:** Screen Grayscale's remaining trust products (GDLC — Digital Large Cap Fund, GFUND, individual altcoin trusts) for current discount levels and conversion filing status.

---

## Entry/Exit Rules

### Entry Conditions (ALL must be met)

1. **Discount threshold:** CEF trading at ≥5% discount to NAV at time of entry
2. **Regulatory trigger:** One of the following must be true:
   - Active conversion application filed with regulator (docketed, not rumored)
   - Regulator has issued a comment letter (indicates active review)
   - Court ruling or regulatory guidance has materially increased conversion probability (e.g., Grayscale v. SEC ruling in August 2023 was the entry signal for GBTC)
3. **Hedge availability:** Liquid perp market exists for the underlying asset on Hyperliquid or equivalent (BTC, ETH — others case-by-case)
4. **Liquidity check:** CEF average daily volume ≥ $1M (to allow position entry/exit without moving the market)
5. **Borrow/short availability:** Confirm perp funding rates are not extreme (>50% annualized) before entry — high funding costs erode the carry

### Entry Execution

- **Leg 1 (Long CEF):** Buy CEF shares on exchange (e.g., OTC Markets, NYSE Arca, TSX) at market or limit within 0.5% of mid
- **Leg 2 (Short hedge):** Short BTC or ETH perp on Hyperliquid equal to the NAV-equivalent crypto notional of the CEF position
  - Hedge ratio = (CEF shares × NAV per share) / crypto spot price
  - Example: 1,000 GBTC shares × $30 NAV = $30,000 notional → short $30,000 BTC perp
- **Sizing:** See Position Sizing section

### Exit Conditions

**Primary exit (target):**
- Discount narrows to <1% of NAV → close both legs simultaneously
- Post-conversion: Close within 5 business days of conversion effective date (APs will have closed it anyway)

**Time-based exit:**
- If conversion does not occur within 18 months of entry, reassess and consider exit regardless of discount level (opportunity cost and funding costs accumulate)

**Partial exit:**
- If discount narrows from 15% to 5% pre-conversion (market pricing in higher probability), consider taking 50% off to lock in partial gains and reduce regulatory timing risk

**Kill switch (see Kill Criteria):**
- Conversion rejected or withdrawn → exit both legs immediately at market

### Rebalancing

- Rebalance the short hedge weekly or when crypto price moves >10% from entry (to maintain delta neutrality)
- Rebalancing formula: New short notional = (CEF shares held × current NAV per share) / current crypto spot price

---

## Position Sizing

**Base sizing:**

- Maximum 15% of portfolio per position (this is a binary regulatory event — concentration risk is real)
- If multiple candidates exist simultaneously, cap total exposure at 30% of portfolio across all CEF arb positions

**Scaling by conviction:**

| Scenario | Max Allocation |
|---|---|
| Active SEC filing + court ruling in favor | 15% |
| Active filing, no court ruling | 10% |
| Filing rumored, not docketed | 5% (speculative) |
| No filing, pure discount play | 0% (do not enter) |

**Funding cost adjustment:**

- Calculate annualized cost of carry on the short perp leg (funding rate × 365)
- If funding cost > (current discount / expected months to conversion) × 12, the trade is not profitable — do not enter
- Example: 10% discount, 12-month timeline, 8% annualized funding cost → 10% gain vs. 8% cost → marginal, only enter if conviction is high

**Leverage:** None on the CEF leg. The perp short is sized to hedge, not to amplify. Use 1x on the perp leg.

---

## Backtest Methodology

### What to Backtest

Since GBTC and ETHE are the only completed examples of this exact mechanism, the backtest is necessarily a case study analysis + forward-looking framework, not a statistical backtest over many observations. Be honest: N=2 for completed US conversions.

**Backtest approach:**

1. **GBTC case study (2022–2024):** Reconstruct the trade as if entered on August 29, 2023 (day of Grayscale v. SEC ruling), exited January 11, 2024 (ETF conversion date)
2. **ETHE case study (2023–2024):** Reconstruct the trade as if entered on May 23, 2024 (SEC approval of ETH ETF in principle), exited July 23, 2024 (conversion date)
3. **Canadian precedent (QBTC.U):** 3iQ Bitcoin ETF conversion in 2021 — reconstruct if data available

### Data Required

| Data Point | Source | Notes |
|---|---|---|
| GBTC daily price | Yahoo Finance (`GBTC`) or Bloomberg | Available from 2015 |
| GBTC NAV per share | Grayscale website (historical) or Bloomberg `GBTC NAV` | Published daily |
| ETHE daily price | Yahoo Finance (`ETHE`) | |
| ETHE NAV per share | Grayscale website (historical) | |
| BTC spot price | CoinGecko API, Binance API | For hedge P&L |
| ETH spot price | CoinGecko API, Binance API | |
| BTC perp funding rates | Binance/Bybit historical funding rate data | For carry cost calculation |
| SEC filing dates | SEC EDGAR full-text search (`efts.sec.gov/LATEST/search-index?q="Grayscale"&dateRange=custom`) | |
| Court ruling dates | Public record | |

### Metrics to Calculate

For each case study:

1. **Entry discount:** % discount to NAV at entry date
2. **Exit discount:** % discount to NAV at exit date
3. **Gross return:** (Entry discount − Exit discount) as % of entry price
4. **Hedge P&L:** Gain/loss on short perp leg (crypto price change × short notional)
5. **Funding cost:** Sum of daily funding payments on short perp leg over holding period
6. **Net return:** Gross return + Hedge P&L − Funding cost − Transaction costs (est. 0.1% per leg)
7. **Holding period:** Days from entry to exit
8. **Annualized return:** Net return / (holding period / 365)
9. **Max drawdown:** Maximum unrealized loss during holding period (discount widening events)

### Sensitivity Analysis

Run the backtest across multiple hypothetical entry dates:
- Entry at 30 days before ruling/catalyst
- Entry at ruling/catalyst date
- Entry at 60 days after ruling (discount partially closed)

This shows how sensitive returns are to entry timing.

### Baseline Comparison

Compare net return to:
- Simply holding BTC/ETH over the same period (directional return)
- 3-month T-bill rate (risk-free rate)
- The strategy should show positive return even in periods when BTC/ETH declined (proving the hedge works)

---

## Go-Live Criteria

Before paper trading any new candidate, the backtest must show:

1. **Positive net return in both GBTC and ETHE case studies** after funding costs and transaction costs
2. **Hedge effectiveness:** Correlation between CEF price and crypto spot must be >0.85 during holding period (confirming the hedge is valid)
3. **Drawdown tolerance:** Maximum intra-trade discount widening must be survivable at proposed position size (i.e., if discount widens 10% before closing, the position doesn't breach risk limits)
4. **Funding cost model:** Annualized funding cost on BTC/ETH perp shorts must be calculable and must not exceed 60% of the expected discount capture in the base case
5. **Candidate identification process:** Must have a documented, repeatable process for monitoring SEC EDGAR, OSC filings, and SFC filings for new conversion applications before paper trading begins

**Paper trading duration:** Minimum 60 days of monitoring a live candidate (even if not yet at entry threshold) before committing real capital.

---

## Kill Criteria

**Immediate kill (exit both legs at market, same day):**

- Conversion application formally withdrawn by the fund manager
- Regulatory rejection with no appeal path (e.g., SEC denial + fund announces no further action)
- Fund manager announces liquidation of the trust (different from conversion — liquidation may not converge to NAV cleanly)
- Underlying asset (BTC/ETH) becomes un-hedgeable (exchange halts, perp market suspended)

**Slow kill (exit within 5 business days):**

- Conversion timeline extends beyond 24 months from entry with no new catalyst
- Funding costs on short perp leg exceed 80% annualized (carry becomes prohibitive)
- CEF AUM drops below $25M (liquidity risk — APs may not bother with small funds)
- A competing fund with the same underlying converts first, reducing regulatory pressure on the target fund

**Strategy-level kill (abandon the strategy entirely):**

- Both GBTC and ETHE case study backtests show negative net returns after costs
- No new conversion candidates emerge within 24 months (strategy becomes dormant, not killed — revisit when candidates appear)

---

## Risks

### Risk 1: Regulatory Timing Uncertainty (HIGH)
The convergence is guaranteed post-conversion, but the conversion date is not guaranteed. Regulatory approval can be delayed years (GBTC waited 10 years). Funding costs accumulate during the wait. **Mitigation:** Only enter when a specific, docketed filing exists with a defined review timeline. Do not enter on rumors.

### Risk 2: Discount Widening Before Convergence (MEDIUM)
Even with a pending conversion, the discount can widen if: (a) crypto prices crash and sentiment deteriorates, (b) the fund manager does something to reduce conversion probability, (c) broader market stress. **Mitigation:** Position sizing limits (15% max), stop-loss at 2× the entry discount (e.g., if entered at 15% discount, exit if discount reaches 30%).

### Risk 3: Funding Rate Blowout on Short Perp (MEDIUM)
If BTC/ETH enters a strong bull market, perp funding rates can reach 100%+ annualized. This eats the discount capture. **Mitigation:** Monitor funding rates weekly. If annualized funding cost exceeds 50%, reduce hedge ratio or exit.

### Risk 4: Hedge Ratio Drift (LOW-MEDIUM)
If crypto price moves significantly, the hedge becomes misaligned. A 20% BTC rally means the short hedge is now under-sized relative to the CEF's NAV. **Mitigation:** Rebalance weekly or on >10% crypto price moves.

### Risk 5: Liquidity Risk on CEF Exit (MEDIUM)
Small CEFs may have wide bid/ask spreads and thin order books. Exiting a large position may move the price against you. **Mitigation:** Minimum $1M ADV requirement at entry. Size position to be <5% of ADV.

### Risk 6: Conversion Structure Risk (LOW but IMPORTANT)
Not all conversions are equal. Some conversions may be to an ETP structure that still doesn't allow full AP redemption (e.g., some European ETP structures have limited redemption). **Mitigation:** Read the prospectus. Confirm AP redemption mechanism is identical to a standard US ETF before entry.

### Risk 7: N=2 Problem (STRUCTURAL)
This strategy has only two completed US examples. The backtest is a case study, not a statistical sample. The edge is mechanically sound but empirically thin. **Mitigation:** Treat this as a high-conviction single-trade strategy when candidates appear, not a systematic strategy. Require strong structural confirmation before each trade.

### Risk 8: Front-Running by Larger Players (MEDIUM)
Institutional arb desks (Citadel, Jane Street) will identify the same trade. They may enter earlier and push the discount to near-zero before retail/smaller funds can enter profitably. **Mitigation:** Monitor filings programmatically to identify candidates early. The edge is in early identification, not speed of execution.

---

## Data Sources

### NAV and Price Data
- **Grayscale fund pages:** `https://www.grayscale.com/funds` — daily NAV published for each trust
- **SEC N-2 filings (closed-end fund registration):** `https://efts.sec.gov/LATEST/search-index?q=%22crypto%22+%22closed-end%22&dateRange=custom&startdt=2024-01-01&forms=N-2`
- **Yahoo Finance:** `https://finance.yahoo.com/quote/GBTC/history` — historical price data (free)
- **Bloomberg Terminal:** `GBTC US Equity NAV` field — most reliable NAV source (paid)

### Regulatory Filing Monitoring
- **SEC EDGAR full-text search:** `https://efts.sec.gov/LATEST/search-index?q=%22bitcoin+trust%22+%22conversion%22&forms=S-1,N-14`
- **SEC EDGAR company search for Grayscale:** `https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&company=grayscale&CIK=&type=&dateb=&owner=include&count=40&search_text=`
- **OSC (Ontario Securities Commission) filings:** `https://www.osc.ca/en/securities-law/instruments-rules-policies/5/51-102/continuous-disclosure-obligations`
- **SFC (Hong Kong) product authorization:** `https://www.sfc.hk/en/Regulatory-functions/Products/Authorization-of-investment-products`

### Crypto Price and Funding Rate Data
- **CoinGecko API (free):** `https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=365`
- **Binance historical funding rates:** `https://www.binance.com/en/futures/funding-history/0` (UI) or `https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=1000` (API)
- **Hyperliquid funding rate history:** `https://hyperliquid.xyz` — available via their API for perp positions
- **Bybit funding rate history:** `https://bybit-exchange.github.io/docs/v5/market/history-fund-rate`

### Candidate Screening Tools
- **Closed-end fund screener (CEFConnect):** `https://www.cefconnect.com/find-a-fund` — filter by asset class "Commodities/Natural Resources" or custom; check discount/premium column
- **Morningstar CEF data:** `https://www.morningstar.com/closed-end-funds` — discount/premium to NAV screening

---

## Implementation Checklist (Pre-Backtest)

- [ ] Pull GBTC daily price and NAV from August 2023 to January 2024
- [ ] Pull ETHE daily price and NAV from May 2024 to July 2024
- [ ] Pull BTC and ETH spot prices for same periods
- [ ] Pull BTC and ETH perp funding rates for same periods (Binance historical)
- [ ] Calculate net return for both case studies with full cost accounting
- [ ] Build discount monitoring spreadsheet for current Grayscale trust products
- [ ] Set up EDGAR alert for new N-2, S-1, N-14 filings mentioning "bitcoin trust" or "ethereum trust"
- [ ] Document current discount levels for GDLC and any remaining Grayscale altcoin trusts
- [ ] Confirm Hyperliquid perp availability and funding rate history for BTC and ETH
