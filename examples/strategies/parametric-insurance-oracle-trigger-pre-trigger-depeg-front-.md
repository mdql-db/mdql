---
title: "Parametric Insurance Oracle Trigger — Pre-Trigger Depeg Front-Run"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 3
frequency: 1
composite: 75
categories:
  - defi-protocol
  - liquidation
  - stablecoin
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a parametric insurance protocol's covered asset trades within 5% of a documented oracle trigger threshold, a mechanical payout liability is imminent. The insurance protocol's capital pool — invested in yield-bearing assets — must be partially or fully liquidated to fund payouts. Simultaneously, sophisticated actors aware of the threshold may accelerate the move toward it (attack vector) or position ahead of the forced liquidation flow. The edge is not in predicting *whether* the depeg happens, but in knowing *exactly* what price level forces a deterministic, protocol-mandated capital movement.

**Causal chain:**
1. Parametric protocol publishes trigger threshold in smart contract or documentation (e.g., stablecoin price < $0.95 for 10 consecutive Chainlink heartbeats)
2. Covered asset approaches threshold — oracle readings become observable in real time
3. If threshold is crossed: smart contract automatically crystallizes payout liability; capital pool begins liquidation to fund claims
4. Liquidation flow is directional and size-bounded by pool TVL (public on-chain)
5. Short the covered asset (capturing continued depeg pressure + forced selling); long the payout denomination asset (USDC/ETH) if pool liquidation creates buying pressure there

**What this is NOT:** A prediction that the asset will depeg. It is a bet that *once the oracle is clearly trending toward the threshold*, the mechanical consequences of crossing it create exploitable flow — and that the threshold itself acts as a gravity well for positioning by other informed actors.

---

## Structural Mechanism — WHY This Must Happen

Parametric insurance is designed to remove human discretion. The payout trigger is encoded in the smart contract or in a governance-ratified parameter set. This is the structural guarantee:

**Layer 1 — Oracle trigger is deterministic.** Chainlink price feeds are public and update on a fixed heartbeat (typically every 1 hour or on 0.5% deviation). The trigger condition (e.g., "price < $0.95 for N consecutive updates") is verifiable before it fires. Anyone can watch the oracle and know with certainty whether the next update will satisfy the condition.

**Layer 2 — Payout liability is automatic.** Unlike Nexus Mutual (which requires a claims committee vote), protocols like Sherlock and Unslashed use parametric triggers: no human can block the payout once the oracle condition is met. The smart contract executes. This is the mechanical forcing function.

**Layer 3 — Capital pool liquidation is bounded and observable.** Protocol capital pools are on-chain. TVL is queryable. The maximum payout liability = min(coverage amount, pool TVL). This sets an upper bound on forced selling. The composition of the pool (staked USDC, staked ETH, protocol tokens) is partially observable via on-chain data.

**Layer 4 — Threshold creates adversarial game theory.** Once an asset is within 5% of the trigger, the threshold becomes a Schelling point. Attackers may push toward it (profitable if they hold puts or shorts). Defenders (protocol LPs) may attempt to defend it. This adversarial dynamic itself creates volatility clustering near the threshold — tradeable in its own right.

**Degree of guarantee:** The oracle trigger firing is guaranteed IF the price reaches the threshold. The price reaching the threshold is NOT guaranteed — hence score 5/10, not 8+. The edge is conditional, not unconditional.

---

## Entry Rules


### Universe Definition
- Covered assets: stablecoins or tokens with active parametric coverage on Sherlock, Unslashed Finance, or Etherisc with documented trigger thresholds
- Minimum pool TVL: $5M (below this, liquidation flow is too small to be meaningful)
- Trigger threshold must be publicly documented (smart contract or governance post)

### Signal Construction
**Proximity signal:** `proximity = (current_price - trigger_threshold) / trigger_threshold`
- Active zone: proximity < 5% (asset within 5% of trigger)
- Hot zone: proximity < 2% (asset within 2% of trigger)

**Trend confirmation:** Price must be trending *toward* threshold, not away from it
- Condition: 4-hour EMA of price is declining (for depeg scenarios) AND current price < price 24 hours ago
- This is a directional filter, not a pattern — it confirms the oracle is accumulating trigger-satisfying readings

**Oracle accumulation count:** For time-based triggers (e.g., "price < $0.95 for 10 consecutive updates"), track how many consecutive qualifying oracle readings have occurred. Entry when count ≥ 50% of required consecutive readings.

### Entry
- **Short the covered asset** (via perp on Hyperliquid if listed, or spot short via borrowing)
- **Long USDC or ETH** (payout denomination) — only if pool composition confirms liquidation will create buying pressure in that asset; skip this leg if pool is already in USDC
- Entry trigger: ALL of the following must be true:
  1. Proximity < 5%
  2. 4H trend is toward threshold
  3. Oracle accumulation count ≥ 50% of required consecutive readings (if time-based trigger)
  4. Pool TVL > $5M (confirmed on-chain at entry)

## Exit Rules

### Exit Rules
**Take profit:**
- Trigger fires: close short within 2 hours of confirmed trigger event (on-chain transaction confirming payout initiation)
- Price reaches trigger threshold: close 50% of position immediately; trail stop on remainder

**Stop loss:**
- Asset recovers > 10% from trigger threshold (proximity increases to > 10%): full exit
- Oracle accumulation count resets to zero (price bounced above threshold): exit within next oracle heartbeat

**Time stop:**
- If trigger has not fired within 72 hours of entry: exit regardless of proximity (prevents capital lock-up in slow-moving situations)

### Position Direction Summary
| Scenario | Action |
|---|---|
| Stablecoin depeg approaching trigger | Short stablecoin perp (or borrow/sell spot) |
| Pool denominated in ETH/BTC | Long ETH/BTC perp (forced liquidation buying) |
| Pool denominated in USDC | Skip long leg (no forced buying pressure) |
| Trigger fires | Close short; monitor for overshoot and mean-reversion |

---

## Position Sizing

**Base size:** 0.5% of portfolio per trade (small — this is a niche, low-frequency strategy)

**Scaling logic:**
- Scale to 1.0% if proximity < 2% AND oracle accumulation count > 75% of required readings
- Never exceed 1.5% of portfolio on a single trigger event

**Rationale for small size:** Pool TVL caps the forced flow. If pool TVL is $10M and coverage is $5M, the maximum forced selling is $5M — a rounding error in most liquid markets. Position size must be calibrated to the *flow size*, not to conviction. This strategy is about being directionally correct on a mechanical event, not about leverage.

**Leverage:** Maximum 3x on perps. The covered asset is already distressed — leverage amplifies both the edge and the gap-risk if the asset flash-recovers.

**Kelly adjustment:** Until backtest provides win rate and average R, use 25% Kelly (i.e., 0.5% base). Revisit after 20 observed events.

---

## Backtest Methodology

### Data Sources
| Data | Source | URL/Endpoint |
|---|---|---|
| Stablecoin prices (hourly) | Chainlink oracle logs | `https://data.chain.link` — historical round data via `AggregatorV3Interface.getRoundData()` |
| Stablecoin prices (tick) | CoinGecko historical | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| Sherlock coverage terms | Sherlock docs + governance | `https://docs.sherlock.xyz` + Sherlock Snapshot |
| Unslashed coverage terms | Unslashed docs | `https://docs.unslashed.finance` |
| Protocol pool TVL | DefiLlama | `https://api.llama.fi/protocol/{protocol}` |
| On-chain pool composition | Etherscan / Dune Analytics | Custom Dune query on protocol contracts |
| Perp prices (Hyperliquid) | Hyperliquid API | `https://api.hyperliquid.xyz/info` |

### Historical Events to Backtest
Identify all historical instances where a parametric-covered asset approached its documented trigger threshold. Candidate events:

1. **USDC depeg — March 2023** (SVB crisis): USDC hit ~$0.87. Check whether any Sherlock/Unslashed coverage had a $0.95 trigger. Document oracle readings and timing.
2. **UST depeg — May 2022**: Extreme event. Most parametric protocols were not yet live, but check Etherisc/Unslashed coverage if any existed.
3. **FRAX/LUSD minor depegs**: Smaller events, better for testing proximity signal without full trigger.
4. **Euler Finance hack — March 2023**: Sherlock had active coverage. Payout was triggered. This is the cleanest historical case — reconstruct the oracle timeline and price action.

### Metrics to Compute
- **Win rate:** % of entries where price continued toward/through threshold before hitting stop
- **Average R:** Average profit/loss in units of initial risk (stop distance)
- **Time to resolution:** Distribution of hours from entry to exit (take profit or stop)
- **False proximity rate:** How often does proximity < 5% occur without eventual trigger? (Base rate of false signals)
- **Slippage estimate:** For each historical event, estimate bid-ask spread and market impact at entry/exit given position size vs. daily volume

### Baseline Comparison
- Compare returns to: (a) random short entry on distressed stablecoins, (b) buy-and-hold short from first 10% depeg
- The strategy must show positive edge *specifically from the proximity-to-threshold signal*, not just from "short depegging stablecoins"

### Backtest Limitations to Document
- Survivorship bias: only events where coverage terms are recoverable post-hoc
- Look-ahead bias risk: ensure trigger threshold was publicly known *before* the event, not reconstructed afterward
- Thin data: parametric insurance is young (~2021 onward); expect fewer than 20 clean events total

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **Minimum 8 historical events** with complete data (price, oracle readings, coverage terms, pool TVL)
2. **Win rate ≥ 55%** on the short leg (covered asset continues toward/through threshold)
3. **Average R ≥ 1.2** (average winner is at least 1.2x the average loser)
4. **No single event accounts for > 40% of total backtest P&L** (not a one-event wonder)
5. **False proximity rate documented:** Know the base rate of "proximity < 5% but no trigger" — if > 70% of proximity signals are false, the entry criteria need tightening
6. **Slippage-adjusted returns remain positive** at estimated 0.3% round-trip cost

If fewer than 8 events exist in history, the strategy cannot be backtested with statistical validity. In that case: move directly to paper trading with 0.1% position size and treat the first 10 live events as the "backtest."

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Backtest shows average R < 0.8** after slippage adjustment — the edge doesn't cover costs
2. **Parametric protocols migrate to TWAP oracles** with 24-hour+ averaging windows — the trigger becomes too slow to front-run and too noisy to track
3. **Pool TVL consistently < $2M** across all active protocols — flow is too small to be meaningful even if directionally correct
4. **3 consecutive live losses** where the covered asset recovered > 10% from threshold — suggests the proximity signal is attracting defenders who successfully repeg
5. **Protocol adds circuit breakers** that delay or gate payouts — removes the deterministic mechanical element
6. **Regulatory action** forces covered assets off Hyperliquid perps — removes the executable instrument

---

## Risks

### High Severity

**TWAP oracle problem:** If the trigger uses a 24-hour TWAP rather than spot price, a brief dip below threshold does not fire the trigger. Many protocols have moved to TWAP specifically to prevent manipulation. *Mitigation: verify oracle type before entry; skip TWAP-triggered protocols.*

**Pool size is irrelevant to market price:** If the covered asset is a major stablecoin (USDC, USDT), the insurance pool ($5-50M) is a rounding error vs. daily volume ($1-10B). The forced liquidation flow will not move the market. *Mitigation: only trade when pool TVL > 1% of covered asset's daily volume.*

**Attack vector risk:** If the strategy becomes known, sophisticated actors may use it to manipulate the oracle (push price to trigger, collect insurance, exit). Being on the same side as attackers is fine until the attack fails — then you're holding a short on an asset that just flash-recovered. *Mitigation: tight stop at 10% recovery from threshold.*

### Medium Severity

**Coverage terms change post-entry:** Protocols can governance-vote to change trigger thresholds. If the threshold moves away from current price, the signal disappears mid-trade. *Mitigation: monitor governance forums; exit if threshold change is proposed.*

**Payout in protocol token, not USDC:** Some protocols pay claims in their native token (e.g., SHER for Sherlock). If the payout token is illiquid, the forced liquidation flow is in an illiquid market — not tradeable. *Mitigation: verify payout denomination before entry.*

**Thin event history:** With fewer than 20 historical events, backtest results have wide confidence intervals. A 60% win rate on 10 events is statistically indistinguishable from 50%. *Mitigation: treat early live trades as extended backtest; size at 0.1% until 20 events observed.*

### Low Severity

**Hyperliquid listing risk:** Not all distressed stablecoins or covered tokens are listed on Hyperliquid perps. May need to use spot borrowing (higher friction, lower leverage). *Mitigation: maintain list of covered assets and their Hyperliquid listing status.*

**Timing of oracle reads:** Chainlink heartbeat is 1 hour for most feeds. Entry signal may lag by up to 1 hour. *Mitigation: monitor oracle contract directly via `latestRoundData()` rather than relying on price aggregators.*

---

## Data Sources

| Purpose | Source | Access Method |
|---|---|---|
| Chainlink oracle historical rounds | Chainlink Data Feeds | `AggregatorV3Interface.getRoundData(roundId)` on Ethereum mainnet |
| Chainlink feed addresses | Chainlink docs | `https://docs.chain.link/data-feeds/price-feeds/addresses` |
| Sherlock coverage terms | Sherlock protocol | `https://app.sherlock.xyz/audits` + `https://docs.sherlock.xyz` |
| Sherlock pool TVL | DefiLlama | `https://api.llama.fi/protocol/sherlock` |
| Sherlock on-chain contracts | Etherscan | Sherlock contract: `0x0865a889183039689034dA55c1Fd12aF5083eabF` |
| Unslashed coverage terms | Unslashed docs | `https://docs.unslashed.finance/products/coverage-mining` |
| Unslashed pool TVL | DefiLlama | `https://api.llama.fi/protocol/unslashed-finance` |
| Historical stablecoin prices | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=365` |
| Dune Analytics (oracle + pool queries) | Dune | `https://dune.com` — search "Sherlock claims" or "Unslashed oracle" |
| Hyperliquid perp listings + prices | Hyperliquid | `https://api.hyperliquid.xyz/info` (POST `{"type": "meta"}`) |
| Governance proposals (threshold changes) | Snapshot | `https://snapshot.org/#/sherlockdefi.eth` |

**Primary on-chain monitoring script requirement:** A lightweight script that polls `latestRoundData()` on relevant Chainlink feeds every 5 minutes and alerts when proximity < 5% of any documented trigger threshold. This is the core operational requirement before paper trading can begin.

---

*This document is a hypothesis specification. No backtest has been run. All claims about edge and win rate are theoretical pending empirical validation.*
