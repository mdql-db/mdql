---
title: "Maker/Spark Stability Fee Governance Change — Pre-Announcement Rate Arbitrage"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - governance
  - defi-protocol
  - lending
created: "2026-04-03T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When MakerDAO/Sky queues a stability fee increase via an on-chain spell, CDP holders with thin collateral margins face a mechanically higher cost of carry and some fraction will close positions before activation — repaying DAI, receiving collateral back, and selling that collateral into the market. The spell queue timestamp and activation block are publicly readable on-chain the moment the spell is cast, creating a known, bounded window of predictable sell pressure on ETH, wstETH, and wBTC. The inverse holds for fee decreases. This is not a "tends to happen" pattern — the cost increase is contractually enforced at a known block — but the *magnitude* of CDP closure response is probabilistic, making this a 5/10 rather than an 8/10.

**Null hypothesis to disprove:** Collateral prices show no statistically significant directional drift in the 24-hour window between spell queue and activation, relative to a matched control window with no governance action.

---

## Structural Mechanism

### Why the edge exists (causal chain, not correlation)

1. **Spell queue is immutable and timestamped.** Once a Maker governance spell is queued via the `DSPause` contract, the activation delay (currently 48 hours for most parameter changes, 24 hours for minor ones) and the exact activation block are fixed and public. There is no ambiguity about *when* the new rate takes effect.

2. **CDP holders face a binary cost event.** A CDP holder paying 3% APR on 1,000 ETH of debt who learns their rate will become 6% APR in 48 hours has a concrete incentive to either (a) close the position, (b) migrate to a cheaper vault type, or (c) accept the higher cost. Options (a) and (b) both require repaying DAI and returning collateral to the market.

3. **The marginal CDP holder is the signal.** Vaults with collateral ratios between 150–200% (near liquidation thresholds) are the most cost-sensitive. Makerburn.com exposes the distribution of collateral ratios in real time. A fee increase that pushes the break-even carry cost above the expected yield of holding ETH will force rational closure among this cohort.

4. **The information asymmetry window is hours, not milliseconds.** Most market participants are not monitoring `DSPause` contract events. The signal propagates: on-chain event → crypto-native monitoring tools → governance forums → mainstream crypto media. This creates a 2–12 hour window where the information is technically public but not yet priced.

5. **Forced behavior is partial, not total.** Unlike a liquidation cascade (which is mechanically forced), CDP closure is incentivized but voluntary. This is the primary reason the score is 5/10, not 8/10. Some CDP holders are passive, some hedge elsewhere, some accept the higher rate.

### Structural diagram

```
Spell queued on-chain (T=0, activation block known)
        │
        ▼
Cost-sensitive CDP holders calculate new carry cost
        │
        ├─► Marginal holders begin closing CDPs
        │         │
        │         ▼
        │   DAI repaid → collateral unlocked → collateral sold
        │         │
        │         ▼
        │   Net sell pressure on ETH/wstETH/wBTC
        │
        └─► Passive holders do nothing (noise floor)
                  │
                  ▼
        Sell pressure magnitude is probabilistic
        (depends on: fee delta, vault utilization,
         market conditions, alternative venues)
```

---

## Market & Instrument Scope

| Collateral | Perp Instrument | Rationale |
|---|---|---|
| ETH | ETH-USDC perp (Hyperliquid) | ETH-A and ETH-B vaults are the largest by collateral value |
| wstETH | ETH-USDC perp (proxy) | wstETH vault holders will sell wstETH → ETH → market; ETH perp captures this |
| wBTC | BTC-USDC perp (Hyperliquid) | wBTC-A vault is third-largest vault by collateral |
| USDS/DAI | Skip | DAI repayment increases DAI supply but DAI is pegged; no directional trade |

**Why perps, not spot:** Perps allow short exposure without borrowing friction. Funding rate is a known cost that must be accounted for in P&L. For a 24–48 hour hold, funding is a minor drag unless rates are extreme (>0.1%/8h).

---

## Trigger Conditions

### Qualifying event criteria (ALL must be met)

1. **Fee delta ≥ 1.5% APR** on ETH-A, ETH-B, wstETH-A, wstETH-B, or wBTC-A vault.
   - *Rationale:* Below 1.5% APR delta, the marginal cost change is insufficient to force rational closure for most CDP holders. 1% was the original proposal; raising to 1.5% reduces false positives.

2. **Vault utilization > 30% of debt ceiling** at time of spell queue.
   - *Rationale:* A vault with minimal utilization has few CDPs to close. Low utilization = low signal.

3. **Spell is queued via `DSPause`**, not merely proposed in the forum.
   - *Rationale:* Forum proposals fail regularly. Only the on-chain queue is actionable. Forum-stage entry is premature and noisy.

4. **No concurrent market-wide stress event** (BTC drawdown > 8% in prior 24h).
   - *Rationale:* During broad market stress, macro factors dominate and the governance signal is swamped.

5. **Funding rate on target perp is not extreme** (|funding| < 0.05%/8h at entry).
   - *Rationale:* Extreme funding rates indicate the market is already positioned in the direction of the trade, reducing edge and increasing cost.

### Disqualifying conditions

- Spell is a **parameter bundle** (multiple changes at once) where the fee change is secondary — signal is diluted.
- Fee change is on a **deprecated or low-TVL vault** (< $50M collateral) — insufficient CDP mass to generate meaningful flow.
- A **competing governance action** (e.g., emergency shutdown drill, collateral offboarding) is active simultaneously.

---

## Entry Rules

### Fee increase scenario (SHORT collateral)

| Parameter | Value |
|---|---|
| Direction | Short ETH-USDC perp (or BTC-USDC perp for wBTC vaults) |
| Entry timing | T+2h after spell is queued on-chain (allow 2h for signal to propagate and initial reaction to settle) |
| Entry method | Market order at open of the next 1-hour candle after T+2h |
| Entry confirmation | Check Makerburn.com vault closure rate is elevated vs. 7-day baseline (qualitative check, not a hard filter) |

**Rationale for T+2h delay:** Entering at T+0 risks being front-run by bots monitoring `DSPause` events. The T+2h entry captures the sustained flow from human CDP holders who process the information more slowly, rather than the initial bot-driven spike.

### Fee decrease scenario (LONG collateral)

| Parameter | Value |
|---|---|
| Direction | Long ETH-USDC perp |
| Entry timing | T+2h after spell is queued on-chain |
| Entry method | Market order at open of next 1-hour candle |
| Entry confirmation | Check that DAI borrow demand has been suppressed (Makerburn utilization below ceiling) — confirms new demand is latent |

**Note:** Fee decrease trades are expected to be weaker signals. New CDP opens require users to actively set up vaults; this is slower and more friction-laden than closing existing ones. Consider halving position size for decrease scenarios.

---

## Exit Rules

### Primary exit (time-based)

- **Close 100% of position at activation block + 4 hours.**
- *Rationale:* The forced-behavior window is the period between spell queue and activation. After activation, the fee is live and the incentive to close has either been acted upon or not. Holding beyond +4h post-activation is speculative, not structural.

### Secondary exit (profit target)

- **Close 50% of position if unrealized P&L reaches +1.5% on the position.**
- Move stop to breakeven on remaining 50%.
- *Rationale:* Locks in partial profit if the move is front-loaded; lets the remainder run to activation.

### Stop loss

- **Hard stop: 2.5% adverse move from entry price.**
- *Rationale:* A 2.5% stop on a 24–48h trade gives the thesis room to breathe through normal volatility while capping loss at a level that preserves capital for future events. ETH 24h realized vol is typically 3–5% annualized daily, so 2.5% is approximately 1.5–2 standard deviations of daily move.
- Stop is placed as a limit order 0.1% inside the stop level to avoid slippage on a market stop.

### Time stop

- **If position is flat (< 0.3% P&L) at T+24h (halfway to activation for a 48h spell), close 50% and tighten stop to 1.5%.**
- *Rationale:* If the market has not moved in the expected direction halfway through the window, the thesis is not playing out and risk should be reduced.

---

## Position Sizing

### Base sizing formula

```
Position size = (Account risk per trade) / (Stop distance in %)

Account risk per trade = 1% of trading capital
Stop distance = 2.5%

Example: $100,000 account
Risk per trade = $1,000
Position size = $1,000 / 0.025 = $40,000 notional
```

### Adjustments

| Condition | Adjustment |
|---|---|
| Fee delta ≥ 3% APR (large change) | 1.25× base size |
| Fee delta 1.5–2% APR (small change) | 0.75× base size |
| Fee decrease scenario | 0.5× base size |
| wBTC vault (smaller market, less liquid perp) | 0.75× base size |
| Funding rate 0.03–0.05%/8h against position | 0.75× base size |

**Maximum position size:** 2× base size regardless of adjustments. This strategy has low frequency and unproven edge; oversizing is the primary risk.

**Leverage:** Target 2–3× on Hyperliquid perps. Do not exceed 5×. This is a slow, structural trade — leverage above 5× introduces liquidation risk from normal volatility that is unrelated to the thesis.

---

## Backtest Methodology

### Data collection

**Step 1: Build the event database**

- Source: Maker governance portal (`vote.makerdao.com`), on-chain `DSPause` contract events (Ethereum mainnet), Dune Analytics dashboard for Maker spell history.
- Extract: All stability fee changes from January 2020 to present, with (a) spell queue timestamp, (b) activation timestamp, (c) vault type, (d) fee delta in APR, (e) vault utilization at time of queue.
- Expected event count: ~40–80 qualifying events over 4 years (rough estimate; needs verification).
- Filter to qualifying events using the trigger conditions above.

**Step 2: Price data**

- Source: Binance/Coinbase historical OHLCV for ETH-USD and BTC-USD (1-hour candles).
- For perp-specific data: Hyperliquid historical data (limited history pre-2023), supplement with Binance perp data for earlier periods.
- Align price data to spell queue timestamps using UTC block timestamps.

**Step 3: Vault closure data**

- Source: Makerburn.com historical data, Dune Analytics (`makerdao` schema, `vat` table for CDP events).
- Measure: Net collateral withdrawn from qualifying vaults in the 48h window around each spell queue event.
- This is a secondary validation metric, not a primary backtest input.

### Backtest execution

**For each qualifying event:**

1. Record entry price at T+2h (open of next 1h candle after spell queue + 2h).
2. Record exit price at activation block + 4h.
3. Apply stop loss: if price moves 2.5% adverse at any point during the hold, record stop-out price.
4. Apply profit target: if price moves +1.5% favorable, record partial exit.
5. Calculate P&L per trade in % terms (not dollar terms, to normalize across time).

### Statistical tests

- **Primary:** t-test on mean return per trade vs. zero. Require p < 0.05 with at least 20 qualifying events.
- **Secondary:** Compare mean return in the spell window vs. a matched control window (same time of day, same day of week, 7 days prior to each event) to isolate the governance signal from baseline drift.
- **Tertiary:** Regress trade return on fee delta magnitude to confirm larger fee changes produce larger price moves.
- **Sharpe ratio:** Calculate annualized Sharpe on the strategy's trade-by-trade returns. Target > 1.0 for go-live consideration.

### Known backtest limitations

- **Survivorship bias in vault data:** Makerburn historical data may be incomplete for pre-2021 events.
- **Execution slippage:** Backtest assumes market order execution at candle open; real slippage on $40K notional in ETH perps is minimal but should be modeled at 0.05% per trade.
- **Funding costs:** Model 48h of funding at the prevailing rate at entry for each event.
- **Look-ahead contamination risk:** Ensure spell queue timestamp is used as the signal, not the forum post date (which is known earlier but is not the actionable trigger).

---

## Go-Live Criteria

All of the following must be satisfied before deploying real capital:

| Criterion | Threshold |
|---|---|
| Minimum qualifying events in backtest | ≥ 20 |
| Mean return per trade (net of costs) | > 0.3% |
| Win rate | > 52% |
| Sharpe ratio (annualized, trade-by-trade) | > 1.0 |
| Max drawdown (consecutive losing trades) | < 8% of capital |
| Return in control window (no event) | Not significantly different from zero (confirms signal is event-driven) |
| Paper trade period | Minimum 3 live events observed and paper traded before real capital |

**Paper trade protocol:** For each live qualifying event during paper trade period, log entry/exit in real time with timestamps. Do not adjust rules retroactively. Compare paper trade results to backtest predictions.

---

## Kill Criteria

Immediately suspend the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| 5 consecutive losing trades | Suspend, re-examine event selection criteria |
| Single trade loss > 4% of account (indicates stop failure) | Suspend, audit execution process |
| Backtest invalidated by new data (e.g., Maker governance process changes, timelocks removed) | Permanent kill |
| MakerDAO/Sky migrates to a governance model without on-chain timelocks | Permanent kill — structural edge no longer exists |
| Funding rates consistently > 0.1%/8h against position direction | Suspend until funding normalizes |
| Strategy return correlation > 0.7 with broad ETH market return | Indicates the edge is not isolated from beta; re-examine |

---

## Risks

### Risk 1: Forum pre-pricing (HIGH probability, MEDIUM impact)
**Description:** The stability fee change is debated publicly in the Maker governance forum for days or weeks before the spell is queued. Sophisticated participants may position during the forum debate, meaning the spell queue event is already priced by T+0.
**Mitigation:** The backtest will reveal whether T+2h entry still captures residual drift. If forum-stage entry outperforms spell-queue entry, revise the trigger to use forum post date — but this introduces more noise and failed proposals.
**Residual risk:** If the market fully prices the change during forum debate, this strategy has zero edge. This is the single most important hypothesis to test in the backtest.

### Risk 2: Low event frequency (HIGH probability, LOW impact)
**Description:** Qualifying events (fee delta ≥ 1.5% APR, high utilization vault) may occur only 6–12 times per year. This limits capital deployment and makes statistical validation slow.
**Mitigation:** Accept low frequency as a feature, not a bug. This strategy is a supplement to higher-frequency strategies, not a standalone. Do not lower the fee delta threshold to manufacture more events.

### Risk 3: CDP holder passivity (MEDIUM probability, HIGH impact)
**Description:** Many CDP holders are passive or have automated hedges. If the majority of CDP holders do not respond to fee changes within the 48h window, the sell pressure thesis fails.
**Mitigation:** Vault closure rate data from Makerburn provides a real-time check. If vault closures are not elevated above baseline within 12h of spell queue, consider early exit.

### Risk 4: Macro dominance (MEDIUM probability, HIGH impact)
**Description:** ETH and BTC are highly correlated with broader risk sentiment. A macro event (Fed announcement, exchange hack, regulatory news) during the 48h window will overwhelm the governance signal.
**Mitigation:** The disqualifying condition (BTC drawdown > 8% in prior 24h) partially addresses this. Add a rule: if VIX equivalent (crypto fear index) spikes > 20 points during the hold, exit immediately regardless of P&L.

### Risk 5: Maker governance process changes (LOW probability, HIGH impact)
**Description:** Sky (formerly MakerDAO) is actively restructuring its governance. Timelocks could be shortened, lengthened, or removed. The structural edge depends entirely on the existence of a known, bounded activation window.
**Mitigation:** Monitor Sky governance forum for any proposals to change `DSPause` delay parameters. If timelock is reduced to < 6 hours, the human-speed edge disappears. Kill the strategy immediately.

### Risk 6: Competing arbitrageurs (MEDIUM probability, MEDIUM impact)
**Description:** Other participants monitoring `DSPause` events will take the same trade, compressing the edge over time.
**Mitigation:** The edge is inherently self-limiting — as more capital front-runs CDP closures, the price impact is absorbed earlier and the signal weakens. Monitor edge decay across the backtest period (split into 2020–2022 vs. 2023–present) to detect this.

### Risk 7: wstETH/wBTC vault specifics (LOW probability, LOW impact)
**Description:** wstETH holders may not sell to ETH spot — they may swap wstETH directly or use other venues. The ETH perp may not capture the full collateral sell pressure.
**Mitigation:** For wstETH vaults, check whether wstETH/ETH spread widens during spell windows as a secondary confirmation. If ETH perp does not capture the signal, consider whether a wstETH spot short is feasible.

---

## Data Sources

| Data type | Source | Access method | Cost |
|---|---|---|---|
| Maker spell history (on-chain) | Ethereum mainnet, `DSPause` contract | Etherscan API, The Graph (Maker subgraph) | Free |
| Governance forum posts | `forum.makerdao.com` | Web scrape or RSS feed | Free |
| Vault utilization & CDP counts | Makerburn.com | Web scrape (no public API) | Free |
| Vault collateral ratios (historical) | Dune Analytics | SQL query on `makerdao.vat` tables | Free (rate-limited) |
| ETH/BTC hourly OHLCV (spot) | Binance, Coinbase | REST API | Free |
| ETH/BTC perp OHLCV | Binance Futures, Hyperliquid | REST API | Free |
| Perp funding rates (historical) | Binance Futures, Coinglass | REST API / CSV export | Free |
| Crypto fear index | Alternative.me | REST API | Free |

### Data pipeline for live monitoring

```
DSPause contract (Ethereum) 
    → Alchemy/Infura webhook on LogNote event
    → Python script parses spell address and activation block
    → Cross-reference with Maker changelog for vault type and fee delta
    → Alert via Telegram bot if qualifying conditions met
    → Manual confirmation via Makerburn.com before entry
```

**Monitoring latency target:** Alert within 5 minutes of spell queue. This is achievable with a simple Ethereum event listener — no HFT infrastructure required.

---

## Open Questions for Backtest Phase

1. **Does the signal exist at spell queue, or is it fully priced at forum post?** This is the make-or-break question. Test both entry points.
2. **What is the optimal entry delay?** Test T+0h, T+2h, T+6h, T+12h entries to find the window with the best risk-adjusted return.
3. **Is the signal stronger for fee increases or decreases?** Hypothesis: increases are stronger (closing is faster than opening new CDPs).
4. **Does vault utilization at time of spell queue predict trade return magnitude?** Higher utilization = more CDPs at risk = stronger signal.
5. **Has the edge decayed over time?** Compare 2020–2022 vs. 2023–present returns to detect crowding.
6. **Is the ETH perp the right instrument, or does wstETH spot/ETH spot show a cleaner signal?**

---

## Next Steps

| Step | Owner | Deadline |
|---|---|---|
| Build Maker spell history database (2020–present) from on-chain data | Researcher | T+7 days |
| Write Dune Analytics query for vault closure rates around spell events | Researcher | T+7 days |
| Pull ETH/BTC hourly OHLCV and align to spell timestamps | Researcher | T+10 days |
| Run primary backtest (entry at T+2h, exit at activation+4h) | Researcher | T+14 days |
| Run sensitivity analysis (entry timing, fee delta threshold) | Researcher | T+21 days |
| Present backtest results for go/no-go decision | Researcher + Zunid | T+28 days |
| If go: set up live `DSPause` event listener | Engineer | T+35 days |
| Paper trade first 3 qualifying events | Trader | Ongoing |
