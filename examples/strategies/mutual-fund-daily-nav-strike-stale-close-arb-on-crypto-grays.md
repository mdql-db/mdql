---
title: "Mutual Fund Daily NAV Strike — Stale Close Arb on Crypto Grayscale Trusts"
status: HYPOTHESIS
mechanism: 4
implementation: 3
safety: 5
frequency: 8
composite: 480
categories:
  - basis-trade
  - exchange-structure
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Grayscale's remaining closed-end trusts and certain European crypto ETPs calculate NAV using a fixed daily reference price (typically a 4:00 PM ET crypto benchmark). When the underlying asset moves materially *after* the NAV fixing window but *before* trust shares trade the next session, the trust's published NAV becomes stale relative to spot. This creates a predictable, temporary discount or premium in the trust's share price at the next open — one that must mechanically compress as the market prices in the overnight move. A delta-hedged position (long/short the trust, offset with an opposing perp position) captures this compression without taking directional crypto exposure.

The edge is **not** that prices tend to revert. The edge is that the trust's NAV is *contractually fixed* at a specific timestamp, creating a known, calculable gap between stale NAV and live spot that the market must eventually close — either intraday as traders arbitrage it, or at the next NAV strike at the latest.

**Null hypothesis to disprove:** The trust share price at open already fully reflects the overnight crypto move, leaving no exploitable gap.

---

## Structural Mechanism

### 2a. Why the gap exists

Grayscale closed-end trusts (LTCN, GDLC, ETCG, BCHG, and the Bitcoin Mini Trust BTC) are **not** ETFs with authorized participant (AP) creation/redemption. They are closed-end vehicles. There is no AP mechanism to arbitrage the trust price back to NAV intraday. The only price discovery mechanism is secondary market trading of trust shares on OTC markets (OTCQX) or, for newer products, on NYSE Arca.

The NAV is struck using the **CoinDesk Bitcoin Price Index (XBX)** or equivalent benchmark at **4:00 PM ET** daily. Crypto markets trade 24/7. If BTC moves +4% between 4:00 PM ET and 9:30 AM ET the next morning, the trust's *last published NAV* is stale by that full 4%. The trust share price at open will partially, but not necessarily fully, reflect this — because:

1. Retail holders of trust shares are not monitoring overnight crypto moves
2. OTC market makers for these trusts are not the same desks running crypto perp books
3. The trust's premium/discount to NAV is a noisy, poorly-followed metric for most participants

### 2b. The convergence guarantee

This is the weakest part of the structural argument and must be stated honestly. Unlike an ETF with AP redemption, there is **no hard contractual mechanism** forcing the trust share price to converge to NAV on any given day. The convergence is:

- **Soft structural:** Rational traders will eventually buy a trust trading at a 10% discount to a calculable NAV, compressing the gap
- **Time-bounded:** The gap is fully resolved at the *next* NAV strike (next 4 PM ET), when a new NAV is published incorporating the current price
- **Not guaranteed intraday:** The trust could trade at a persistent discount/premium for days (as GBTC famously did for months)

This is why the score is 5/10, not 8/10. The mechanism is real; the convergence timeline is not contractually guaranteed.

### 2c. Why the hedge matters

Without the perp hedge, this is a leveraged directional crypto bet dressed up as arb. The hedge is essential:

- Long trust + short BTC perp = isolated exposure to the trust's discount/premium compression, not to BTC price direction
- The hedge ratio must account for the trust's underlying asset composition (GDLC is multi-asset; LTCN is LTC-only, etc.)
- Funding costs on the perp are a real drag and must be modeled

---

## Universe Definition

### Eligible instruments (as of 2026)

| Trust | Ticker | Underlying | Exchange | Liquidity | Shortable? |
|-------|--------|------------|----------|-----------|------------|
| Grayscale Bitcoin Mini Trust | BTC | BTC | NYSE Arca | Medium | Likely yes |
| Grayscale Ethereum Mini Trust | ETH | ETH | NYSE Arca | Medium | Likely yes |
| Grayscale Litecoin Trust | LTCN | LTC | OTCQX | Thin | Uncertain |
| Grayscale Bitcoin Cash Trust | BCHG | BCH | OTCQX | Thin | Uncertain |
| Grayscale Digital Large Cap Fund | GDLC | BTC/ETH/SOL/XRP/ADA | OTCQX | Thin | Uncertain |
| Grayscale Ethereum Classic Trust | ETCG | ETC | OTCQX | Thin | Uncertain |

**European ETPs (secondary universe — harder to hedge):**

| Product | Issuer | Exchange | Notes |
|---------|--------|----------|-------|
| BTCE | ETC Group | Xetra | Physical BTC, daily NAV |
| ABTC | 21Shares | SIX/Euronext | Daily NAV, EUR-denominated |
| VBTC | VanEck | Euronext | Daily NAV |

European ETPs are lower priority because: (a) hedging with USD-denominated perps introduces FX basis, (b) trading hours overlap with US crypto session is limited, (c) AP mechanisms exist for some, reducing the gap.

### Immediate liquidity audit required (pre-backtest gate)

Before any backtest, confirm for each instrument:
- Average daily volume (ADV) in USD
- Borrow availability and cost at prime brokers (Interactive Brokers, Schwab)
- Bid-ask spread at open vs. intraday
- Whether AP redemption exists (if yes, exclude — gap will be arbed away mechanically)

**Minimum ADV threshold for inclusion:** $500K/day. Anything below this makes position sizing trivially small and transaction costs punishing.

---

## Entry Rules

### Trigger condition

Calculate the **Overnight Crypto Move (OCM)** for each trust's underlying:

```
OCM = (Spot_price_at_930am_ET - NAV_fixing_price_at_4pm_ET_prior_day) / NAV_fixing_price_at_4pm_ET_prior_day
```

**Entry threshold:** |OCM| ≥ 3.0%

This threshold is chosen to exceed typical bid-ask spreads, borrow costs, and perp funding. It is a hypothesis — backtest must validate the optimal threshold.

### Entry direction

**Scenario A — BTC rallies overnight (OCM > +3%):**
- Trust NAV is stale-low relative to spot
- Trust shares should open at a discount to "true" NAV (or a smaller premium than warranted)
- Action: **Buy trust shares at open + Short BTC perp** (delta-neutral)
- Profit from: trust share price rising to reflect the overnight move

**Scenario B — BTC drops overnight (OCM < -3%):**
- Trust NAV is stale-high relative to spot
- Trust shares should open at a premium to "true" NAV (or a smaller discount than warranted)
- Action: **Short trust shares at open + Long BTC perp** (delta-neutral)
- Profit from: trust share price falling to reflect the overnight move
- **Constraint:** Only executable if shares are borrowable. OTCQX trusts may not be.

### Entry timing

- Enter trust position at **market open (9:30 AM ET)** using a limit order within 0.5% of the prevailing bid/ask midpoint
- Enter perp hedge simultaneously on Hyperliquid at 9:30 AM ET
- Do **not** chase: if limit order not filled within first 5 minutes, cancel and stand down for the day

### Hedge ratio calculation

For single-asset trusts (LTCN, BCHG, BTC mini, ETH mini):

```
Perp_notional = Trust_shares_purchased × Trust_NAV_per_share × (1 / Trust_underlying_per_share_ratio)
```

For GDLC (multi-asset):
- Decompose into BTC and ETH components using published portfolio weights
- Hedge BTC component with BTC perp, ETH component with ETH perp
- Residual SOL/XRP/ADA exposure is unhedged (too small, illiquid perps for some)
- This makes GDLC a lower-quality candidate

---

## Exit Rules

### Primary exit: Intraday reversion

- Monitor the trust's **live premium/discount to updated spot NAV** throughout the session
- Exit when the premium/discount compresses to ≤ 0.5% (transaction cost floor)
- Unwind trust position first, then close perp hedge

### Secondary exit: End-of-day hard stop

- If position has not reverted by **3:45 PM ET**, exit both legs regardless
- Rationale: Holding overnight reintroduces the stale NAV problem in reverse; also, perp funding accrues

### Stop-loss exit

- If the trust's premium/discount *widens* by more than 2× the entry gap (i.e., the market is moving against the thesis), exit immediately
- This indicates either: (a) the market is pricing in something we don't know, or (b) liquidity has dried up and we're being adversely selected

### Do not hold overnight

This is a strict rule. The strategy is explicitly an intraday convergence trade. Overnight holding converts it into a directional trust discount/premium bet, which is a different (and worse) strategy.

---

## Position Sizing

### Base sizing

```
Max_position_size = min(
    0.5% of portfolio NAV,
    10% of trust's average daily volume,
    $50,000 notional per trade
)
```

The ADV constraint is critical — these are illiquid instruments. Moving 10% of ADV will cause significant market impact, especially at open.

### Scaling with signal strength

| OCM magnitude | Position scalar |
|---------------|----------------|
| 3–5% | 0.5× base |
| 5–8% | 1.0× base |
| >8% | 1.5× base (cap at hard limit) |

### Perp hedge sizing

Perp notional = trust notional × delta of underlying per trust share (from prospectus). Rebalance the hedge if the underlying moves >2% intraday (gamma of the basis position).

---

## Backtest Methodology

### Data requirements

| Data series | Source | Notes |
|-------------|--------|-------|
| Trust daily NAV | Grayscale website (grayscale.com/funds) | Published daily, free |
| Trust share OHLCV | Yahoo Finance, Bloomberg | Adjust for any splits |
| BTC/ETH/LTC/BCH spot | CoinGecko, Kaiko, or Tardis | Need 4 PM ET and 9:30 AM ET snapshots |
| Perp funding rates | Hyperliquid API, Coinglass | Historical funding by 8-hour period |
| Borrow rates for trust shares | IBKR historical borrow data | May require account access |

### Backtest period

- **Primary:** January 2023 – present (post-FTX, more mature perp market)
- **Secondary check:** 2021–2022 (higher volatility, different funding regime)
- **Exclude:** GBTC pre-ETF conversion data (different structure, different AP dynamics)

### Metrics to compute

1. **Gap frequency:** How often does |OCM| exceed 3% threshold? (Expected: 15–25% of trading days given crypto volatility)
2. **Gap capture rate:** Of those days, what % does the trust share price at open reflect <50% of the OCM? (This validates the stale pricing hypothesis)
3. **Intraday reversion rate:** Of those days, what % does the gap compress to ≤0.5% by 3:45 PM ET?
4. **P&L per trade:** (Trust price move - perp hedge P&L - funding cost - borrow cost - bid-ask spread × 2)
5. **Sharpe ratio** on trade-level P&L
6. **Max drawdown** (consecutive losing trades)

### Key backtest caveat

Historical trust share prices from Yahoo Finance may not reflect actual executable prices at open. The open price is often a single print, not a tradeable quote. Backtest results will be optimistic. Apply a **10–15 bps additional slippage assumption** on trust legs.

### Validation test

Run the same analysis on GBTC *before* its ETF conversion (pre-January 2024). GBTC had a well-documented persistent discount. The question is whether *intraday* gaps around overnight moves were tradeable — this is a cleaner historical dataset with more volume.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

- [ ] **Liquidity audit complete:** At least 2 trusts confirmed with ADV > $500K and borrowable shares at IBKR
- [ ] **Backtest shows positive expectancy:** Mean P&L per trade > 0 after all costs, on ≥ 50 trade observations
- [ ] **Gap capture rate validated:** ≥ 40% of OCM > 3% days show trust open price reflecting < 60% of the move (i.e., the stale pricing effect is real)
- [ ] **Intraday reversion rate ≥ 60%:** Gaps compress intraday on most triggered days
- [ ] **Brokerage setup confirmed:** Margin account at IBKR with trust share borrow confirmed, Hyperliquid account for perp hedge
- [ ] **Execution tested:** Manual paper trade executed on at least 5 live days before capital deployment

**Paper trading period:** Minimum 30 trading days before live capital.

---

## Kill Criteria

Stop the strategy immediately if any of the following occur:

| Condition | Threshold | Action |
|-----------|-----------|--------|
| Consecutive losing trades | 5 in a row | Pause, review |
| Drawdown from peak | >15% of strategy allocation | Stop, full review |
| Borrow cost spike | Trust borrow rate > 20% annualized | Suspend short-side trades |
| Gap capture rate degrades | <20% of OCM days show exploitable gap over rolling 60 days | Strategy may be arbitraged away; stop |
| Trust converts to ETF | Any remaining trust gains AP mechanism | Remove from universe immediately |
| Liquidity collapse | ADV drops below $200K for 10 consecutive days | Remove from universe |

---

## Risks

### Risk 1: No exploitable gap exists (primary risk)
The market may already efficiently price in overnight crypto moves into trust share prices at open. If professional traders are monitoring this, the gap is arbed away before retail can act. **Mitigation:** Backtest validates this before any capital is deployed.

### Risk 2: Borrow unavailability kills the short side
For Scenario B (trust premium), the trade requires borrowing trust shares. OTCQX-listed trusts are notoriously hard to borrow. This may make the strategy one-directional (long-only on discounts). **Mitigation:** Confirm borrow at IBKR before going live. Accept that the strategy may only be executable in Scenario A.

### Risk 3: Perp funding eats the edge
If BTC perp funding is highly positive (longs pay shorts), the hedge leg has a cost. On a 3% OCM trade held for 6 hours, funding might be 0.1–0.3% — meaningful relative to the edge. **Mitigation:** Model funding explicitly in backtest. Set minimum OCM threshold higher if funding is elevated.

### Risk 4: Execution slippage at open
Trust shares at open may have wide spreads (1–3% for thin OTCQX names). This can consume the entire edge. **Mitigation:** Use limit orders, not market orders. If spread > 1%, stand down.

### Risk 5: Trust discount/premium persistence
Closed-end fund discounts can persist for months (see: GBTC 2022–2023 at 40–50% discount). If the trust is in a structural discount regime, the intraday gap may not compress — it may widen. **Mitigation:** Monitor rolling 30-day premium/discount trend. Do not trade if trust is in a persistent discount/premium regime (>10% structural gap), as mean reversion assumptions break down.

### Risk 6: Regulatory risk on trust structure
Grayscale may convert remaining trusts to ETFs (as they did with GBTC and ETHE). This would eliminate the edge entirely. **Mitigation:** Monitor Grayscale SEC filings. This is a strategy with a finite life — plan for it.

### Risk 7: Correlation breakdown in multi-asset trusts
GDLC's NAV depends on BTC, ETH, SOL, XRP, and ADA. If SOL rallies 10% overnight while BTC is flat, the trust's NAV moves but the hedge (BTC/ETH perps) does not capture this. **Mitigation:** Treat GDLC as lower-quality; size it at 0.5× base or exclude.

---

## Data Sources

| Source | Data | Access | Cost |
|--------|------|--------|------|
| grayscale.com/funds | Daily NAV for all trusts | Public, manual download | Free |
| Yahoo Finance | Trust share OHLCV | API (yfinance) | Free |
| Kaiko / Tardis | BTC/ETH/LTC/BCH spot at specific timestamps | API | Paid (~$500/mo for Tardis) |
| Coinglass | Historical perp funding rates | API | Free tier available |
| Hyperliquid API | Live perp prices, funding | API | Free |
| IBKR | Borrow availability and rates | Account required | Brokerage account |
| SEC EDGAR | Trust prospectuses, NAV fixing methodology | Public | Free |

---

## Open Questions (Pre-Backtest Research Tasks)

1. **What is the exact NAV fixing methodology for each remaining Grayscale trust?** (Is it a single 4 PM print, a TWAP, or a specific index like XBX? This determines how "stale" the NAV actually is.)
2. **Are any remaining trusts borrowable at IBKR?** (Call the securities lending desk — this is the single most important pre-backtest question.)
3. **What is the typical bid-ask spread for LTCN, BCHG, GDLC at open vs. midday?** (Determines whether the edge survives transaction costs.)
4. **Has anyone published academic or practitioner research on intraday closed-end fund discount dynamics in crypto?** (Would validate or invalidate the hypothesis before spending backtest time.)
5. **Do European ETPs (21Shares, ETC Group) have AP mechanisms?** (If yes, the gap is arbed away mechanically and they should be excluded.)

---

## Summary Assessment

| Dimension | Assessment |
|-----------|------------|
| Structural mechanism | Real but soft — no hard convergence guarantee |
| Opportunity frequency | Low-medium (15–25% of days trigger) |
| Opportunity size | Small per trade (1–3% gross, <1% net) |
| Scalability | Very low — liquidity-constrained |
| Execution complexity | Medium-high — two-leg, cross-venue |
| Data availability | Good for backtest |
| Competition | Low — cross-domain knowledge required |
| Strategy lifespan | Finite — trust conversions will kill it |

**Verdict:** This is a real structural inefficiency with a plausible but unproven edge. The opportunity set has shrunk post-ETF conversion and may be too illiquid to trade at meaningful size. The single most important next step is the **liquidity and borrow audit** — if LTCN and BCHG cannot be borrowed and have ADV < $500K, the strategy is not executable and should be archived. If BTC Mini Trust and ETH Mini Trust show exploitable gaps with adequate liquidity, this becomes a legitimate niche strategy worth paper trading.

**Next step:** Liquidity audit (1 week) → Backtest if audit passes → Paper trade if backtest passes.
