---
title: "Ribbon/Thetanuts Vault Expiry Delta-Unwind"
status: HYPOTHESIS
mechanism: 4
implementation: 6
safety: 6
frequency: 6
composite: 864
categories:
  - options-derivatives
  - calendar-seasonal
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Every Friday at 08:00 UTC, Deribit weekly options expire. Ribbon Finance and Thetanuts covered-call vaults mechanically sell call options each week, concentrating option supply at near-money strikes. Market makers who absorb this supply delta-hedge by shorting the underlying spot or perpetual. At expiry, those calls go to zero delta — the hedge is no longer needed and market makers buy back their short positions. This creates a mechanical, time-stamped buy impulse in BTC and ETH in the window surrounding 08:00 UTC Friday.

**Causal chain:**

1. Vault sells calls (Thursday evening / Friday pre-market) → market maker buys calls, goes short delta in spot/perp to hedge
2. As expiry approaches and calls decay toward zero delta, market maker reduces short hedge incrementally
3. At 08:00 UTC expiry, remaining delta collapses to zero instantaneously → market maker buys back residual short in a compressed window
4. Net effect: concentrated buy flow in BTC/ETH perp between ~07:30–09:00 UTC Friday
5. Post-expiry, vault re-deposits and sells new calls (typically Friday 12:00–16:00 UTC) → new market maker short hedges established → renewed sell pressure in afternoon

**Secondary hypothesis:** The afternoon re-hedge creates a fade opportunity — short BTC/ETH perp after new vault option sales are confirmed on-chain.

---

## Structural Mechanism — WHY This Must Happen

The mechanism is real but **probabilistic, not guaranteed**. Here is the honest decomposition:

**What is guaranteed:**
- Deribit weekly expiry occurs every Friday at 08:00 UTC — this is contractually fixed
- Options at expiry have delta = 0 (if OTM) or delta = 1 (if ITM) — no ambiguity
- A market maker holding a delta-hedged call position MUST adjust their hedge at expiry — this is a mechanical accounting necessity

**What is probabilistic:**
- The SIZE of the vault's option sale relative to total Deribit OI is variable and has declined since 2022 peak TVL
- Market makers may pre-hedge the unwind (gamma scalping into expiry), spreading the buy flow across Thursday night rather than concentrating it at 08:00
- Other large option positions at the same expiry may have opposing delta effects (e.g., large put OI creates opposite hedge dynamics)
- The vault may not roll on a given Friday (governance pause, low TVL, gas issues)

**Why it's still worth testing:**
The vault's selling is mechanical and on-chain verifiable. The timing is fixed. Even if the effect is small, it is directional and repeating — suitable for a systematic, low-frequency strategy.

---

## Entry / Exit Rules

### Morning Leg (Primary)

| Parameter | Value |
|-----------|-------|
| Instrument | BTC-PERP and/or ETH-PERP on Hyperliquid |
| Direction | Long |
| Entry time | 07:30 UTC every Friday |
| Entry type | Market order (or limit within 0.05% of mid) |
| Exit time | 09:00 UTC (hard time exit) |
| Stop loss | -1.5% from entry price (hard stop, resting order) |
| Take profit | None — time exit only; let the window close the trade |
| Trade filter | Only trade if combined Ribbon + Thetanuts vault TVL > $20M (check Thursday evening) |
| Trade filter 2 | Only trade if net Deribit call OI for Friday expiry > 5,000 BTC equivalent (confirms material hedge exists) |

### Afternoon Leg (Secondary — lower conviction)

| Parameter | Value |
|-----------|-------|
| Direction | Short |
| Entry trigger | On-chain confirmation of new vault option sale (Ribbon vault transaction on Ethereum mainnet, or Thetanuts vault state change) |
| Entry window | 12:00–16:00 UTC Friday only |
| Entry type | Market order within 10 minutes of on-chain confirmation |
| Exit time | 20:00 UTC Friday (hard time exit) |
| Stop loss | -1.5% from entry |
| Trade filter | Same TVL filter as morning leg |

### Non-trade conditions (skip the week entirely)
- FOMC meeting day falls on Friday
- Scheduled major protocol upgrade or Deribit maintenance window
- BTC 24h realized volatility > 5% (macro noise overwhelms the signal)
- Vault TVL < $20M

---

## Position Sizing

- **Base size:** 0.5% of portfolio notional per leg (morning and afternoon treated as separate trades)
- **Maximum combined exposure:** 1% of portfolio at any time (morning leg must be closed before afternoon leg opens)
- **Leverage:** 2x maximum — this is a low-conviction, short-window trade; leverage amplifies noise as much as signal
- **Scaling:** Do not scale up based on recent wins. TVL-proportional scaling is theoretically justified but introduces data dependency — keep flat sizing during backtest phase
- **No pyramiding** within the 90-minute morning window

---

## Backtest Methodology

### Data Required

| Dataset | Source | Notes |
|---------|--------|-------|
| BTC/ETH 1-minute OHLCV | Tardis.dev (historical Deribit and Binance) | Need Friday windows 2021–2024 |
| Deribit options OI by expiry and strike | Tardis.dev options data feed | `https://tardis.dev/api/v1/exchanges/deribit` |
| Ribbon Finance vault TVL and option sale transactions | Ethereum mainnet via The Graph or Dune Analytics | Ribbon subgraph: `https://thegraph.com/hosted-service/subgraph/ribbon-finance/ribbon-v2` |
| Thetanuts vault TVL | Thetanuts on-chain data via Dune | Search Dune for Thetanuts vault addresses |
| Hyperliquid perp price (for live execution reference) | Hyperliquid public API | `https://api.hyperliquid.xyz/info` |
| Funding rates (to adjust PnL) | Coinglass or Hyperliquid API | Deduct funding cost from trade PnL |

### Backtest Period
- **Primary:** January 2022 – December 2024 (covers peak TVL and decline)
- **Sub-period analysis:** 2022 (high TVL), 2023 (mid TVL), 2024 (low TVL) — test whether effect decays with TVL

### Methodology Steps

1. **Identify all Friday expiry dates** in the period (Deribit expiry calendar is fixed)
2. **Filter by TVL threshold:** For each Friday, check vault TVL as of Thursday 20:00 UTC. Exclude weeks below $20M
3. **Filter by OI threshold:** Check Deribit call OI for that Friday's expiry as of Thursday 20:00 UTC. Exclude weeks below 5,000 BTC equivalent
4. **Simulate morning leg:** Enter long at 07:30 UTC close price (1-min bar), exit at 09:00 UTC close price, apply -1.5% stop
5. **Simulate afternoon leg:** Use on-chain vault transaction timestamp as entry signal. If no transaction detected by 16:00 UTC, skip afternoon leg
6. **Calculate PnL:** Include 0.05% round-trip slippage per leg, deduct funding rate for holding period
7. **Baseline comparison:** Compare against a naive "long BTC every Friday 07:30–09:00" (no filters) to isolate the structural filter's contribution

### Metrics to Report

| Metric | Target |
|--------|--------|
| Win rate (morning leg) | > 55% |
| Average PnL per trade (net of costs) | > 0.15% |
| Sharpe ratio (annualized, morning leg only) | > 0.8 |
| Max drawdown (consecutive losing Fridays) | < 5% portfolio |
| TVL-stratified win rate | Higher TVL weeks should show higher win rate |
| Decay test | Win rate in 2024 vs 2022 — expect lower; quantify how much |

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. Morning leg win rate ≥ 55% across full backtest period (not just cherry-picked sub-period)
2. Average net PnL per trade ≥ 0.15% after 0.05% round-trip slippage and funding
3. TVL-stratified analysis shows statistically higher win rate in high-TVL weeks (confirms the mechanism, not noise)
4. 2022 sub-period Sharpe > 1.0 (validates the mechanism existed when vault size was material)
5. Strategy is not explained by "long BTC on Friday mornings" alone — filtered sample must outperform naive baseline by ≥ 0.10% per trade
6. Afternoon leg analyzed separately — only include in live trading if it shows independent positive expectancy

---

## Kill Criteria

Abandon the strategy (or demote to archive) if any of the following occur:

| Trigger | Action |
|---------|--------|
| Combined Ribbon + Thetanuts TVL falls below $10M and stays there for 4 consecutive weeks | Kill — mechanism is too small to matter |
| Backtest win rate < 52% across full period | Kill — no edge above noise |
| Live paper trading: 8 consecutive losing Friday morning legs | Kill — mechanism has broken down |
| Deribit changes weekly expiry timing or structure | Re-evaluate from scratch |
| Ribbon or Thetanuts vaults are deprecated or migrate to a different option protocol | Kill unless equivalent vault TVL found elsewhere |
| Morning leg average PnL < 0.05% net after 6 months paper trading | Kill — costs eat the edge |

---

## Risks — Honest Assessment

### Mechanism Risks

**Delta hedge pre-unwind:** Sophisticated market makers begin reducing their delta hedge as gamma decays through Thursday night, not just at 08:00 UTC. If the unwind is spread over 12 hours, the 90-minute window captures only a fraction of the flow — the signal-to-noise ratio collapses.

**Vault TVL decline is structural:** Ribbon Finance's vault TVL peaked at ~$300M in late 2021 and has declined significantly. At current TVL levels, the vault's option sales are a small fraction of total Deribit OI. The mechanical flow may be too small to move price against macro and other expiry-related flows.

**Max-pain overlap:** Options expiry gravity (max-pain pinning) is a known, widely-traded effect. This strategy's morning leg may simply be capturing the same phenomenon already in Zunid's pipeline. If so, it adds no diversification — it's a correlated duplicate.

**Competing expiry flows:** Large institutional put positions, barrier options, and exotic structures also expire on Fridays. Their delta dynamics may be opposite to the vault's call dynamics, netting out the effect.

### Execution Risks

**Slippage in the window:** If the effect is real and known, other participants front-run it. Entering at 07:30 UTC may mean entering after the move has already started. Test sensitivity to entry time (07:00, 07:15, 07:30, 07:45).

**Funding rate drag:** Holding a long perp position, even for 90 minutes, incurs pro-rated funding. In high-funding environments (bull markets), this can meaningfully erode a small expected PnL.

**On-chain latency for afternoon leg:** Detecting the vault's new option sale on-chain and executing within the hedge establishment window requires monitoring infrastructure. Latency of 10–30 minutes may mean entering after the new hedge is already established.

### Strategic Risks

**This is a known effect:** Options expiry patterns are documented in academic literature and widely traded. The edge, if it exists, is likely already partially arbitraged. Expect the backtest to show a decaying edge over time.

**Small expected PnL per trade:** At 0.5% position size and ~0.2% expected move, gross PnL per trade is ~0.1% of portfolio. After costs, this is marginal. A single bad week (macro shock on a Friday) can wipe out months of gains.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|--------|---------------|--------------|
| Tardis.dev (options OI) | `https://tardis.dev/api/v1/exchanges/deribit/options-chain` | Weekly call OI by strike, Friday expiry dates |
| Tardis.dev (price data) | `https://tardis.dev/api/v1/exchanges/binance/candles` | BTC/ETH 1-min OHLCV, 2021–2024 |
| Dune Analytics (Ribbon TVL) | `https://dune.com/queries/` — search "Ribbon Finance vault TVL" | Weekly TVL snapshots |
| Dune Analytics (Thetanuts) | `https://dune.com/queries/` — search "Thetanuts vault" | Weekly TVL and option sale transactions |
| The Graph (Ribbon V2) | `https://api.thegraph.com/subgraphs/name/ribbon-finance/ribbon-v2` | Vault option sale events with timestamps |
| Coinglass (funding rates) | `https://www.coinglass.com/FundingRate` | BTC/ETH 8h funding rate history |
| Hyperliquid API (live) | `https://api.hyperliquid.xyz/info` | Live perp prices for paper trading |
| Deribit public API | `https://www.deribit.com/api/v2/public/get_book_summary_by_currency` | Real-time OI verification pre-trade |

---

## Notes for Backtest Builder

- The afternoon leg is lower conviction and harder to automate (requires on-chain monitoring). Backtest the morning leg first and treat the afternoon leg as a separate, optional module.
- Test the morning leg with and without the TVL filter to isolate its contribution. If the unfiltered version performs equally well, the TVL filter is noise and the strategy is just "long BTC Friday mornings" — which is not a structural edge.
- Pay particular attention to the 2022 vs 2024 performance split. A decaying edge is expected; the question is whether it's still tradeable at current TVL levels or whether the strategy is a historical artifact.
- Consider running the same logic on ETH separately from BTC — Thetanuts has historically had more ETH vault TVL, so the ETH signal may be stronger.
