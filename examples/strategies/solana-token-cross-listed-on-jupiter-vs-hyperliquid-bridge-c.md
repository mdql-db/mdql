---
title: "Solana Cross-Venue Basis During Bridge Congestion"
status: HYPOTHESIS
mechanism: 5
implementation: 3
safety: 3
frequency: 3
composite: 135
categories:
  - defi-protocol
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Solana network congestion physically prevents bridging capital between Solana-native spot markets (Jupiter/Orca) and Hyperliquid perpetual futures, the normal arbitrage mechanism that keeps prices aligned breaks down. During these windows, Solana spot prices for cross-listed tokens can diverge from Hyperliquid mark prices because the capital that would normally close the gap cannot move fast enough. The divergence is not a signal — it is a consequence of a mechanical blockage. When the blockage clears, prices must reconverge because the arbitrage becomes executable again.

**Causal chain:**

1. Solana network degrades (high failure rate, low TPS, validator congestion)
2. Wormhole/deBridge bridge queues back up — pending transactions accumulate, confirmation times extend from seconds to minutes or hours
3. Cross-venue arbitrageurs who hold capital on both sides can still trade, but new capital cannot be deployed to the underpriced side fast enough to close the gap
4. Solana-native liquidity pools (Jupiter aggregator routing through Orca, Raydium, Meteora) reprice based on local supply/demand, disconnected from CEX/Hyperliquid reference prices
5. Hyperliquid mark price remains anchored to its index (weighted median of Binance, Coinbase, and other CEX feeds) — it does not "see" the Solana dislocation
6. Basis opens: `(HL mark price − Jupiter spot price) / Jupiter spot price`
7. When congestion resolves, bridge throughput normalises, arbitrage capital flows, basis closes

**Why this is structural, not statistical:** The bridge is a physical rate limiter on capital flow. It is not that arbitrageurs choose not to act — they mechanically cannot move capital fast enough. The constraint is the Wormhole/deBridge smart contract queue and Solana block inclusion latency, not participant behaviour.

---

## Structural Mechanism

### The Dam

Wormhole and deBridge are the primary bridges connecting Solana liquidity to the rest of crypto. Both operate as message-passing systems with:

- **Wormhole:** Guardian network must reach consensus (13/19 guardians) before a VAA (Verified Action Approval) is issued. Under Solana congestion, the originating transaction may fail to land, requiring resubmission. Effective bridge time can extend from ~30 seconds to 10–30+ minutes.
- **deBridge:** Similar guardian/validator model with Solana-side confirmation requirements. Queue depth is observable on-chain.

### Why Most Arb Capital Cannot Close the Gap

- Arb bots that hold pre-positioned capital on both sides (Solana USDC + Hyperliquid USDC) can still trade but are capital-constrained — they cannot reload without bridging
- New entrants cannot deploy capital to the cheap side without bridging first
- The effective arb capacity during congestion = only pre-positioned float, which is finite
- As the basis widens, pre-positioned arb capital gets consumed; once exhausted, basis can persist until congestion clears

### Why Hyperliquid Mark Price Stays Anchored

Hyperliquid's mark price is computed as a weighted median of external CEX prices (Binance spot, Coinbase spot, and others depending on the asset). It does not reference Jupiter or any Solana DEX. Therefore, Hyperliquid mark price is insulated from Solana-side dislocations. This is the structural asymmetry that creates the trade.

### Reconvergence Mechanism

When Solana TPS normalises and bridge queues drain, the first arb bots to bridge will immediately close the basis. Reconvergence is typically sharp (minutes, not hours) once the dam breaks. This means the trade has a defined exit catalyst.

---

## Entry / Exit Rules

### Universe

Tokens that satisfy ALL of the following:
- Listed as a perpetual on Hyperliquid
- Solana-native (primary liquidity on Jupiter, not just bridged USDC pairs)
- Sufficient Jupiter liquidity: >$500k in relevant pools to absorb a meaningful position without excessive slippage
- Hyperliquid open interest >$1M (ensures mark price is actively maintained)

**Candidate tokens (as of writing):** JTO, JUP, PYTH, WIF, BONK, POPCAT, RAY — verify current HL listings at `https://app.hyperliquid.xyz/trade`

### Congestion Trigger (Entry Gate)

Both conditions must be true simultaneously:

| Condition | Threshold | Data Source |
|-----------|-----------|-------------|
| Solana TPS (non-vote) | < 800 sustained for ≥ 5 minutes | Solana RPC `getRecentPerformanceSamples` |
| Solana transaction failure rate | > 15% over trailing 5-minute window | Same RPC endpoint |

*Rationale for dual condition:* TPS alone can be low during quiet periods. High failure rate confirms active congestion, not just low activity.

**Optional secondary confirmation:** Wormhole bridge queue > 30 pending unconfirmed VAAs. Observable at `https://wormholescan.io` (API available).

### Basis Measurement

```
basis = (HL_mark_price − Jupiter_best_price) / Jupiter_best_price
```

- `HL_mark_price`: Hyperliquid mark price via WebSocket feed (`wss://api.hyperliquid.xyz/ws`, subscribe to `markPrice` for the asset)
- `Jupiter_best_price`: Jupiter Price API v2 (`https://price.jup.ag/v2/price?ids=<token_mint>`) — this returns the best executable price across all Solana DEX routes

**Minimum basis to enter:** `|basis| > 1.5%` (after estimated fees — see Position Sizing section)

### Entry

- If `basis > +1.5%` (HL mark > Jupiter spot): **Short HL perp, Long Jupiter spot**
- If `basis < −1.5%` (HL mark < Jupiter spot): **Long HL perp, Short Jupiter spot** *(note: shorting on Jupiter requires borrowing — treat as long-only on Solana side for initial backtest)*
- Enter both legs simultaneously (or as close as possible — accept up to 30-second leg lag)
- Use limit orders on Hyperliquid (post-only to capture maker rebate); use Jupiter's swap with 0.5% slippage tolerance

### Exit

**Primary exit (basis closes):**
- Exit when `|basis| < 0.3%`

**Secondary exit (congestion resolves):**
- Exit when Solana TPS > 1,500 AND failure rate < 5% for ≥ 3 consecutive minutes, regardless of basis level

**Hard stop (congestion persists, basis widens further):**
- Exit if `|basis| > 5%` — this signals something beyond normal congestion (potential exploit, oracle attack, or structural break) where the model assumptions no longer hold
- Exit if position has been open > 4 hours without either exit condition triggering

**Do not hold overnight** unless congestion is still active and basis is still > 0.5%.

---

## Position Sizing

### Fee Budget

| Fee | Estimate |
|-----|----------|
| Jupiter swap fee | ~0.1–0.3% (route-dependent) |
| Solana gas (congested) | $0.01–$0.50 per tx (negligible in % terms) |
| Hyperliquid taker fee | 0.035% |
| Hyperliquid maker rebate | −0.01% (use limit orders) |
| Round-trip total | ~0.3–0.7% |

**Minimum basis to enter = 1.5%** leaves ~0.8–1.2% net after fees, assuming clean execution.

### Position Size

- **Maximum per trade:** 2% of total strategy capital
- **Rationale:** Events are infrequent; sizing should be conservative until backtest validates frequency and average basis
- **Leg sizing:** Size both legs to be dollar-equivalent at entry. If Jupiter liquidity is the constraint (shallow pools), size to Jupiter leg first, then match HL perp size.
- **Leverage on HL perp:** 1x–2x only. This is a basis trade, not a directional bet. Higher leverage increases funding cost drag and liquidation risk if basis widens before closing.
- **Capital allocation:** Keep 50% of strategy capital pre-positioned on Solana (as USDC or stablecoins) and 50% on Hyperliquid. This avoids needing to bridge at entry time — bridging during congestion is the whole problem.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format | Notes |
|---------|--------|--------|-------|
| Solana TPS / failure rate (historical) | Solana Beach API `https://api.solanabeach.io/v1/network-stats` | JSON, 1-min intervals | May require paid tier for full history |
| Solana RPC performance samples | Archive node or Triton RPC `getRecentPerformanceSamples` | JSON | Free but requires own archive node for deep history |
| Jupiter price history | Jupiter Price API does not store history — use **Birdeye** `https://public-api.birdeye.so/defi/history_price` | OHLCV, 1-min | Free tier available; paid for full history |
| Hyperliquid mark price history | Hyperliquid API `https://api.hyperliquid.xyz/info` endpoint `candleSnapshot` | OHLCV, 1-min | Free, no auth required |
| Wormhole VAA queue history | Wormholescan API `https://api.wormholescan.io/api/v1/vaas` | JSON | Paginated; historical depth unclear — verify |

### Congestion Event Identification

1. Download Solana TPS data at 1-minute resolution for the maximum available history (target: 2022–present to capture multiple congestion episodes)
2. Define a "congestion event" as: TPS < 800 AND failure rate > 15% for ≥ 5 consecutive minutes
3. Merge adjacent events separated by < 10 minutes into a single event
4. Catalogue all events: start time, end time, duration, peak failure rate

**Known historical congestion episodes to verify coverage:**
- September 2021 (NFT mint congestion)
- January 2022 (Candy Machine congestion)
- April–May 2022 (multiple outages)
- February 2023 (NFT mint congestion)
- Various 2024 memecoin launch congestion events

### Basis Calculation

For each congestion event:
1. Pull Jupiter (Birdeye) 1-minute OHLCV for each candidate token during the event window ± 30 minutes
2. Pull Hyperliquid 1-minute mark price for the same window
3. Compute `basis_t = (HL_mark_t − Jupiter_close_t) / Jupiter_close_t` at each minute
4. Record: max basis, time-to-max, time-to-resolution (basis < 0.3%), direction of basis

### Metrics to Compute

| Metric | Target |
|--------|--------|
| Number of qualifying events (basis > 1.5%) | ≥ 10 across history |
| Average basis at entry | > 2.0% |
| Average time to resolution | < 2 hours |
| Win rate (basis closes before hard stop) | > 65% |
| Average net P&L per trade (after fees) | > 0.5% |
| Max adverse excursion (basis widening before closing) | < 3% |
| Sharpe (annualised, on deployed capital) | > 1.5 |

### Baseline Comparison

Compare against a naive strategy: enter any time basis > 1.5% regardless of congestion signal. This tests whether the congestion trigger adds value or whether the basis alone is sufficient signal.

### Simulation Notes

- Assume 0.5% slippage on Jupiter entry/exit (conservative for congested conditions)
- Assume Hyperliquid limit orders fill at mark price (reasonable for liquid perps)
- Do not assume simultaneous fills — model 30-second leg lag
- For tokens where Hyperliquid history predates Birdeye history, truncate to overlapping period

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **≥ 10 qualifying events** identified in historical data with basis > 1.5%
2. **Win rate ≥ 60%** (basis closes before hard stop triggers)
3. **Average net P&L ≥ 0.5%** per trade after modelled fees and slippage
4. **No single event** produces a loss > 3% (validates hard stop effectiveness)
5. **Congestion trigger adds value:** win rate with congestion filter > win rate without (baseline comparison)
6. **At least 2 different tokens** show the pattern — rules out token-specific idiosyncrasy
7. **Infrastructure test:** Solana TPS monitoring script running live for ≥ 2 weeks, confirming data feed reliability before capital is deployed

---

## Kill Criteria

Abandon the strategy (stop paper trading, do not go live) if:

- Fewer than 5 qualifying events found in full historical data — insufficient frequency to justify infrastructure build
- Win rate < 50% in backtest — basis is not reliably closing, suggesting the congestion-to-reconvergence mechanism is weaker than hypothesised
- Average time to resolution > 6 hours — holding period too long, funding costs and opportunity cost erode edge
- Wormhole or deBridge is replaced by a faster, more reliable bridge mechanism that eliminates the queue bottleneck (structural change kills the edge)
- Hyperliquid changes its mark price methodology to include Solana DEX prices (eliminates the asymmetry)
- After 6 months of paper trading: fewer than 3 live events observed, or live results diverge materially from backtest (win rate < 50% on live paper trades)

---

## Risks

### Execution Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Jupiter swap fails during congestion | High | Pre-position Solana capital before congestion; do not need to swap stables to token during event |
| Hyperliquid limit order not filled | Medium | Accept taker fill if basis is > 2% — fee difference is small relative to edge |
| Leg lag > 30 seconds creates directional exposure | Medium | Size positions conservatively (1x leverage); accept this as a cost of the strategy |
| Solana RPC endpoint unreliable during congestion | High | Run 3 independent RPC endpoints (Triton, Helius, own node); require 2/3 agreement on TPS reading |

### Model Risks

| Risk | Severity | Notes |
|------|----------|-------|
| Hyperliquid mark price also diverges from its CEX index | Medium | If Binance/Coinbase also dislocate (e.g., broad market crash coinciding with congestion), the HL mark may not reflect "true" price either. Monitor HL mark vs Binance spot as a sanity check. |
| Basis widens further before closing (adverse excursion) | Medium | Hard stop at 5% basis limits this. Pre-positioned capital means no forced liquidation. |
| Congestion caused by exploit or hack | High | If congestion is caused by a protocol exploit affecting the token being traded, the basis may never close. Kill switch: exit immediately if any security incident is announced for the token or bridge. |
| Funding rate drag on HL perp | Low | Holding period is short (target < 2 hours). At typical funding rates (0.01%/8h), cost is negligible. |
| Regulatory/compliance | Low | Both venues are accessible; no specific regulatory concern beyond standard crypto trading. |

### Structural Risks

- **Bridge improvement:** Wormhole v2 and newer bridge designs are faster and more resilient. As bridge infrastructure improves, the congestion window shrinks and the edge may disappear. Monitor bridge upgrade announcements.
- **Pre-positioned arb capital increases:** As more sophisticated arb desks pre-position capital on both sides, the basis may never reach 1.5% even during congestion. This would show up as declining average basis in live paper trading.
- **Solana reliability improvement:** Solana's validator client improvements (Firedancer, QUIC) are reducing congestion frequency and severity. The edge is likely diminishing over time — backtest should be weighted toward recent data to check for decay.

---

## Data Sources

| Resource | URL / Endpoint |
|----------|----------------|
| Hyperliquid mark price (live) | `wss://api.hyperliquid.xyz/ws` — subscribe to `{"method":"subscribe","subscription":{"type":"activeAssetCtx","coin":"TOKEN"}}` |
| Hyperliquid candle history | `POST https://api.hyperliquid.xyz/info` body: `{"type":"candleSnapshot","req":{"coin":"TOKEN","interval":"1m","startTime":UNIX_MS,"endTime":UNIX_MS}}` |
| Jupiter price (live) | `https://price.jup.ag/v2/price?ids=<MINT_ADDRESS>` |
| Birdeye price history | `https://public-api.birdeye.so/defi/history_price?address=<MINT>&type=1m` (API key required) |
| Solana RPC performance | `POST <RPC_URL>` body: `{"jsonrpc":"2.0","id":1,"method":"getRecentPerformanceSamples","params":[60]}` |
| Solana Beach network stats | `https://api.solanabeach.io/v1/network-stats` |
| Helius RPC (reliable during congestion) | `https://mainnet.helius-rpc.com/?api-key=<KEY>` |
| Triton RPC | `https://api.rpcpool.com/<KEY>` |
| Wormholescan VAA queue | `https://api.wormholescan.io/api/v1/vaas?pageSize=50&page=0` |
| Wormholescan historical | `https://api.wormholescan.io/api/v1/vaas?chainId=1&emitterAddress=<ADDR>&pageSize=100` |
| deBridge queue | `https://stats.debridge.finance/api/Transactions` |
| Solana status history | `https://status.solana.com/history` (incident log for known outages) |

---

## Implementation Notes

### Monitoring Script Requirements

Build a lightweight Python daemon that:
1. Polls `getRecentPerformanceSamples` every 60 seconds across 3 RPC endpoints
2. Computes rolling 5-minute average TPS and failure rate
3. Polls Jupiter Price API every 30 seconds for each candidate token
4. Polls Hyperliquid WebSocket for mark prices (persistent connection)
5. Computes basis in real time
6. Fires an alert (Telegram/Discord webhook) when: congestion trigger fires AND basis > 1.0% (pre-alert before entry threshold)
7. Logs all data to a local database for ongoing live validation

### Pre-Positioning Protocol

- Maintain 50% of strategy capital on Hyperliquid (USDC)
- Maintain 50% on Solana (split: 70% USDC, 30% pre-bought token positions in candidate assets)
- Rebalance pre-positioned token holdings weekly based on which tokens are most likely to show basis (highest HL OI, most Solana-native liquidity)
- Do not attempt to bridge during a congestion event — this defeats the purpose
