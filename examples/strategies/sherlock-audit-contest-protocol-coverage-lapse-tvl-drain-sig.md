---
title: "Sherlock Coverage Lapse → Governance Token Short"
status: HYPOTHESIS
mechanism: 4
implementation: 5
safety: 5
frequency: 1
composite: 100
categories:
  - defi-protocol
  - governance
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Sherlock's smart contract coverage lapses for a protocol (expiry with no renewal, or failed re-audit), risk-aware capital exits the protocol's TVL. This TVL drain is observable on-chain before the governance token prices the increased risk. The governance token underperforms its sector peers in the window surrounding the lapse date.

**Causal chain:**

1. Sherlock coverage expiry date is published on-chain and on sherlock.xyz — it is a fixed, known date
2. Sophisticated LPs (institutional, DAO treasuries, yield aggregators) monitor coverage status as part of risk management; many have internal policies requiring covered protocols
3. These LPs begin withdrawing 3–7 days before lapse to avoid queue delays and to exit before any adverse selection from other withdrawers
4. TVL drain is visible on DeFiLlama in real time; retail and governance token holders do not systematically monitor coverage registries
5. Governance token price lags TVL drain by 1–5 days because retail pricing is driven by narrative, not protocol risk parameters
6. Short the governance token in the window between TVL drain onset and governance token repricing

**What must be true for this to work:** The governance token must have a meaningful correlation with protocol TVL (i.e., TVL is a key valuation driver), and the market must not already be pricing coverage lapse risk at announcement.

---

## Structural Mechanism

**Why this happens (not just tends to happen):**

Sherlock coverage is a contractual backstop. Its removal is not a sentiment event — it is a binary, date-certain change in the protocol's risk profile. The mechanism is:

- **Forced exit by policy-constrained capital:** Institutional LPs and DAO treasury managers operating under risk frameworks that require covered protocols *must* exit when coverage lapses. This is not discretionary — it is a compliance trigger. The exit is mechanical.
- **Queue mechanics amplify early exit:** DeFi withdrawal queues (especially in lending protocols) mean that rational actors exit *before* the lapse, not after. The first mover avoids queue congestion. This creates a predictable front-running of the lapse date.
- **Information asymmetry:** Coverage status lives in Sherlock's on-chain registry and website. It is not surfaced by CoinGecko, CoinMarketCap, or any retail-facing aggregator. Governance token holders are systematically less informed than protocol LPs.

**Why the governance token lags:**
Governance tokens are priced primarily by retail sentiment, TVL narratives, and token emissions. A coverage lapse does not generate a press release or a tweet from the protocol team (they have incentive to downplay it). The signal propagates through LP exits → TVL drop → analyst notice → token repricing. This lag is the tradeable window.

**Structural ceiling on this edge:** Sherlock's market share is currently limited (~15–20 protocols under active coverage at any time). Signal count is low. This is a niche, manual strategy — not a systematic one at current Sherlock scale.

---

## Entry/Exit Rules

### Universe Filter (apply before entry)
- Protocol must have active Sherlock coverage with a documented expiry date
- Coverage renewal must NOT be announced as of entry date
- Protocol TVL must be >$50M at entry (ensures liquid governance token or tradeable perp)
- Governance token must have a liquid perpetual on Hyperliquid, dYdX, or a CEX perp (Binance/Bybit) OR spot token with >$1M daily volume
- Exclude protocols where the team has publicly stated renewal is in progress (reduces false positives)

### Entry Signal
- **Primary trigger:** TVL decline of ≥5% over any 3-day rolling window within the 14-day window before coverage expiry, with no protocol-specific news explaining the decline (e.g., not a hack, not a market-wide event)
- **Timing:** Enter short position 5–7 days before documented coverage lapse date, contingent on TVL drain signal being active
- **Entry price:** Use VWAP of the 4-hour candle following TVL drain confirmation
- **No TVL drain signal → no trade:** If TVL is flat or rising into lapse date, skip the trade (mechanism is not activating)

### Exit Rules
- **Primary exit:** 48 hours after coverage lapse date (T+2 post-lapse)
- **Stop loss:** If governance token rallies >12% from entry (absolute), close position — market is not pricing the lapse
- **Early exit — renewal announced:** If Sherlock announces coverage renewal at any point, close immediately at market (this is the primary tail risk)
- **Early exit — TVL stabilises:** If TVL stops declining and recovers >50% of the drawdown before lapse date, close position (mechanism has stalled)
- **Maximum hold:** 14 days from entry regardless of outcome

### Position Direction
- Primary: Short governance token perpetual
- Secondary (optional, if available): Short protocol LP tokens on secondary markets (e.g., Curve LP tokens on secondary DEX)

---

## Position Sizing

- **Base size:** 0.5% of portfolio per trade (small — low signal count, high idiosyncratic risk)
- **Maximum size:** 1.0% of portfolio (never exceed, given illiquidity of governance token perps)
- **Scaling:** Do not scale into position. Single entry at signal confirmation.
- **Leverage:** 1x–2x maximum. This is not a high-conviction mechanical trade — it is a probabilistic structural signal. Governance tokens are volatile; leverage amplifies idiosyncratic noise.
- **Correlation cap:** If two protocols lapse within the same 14-day window, treat as correlated (both are DeFi risk-off signals). Cap combined exposure at 1.5% of portfolio.

---

## Backtest Methodology

### Data Sources
- **Sherlock coverage registry:** sherlock.xyz/protocols (manual scrape; no public API confirmed — check GitHub repo at github.com/sherlock-audit for on-chain registry contract)
- **Sherlock on-chain data:** Sherlock deploys on Ethereum mainnet; coverage terms are stored in smart contracts — query via Etherscan or The Graph for historical coverage start/end dates
- **TVL data:** DeFiLlama API — `https://api.llama.fi/protocol/{protocol-slug}` returns daily TVL history
- **Governance token prices:** CoinGecko API — `https://api.coingecko.com/api/v3/coins/{id}/market_chart` for OHLCV history
- **Sector benchmark:** DeFiLlama governance token index or manually construct equal-weight basket of comparable protocols (same category: lending, DEX, etc.)

### Sample Construction
- Identify all historical Sherlock coverage lapses (non-renewals) since Sherlock launched (~2022)
- Expected sample size: 5–15 events (small — this is the core limitation)
- For each event, record: lapse date, protocol name, TVL at T-14, T-7, T-0, T+2, T+7; governance token price at same intervals

### Metrics to Compute
| Metric | Definition |
|---|---|
| TVL drawdown | % TVL change from T-14 to T+2 |
| Token alpha | Governance token return minus sector benchmark return, T-7 to T+2 |
| Signal lead time | Days between TVL drain onset (≥5% 3-day drop) and lapse date |
| Win rate | % of trades where token alpha is negative (short profits) |
| Average return | Mean token alpha across all events |
| Max adverse excursion | Worst intra-trade drawdown against the short |
| Sharpe (annualised) | If sample allows; likely not meaningful at N<15 |

### Baseline Comparison
- Compare governance token return (T-7 to T+2) against: (a) BTC return same window, (b) equal-weight DeFi governance token basket same window
- Null hypothesis: coverage lapse has no effect on governance token relative performance
- Reject null if: mean alpha is negative AND p-value <0.10 (use t-test; accept low bar given small N)

### Backtest Limitations to Document
- Sample size is likely <15 events — no statistical significance is achievable; treat as case study analysis
- Survivorship bias: protocols that lapsed and were hacked afterward will show extreme returns; flag these separately
- Confounding events: market-wide DeFi drawdowns will contaminate the signal; control by using sector-relative returns

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **Directional consistency:** ≥60% of historical lapse events show negative governance token alpha (T-7 to T+2) vs. sector benchmark
2. **TVL lead signal validity:** In ≥60% of events, TVL decline of ≥5% is observable in the 7 days before lapse (confirming the entry trigger fires in time)
3. **No catastrophic adverse cases:** No single event shows >25% adverse move against the short within the hold window (if one does, document the cause and assess whether the stop-loss rule would have contained it)
4. **Renewal announcement risk quantified:** Document how many historical lapses were preceded by a surprise renewal announcement within the entry window — this is the primary stop-loss trigger frequency

**If sample size is <8 events:** Do not go-live based on backtest alone. Move directly to paper trading with live monitoring of the next 3 lapse events before committing capital.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

- **Paper trading:** 3 consecutive paper trades show positive governance token alpha (short loses) with no confounding market events — mechanism is not activating
- **Live trading:** 2 consecutive live losses exceeding 8% each (stop-loss triggered both times) — market is pricing lapse risk faster than our entry
- **Structural change:** Sherlock introduces automatic renewal or a public renewal announcement system that eliminates the information asymmetry window
- **Market structure change:** Governance token perps become unavailable or spreads widen >3% (execution cost destroys edge)
- **Signal count drops to zero:** Sherlock loses market share and no new lapses occur for 12 months — strategy is dormant, not dead; revisit if Sherlock grows

---

## Risks

### Primary Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Renewal announced after entry | High | Stop-loss rule: close immediately on renewal announcement; monitor Sherlock Twitter/Discord daily |
| Governance token doesn't price TVL | High | Entry filter: require TVL drain signal before entering; skip if TVL flat |
| No liquid perp available | Medium | Pre-screen universe; spot short via borrowing if perp unavailable (check Aave/Compound borrow availability) |
| Market-wide DeFi selloff contaminates signal | Medium | Use sector-relative returns in backtest; in live trading, note if BTC/ETH down >5% in same window |
| Sample size too small for inference | High | Accept this limitation; treat as qualitative signal until Sherlock scales |
| Protocol team downplays lapse publicly | Low | Irrelevant if TVL drain is already occurring — trade the flow, not the narrative |

### Tail Risk
A protocol losing coverage could subsequently get hacked (coverage lapse → hack → governance token collapses 80%+). This looks like a win for the short but is actually a dangerous outcome: (a) it may be unforeseeable, (b) it creates a false sense of edge from a tail event. **Flag any backtest event where a hack occurred within 30 days of lapse — exclude from return calculations and analyse separately.**

### Honest Assessment
This strategy has a real causal mechanism but is severely limited by signal count. Sherlock covers ~15–20 protocols at any time; lapses are rare. This is a **monitoring strategy** more than a systematic one — it requires a researcher to track Sherlock's coverage registry weekly and act manually when a lapse approaches. The edge is real in theory; whether it is real in practice depends on whether institutional LPs actually exit mechanically (unverified assumption) and whether governance tokens have sufficient TVL sensitivity (varies widely by protocol).

---

## Data Sources

| Source | URL / Endpoint | Data |
|---|---|---|
| Sherlock protocol list | https://sherlock.xyz/protocols | Active coverage, expiry dates (manual) |
| Sherlock GitHub | https://github.com/sherlock-audit | On-chain contract addresses for registry |
| DeFiLlama protocol TVL | `https://api.llama.fi/protocol/{slug}` | Daily TVL history |
| DeFiLlama protocol list | `https://api.llama.fi/protocols` | Protocol slugs and metadata |
| CoinGecko OHLCV | `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=90` | Governance token price history |
| Hyperliquid perp list | https://app.hyperliquid.xyz/trade | Check perp availability for governance tokens |
| Etherscan | https://etherscan.io | On-chain verification of coverage contract state |
| The Graph | https://thegraph.com/explorer | Query Sherlock subgraph if available |

**Weekly monitoring action:** Every Monday, check sherlock.xyz/protocols for any coverage expiry dates within the next 21 days. If found, begin TVL monitoring via DeFiLlama daily. Set a calendar alert for T-7 before lapse date.
