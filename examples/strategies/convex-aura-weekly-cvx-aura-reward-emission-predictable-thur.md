---
title: "CVX/AURA Weekly Epoch Dump"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 7
composite: 875
categories:
  - token-supply
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Convex Finance and Aura Finance distribute CVX and AURA rewards to vlCVX/vlAURA lockers on a fixed weekly epoch schedule. The epoch reset is encoded in the smart contract and executes at a deterministic block. A structurally significant fraction of claimants are yield farmers whose dominant strategy is immediate liquidation ("farm and dump"). This creates a predictable, recurring sell wave in the 24–48h window following each epoch reset.

**Causal chain:**

1. Convex epoch resets every Thursday ~00:00 UTC (block-deterministic)
2. Smart contract moves accrued CVX rewards from `pending` → `claimable` state at reset
3. Yield farmers (bots and manual claimers) claim within hours of reset — gas cost incentivises batching at epoch boundary rather than continuous claiming
4. Claimed tokens are sold on-market; no lockup, no vesting, immediate liquidity
5. Sell pressure is concentrated in a 12–48h window post-reset
6. CVX/AURA price depresses relative to pre-reset level
7. Price recovers as sell flow exhausts and natural demand absorbs

The edge is **not** that price always falls — it is that a known, contractually scheduled supply event creates a predictable window of elevated sell-side flow. The mechanism is structurally identical to token unlock shorts but at weekly cadence with smaller per-event magnitude.

---

## Structural Mechanism (WHY This Must Happen)

**What is guaranteed (score: 8):**
- Epoch reset timestamp is deterministic from contract state: `CvxLocker.epochCount()` and `rewardsDuration` (7 days, immutable)
- Reward tokens move to claimable state at reset — this is a smart contract state transition, not a human decision
- Total claimable CVX per epoch is readable on-chain before the reset occurs (accrued but not yet claimable)

**What is probabilistic (drops score to 6):**
- Fraction of claimants who sell immediately vs. re-lock or hold
- Timing of claim transactions within the post-reset window
- Market depth sufficient to absorb or resist the sell flow
- Whether sell pressure is already priced in by sophisticated actors front-running the front-runners

**Why the sell conversion rate is structurally high (not just historical):**
- vlCVX lockers are predominantly yield-maximising protocols and aggregators (Yearn, Beefy, etc.) whose mandates require liquidating non-native rewards
- Retail lockers seeking yield have no incentive to accumulate CVX beyond their locking position
- CVX has no productive use for non-Curve-governance actors beyond selling or re-locking; re-locking requires 16-week commitment, creating high friction

**Why this is not fully arbitraged away:**
- CVX and AURA perps have thin liquidity — large players cannot express the trade at scale without moving the market against themselves
- Weekly cadence means the trade must be re-executed 52x/year with active management, deterring set-and-forget funds
- The magnitude per epoch is small enough to be below the attention threshold of large desks

---

## Entry Rules


### Universe
- Primary: CVX perpetual futures (check Hyperliquid, dYdX, or spot borrow on Aave/Morpho)
- Secondary: AURA perpetual futures or spot borrow
- If perps unavailable or funding rate is prohibitive: skip that token for that epoch

### Pre-trade Check (T-24h before epoch reset)
1. Confirm epoch reset timestamp from contract (see Data Sources)
2. Pull current funding rate on CVX/AURA perp
3. Pull claimable reward estimate from `CvxLocker.claimableRewards(address)` aggregated across top 20 locker addresses (proxy for total sell volume)
4. **Skip trade if:** annualised funding rate cost > 50% (i.e., paying >1% per week to hold short)
5. **Skip trade if:** 7-day CVX price decline already >15% (sell pressure may be exhausted or capitulation already occurred)

### Entry
- **Time:** T-12h before epoch reset (e.g., if reset is Thursday 00:00 UTC, enter Wednesday 12:00 UTC)
- **Instrument:** CVX-PERP short (or spot borrow if perp unavailable)
- **Order type:** Limit order within 0.3% of mid; if unfilled within 30 minutes, use market order
- **Rationale for T-12h:** Early enough to capture pre-reset positioning, late enough to avoid multi-day carry cost

## Exit Rules

### Exit Rules (first condition met)
1. **Primary exit:** T+36h after epoch reset (Saturday ~12:00 UTC) — captures the bulk of the sell window without overstaying
2. **Stop loss:** Price rises >4% above entry — structural sell pressure has failed or been overwhelmed
3. **Profit take:** Price falls >8% from entry — take 50% off, trail stop on remainder at entry price
4. **Funding emergency exit:** If funding rate spikes to annualised >100% during the hold, exit immediately regardless of P&L

### AURA Mirror Trade
- Execute identical logic on AURA with 50% of CVX position size
- AURA epoch resets are correlated with Convex (Aura is built on Convex infrastructure) — confirm AURA epoch timing independently from `AuraLocker` contract

---

## Position Sizing

**Base position:** 0.5% of total portfolio NAV per trade (CVX) + 0.25% (AURA)
**Maximum combined exposure:** 1% of NAV per epoch

**Rationale:**
- Thin liquidity on CVX/AURA means larger positions create adverse slippage that erodes edge
- Weekly frequency means 52 occurrences/year — small per-trade sizing with high frequency is appropriate
- At 0.5% NAV, a 5% adverse move costs 0.025% NAV — acceptable given weekly cadence

**Scaling rule:** If backtest shows Sharpe > 1.5 and max drawdown < 10% over 52+ epochs, scale to 1% NAV (CVX) + 0.5% (AURA). Do not scale before go-live criteria are met.

**Liquidity check:** Before entry, verify that position size is <5% of 24h CVX volume on the target venue. If not, reduce size proportionally.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format |
|---|---|---|
| CVX price (hourly OHLCV) | CoinGecko API `/coins/convex-finance/market_chart` | JSON, free |
| AURA price (hourly OHLCV) | CoinGecko API `/coins/aura-finance/market_chart` | JSON, free |
| Epoch reset timestamps | On-chain: `CvxLocker` contract `0xD18140b4B819b895A3dba5442F959fA44994AF50` | Etherscan event logs |
| Claim volume per epoch | Dune Analytics: `convex.finance` dashboard, query `cvx_claims_by_epoch` | CSV export |
| Funding rates (historical) | Coinglass API or dYdX historical funding | JSON |
| vlCVX total locked | Convex API `https://api.convexfinance.com/api/cvx-locker` | JSON |

### Epoch Timestamp Extraction
```python
# Pull epoch boundaries from CvxLocker contract
from web3 import Web3
w3 = Web3(Web3.HTTPProvider('https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY'))
locker = w3.eth.contract(address='0xD18140b4B819b895A3dba5442F959fA44994AF50', abi=LOCKER_ABI)
epoch_count = locker.functions.epochCount().call()
rewards_duration = locker.functions.rewardsDuration().call()  # Should return 604800 (7 days)
# Reconstruct all epoch boundaries from genesis + n * rewards_duration
```

### Backtest Period
- **Start:** January 2022 (sufficient CVX liquidity established)
- **End:** Most recent completed epoch
- **Minimum epochs:** 52 (one full year) before drawing conclusions
- **Target:** 130+ epochs (2.5 years) for statistical robustness

### Simulation Rules
- Enter short at hourly close price at T-12h
- Exit at hourly close price at T+36h (or stop/take-profit if triggered intrabar)
- Apply 0.15% round-trip transaction cost (realistic for thin perp markets)
- Apply funding rate cost: pull actual historical funding for each epoch window
- No look-ahead bias: all signals derived from data available at entry time

### Metrics to Compute

| Metric | Target | Minimum Acceptable |
|---|---|---|
| Win rate | >55% | >50% |
| Average return per trade (net of costs) | >0.8% | >0.3% |
| Sharpe ratio (annualised) | >1.2 | >0.8 |
| Max drawdown (consecutive losing epochs) | <15% NAV | <25% NAV |
| Profit factor | >1.4 | >1.1 |
| Skew | Negative preferred (small losses, occasional large wins) | Neutral |

### Baseline Comparison
- **Null hypothesis:** Random 48h short entries on CVX (same position size, same exit rules) — if strategy does not beat random timing, the epoch effect is not real
- **Secondary baseline:** Buy-and-hold CVX short (always short) — strategy must outperform on risk-adjusted basis

### Segmentation Analysis (required, not optional)
- Split results by: bull market epochs vs. bear market epochs (use BTC 200-day MA as regime filter)
- Split by: high claim volume epochs vs. low claim volume epochs (use Dune data)
- Split by: epochs where funding was positive vs. negative at entry
- **If the edge only exists in bear markets or high-claim epochs, the strategy is conditional and must be filtered accordingly**

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

1. **Win rate ≥ 52%** over minimum 52 epochs
2. **Average net return per trade ≥ 0.4%** after all costs
3. **Sharpe ratio ≥ 0.8** annualised
4. **Strategy beats random-entry baseline** at p < 0.10 (one-tailed t-test on per-trade returns)
5. **No single epoch loss > 6% NAV** (position sizing discipline confirmed)
6. **Claim volume correlation confirmed:** Dune data shows higher claim volume epochs have worse CVX returns in the post-reset window (validates causal mechanism, not just pattern)
7. **Liquidity confirmed:** CVX average 24h volume on target venue > $500k (minimum viable for 0.5% NAV position without excessive slippage)

---

## Kill Criteria

Abandon strategy (stop paper trading or live trading) if any of the following occur:

### During Backtesting
- Win rate < 50% over 52+ epochs with no identifiable regime filter that rescues it
- Strategy fails to beat random-entry baseline at any significance level
- Claim volume shows no correlation with post-reset price action (mechanism is not operative)

### During Paper/Live Trading
- **5 consecutive losing epochs** — pause and re-examine; do not resume without identifying cause
- **Cumulative paper trading loss > 3% NAV** over any 13-epoch (quarter) window
- **CVX or AURA perp delisted** from primary venue — reassess liquidity before continuing
- **Convex governance vote passes** that changes epoch duration or reward distribution mechanism — re-backtest from scratch
- **vlCVX total locked increases >50% in a single month** — structural shift in locker composition may have changed sell conversion rate
- **Funding rate environment shifts** such that average cost per epoch exceeds 0.5% — edge is consumed by carry

---

## Risks

### Liquidity Risk (HIGH)
CVX and AURA are small-cap tokens. Perp open interest on most venues is thin. A 0.5% NAV position may represent a meaningful fraction of daily volume, creating adverse selection on entry and exit. **Mitigation:** Hard cap at 5% of 24h volume; use limit orders exclusively on entry.

### Mechanism Degradation Risk (MEDIUM)
If a large fraction of vlCVX lockers shift from "farm and dump" to "re-lock" behaviour (e.g., due to improved CVX utility or governance incentives), sell pressure per epoch decreases. This is detectable via Dune claim-vs-relock ratio. **Mitigation:** Monitor monthly; kill if re-lock rate exceeds 60% of claims.

### Front-Running Risk (MEDIUM)
If this pattern becomes widely known, sophisticated actors will short earlier (T-48h, T-72h), pulling the price impact forward and potentially causing a reversal by T-12h entry. **Mitigation:** Monitor whether the optimal entry window shifts over time in backtest; adjust entry timing if needed.

### Governance Risk (LOW-MEDIUM)
Convex can change epoch duration or reward mechanics via governance (multisig + timelock). Changes are visible on-chain with advance notice. **Mitigation:** Monitor Convex governance forum and on-chain timelock queue; exit all positions if relevant proposal passes.

### Correlation Risk (LOW)
CVX and AURA are highly correlated. Running both simultaneously does not provide diversification — it doubles concentration. **Mitigation:** Treat CVX + AURA as a single trade for risk purposes; combined exposure capped at 1% NAV.

### Perp Unavailability Risk (MEDIUM)
If CVX/AURA perps are not listed on Hyperliquid or accessible venues, the trade requires spot borrow (Aave, Morpho). Borrow rates are variable and can spike, consuming edge. **Mitigation:** Check borrow rate at T-24h; skip if annualised borrow cost > 30%.

---

## Data Sources

| Resource | URL / Endpoint |
|---|---|
| CvxLocker contract (Etherscan) | `https://etherscan.io/address/0xD18140b4B819b895A3dba5442F959fA44994AF50` |
| AuraLocker contract (Etherscan) | `https://etherscan.io/address/0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` |
| Convex API (locker stats) | `https://api.convexfinance.com/api/cvx-locker` |
| CVX price history (CoinGecko) | `https://api.coingecko.com/api/v3/coins/convex-finance/market_chart?vs_currency=usd&days=max&interval=hourly` |
| AURA price history (CoinGecko) | `https://api.coingecko.com/api/v3/coins/aura-finance/market_chart?vs_currency=usd&days=max&interval=hourly` |
| Dune Analytics — Convex claims | `https://dune.com/convex_community` (query: weekly claim volume) |
| Coinglass funding rates | `https://www.coinglass.com/FundingRate` (CVX if listed) |
| Etherscan event logs API | `https://api.etherscan.io/api?module=logs&action=getLogs&address=0xD18140b4B819b895A3dba5442F959fA44994AF50` |
| Convex governance forum | `https://gov.convexfinance.com` |

### On-Chain Epoch Verification Script (pseudocode)
```python
LOCKER_ADDRESS = "0xD18140b4B819b895A3dba5442F959fA44994AF50"
REWARDS_DURATION = 604800  # 7 days in seconds — verify from contract

# Get current epoch number and start time
current_epoch = locker.functions.epochCount().call()
# Each epoch boundary = genesis_timestamp + (epoch_number * REWARDS_DURATION)
# Genesis timestamp derivable from contract deployment block + first epoch event

# Next reset = current_epoch_start + REWARDS_DURATION
# Entry time = next_reset - 43200  (12 hours in seconds)
```

---

## Open Questions for Researcher Before Backtesting

1. Are CVX or AURA perps currently listed on Hyperliquid or any accessible venue? If not, what is the current borrow rate on Aave for CVX?
2. Does Dune have a pre-built query for CVX claim volume by epoch, or does a custom query need to be written?
3. Has the Convex epoch schedule ever been modified via governance? If so, identify the block where the change took effect to avoid contaminating backtest data.
4. What fraction of vlCVX is held by protocol-owned treasuries (Yearn, Beefy, etc.) vs. retail? This determines the structural floor on sell conversion rate.
5. Is there a meaningful options market on CVX that could provide an alternative expression of the trade with defined risk?
