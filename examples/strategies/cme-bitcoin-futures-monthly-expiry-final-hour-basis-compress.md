---
title: "CME Bitcoin Futures Monthly Expiry — Final Hour Basis Compression"
status: HYPOTHESIS
mechanism: 6
implementation: 4
safety: 6
frequency: 2
composite: 288
categories:
  - basis-trade
  - calendar-seasonal
created: "2025-07-14T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

In the 2–4 hours before the CME CF Bitcoin Reference Rate (BRR) calculation window opens (3–4pm London time on the last Friday of each month), the CME front-month futures basis compresses toward zero as arbitrageurs mechanically unwind carry positions ahead of cash settlement. This compression is directional: if basis is positive (contango) entering the window, it must fall; if negative (backwardation), it must rise. The causal chain is:

1. CME BTC futures settle to the BRR — a contractually fixed price derived from spot trades on constituent exchanges during the 3–4pm London window.
2. Any arbitrageur holding a cash-and-carry (long spot / short futures) or reverse carry position faces zero P&L risk from basis after settlement — the basis is extinguished.
3. Rational arbitrageurs begin unwinding before settlement to avoid execution risk inside the BRR window itself (trading during the window affects the settlement price they're settling against — a self-referential risk).
4. This unwinding flow is directional and predictable: carry unwinds compress the basis toward zero.
5. A trader who enters the fade-the-basis position 2–4 hours before the window opens captures the tail end of this compression without needing to be first.

**Testable prediction:** The absolute value of the CME front-month basis (CME price minus BRR-constituent spot index) decreases monotonically in the 2–4 hours before 3pm London on BRR settlement Fridays, more than on non-settlement Fridays at the same time.

---

## Structural Mechanism — WHY This MUST Happen

The BRR is defined by CF Benchmarks methodology as the volume-weighted median price of BTC/USD transactions on constituent exchanges (currently: Coinbase Advanced, Kraken, Bitstamp, Gemini, itBit/Paxos) during the 60-minute calculation window 3:00–4:00pm London time, partitioned into 12 × 5-minute intervals with outlier filtering.

**Contractual guarantee:** CME BTC futures (ticker: BTC) final settlement price = BRR on the last Friday of the contract month. This is in the CME rulebook (Chapter 35, Rule 35102.F). There is no discretion. Basis = 0 at settlement by definition.

**Why compression happens before, not at, the window:**
- Executing large spot sells (to close long spot legs of carry trades) during the BRR window directly depresses the settlement price the trader is settling against. This is adverse self-impact — the larger the position, the worse the settlement price received.
- Therefore rational carry holders unwind spot legs before 3pm London, creating selling pressure on spot and buying pressure on futures (closing short futures legs), compressing the basis.
- This is not a tendency — it is the dominant rational strategy for any carry holder of meaningful size. The mechanism is game-theoretic forced action, not historical pattern.

**Why the edge is not fully arbitraged away:**
- The compression window is short (2–4 hours), requiring active monitoring.
- Execution requires simultaneous positions on CME (futures) and spot exchanges — cross-venue complexity deters retail.
- The residual basis entering the window varies month-to-month, making the trade sometimes too small to bother with after transaction costs.

---

## Entry Rules


### Universe
- Instrument: CME BTC front-month futures (continuous front-month roll, or specific monthly contract expiring that day)
- Hedge leg: BTC spot on Coinbase Advanced (deepest liquidity among BRR constituents)

### Settlement Friday Identification
- Last Friday of each calendar month where a CME BTC monthly contract expires
- CME BTC monthly contracts expire every month (not just quarterly) — verify against CME expiry calendar: https://www.cmegroup.com/trading/equity-index/us-index/bitcoin_product_calendar_futures.html

### Entry Signal
- Time: 11:00am London time (T-4h before BRR window open)
- Measure basis: `Basis = CME_front_month_mid_price - Coinbase_BTC_USD_mid_price`
- **Entry condition:** `|Basis| > $150` (approximately 0.2% at $75k BTC — below this, transaction costs consume the edge)
- **If Basis > +$150 (contango):** Short CME futures, Long BTC spot on Coinbase
- **If Basis < -$150 (backwardation):** Long CME futures, Short BTC spot on Coinbase (or skip — backwardation is rarer and spot shorting on Coinbase is unavailable; use Hyperliquid perp as spot proxy if needed)
- **If |Basis| ≤ $150:** No trade this month

### Position Sizing
See Position Sizing section below.

## Exit Rules

### Exit Rules
- **Primary exit:** Close both legs at 2:55pm London time (5 minutes before BRR window opens). Do NOT trade during the 3–4pm window.
- **Stop loss:** If basis widens by more than 50% from entry level (e.g., entered at +$200 basis, stop if basis reaches +$300), exit immediately. This indicates an unexpected supply/demand shock overriding the compression mechanism.
- **Profit target:** None — ride to primary exit. The mechanism defines the exit, not a P&L target.

### Roll / Contract Selection
- Use the front-month contract only. If entry is within 2 days of expiry (which it always is for this trade), liquidity may be thin — check open interest. Minimum OI threshold: 500 contracts on the front month at entry.

---

## Position Sizing

### Base Sizing
- Risk per trade: 0.5% of portfolio NAV
- Stop distance: 50% basis widening from entry (in dollar terms)
- Example: Portfolio = $1,000,000. Entry basis = +$200. Stop at +$300 (widening of $100/BTC).
  - Risk = $5,000
  - Position size = $5,000 / $100 = 50 BTC notional
  - CME contract = 5 BTC → 10 contracts
  - Coinbase spot leg = 50 BTC

### Constraints
- Maximum position: 2% of portfolio NAV notional (prevents over-concentration in a single monthly event)
- Do not size up if basis is unusually wide (>$500) — wide basis may indicate a structural dislocation, not a compression opportunity
- Both legs must be executable simultaneously within 60 seconds; if not, abort

### Margin
- CME BTC futures initial margin: ~$80,000–$100,000 per contract (check current SPAN margin at https://www.cmegroup.com/clearing/risk-management/span-overview.html)
- Ensure sufficient margin headroom for 2× initial margin before entry

---

## Backtest Methodology

### Data Sources

| Data | Source | URL / Notes |
|------|---------|-------------|
| CME BTC front-month OHLCV (hourly) | CME DataMine or Quandl/Nasdaq Data Link | https://datamine.cmegroup.com — paid; Quandl: `CHRIS/CME_BTC1` (free, daily only) |
| BRR historical settlement values | CF Benchmarks | https://www.cfbenchmarks.com/data/BRR — free download |
| BTC spot (Coinbase hourly OHLCV) | Coinbase Advanced API | https://api.exchange.coinbase.com/products/BTC-USD/candles |
| BTC spot (Kraken hourly OHLCV) | Kraken REST API | https://api.kraken.com/0/public/OHLC?pair=XBTUSD&interval=60 |
| CME expiry calendar | CME Group website | https://www.cmegroup.com/trading/equity-index/us-index/bitcoin_product_calendar_futures.html |

**Note on CME intraday data:** Daily OHLCV is insufficient — you need hourly or 5-minute CME futures prices to measure basis compression intraday. CME DataMine is the authoritative source (~$50–200/month depending on plan). Alternative: use CME futures prices proxied from Bloomberg terminal if available, or use Refinitiv Eikon tick data.

**Workaround if CME intraday unavailable:** Use Deribit BTC monthly futures (which also settle to BRR-adjacent indices) as a proxy — Deribit provides free historical OHLCV via API: https://www.deribit.com/api/v2/public/get_tradingview_chart_data

### Sample Period
- Start: January 2020 (CME BTC monthly futures well-established by then)
- End: Most recent completed month
- Expected sample: ~60 monthly settlement events

### Backtest Steps

1. **Identify all settlement Fridays** from CME expiry calendar.
2. **For each settlement Friday:**
   - Pull CME front-month mid price at 11:00am, 12:00pm, 1:00pm, 2:00pm, 2:55pm London time.
   - Pull Coinbase BTC/USD mid price at same timestamps.
   - Calculate basis at each timestamp.
   - Record: entry basis (11am), exit basis (2:55pm), basis at each intermediate hour, BRR settlement value.
3. **Apply entry filter:** Only include months where |entry basis| > $150.
4. **Calculate P&L per trade:**
   - Contango trade: P&L = Entry_basis - Exit_basis (in $/BTC), minus transaction costs
   - Transaction costs: CME round-trip ~$10/contract (2 × $5 exchange fee) = $2/BTC; Coinbase taker fee ~0.05% each way ≈ $75/BTC at $75k → total ~$77/BTC round-trip
   - Net P&L per BTC = (Entry_basis - Exit_basis) - $77
5. **Control test:** Run identical measurement on non-settlement Fridays at the same London time (11am–2:55pm). If basis compression is similar on non-settlement Fridays, the mechanism is not specific to settlement and the edge is spurious.

### Key Metrics

| Metric | Target | Rationale |
|--------|--------|-----------|
| Win rate | >60% | Mechanism should be directionally reliable |
| Average net P&L per trade | >$100/BTC | Must exceed transaction costs with margin |
| Sharpe ratio (annualised) | >1.0 | ~12 trades/year, needs consistency |
| Max drawdown | <3% NAV | Single-event risk must be bounded |
| Basis compression rate | >70% of entry basis eliminated by exit | Validates the mechanism, not just P&L |
| Settlement Fridays vs control Fridays | Compression significantly larger on settlement Fridays | Validates structural vs random |

### Baseline Comparison
- Null hypothesis: basis compression on settlement Fridays is not statistically different from compression on the preceding Friday (same time window). Use paired t-test on basis change (entry to exit) across both groups.

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Win rate ≥ 60%** on trades that passed the entry filter (|basis| > $150)
2. **Average net P&L > $100/BTC** after transaction costs
3. **Settlement Friday compression statistically larger than control Fridays** (p < 0.05, paired t-test)
4. **No single losing month exceeds 1.5% NAV** (validates stop-loss sizing)
5. **Minimum 30 qualifying trade observations** (months where |basis| > $150 at entry) — if fewer than 30, extend sample or reduce entry threshold to $100 and re-evaluate
6. **Basis compression begins before 2pm London** in >50% of winning trades — confirms the mechanism is pre-window unwinding, not noise at the close

---

## Kill Criteria

Abandon the strategy (in backtest, paper trade, or live) if:

1. **Win rate falls below 50%** over any rolling 12-month live period (6+ trades)
2. **Average basis at entry drops below $75/BTC** for 3 consecutive months — transaction costs make the trade uneconomical; the opportunity has been arbitraged away
3. **CME changes BRR methodology** (e.g., extends calculation window, changes constituent exchanges, moves settlement time) — the structural mechanism changes and must be re-evaluated from scratch. Monitor: https://www.cfbenchmarks.com/methodology
4. **CME open interest on front-month < 300 contracts** at entry — insufficient liquidity to execute without moving the market
5. **Control test invalidation:** If a re-run of the control test (non-settlement Fridays) shows equivalent compression, the edge was never structural — kill immediately
6. **Two consecutive stop-loss hits** — indicates a regime change where the mechanism is being overwhelmed by directional flow

---

## Risks

### Crowding Risk (HIGH)
This is the most significant risk. Large basis traders (crypto hedge funds, prop desks) run this trade at scale. Their earlier unwinding may compress the basis before 11am London, leaving nothing to capture by the time retail/smaller participants enter. **Mitigation:** Check basis at 9am London before committing to the 11am entry; if basis is already <$100, skip the month.

### Execution Risk — Cross-Venue Simultaneity (MEDIUM)
The trade requires simultaneous execution on CME (futures) and Coinbase (spot). CME opens at specific hours; Coinbase is 24/7. If CME is illiquid at entry time, the hedge is imperfect. **Mitigation:** Use limit orders on CME with a 2-minute fill window; if unfilled, abort.

### Self-Referential Settlement Risk (LOW-MEDIUM)
If the spot leg (Coinbase) is large enough to move the Coinbase price during the BRR window, the settlement price is adversely affected. At the position sizes specified here (<50 BTC), this is negligible. At scale (>500 BTC), this becomes a real concern.

### Basis Widening on Settlement Friday (MEDIUM)
Occasionally, a macro shock (exchange hack, regulatory news, large liquidation) hits on settlement Friday and drives the basis wider, triggering the stop. This is not a mechanism failure — it is a genuine risk event. The stop-loss at 50% widening is designed to limit damage.

### Regulatory / Account Risk (LOW)
CME futures require a futures-enabled brokerage account (Interactive Brokers, Wedbush, etc.) with CFTC-regulated clearing. Non-US entities may face restrictions. Ensure regulatory compliance before live trading.

### Opportunity Frequency (STRUCTURAL LIMITATION)
This trade fires at most 12 times per year, and only when |basis| > $150. In low-volatility, low-carry environments, the trade may fire 4–6 times per year. Annual return contribution is inherently limited — this is a supplementary strategy, not a core book.

### Transaction Cost Sensitivity (HIGH)
At $77/BTC round-trip cost and a typical basis compression of $150–300, the trade operates on thin margins. Any increase in Coinbase fees, CME fees, or slippage materially impacts profitability. Re-calculate break-even basis threshold whenever fee schedules change.

---

## Data Sources

| Source | What | Access | Cost |
|--------|------|--------|------|
| CME DataMine | CME BTC futures intraday OHLCV | https://datamine.cmegroup.com | Paid (~$50–200/mo) |
| CF Benchmarks | BRR historical values + methodology | https://www.cfbenchmarks.com/data/BRR | Free |
| Coinbase Advanced API | BTC/USD spot OHLCV (1h candles) | `GET https://api.exchange.coinbase.com/products/BTC-USD/candles?granularity=3600` | Free |
| Kraken REST API | BTC/USD spot OHLCV (1h candles) | `GET https://api.kraken.com/0/public/OHLC?pair=XBTUSD&interval=60` | Free |
| CME Expiry Calendar | Settlement Friday dates | https://www.cmegroup.com/trading/equity-index/us-index/bitcoin_product_calendar_futures.html | Free |
| Deribit API (proxy) | BTC monthly futures if CME data unavailable | `GET https://www.deribit.com/api/v2/public/get_tradingview_chart_data` | Free |
| SPAN Margin Calculator | Current CME margin requirements | https://www.cmegroup.com/clearing/risk-management/span-overview.html | Free |

---

## Open Questions for Researcher

1. Does CME DataMine provide 5-minute or hourly OHLCV for BTC futures going back to 2020? Confirm before committing to this backtest approach.
2. Is the basis compression effect visible in Deribit monthly futures (which settle to DERIBIT-BTC index, not BRR) — if yes, the mechanism may be broader than BRR-specific.
3. Has anyone published academic work on BRR settlement mechanics and basis behaviour? Search: "Bitcoin Reference Rate settlement arbitrage" on SSRN.
4. What is the typical open interest on CME BTC front-month in the final week? If consistently <500 contracts, liquidity risk is structural.
5. Consider whether the Micro BTC futures (MBT, 0.1 BTC/contract) on CME offer better liquidity and lower capital requirements for initial testing.
