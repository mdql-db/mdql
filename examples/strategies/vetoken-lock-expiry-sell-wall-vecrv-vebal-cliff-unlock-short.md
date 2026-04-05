---
title: "veToken Lock Expiry Sell Wall — veCRV/veBAL Cliff Unlock Short"
status: HYPOTHESIS
mechanism: 6
implementation: 5
safety: 6
frequency: 3
composite: 540
categories:
  - token-supply
  - governance
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Large veCRV unlock events (≥500K CRV unlocking from a single address or address cohort within the same Thursday epoch) cause measurable net sell pressure on CRV within a 72-hour post-unlock window, because:

1. The locker has borne illiquidity cost (opportunity cost of locked capital) for up to 4 years
2. The unlock date is written into the lock contract at creation — it is not a surprise to the locker, meaning any re-lock decision is pre-planned
3. Addresses that do not re-lock within the same block or same epoch as expiry reveal, by inaction, an intent to exit or rotate
4. CRV price at unlock is frequently below the price at lock entry (given CRV's long-term downtrend), creating additional incentive to exit rather than re-lock at a loss

**Causal chain:**
Lock expires (on-chain, deterministic) → CRV becomes transferable → locker either re-locks immediately or holds liquid CRV → liquid CRV represents latent sell supply → if locker does not re-lock within 1 epoch (7 days), probability of sell increases → price impact materialises within 72h of unlock for large single-address events.

**Null hypothesis to disprove:** CRV price in the 72h post-unlock window for qualifying events is not statistically different from a random 72h window in the same period.

---

## Structural Mechanism

### Why this is real (not just pattern-based)

veCRV is governed by `VotingEscrow.vy` on Ethereum mainnet. The lock structure is:

- User calls `create_lock(amount, unlock_time)` or `increase_unlock_time()`
- `unlock_time` is rounded down to the nearest Thursday (weekly epoch boundary, Unix timestamp % 604800 == 0)
- At `unlock_time`, the contract allows `withdraw()` — CRV becomes freely transferable
- There is **no automatic re-lock** — the user must actively call `create_lock()` again

This means:
- Every unlock date is readable from contract state at lock creation time
- The supply event is **contractually guaranteed** — the CRV will be withdrawable at that timestamp regardless of price, governance, or market conditions
- The only uncertainty is the locker's behavioural response (re-lock vs. sell)

### Why re-locking is not always the default

- Protocols (Convex, Yearn, Frax) tend to re-lock perpetually — these are **noise** and should be filtered
- EOA wallets and non-protocol multisigs have no programmatic obligation to re-lock
- CRV's declining bribe yield (as TVL has fallen) reduces the economic incentive to re-lock for yield-seeking lockers
- A locker who locked at $3+ CRV in 2021–2022 and is unlocking now at $0.30–0.50 has no unrealised gain to protect via re-lock

### Why the signal is Thursday-concentrated

All veCRV unlocks occur on Thursdays (weekly epoch). This means sell pressure, if it materialises, is clustered. Large Thursday unlocks are visible 1–4 years in advance. This is the entry trigger window.

---

## Entry Rules


### Universe

- **Instrument:** CRV-USDC perpetual on Hyperliquid (ticker: `CRV`)
- **Direction:** Short only
- **Minimum event size:** ≥500K CRV unlocking from a single address, OR ≥1M CRV unlocking from addresses sharing the same unlock epoch (Thursday) where no address is a known protocol (see filter below)

### Filters (apply ALL before entry)

1. **Address type filter:** Exclude known protocol addresses:
   - Convex Finance: `0x989AEb4d175e16225E39E87d0D97A3360524AD80` (cvxCRV staking) and related
   - Yearn Finance vaults
   - Frax Finance
   - Any address tagged "Protocol" or "DAO" on Etherscan/Arkham
   - Maintain a static exclusion list; update monthly
2. **Re-lock detection:** If the unlocking address calls `create_lock()` within the same block as `withdraw()`, exclude the event entirely. If re-lock occurs within 24h of unlock, mark as "re-locked" and do not enter.
3. **Minimum notional:** Event must be ≥$100K USD notional at current CRV price at time of detection
4. **Perp liquidity check:** CRV perp on Hyperliquid must have ≥$500K open interest at entry time

### Entry

- **Trigger time:** Thursday epoch unlock detected on-chain (via event listener or scheduled scan — see Data Sources)
- **Entry window:** Open short between **Tuesday 00:00 UTC** (48h before Thursday unlock) and **Wednesday 23:59 UTC** (day before unlock)
- **Entry price:** Market order at open of the 1H candle following trigger confirmation, or limit order at mid of current 1H candle ±0.3%
- **Entry condition:** Enter only if CRV perp funding rate is not strongly negative (funding < −0.05% per 8h would indicate crowded short — skip if so)

## Exit Rules

### Exit

- **Primary exit:** 72 hours after the Thursday unlock timestamp (i.e., Sunday 00:00 UTC if unlock was Thursday 00:00 UTC)
- **Profit target:** Close 50% of position at 4% gain; trail remainder with 2% stop from high-water mark
- **Stop loss:** 5% adverse move from entry price (hard stop, no exceptions)
- **Re-lock abort:** If on-chain monitoring detects the unlocking address calling `create_lock()` post-unlock, close position immediately at market

### Position management

- One position per qualifying event
- Do not pyramid — if a second unlock event triggers while a position is open, log it but do not add
- Maximum 2 concurrent CRV short positions (if two independent events qualify in the same week)

---

## Position Sizing

- **Base risk per trade:** 1% of total strategy capital
- **Calculation:** Position size = (Strategy capital × 0.01) / (entry price × 0.05)
  - Example: $100K capital, CRV at $0.40, stop at 5% = $0.02/CRV → position = $1,000 / $0.02 = 50,000 CRV notional = $20,000 notional
- **Leverage:** Implied leverage from above formula; do not exceed 3× notional leverage on Hyperliquid
- **Scaling modifier:**
  - Event size 500K–1M CRV: 0.75× base size
  - Event size 1M–5M CRV: 1.0× base size
  - Event size >5M CRV: 1.25× base size (cap at 1.5× regardless)
- **Maximum single-trade notional:** 5% of strategy capital

---

## Backtest Methodology

### Data sources

| Data | Source | Notes |
|------|---------|-------|
| veCRV lock/unlock events | Ethereum mainnet RPC or The Graph (`curve-dao` subgraph) | Pull all `Deposit` and `Withdraw` events from `VotingEscrow` contract |
| CRV price (hourly OHLCV) | CoinGecko API (free), Binance REST API (`CRVUSDT` 1h klines) | Use Binance as primary; CoinGecko as backup |
| CRV perp funding rate | Hyperliquid public API (`/info` endpoint, `fundingHistory`) | Available from Hyperliquid launch ~2023; use Binance perp before that |
| Address labels | Etherscan labels API, Arkham Intelligence export, Nansen (paid) | Build static exclusion list from these |
| veCRV total supply (for context) | Curve DAO subgraph or `totalSupply()` calls | Normalise event size as % of total veCRV |

### VotingEscrow contract address
`0x5f3b5DfEb7B28CDbD7FAba78963Ee202a494e2A2` (Ethereum mainnet)

### Event reconstruction

```
# Pseudocode for unlock event extraction
for each Withdraw(provider, value, ts) event in VotingEscrow:
    unlock_epoch = round_down_to_thursday(ts)
    record: (address, value_crv, unlock_epoch)

# Group by unlock_epoch, filter by size threshold
# Cross-reference with Deposit events to detect same-block re-locks
```

### Backtest period

- **Primary:** January 2021 – December 2024 (covers full CRV lock lifecycle, multiple market regimes)
- **Out-of-sample hold-out:** January 2025 onwards (do not touch until in-sample backtest is complete)

### Metrics to compute

For each qualifying event (post-filter):

1. **CRV return T+0 to T+72h** (where T = Thursday unlock timestamp)
2. **CRV return T−48h to T+0** (pre-unlock drift)
3. **CRV return T+0 to T+168h** (full week, to check if effect persists or reverses)
4. **Hit rate:** % of events where CRV is down >2% at T+72h
5. **Average return** across all qualifying events vs. random 72h windows (bootstrap test, n=1000)
6. **Sharpe ratio** of strategy (annualised, using 72h holding periods)
7. **Max drawdown** across all trades
8. **Funding cost drag:** Sum of funding paid on short positions (use historical funding data)
9. **Re-lock rate:** % of qualifying events where address re-locked within 7 days (this is the primary signal killer — must be <40% for strategy to be viable)

### Baseline comparison

- Random 72h short CRV positions (same number as qualifying events, randomly sampled from same date range)
- Buy-and-hold short CRV (to separate alpha from beta)
- Unlock events that were filtered out (protocol addresses) — expect these to show no signal, confirming filter validity

### Statistical test

- Two-sided t-test on mean 72h return: qualifying events vs. random baseline
- Require p < 0.05 to proceed
- Report effect size (Cohen's d) — need d > 0.3 to be practically meaningful

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Hit rate ≥ 55%** on qualifying events (CRV down >2% at T+72h)
2. **Mean 72h return ≤ −2.5%** (i.e., short makes money on average)
3. **p-value < 0.05** vs. random baseline
4. **Sharpe ratio ≥ 0.8** (annualised, net of estimated funding costs)
5. **Re-lock rate < 40%** among qualifying events (if higher, the filter is insufficient)
6. **Minimum 20 qualifying events** in backtest period (below this, results are not statistically meaningful)
7. **Signal holds in at least 2 of 3 sub-periods:** 2021–2022, 2022–2023, 2023–2024 (regime stability check)

---

## Kill Criteria

Abandon strategy (stop paper trading or live trading) if:

1. **Paper trading:** 5 consecutive losses, OR cumulative paper-trade return < −8% after ≥10 trades
2. **Live trading:** Drawdown exceeds 15% of strategy capital allocated to this strategy
3. **Structural change:** Curve governance votes to change lock mechanics (e.g., remove weekly epoch, allow early withdrawal) — monitor Curve governance forum and Snapshot
4. **Re-lock rate rises above 60%** in rolling 90-day window of live events (signal has degraded)
5. **Convex/vlCVX absorbs >80% of CRV supply** — at that point, free-float unlocks are too small to move price
6. **CRV perp delisted from Hyperliquid** or open interest falls below $200K (execution becomes impractical)
7. **Funding rate persistently negative** (< −0.03% per 8h for >30 days) — short is too expensive to carry

---

## Risks

### Primary risks (honest assessment)

**Re-lock risk (HIGH):** The single biggest threat to this strategy. Convex, Yearn, and protocol DAOs re-lock perpetually. Even EOA wallets may re-lock if bribe yields are attractive. The filter on address type is imperfect — a whale EOA may behave like a protocol. Estimated re-lock rate among non-filtered events: 30–50% (hypothesis — must be measured in backtest).

**Crowded short risk (MEDIUM):** veCRV unlock schedules are public. Other quant funds may be running the same trade. If the short is crowded, funding rates will be negative and the trade becomes expensive. The funding rate filter at entry partially mitigates this.

**CRV illiquidity risk (MEDIUM):** CRV perp on Hyperliquid has limited open interest. Large position sizes relative to OI will cause slippage and may move the market against entry. Position sizing caps (5% of strategy capital, 3× leverage max) partially mitigate this.

**Bribe/incentive regime change (MEDIUM):** If Curve bribe yields spike (e.g., due to a new protocol needing gauge votes), re-lock rates will rise sharply and the signal will disappear. This is not predictable from on-chain data alone — requires monitoring Votium/Paladin bribe markets.

**CRV price level dependency (LOW-MEDIUM):** At very low CRV prices, the notional threshold ($100K) filters out many events. The strategy may have fewer qualifying events in a prolonged bear market for CRV.

**False positive from OTC/internal transfers (LOW):** A large address may withdraw CRV to transfer to a new wallet and re-lock from there. This would look like a sell signal but is not. Mitigation: track destination address of withdrawn CRV — if it calls `create_lock()` within 24h, treat as re-lock.

**Smart contract risk (NEGLIGIBLE for strategy purposes):** VotingEscrow is battle-tested. Not a meaningful risk for the trading strategy itself.

---

## Data Sources

| Resource | URL / Endpoint |
|----------|---------------|
| VotingEscrow contract (Etherscan) | `https://etherscan.io/address/0x5f3b5DfEb7B28CDbD7FAba78963Ee202a494e2A2` |
| Curve DAO subgraph (The Graph) | `https://thegraph.com/hosted-service/subgraph/convex-community/curve-dao` |
| Ethereum RPC (free tier) | Alchemy (`https://eth-mainnet.g.alchemy.com/v2/{key}`) or Infura |
| CRV/USDT 1h OHLCV (Binance) | `https://api.binance.com/api/v3/klines?symbol=CRVUSDT&interval=1h` |
| CRV price history (CoinGecko) | `https://api.coingecko.com/api/v3/coins/curve-dao-token/market_chart?vs_currency=usd&days=max&interval=hourly` |
| Hyperliquid funding history | `https://api.hyperliquid.xyz/info` (POST `{"type": "fundingHistory", "coin": "CRV"}`) |
| Hyperliquid OI / market data | `https://api.hyperliquid.xyz/info` (POST `{"type": "metaAndAssetCtxs"}`) |
| Etherscan address labels | `https://etherscan.io/labelcloud` (manual export) |
| Arkham Intelligence | `https://platform.arkhamintelligence.com` (entity labels, requires account) |
| Votium bribe data (for re-lock incentive context) | `https://api.votium.app/api/v1/bribes` |
| Curve governance (Snapshot) | `https://snapshot.org/#/curve.eth` |

### Recommended implementation stack

- **Event indexing:** Python + `web3.py`, pull `Withdraw` and `Deposit` events from VotingEscrow using `get_logs()` with block range batching
- **Scheduling:** Cron job runs every Thursday 00:00 UTC, scans upcoming unlock epochs for the next 7 days
- **Alert:** Telegram bot or email when qualifying event detected (≥48h before unlock)
- **Execution:** Manual or semi-automated via Hyperliquid Python SDK (`https://github.com/hyperliquid-dex/hyperliquid-python-sdk`)

---

## Open Questions for Backtest Phase

1. What is the actual re-lock rate among non-protocol EOA addresses historically? (This determines whether the strategy is viable at all)
2. Is there a pre-unlock drift (T−48h to T+0) that could improve entry timing?
3. Does event size as % of total veCRV supply (rather than absolute CRV amount) better predict price impact?
4. Is the signal stronger when CRV is trading below the average lock price of the unlocking cohort? (Requires reconstructing cost basis per lock event)
5. Does the Votium bribe level in the week prior to unlock predict re-lock probability? (If bribes are high, re-lock is more likely — could be used as a filter)
