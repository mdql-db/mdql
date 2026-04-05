---
title: "vlCVX Re-Lock Deadline — Convex Vote-Lock Expiry Cliff Sell"
status: HYPOTHESIS
mechanism: 6
implementation: 6
safety: 6
frequency: 3
composite: 648
categories:
  - token-supply
  - governance
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a large cohort of vlCVX (vote-locked CVX) tokens reaches its 16-week expiry epoch, a structurally predictable fraction of that cohort will not re-lock and will instead withdraw CVX to spot. This creates a measurable, time-stamped supply event that is visible on-chain before it occurs. The causal chain:

1. vlCVX holders who do not re-lock within the 1-week grace window **must** withdraw — the contract enforces this; there is no third option
2. Withdrawn CVX is liquid and no longer earning bribe yield — holders who chose not to re-lock have already revealed exit intent by inaction
3. The fraction that exits (historically estimated 30–50% of expiring cohort) represents net new sell supply hitting spot/perp markets in a concentrated window
4. If the expiring cohort is large (>2% of circulating CVX), the sell pressure is material relative to normal daily volume
5. Secondary effect: large exits reduce total vlCVX, reducing Curve gauge voting power controlled by Convex, which reduces bribe yields for remaining lockers, potentially triggering further exits (reflexive but not guaranteed)

**This is not "CVX tends to fall before expiry." It is: a contractually enforced supply release event occurs on a known date, and a predictable fraction of that supply will be sold.**

---

## Structural Mechanism — WHY This Must Happen

The Convex vote-lock contract (`CvxLockerV2`, deployed at `0x72a19342e8F1838460eBFCCEf09F6585e32db86E` on Ethereum mainnet) enforces the following rules in code:

- CVX can only be locked in 16-week epochs aligned to Thursday 00:00 UTC
- After expiry, tokens enter "expired" status — they cannot vote, earn bribes, or be re-locked without first withdrawing and re-depositing
- The grace period is 1 epoch (1 week) — after that, the tokens are simply idle capital earning nothing
- `lockedBalances(address)` and `epochCount()` expose the full expiry schedule publicly

**The supply release is not probabilistic — it is guaranteed.** What is probabilistic is the re-lock rate. The structural edge is that:

- The *total* expiring supply is known with certainty T-112 days in advance (when locks were created)
- The *net sell supply* = expiring cohort × (1 − re-lock rate)
- Even at 70% re-lock rate, a 10M CVX expiry epoch produces 3M CVX of net new supply
- CVX 30-day average daily volume (spot + perp) is approximately 2–5M USD equivalent — a 3M CVX release at ~$2/CVX = $6M is 1–3 days of volume

The mechanism is structurally analogous to token unlocks but with one important difference: re-locking requires active intent, meaning non-re-lockers are self-selected sellers, not passive recipients of vested tokens.

---

## Entry Rules


### Pre-Trade Checklist (run weekly, every Thursday)

1. Query `CvxLockerV2.epochCount()` and iterate epochs to find the next expiry date
2. Sum all locks expiring in the target epoch using `lockedBalances()` events or `Locked` event logs
3. Calculate expiry cohort as % of circulating CVX supply (source: CoinGecko API)
4. Check prior 3 epochs' re-lock rates from on-chain data (see Data Sources)
5. Check current CVX bribe APR (source: Llama Airforce or Votium dashboard) — if APR > 40%, re-lock incentive is high; apply stricter threshold

### Entry Conditions (ALL must be true)

| Condition | Threshold |
|-----------|-----------|
| Expiring cohort size | > 2M CVX **AND** > 2% of circulating supply |
| Prior epoch average re-lock rate | < 75% (i.e., historical exit rate > 25%) |
| CVX perp open interest on Hyperliquid | > $500K (liquidity check) |
| CVX funding rate | Not persistently negative > −0.05%/8hr for past 48hr (avoid crowded short) |
| Days to expiry | Enter at T−7 (exactly 7 days before epoch expiry Thursday) |

### Entry Execution

- Instrument: CVX-USDC perpetual on Hyperliquid
- Direction: Short
- Entry: Market order at Thursday open (00:00 UTC) at T−7
- Slippage budget: Accept up to 0.5% slippage from mid; abort if worse

## Exit Rules

### Exit Rules (first condition hit)

| Condition | Action |
|-----------|--------|
| T+7 days post-expiry (T+14 from entry) | Close 100% at market |
| Price moves +8% against entry (stop loss) | Close 100% at market |
| Price moves −15% in favor (take profit) | Close 50%; trail stop on remainder at −8% from peak |
| Funding rate exceeds −0.10%/8hr for 24hr | Close 100% (carry cost too high) |
| Re-lock rate tracking shows >85% re-locked mid-epoch | Close 100% early (signal invalidated) |

### Re-Lock Rate Mid-Epoch Monitoring

Query `CvxLockerV2` every 24 hours during the grace week. If new lock events (same cohort addresses re-locking) account for >85% of the expiring cohort, the sell pressure thesis is invalidated — exit early.

---

## Position Sizing

- **Base risk per trade:** 1% of portfolio NAV
- **Position size formula:** `Size = (Portfolio NAV × 0.01) / (Entry Price × 0.08)`
  - The 0.08 denominator is the stop-loss distance (8%)
  - Example: $100K portfolio, CVX at $2.00 → Size = $1,000 / $0.16 = 6,250 CVX short = $12,500 notional (12.5% of NAV, ~1.25× leverage)
- **Maximum notional:** 20% of portfolio NAV regardless of formula output
- **Leverage cap:** 2× — CVX is illiquid enough that higher leverage creates liquidation risk from wick moves
- **Scaling rule:** If expiry cohort > 5% of circulating supply AND prior re-lock rate < 60%, scale to 1.5% NAV risk (multiply position by 1.5×)

---

## Backtest Methodology

### Data Required

**On-chain (primary):**
- All `Locked` and `Withdrawn` events from `CvxLockerV2` (0x72a19342e8F1838460eBFCCEf09F6585e32db86E) from contract deployment (~October 2021) to present
- Extract via: Ethereum archive node RPC, The Graph (Convex subgraph), or Dune Analytics query
- Fields needed: `user`, `amount`, `lockIndex`, `epoch`, `timestamp`

**Price data:**
- CVX/USDT hourly OHLCV from Binance (available via Binance public API, symbol `CVXUSDT`, from ~Oct 2021)
- CVX perp funding rate history from Hyperliquid (available from ~2023; use Binance spot as proxy for earlier periods)

**Supplementary:**
- CVX circulating supply by date: CoinGecko `/coins/convex-finance/market_chart` endpoint
- Bribe APR by epoch: Llama Airforce API or Votium `claimable` contract events

### Backtest Steps

1. **Reconstruct epoch schedule:** Map all 16-week epochs from contract genesis. Each epoch starts Thursday 00:00 UTC.

2. **Compute expiry cohort per epoch:** For each epoch E, sum all CVX locked 16 weeks prior (epoch E−16). This is the expiring cohort.

3. **Compute re-lock rate per epoch:** For each address in the expiring cohort, check if they submitted a new `Locked` event within 7 days of expiry. Re-lock rate = re-lockers / total expiring addresses (weighted by CVX amount).

4. **Identify signal epochs:** Filter epochs where expiry cohort > 2M CVX AND > 2% circulating supply.

5. **Simulate trades:** For each signal epoch:
   - Entry: Short at T−7 close price (Thursday 00:00 UTC, 7 days before expiry)
   - Apply stop loss, take profit, and time exit rules
   - Record P&L in % terms

6. **Metrics to compute:**

| Metric | Target |
|--------|--------|
| Win rate | > 55% |
| Average win / average loss | > 1.5 |
| Sharpe ratio (annualized) | > 1.0 |
| Max drawdown (strategy-level) | < 20% |
| Number of trades | Minimum 15 for statistical validity |
| Median holding period | Report (expected 7–14 days) |

7. **Baseline comparison:** Compare against naive "short CVX every Thursday for 14 days" (no signal filter) to isolate the epoch-specific edge from general CVX downward drift.

8. **Subgroup analysis:**
   - Large cohort (>5% circulating) vs. medium cohort (2–5%)
   - High bribe APR epochs vs. low bribe APR epochs
   - Pre-2023 (Curve Wars peak) vs. post-2023 (cooling)

### Known Backtest Limitations

- CVX perp on Hyperliquid only exists from ~2023; earlier periods must use spot price as proxy (no funding cost modeled)
- Re-lock rate computation requires archive node access or Dune query — not trivial but feasible
- Sample size may be small: ~16 epochs per year × 3 years = ~48 epochs total, of which perhaps 15–25 meet the size threshold

---

## Go-Live Criteria

The backtest must show ALL of the following before moving to paper trading:

1. **Win rate ≥ 55%** across all signal epochs
2. **Profit factor ≥ 1.4** (gross wins / gross losses)
3. **Sharpe ≥ 0.8** on trade-level returns (not annualized portfolio)
4. **Statistically significant outperformance** vs. naive baseline (p < 0.10 acceptable given small sample; use bootstrap permutation test)
5. **No single trade > 25% of total strategy P&L** (concentration check)
6. **Re-lock rate < 75% is a valid filter:** Confirm that epochs where prior re-lock rate was > 75% had worse outcomes — if not, the filter adds no value and the entry rule must be revised

If fewer than 12 qualifying epochs exist in the backtest period, do not go live — insufficient sample. Wait for more data or widen the cohort size threshold to 1.5% circulating.

---

## Kill Criteria

Abandon the strategy (close positions, halt signal generation) if ANY of the following occur:

| Trigger | Action |
|---------|--------|
| 4 consecutive losing trades | Halt, review re-lock rate assumptions |
| Realized Sharpe < 0 over trailing 6 months (live) | Halt |
| CVX perp OI on Hyperliquid drops below $200K | Halt (liquidity insufficient) |
| Convex protocol announces deprecation or migration | Immediate halt |
| Curve governance changes vlCVX lock mechanics | Immediate halt — structural mechanism invalidated |
| Re-lock rates structurally shift above 85% for 3+ consecutive epochs | Halt — sellers are no longer exiting |
| CVX market cap drops below $50M | Halt — signal epochs will be too small in absolute terms |

---

## Risks

### High Severity

**Re-lock rate is the core unknown.** If bribe yields spike (e.g., a new protocol launches a large Curve bribe campaign), re-lock rates could jump to 90%+, eliminating net sell supply entirely. The strategy has no way to predict this in advance — only monitor it during the grace week.

**Convex relevance decay.** The Curve Wars narrative peaked in 2022. vlCVX lock participation has been declining. Fewer large epoch cohorts means fewer signal events. The strategy may have 2–3 qualifying epochs per year going forward, making it a very low-frequency trade.

**Reflexivity risk (adverse).** If large holders anticipate the sell pressure and buy CVX cheap before the expiry, the price may have already adjusted by T−7, leaving no edge at entry.

### Medium Severity

**Hyperliquid CVX liquidity.** CVX perp on Hyperliquid has thin OI. A $50K short may move the market. Position sizing must be conservative (see above). Slippage at exit during a volatile epoch could eliminate P&L.

**Funding rate carry.** If CVX perp is in backwardation (negative funding), shorts pay funding. At −0.05%/8hr, a 14-day hold costs ~2.1% in carry — meaningful against a 15% expected move target.

**Epoch timing ambiguity.** Convex epochs are Thursday-aligned but the exact unlock time depends on block timestamps. Off-by-one-day errors in entry timing could matter. Must verify epoch boundaries against contract state, not assumed calendar.

### Low Severity

**Smart contract upgrade risk.** Convex has upgraded the locker contract before (V1 → V2). A future upgrade could change lock mechanics. Monitor Convex governance forum and multisig activity.

**Correlation with broader crypto drawdowns.** CVX is highly correlated with ETH/BTC in risk-off environments. A macro sell-off during the hold window could produce a false positive (CVX falls but not because of the unlock).

---

## Data Sources

| Data | Source | URL / Endpoint |
|------|---------|----------------|
| vlCVX lock/unlock events | Ethereum RPC (archive) | `eth_getLogs` on `CvxLockerV2` at `0x72a19342e8F1838460eBFCCEf09F6585e32db86E` |
| vlCVX lock/unlock events (alternative) | Dune Analytics | `dune.com` — query `convex_finance.CvxLockerV2_evt_Locked` and `_evt_Withdrawn` |
| Convex subgraph | The Graph | `https://api.thegraph.com/subgraphs/name/convex-community/curve-pools` |
| CVX price history (spot) | Binance API | `https://api.binance.com/api/v3/klines?symbol=CVXUSDT&interval=1h` |
| CVX perp funding rate | Hyperliquid API | `https://api.hyperliquid.xyz/info` (fundingHistory endpoint) |
| CVX circulating supply | CoinGecko | `https://api.coingecko.com/api/v3/coins/convex-finance/market_chart?vs_currency=usd&days=max` |
| Bribe APR by epoch | Llama Airforce | `https://api.llama.airforce/bribes` |
| Votium bribe data | Votium | `https://votium.app/api/v2/bribes` |
| Convex epoch schedule | Convex UI / contract | `https://www.convexfinance.com/lock-cvx` — epoch table visible; verify against contract |
| Convex governance / upgrades | Convex forum | `https://gov.convex.finance` |

### Recommended Dune Query Starting Point

```sql
SELECT
  evt_block_time,
  "_user" as user_address,
  "_amount" / 1e18 as cvx_amount,
  "_lockIndex" as lock_index,
  contract_address
FROM convex_finance."CvxLockerV2_evt_Locked"
ORDER BY evt_block_time ASC
```

Cross-reference with `CvxLockerV2_evt_Withdrawn` to compute per-epoch re-lock rates. Group by 16-week epoch boundaries (first Thursday on or after contract deployment, then +112 days per epoch).

---

## Implementation Notes

- **Monitoring cadence:** Run epoch scanner every Thursday at 01:00 UTC (after epoch boundary). Alert if next expiry qualifies.
- **Re-lock rate tracker:** During the 7-day grace window, poll `lockedBalances()` for the top 20 addresses in the expiring cohort daily. If cumulative re-lock rate crosses 85%, trigger early exit.
- **Manual override:** Given low trade frequency (2–4 per year), each trade should have a human review of current Convex/Curve ecosystem context before entry. This is not a fully automated strategy.
- **Tax jurisdiction note:** 14-day hold periods may have specific short-term capital gains treatment depending on jurisdiction — factor into net return expectations.
