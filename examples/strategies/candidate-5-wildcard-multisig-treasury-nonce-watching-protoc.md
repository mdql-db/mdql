---
title: "Multisig Treasury Nonce Watching — Protocol Spend Pre-Signal"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 5
frequency: 3
composite: 375
categories:
  - token-supply
  - governance
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a protocol treasury Gnosis Safe has an execution-ready pending transaction (M-of-N signatures collected, not yet executed) directing governance tokens to a DEX router, CEX deposit address, or known OTC desk, a mechanically committed sell event is imminent. The token price has not yet reacted because the token has not moved on-chain, but the economic decision is final. Shorting the governance token between signature completion and on-chain execution captures the price impact of a known, imminent supply event.

**Causal chain:**

1. Protocol treasury decides to sell/deploy governance tokens (governance vote, committee decision, or operational need)
2. Signer 1 proposes transaction on Gnosis Safe → pending tx appears in Safe Transaction Service API with full calldata, destination, and value
3. Co-signers confirm → once M signatures collected, tx is execution-ready; this state is queryable via API field `confirmations` count vs. `confirmationsRequired`
4. Execution is delegated to any EOA (often a bot or the final signer) — typically occurs within 1–6 hours of final signature
5. Token arrives at DEX/CEX → sell pressure hits the market
6. Price declines proportional to sell size relative to liquidity depth

**The edge:** The signed-but-unexecuted window is an information gap. The commitment to sell is public (Safe API), but the market has not priced it in because most participants are not watching Safe pending queues. This is the same structural logic as token unlock shorts: supply event is mechanically committed, timing is bounded, price has not adjusted.

---

## Structural Mechanism (WHY This MUST Happen)

This is not a tendency — it is a mechanical sequence with one probabilistic escape valve (cancellation):

**What is guaranteed:**
- A Gnosis Safe pending transaction with M/N confirmations is cryptographically signed by M keyholders. The signatures are valid and irrevocable individually.
- The transaction CAN be executed by any address that calls `execTransaction()` on the Safe contract with the collected signatures. No further permission is required.
- The destination address, token contract, and transfer amount are encoded in the calldata and readable before execution via the Safe Transaction Service API response field `data` (decoded via the ERC-20 `transfer` or `approve` ABI).

**What is probabilistic (the escape valve):**
- Cancellation requires proposing a new transaction with the same nonce and getting M signatures on the cancellation. This is a non-trivial coordination cost. Empirically rare for execution-ready transactions (hypothesis — needs validation in backtest).
- The destination address classifier must correctly identify the recipient as a sell venue (DEX router, CEX hot wallet, OTC desk) rather than a protocol-internal address (liquidity pool owned by protocol, grant recipient, etc.).

**Why the market doesn't price this immediately:**
- Safe pending transactions are not indexed by any major price oracle, news aggregator, or on-chain analytics dashboard in real time
- The data requires polling a non-obvious API endpoint (`/api/v1/safes/{address}/multisig-transactions/?executed=false`)
- Most on-chain analytics tools (Nansen, Arkham) show transactions post-execution, not pre-execution pending state
- The signal is noisy (most treasury txs are routine ops) — filtering cost deters systematic monitoring

---

## Entry/Exit Rules

### Universe
Top 50 protocol treasuries by AUM, limited to those using Gnosis Safe (covers ~90% of major DeFi protocols). Initial list:

| Protocol | Token | Safe Address (Ethereum mainnet) |
|---|---|---|
| Uniswap | UNI | `0x1a9C8182C09F50C8318d769245beA52c32BE35BC` |
| Compound | COMP | `0xbbf3f1421D886E9b2c5D716B5192aC998af2012c` |
| Lido | LDO | `0x3e40D73EB977Dc6a537aF587D48316feE66E9C8c` |
| Aave | AAVE | `0xEC568fffba86c094cf06b22134B23074DFE2252c` |
| ENS | ENS | `0xFe89cc7aBB2C4183683ab71653C4cdc9B02D44b7` |
| Optimism | OP | Monitor via OP mainnet Safe API |
| Arbitrum | ARB | Monitor via Arbitrum One Safe API |

*Full list to be compiled during build phase. Expand to L2 Safes (Optimism, Arbitrum, Base) using chain-specific Safe Transaction Service endpoints.*

### Signal Detection (Poll every 15 minutes)

**Trigger conditions — ALL must be true:**

1. **Execution-ready:** `confirmations.count >= confirmationsRequired` AND `isExecuted = false` AND `isSuccessful = null`
2. **Token transfer:** Calldata decodes to ERC-20 `transfer(address recipient, uint256 amount)` or `approve()` followed by a known DEX interaction, OR native ETH transfer to a classified address
3. **Destination classified as sell venue:** Recipient address matches one of:
   - Known DEX router addresses (Uniswap V2/V3, Curve, 1inch, Paraswap, Cowswap settlement contract)
   - Known CEX deposit address (use Arkham Intelligence labels, Etherscan labels, or maintain internal DB seeded from public sources)
   - Known OTC desk addresses (Cumberland, Wintermute, GSR — compile from public Etherscan labels and on-chain flow analysis)
4. **Size threshold:** Token amount > 0.5% of the token's 30-day average daily volume (use CoinGecko API for volume data)
5. **Price filter:** Token has NOT already declined >5% in the prior 24 hours (avoids entering after the market has already front-run)
6. **Governance announcement check:** Search governance forum (Snapshot, Tally, Commonwealth) for posts mentioning the sale in the prior 7 days. If a public announcement exists, reduce position size by 50% (market may be partially priced in) but do not skip entirely (execution timing still uncertain)

### Entry
- **Instrument:** Short the governance token via Hyperliquid perpetual futures (where listed) or spot short via margin on a CEX
- **Entry timing:** Market order within 5 minutes of signal detection
- **Entry price:** Record mid-price at signal detection for P&L tracking

### Exit
- **Primary exit:** Close position 4 hours after on-chain execution confirmation (tx hash appears in `isExecuted = true` response)
- **Early exit — cancellation:** If the pending tx transitions to `isSuccessful = false` (cancelled via same-nonce override), exit immediately at market. This is a stop-loss event.
- **Early exit — price spike:** If token rallies >8% from entry price before execution, exit at market (position thesis invalidated by external catalyst)
- **Time stop:** If tx is not executed within 48 hours of signal detection, exit at market (execution delay suggests internal hesitation; cancellation risk elevated)
- **Profit target:** No fixed TP — let the 4-hour post-execution window run. The sell impact is the edge; don't cut it short.

---

## Position Sizing

**Base size:** 0.5% of portfolio per signal

**Scaling rules:**

| Condition | Size multiplier |
|---|---|
| Token amount 0.5–1% of 30d ADV | 0.5x (base = 0.25% portfolio) |
| Token amount 1–3% of 30d ADV | 1.0x (base = 0.5% portfolio) |
| Token amount >3% of 30d ADV | 1.5x (base = 0.75% portfolio) |
| Governance announcement exists | 0.5x multiplier applied on top |
| Token is illiquid (<$1M 30d ADV) | Skip — slippage will consume edge |
| Multiple signals firing simultaneously | Cap total exposure at 2% portfolio across all active positions |

**Leverage:** Maximum 3x. These are low-frequency, multi-hour holds — leverage amplifies slippage and funding costs disproportionately.

**Funding cost consideration:** At 3x leverage, funding rate of 0.01%/8h = ~0.03%/day. For a 4-hour hold, funding cost is ~0.015%. Acceptable if expected edge is >0.3%.

---

## Backtest Methodology

### Data Collection

**Step 1: Historical Safe transaction archive**
- Query Safe Transaction Service API for all multisig transactions (executed and cancelled) for each treasury address, going back as far as the API allows (typically 12–24 months)
- Endpoint: `GET https://safe-transaction-mainnet.safe.global/api/v1/safes/{address}/multisig-transactions/?limit=100&offset=0`
- For each transaction, record: `nonce`, `submissionDate`, `executionDate`, `isExecuted`, `isSuccessful`, `confirmations` (with timestamps per signer), `to`, `value`, `data`, `dataDecoded`
- Reconstruct "execution-ready timestamp" = timestamp of the M-th confirmation (final required signature)

**Step 2: Address classification**
- Build classifier using:
  - Etherscan labels API (requires API key, free tier available)
  - Arkham Intelligence public labels (export via UI or API)
  - Hardcoded DEX router addresses (Uniswap V2: `0x7a250d...`, V3: `0xE592427...`, Cowswap: `0x9008D19...`, 1inch V5: `0x1111111...`)
  - Known CEX hot wallets (Binance: `0x28C6c0...`, Coinbase: `0x71660c...` — use public Etherscan label lists)
- Classify each historical tx destination as: DEX / CEX / OTC / Protocol-internal / Unknown
- Validate classifier manually on 50 random transactions before using in backtest

**Step 3: Price data**
- Source: CoinGecko API (free, hourly OHLCV) or Kaiko (paid, tick-level)
- For each signal event, extract: price at execution-ready timestamp, price at on-chain execution, price 1h/2h/4h/8h/24h post-execution
- Calculate: return from entry (execution-ready) to each exit window

**Step 4: Volume data**
- Source: CoinGecko `/coins/{id}/market_chart` for 30-day rolling ADV
- Calculate token amount as % of 30d ADV at time of signal

### Backtest Metrics

**Primary metrics:**
- Hit rate: % of signals where price declined from entry to 4h post-execution exit
- Median return per trade (entry to 4h post-execution)
- Mean return per trade
- Return distribution (are there fat left tails from cancellations?)
- Sharpe ratio (annualized, assuming ~2 signals/month across 50 treasuries)

**Secondary metrics:**
- Cancellation rate: % of execution-ready txs that were subsequently cancelled
- Time-to-execution distribution: histogram of minutes from M-th signature to on-chain execution
- Signal frequency: how many qualifying signals per month historically
- False positive rate: % of classified "DEX/CEX" destinations that were actually protocol-internal (validate post-hoc by tracing token flow)

**Baseline comparison:**
- Random short entry on same tokens at same timestamps (no signal) → measures alpha vs. noise
- Buy-and-hold short of governance token basket → measures alpha vs. sector beta

**Minimum sample size:** 30 qualifying signals before drawing conclusions. If fewer than 30 exist in historical data, the strategy is too infrequent to validate statistically — note this as a risk.

### What to Look For
- Median return >0.3% (to cover funding + slippage)
- Hit rate >55%
- Cancellation rate <15% (if higher, the escape valve materially degrades the edge)
- Time-to-execution: median <6 hours (if median is >12 hours, the 48h time stop will trigger frequently)

---

## Go-Live Criteria (Paper Trading Threshold)

All of the following must be satisfied in backtest before paper trading:

1. **Sample size:** ≥30 qualifying signals in historical data
2. **Hit rate:** ≥55% of trades profitable at 4h post-execution exit
3. **Median return:** ≥0.3% per trade (net of estimated 0.1% slippage each way + funding)
4. **Cancellation rate:** ≤20% of execution-ready signals cancelled before execution
5. **No single trade >15% loss** (catastrophic misclassification check)
6. **Address classifier precision:** ≥85% on manually validated sample of 50 txs (precision = correctly classified sell venues / all classified as sell venues)

**Paper trading duration:** 60 days or 10 live signals, whichever comes later, before committing real capital.

---

## Kill Criteria

**Abandon the strategy if any of the following occur:**

### During backtest
- Fewer than 15 qualifying signals found in 24 months of historical data (too infrequent to be a meaningful strategy)
- Hit rate <45% in backtest (worse than coin flip after costs)
- Cancellation rate >30% (escape valve too common; signed check analogy breaks down)
- Address classifier precision <70% on validation set (too many false positives)

### During paper trading or live trading
- 10 consecutive losing trades
- Cumulative drawdown >5% of allocated capital
- Discovery that a major analytics provider (Nansen, Arkham, Dune) has launched a public dashboard tracking Safe pending transactions (edge commoditized)
- Regulatory change making short selling of governance tokens illegal in primary jurisdiction
- Safe protocol migrates to a private/encrypted pending transaction model (would eliminate the information gap)

### Ongoing monitoring
- If signal frequency drops below 1 qualifying signal per 2 months across the full universe, reassess universe composition (protocols may have migrated to different treasury management tools)

---

## Risks (Honest Assessment)

### Risk 1: Cancellation (HIGH IMPACT, LOW-MEDIUM PROBABILITY)
A transaction with M signatures can be cancelled by proposing a new transaction with the same nonce and collecting M signatures on the cancellation. Estimated cancellation rate for execution-ready txs: unknown — **this is the most important thing to measure in the backtest.** If >20%, the strategy's expected value degrades significantly. Mitigation: exit immediately on cancellation detection; 48h time stop limits exposure.

### Risk 2: Address misclassification (MEDIUM IMPACT, MEDIUM PROBABILITY)
A treasury may send tokens to a protocol-owned liquidity pool, a grant recipient, or a smart contract that is not a sell venue. If classified as a sell venue, this generates a false signal. Mitigation: build classifier conservatively (only classify addresses with high-confidence labels); accept lower signal frequency in exchange for higher precision.

### Risk 3: Pre-announced sales (MEDIUM IMPACT, HIGH PROBABILITY for large treasuries)
Major protocols (Uniswap, Aave) often announce treasury sales via governance forums before execution. The market may partially price in the sale before signatures are collected. Mitigation: governance forum check in signal filter; reduce size if announcement found. Note: even with pre-announcement, the exact timing of execution is uncertain, so some edge may remain.

### Risk 4: Private mempool / custom executor (LOW IMPACT, LOW PROBABILITY)
Some protocols use Flashbots or private RPC endpoints for execution, which would not change the Safe API pending state but would make the execution appear instantly on-chain. This doesn't eliminate the edge (the pending tx is still visible pre-execution) but may compress the entry window. Mitigation: monitor for unusually fast execution patterns in backtest.

### Risk 5: Protocol-internal destinations (MEDIUM IMPACT, MEDIUM PROBABILITY)
Treasury sends tokens to a protocol-owned address (e.g., liquidity mining contract, DAO-controlled pool). No market sell occurs. Mitigation: address classifier + post-execution token flow tracing to validate.

### Risk 6: Market impact on entry (LOW-MEDIUM IMPACT, LOW PROBABILITY for large-cap tokens)
For illiquid governance tokens, the short entry itself may move the market. Mitigation: minimum $1M 30d ADV filter; maximum position size cap.

### Risk 7: Low signal frequency (MEDIUM IMPACT, UNKNOWN PROBABILITY)
If qualifying signals occur only 1–2 times per month across the entire universe, the strategy contributes minimal portfolio alpha and may not justify the infrastructure cost. Mitigation: measure in backtest; expand universe to top 100 protocols if needed.

### Risk 8: Competitive front-running (LOW IMPACT, LOW PROBABILITY currently)
If other firms begin monitoring Safe pending transactions systematically, the edge compresses. Currently assessed as low probability given the infrastructure barrier. Monitor by tracking whether execution-to-price-impact lag shortens over time.

---

## Data Sources

| Data | Source | Endpoint / URL | Cost |
|---|---|---|---|
| Safe pending transactions | Safe Transaction Service | `https://safe-transaction-mainnet.safe.global/api/v1/safes/{address}/multisig-transactions/` | Free |
| Safe API (Optimism) | Safe Transaction Service | `https://safe-transaction-optimism.safe.global/api/v1/safes/{address}/multisig-transactions/` | Free |
| Safe API (Arbitrum) | Safe Transaction Service | `https://safe-transaction-arbitrum.safe.global/api/v1/safes/{address}/multisig-transactions/` | Free |
| Address labels | Etherscan Labels | `https://api.etherscan.io/api?module=account&action=txlist` + label DB | Free (API key required) |
| Address labels | Arkham Intelligence | `https://platform.arkhamintelligence.com` (UI export or API) | Free tier available |
| DEX router addresses | Uniswap docs / 1inch docs | Hardcoded from official protocol documentation | Free |
| Token price (hourly) | CoinGecko API | `https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=usd&days=90&interval=hourly` | Free (rate limited) |
| Token volume (30d ADV) | CoinGecko API | `https://api.coingecko.com/api/v3/coins/{id}` → `market_data.total_volume` | Free |
| On-chain tx confirmation | Etherscan API | `https://api.etherscan.io/api?module=transaction&action=gettxreceiptstatus&txhash={hash}` | Free |
| Governance announcements | Snapshot API | `https://hub.snapshot.org/graphql` | Free |
| Governance announcements | Tally API | `https://api.tally.xyz/query` | Free |
| Historical token prices (tick) | Kaiko | `https://www.kaiko.com` | Paid — use only if CoinGecko resolution insufficient |
| Treasury address list | DeepDAO / manual | `https://deepdao.io` + manual compilation | Free |

### Implementation Notes

**Polling architecture:** A simple Python script polling the Safe API every 15 minutes is sufficient. No websocket or streaming infrastructure required. Store results in a local SQLite or Postgres database. Flag state transitions (new pending tx, new confirmation, execution, cancellation).

**Calldata decoding:** Use the `dataDecoded` field in the Safe API response — the Safe service automatically decodes standard ERC-20 calls. For complex calldata (e.g., multisend), parse the `method` and `parameters` fields.

**Nonce tracking:** Track the `nonce` field to detect cancellations — if a transaction with the same nonce appears as executed but with a different `safeTxHash`, the original was cancelled.

**Build time estimate:** Address classifier + Safe poller + signal detector: 3–5 days of engineering. Historical backtest data collection: 1–2 days. Total to backtest-ready: ~1 week.
