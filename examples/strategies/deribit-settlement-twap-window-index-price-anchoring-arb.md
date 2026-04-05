---
title: "Deribit Settlement TWAP Window — Index Price Anchoring Arb"
status: HYPOTHESIS
mechanism: 5
implementation: 7
safety: 6
frequency: 3
composite: 630
categories:
  - options-derivatives
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

On Deribit options expiry days, the settlement price is determined by a contractually defined 30-minute TWAP of the Deribit index (08:00–08:30 UTC). Large holders of in-the-money options have a direct economic incentive to trade the underlying spot markets during this window to influence the TWAP in their favour. This creates a predictable, observable directional bias in spot/perp markets during the settlement window. The bias is not random — it is proportional to the net dollar value of ITM options that benefit from price movement in a given direction. We can observe the full OI distribution before the window opens and calculate which side has more to gain.

**The core claim:** When aggregate ITM call OI significantly outweighs ITM put OI (measured in notional USD), rational option holders will apply net upward pressure on spot during the 08:00–08:30 UTC window. The reverse holds for dominant put OI. This pressure is not guaranteed to dominate — counterparties, hedgers, and noise traders all participate — but the structural incentive is real, observable, and directional.

**This is NOT:** A manipulation thesis. It is a rational-actor thesis. Holders of large ITM positions do not need to coordinate. Each acts in their own interest, and the aggregate effect is directional flow.

---

## Structural Mechanism

### Why the window exists
Deribit settles all BTC and ETH options to the **Deribit BTC Index** or **Deribit ETH Index** — a weighted average of spot prices across a defined basket of exchanges (Coinbase, Kraken, Bitstamp, Bitfinex, Gemini, LMAX, and others). The settlement price is the **arithmetic mean of the index value sampled every second** over the 30-minute window from 08:00 to 08:30 UTC on the last Friday of each month (and quarterly dates).

This is published in Deribit's contract specifications. It is not an estimate — it is a deterministic formula applied to observable data.

### Why rational actors trade during the window
Consider a trader holding 500 BTC worth of call options at the $70,000 strike when BTC is trading at $70,500. Each $100 increase in the TWAP settlement price is worth $50,000 in additional payout across their position. Buying spot BTC during the window costs them market impact but potentially gains multiples of that in option payout. The incentive is asymmetric and calculable.

The same logic applies in reverse for put holders and for option writers on the losing side who may attempt to push price the other way.

### Why this is not fully arbed away
1. **Coordination problem:** Each actor acts independently, creating imperfect but directionally consistent pressure.
2. **Capital constraints:** Not every ITM holder has sufficient spot capital to move the market.
3. **Counterparty uncertainty:** Writers may or may not defend, creating variance.
4. **Small enough window:** 30 minutes is too short for most systematic funds to build and unwind positions around.
5. **Expiry frequency:** Monthly/quarterly — not frequent enough for HFT to dedicate infrastructure.

### The "max pain" distinction
This strategy is **not** a max pain strategy. Max pain predicts where price will gravitate to minimise total option payout (a contested, weak hypothesis). This strategy predicts directional pressure from the dominant ITM side — a stronger, incentive-based mechanism. The two can point in opposite directions.

---

## Universe

- **BTC-USD perpetual futures on Hyperliquid** (primary execution venue)
- **ETH-USD perpetual futures on Hyperliquid** (secondary)
- **Applicable expiry dates:** Last Friday of each month (monthly expiry); additionally the last Friday of March, June, September, December (quarterly expiry — larger OI, stronger signal)
- **Frequency:** ~12–16 events per year per asset

---

## Signal Construction

### Step 1 — Snapshot OI at 07:45 UTC on expiry day
Pull from Deribit public API:
- All active BTC (or ETH) option contracts expiring today
- For each strike: call OI (contracts), put OI (contracts)
- Current Deribit index price at 07:45 UTC

### Step 2 — Classify ITM positions
- **ITM calls:** All call strikes < current index price
- **ITM puts:** All put strikes > current index price

### Step 3 — Calculate Net ITM Notional
```
ITM_Call_Notional = Σ (call_OI_contracts × contract_size × index_price) for all ITM calls
ITM_Put_Notional  = Σ (put_OI_contracts × contract_size × index_price) for all ITM puts

Net_Signal = ITM_Call_Notional - ITM_Put_Notional
```

### Step 4 — Signal threshold
- If `Net_Signal > +threshold`: LONG bias (calls dominant)
- If `Net_Signal < -threshold`: SHORT bias (puts dominant)
- If `|Net_Signal| < threshold`: NO TRADE

**Threshold (initial hypothesis):** Net_Signal must exceed $50M notional imbalance. This is a free parameter to be optimised in backtest. Too low = noise; too high = too few trades.

### Step 5 — Confirm with gamma profile (optional refinement)
Calculate net dealer gamma at current spot. If dealers are net short gamma near the dominant strike cluster, they will be forced to buy (calls) or sell (puts) as price moves — amplifying the signal. This is a secondary filter, not the primary signal.

---

## Entry Rules

| Parameter | Value |
|-----------|-------|
| Entry time | 07:55 UTC (5 minutes before window) |
| Direction | Long if Net_Signal > +$50M; Short if Net_Signal < -$50M |
| Instrument | BTC-USD or ETH-USD perp on Hyperliquid |
| Order type | Market order (liquidity is sufficient; slippage is a cost to model) |
| Entry confirmation | Signal must be calculated from 07:45 UTC snapshot; no re-calculation after entry |

**Rationale for 07:55 entry:** Entering before the window captures any pre-window positioning by other participants who read the same data. Entering at 08:00 means competing with the very flow we are trying to front-run.

---

## Exit Rules

| Scenario | Action |
|----------|--------|
| Primary exit | Market close at 08:30 UTC (window close) |
| Hard stop | If price moves 0.8% against position at any point before 08:30 UTC, close immediately |
| Soft stop | If price moves 0.5% against position before 08:00 UTC (pre-window), reduce size by 50% |
| Time stop | If position is flat P&L at 08:15 UTC (halfway through window), close — signal has not materialised |

**No trailing stop during window.** The thesis is that pressure builds through the window. Exiting early on a small adverse move during the window itself defeats the mechanism.

---

## Position Sizing

### Base size
```
Position_Size_USD = Account_Equity × Risk_Per_Trade / Expected_Stop_Distance

Risk_Per_Trade     = 0.5% of account equity per event
Expected_Stop      = 0.8% (hard stop distance)
Position_Size_USD  = Account × 0.005 / 0.008 = Account × 0.625
```

For a $100,000 account: position size ≈ $62,500 notional. This is intentionally modest — the edge is probabilistic, not guaranteed.

### Scaling with signal strength
```
If |Net_Signal| > $200M: 1.5× base size
If |Net_Signal| > $500M: 2.0× base size (cap)
```

### Maximum exposure
- Never exceed 3× base size
- Never hold position through the window on both BTC and ETH simultaneously unless signals are independent and non-correlated (they usually are not — skip ETH if BTC signal is active)

---

## Backtest Methodology

### Data requirements
1. **Deribit OI snapshots** — Historical OI by strike, by expiry, timestamped. Available via Deribit API historical data or third-party providers (Tardis.dev has full Deribit order book and OI history).
2. **Deribit index price** — Second-by-second index values during 08:00–08:30 UTC windows. Available from Tardis.dev.
3. **BTC/ETH perp price** — Hyperliquid or Binance perp OHLCV at 1-minute resolution for entry/exit simulation.

### Backtest period
- **Minimum:** January 2022 – present (covers multiple market regimes: bull, bear, sideways)
- **Target:** 36+ expiry events per asset = 72+ total observations

### Backtest procedure
```
For each expiry date:
  1. Load OI snapshot at 07:45 UTC
  2. Calculate Net_Signal
  3. If |Net_Signal| > threshold: record hypothetical trade
  4. Simulate entry at 07:55 UTC market price + 0.05% slippage
  5. Apply stop logic tick-by-tick
  6. Record exit at 08:30 UTC or stop
  7. Calculate P&L net of fees (Hyperliquid taker fee ~0.035%)
```

### Metrics to report
- Win rate (directional accuracy)
- Average P&L per trade (in bps)
- Sharpe ratio (annualised, using per-trade returns)
- Maximum drawdown (consecutive losing expiries)
- Signal strength vs outcome correlation (does larger Net_Signal → larger move?)
- Regime breakdown: bull vs bear vs sideways (does the edge hold across regimes?)
- Quarterly vs monthly expiry comparison (hypothesis: quarterly has stronger signal due to larger OI)

### Null hypothesis to reject
*"The direction of price movement during the 08:00–08:30 UTC window is uncorrelated with the sign of Net_Signal."*

We need win rate > 55% with p-value < 0.05 across the full sample to consider this a real edge.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

| Criterion | Threshold |
|-----------|-----------|
| Minimum observations | ≥ 30 qualifying trades (Net_Signal > threshold) |
| Win rate | ≥ 55% |
| Statistical significance | p-value < 0.05 (binomial test vs 50% null) |
| Average net P&L per trade | > 0 after fees and slippage |
| No single expiry accounts for >30% of total P&L | Concentration check |
| Signal strength correlation | Pearson r > 0.2 between Net_Signal magnitude and price move magnitude |

**Paper trading period:** Minimum 3 months (3–4 expiry events) before live capital deployment.

---

## Kill Criteria

Stop trading this strategy immediately if any of the following occur:

| Trigger | Condition |
|---------|-----------|
| Live win rate collapse | Win rate drops below 45% over trailing 10 trades |
| Consecutive losses | 5 consecutive losing trades |
| Structural change | Deribit changes settlement methodology (monitor contract specs) |
| Liquidity change | Deribit OI on BTC options drops below $500M total (signal too weak) |
| Crowding signal | Price begins moving strongly in signal direction before 07:55 UTC entry — edge is being front-run |
| Regulatory event | Any jurisdiction restricts crypto options settlement mechanics |

---

## Risks and Mitigants

### Risk 1: Competing flows dominate
**Description:** Macro news, liquidation cascades, or large spot flows during the window overwhelm the options-driven pressure.
**Mitigant:** Hard stop at 0.8% limits damage. Do not trade within 2 hours of scheduled macro events (FOMC, CPI, etc.) that fall on expiry day.

### Risk 2: Signal is already priced in
**Description:** Other participants read the same OI data and front-run the window before 07:55 UTC, eliminating the edge.
**Mitigant:** Monitor pre-window price action. If price has already moved >0.5% in signal direction before 07:55 UTC, skip the trade (edge consumed).

### Risk 3: OI data is stale or manipulated
**Description:** Large positions can be opened/closed in the hours before expiry, making the 07:45 snapshot unreliable.
**Mitigant:** Cross-check OI at 07:45 vs 06:00 UTC. If OI has changed by >20% in 90 minutes, treat signal as unreliable and skip.

### Risk 4: Index basket divergence
**Description:** One of the index component exchanges has a temporary price anomaly, distorting the TWAP without reflecting true market pressure.
**Mitigant:** Monitor individual component prices during window. If one exchange is >0.3% away from the others, the index is being distorted — exit immediately.

### Risk 5: Low OI expiries
**Description:** On low-OI monthly expiries, the Net_Signal may be large in percentage terms but small in absolute dollar terms, meaning few actors have incentive to trade.
**Mitigant:** Absolute dollar threshold ($50M minimum Net_Signal) filters out low-OI events.

### Risk 6: Execution venue mismatch
**Description:** We trade Hyperliquid perps but the pressure is on Deribit index component spot markets. The perp may not track spot during the 30-minute window.
**Mitigant:** Monitor perp basis during backtesting. If perp consistently diverges from spot during expiry windows, switch execution to spot markets on a Deribit index component exchange.

---

## Data Sources

| Data | Source | Cost | Notes |
|------|--------|------|-------|
| Deribit OI by strike (historical) | Tardis.dev | Paid (~$200/month) | Full tick data, all strikes |
| Deribit OI by strike (live) | Deribit public REST API | Free | `/api/v2/public/get_book_summary_by_currency` |
| Deribit index price (historical) | Tardis.dev | Included above | Second-by-second |
| Deribit index price (live) | Deribit public WebSocket | Free | `deribit_price_index.btc_usd` |
| BTC/ETH perp OHLCV | Hyperliquid public API | Free | 1-minute candles sufficient |
| Expiry calendar | Deribit website / API | Free | Verify dates manually each quarter |

**Alternative to Tardis:** Deribit provides some historical data via their API directly, but coverage is incomplete for older dates. Tardis is the recommended source for a clean backtest dataset.

---

## Open Questions for Backtest Phase

1. **What is the optimal Net_Signal threshold?** $50M is a hypothesis. Backtest should sweep $20M–$200M.
2. **Does the edge concentrate in quarterly expiries?** Hypothesis: yes, due to 3× larger OI. Test separately.
3. **Is the 07:55 entry optimal, or does 08:00 (window open) perform better?** Test both.
4. **Does the time-stop at 08:15 improve or hurt performance?** Test with and without.
5. **Is there a post-window reversal?** If price is pushed during the window, does it snap back 08:30–09:00 UTC? If yes, a counter-trade at 08:30 may be a second edge.
6. **ETH vs BTC:** Does ETH show a stronger or weaker signal? ETH options OI is lower but the market is thinner, so the same dollar flow may have more impact.

---

## Notes for Researcher

This strategy sits at the intersection of options market structure and spot/perp execution — a genuinely unusual combination that most quant shops ignore because it requires understanding two separate market microstructures simultaneously. The edge is not guaranteed (hence 6/10, not 8/10), but the causal mechanism is sound: contractually defined settlement windows with observable, concentrated economic incentives create directional flow. The primary risk is crowding — if this is already well-known among options desks, the edge may be consumed before our entry. The crowding check (pre-window price movement) is therefore a critical live filter, not optional.

**Next step:** Acquire Tardis.dev Deribit dataset. Build OI snapshot reconstruction for all BTC monthly/quarterly expiries from January 2022 onward. Run backtest per methodology above. Report back with win rate, Sharpe, and signal-strength correlation before any further development.
