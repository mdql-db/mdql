---
title: "Airdrop Claim Deadline Forced Sell"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - airdrop
  - token-supply
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

In the 7 days before an airdrop claim deadline, procrastinating recipients face a binary choice: claim now or lose tokens permanently. This creates a mechanically compressed selling window. Recipients who farmed the airdrop have a cost basis near zero — any price is profit — so the rational action upon claiming is immediate sale. The on-chain claim event log is a leading indicator of incoming sell pressure, visible before the sell hits the order book. Shorting the token 5–7 days before the deadline and covering at T+1 captures the price impact of this forced, time-compressed supply event.

**Causal chain:**

1. Protocol sets hard claim deadline in smart contract (immutable or governance-changeable)
2. Procrastinating farmers monitor deadline; urgency spikes as deadline approaches
3. Claim transactions cluster in T-7 to T-0 window (verifiable on-chain in real time)
4. Claimed tokens hit recipient wallets → immediate or near-immediate sell (zero cost basis, farming economics)
5. Sell pressure concentrated in short window → price decline
6. After T+0, unclaimed supply reverts to treasury/burn → supply event ends → short cover

---

## Structural Mechanism

**Why this MUST happen (partially):**

- The deadline is encoded in the claim contract. The expiry is not a tendency — it is a hard state transition. After block N, `claim()` reverts. This is verifiable by reading the contract.
- Farmers' cost basis is provably near zero (gas + time). At any positive token price, claiming and selling is strictly dominant over not claiming. The only reason not to claim before deadline is ignorance or negligence — both of which resolve as deadline approaches and social/community reminders circulate.
- Claim volume MUST increase toward the deadline for any token where a meaningful unclaimed balance remains. This is a game-theoretic near-certainty: rational actors claim before expiry.

**Why sell pressure is NOT guaranteed (honest):**

- Claiming does not force selling. Some recipients may hold.
- If the token has appreciated significantly, recipients may be long-biased.
- Large holders (VCs, foundations) may have received tokens via vesting, not airdrop claim contracts — their behavior is separate.
- The market may have already priced in the expected sell pressure.

**The edge is in the TIMING COMPRESSION, not the direction alone.** Even if sell pressure is moderate, it is concentrated into a known, short window — which is exploitable if the market has not fully front-run it.

---

## Entry/Exit Rules

### Pre-conditions (all must be met before entering)

1. Claim deadline is confirmed in smart contract (`claimDeadline`, `endTime`, or equivalent state variable) — not just in docs
2. Unclaimed supply at T-7 is ≥ 5% of circulating supply (otherwise sell pressure is immaterial)
3. Token has sufficient liquidity: ≥ $500K average daily volume on CEX or DEX over prior 14 days
4. Short is available on Hyperliquid perps OR token is borrowable for spot short
5. No protocol governance vote to extend deadline is active or passed

### Entry

- **Primary entry:** T-7 days before claim deadline, market open of that UTC day
- **Secondary confirmation (optional scale-in):** If on-chain daily claim volume at T-5 or T-4 is ≥ 2× the 7-day average claim rate prior to T-7, add 50% to position
- **Do not enter** if token price has already declined >20% in the 14 days prior to T-7 (sell pressure may already be priced)

### Exit

- **Primary exit:** T+1 day after deadline (UTC close), regardless of P&L
- **Early exit:** If claim volume fails to materialize by T-3 (daily claims < 1.5× baseline for two consecutive days), reduce position by 50% at T-3 close
- **Momentum exit:** If price drops >15% from entry before T+1, take 50% profit and trail stop on remainder

### Stop Loss

- Hard stop: +8% adverse move from entry price at any point
- Rationale: If price is rising into the deadline, the market is pricing in a different narrative (e.g., deadline extension, token buyback) — the thesis is broken

---

## Position Sizing

- **Base size:** 1% of portfolio per trade
- **Maximum size:** 2% of portfolio (including any scale-in)
- **Liquidity constraint:** Position size must not exceed 2% of the token's 7-day average daily volume. Exceeding this risks moving the market on entry/exit.
- **Leverage:** 2–3× on perps maximum. This is a 7-day hold — funding costs matter. At 3× with 0.01% daily funding, cost is ~0.21% over the window. Acceptable if expected move is >3%.
- **Correlation cap:** Do not run this simultaneously with a token unlock short on the same token or a correlated token (double exposure to same sector sell pressure)

---

## Backtest Methodology

### Universe

Identify 15–25 historical airdrop claim deadlines from 2021–2024. Target list:

- Arbitrum (ARB) — April 2023 deadline
- Optimism (OP) — multiple rounds
- Aptos (APT)
- Sui (SUI)
- dYdX (DYDX)
- Blur (BLUR)
- Jito (JTO)
- Celestia (TIA)
- Starknet (STRK)
- Eigenlayer (EIGEN)
- zkSync (ZK)
- Scroll (SCR)
- LayerZero (ZRO)
- Wormhole (W)
- Pyth (PYTH)

For each: confirm deadline via contract, not just docs.

### Data Sources

See Data Sources section below.

### Metrics to Compute Per Trade

| Metric | Definition |
|---|---|
| Entry price | Token price at T-7 open |
| Exit price | Token price at T+1 close |
| Raw return | (Entry − Exit) / Entry (short = positive if price falls) |
| Max adverse excursion | Peak price between entry and exit vs. entry |
| Max favorable excursion | Trough price between entry and exit vs. entry |
| Claim volume ratio | Daily claims T-7 to T-0 vs. 30-day prior baseline |
| Unclaimed % at T-7 | Unclaimed tokens / total airdrop allocation |
| BTC-adjusted return | Raw return minus BTC return over same window (isolate alpha) |

### Baseline

Compare against:
1. **Random 7-day short:** Short the same token on a random date 30–90 days away from any deadline. Same duration. Measures whether the deadline window is special.
2. **BTC short same window:** Measures whether the return is just crypto beta.
3. **Token unlock short same token:** If data available, compare deadline short vs. unlock short for same token.

### Minimum Sample

- 15 completed deadline events with confirmed on-chain data
- Subset analysis: split by unclaimed % (>10% vs. <10% of supply) and by market cap tier (<$200M vs. >$200M)

### What to Look For

- Mean BTC-adjusted return > 3% over 7-day window
- Win rate > 55%
- Claim volume spike (≥2× baseline) at T-5 to T-3 is predictive of larger price decline (correlation analysis)
- Deadline extension events: flag separately, do not include in main results — analyze as a separate risk category

---

## Go-Live Criteria

All of the following must be true before moving to paper trading:

1. **Sample:** ≥ 15 historical events analyzed
2. **Mean BTC-adjusted return:** > 3% per trade
3. **Win rate:** > 55% (short profitable more often than not)
4. **Claim volume as leading indicator:** Pearson correlation between claim volume spike (T-5 to T-3) and subsequent price decline > 0.3
5. **Max adverse excursion:** < 10% on average (confirms stop loss at 8% is reasonable)
6. **Deadline extension rate:** < 30% of cases (if protocols routinely extend, the edge is unreliable)
7. **No single event drives >40% of total P&L** (concentration risk — result must be robust across cases)

---

## Kill Criteria

Abandon the strategy if any of the following occur:

### At backtest stage
- Mean BTC-adjusted return < 1.5% (not worth the operational overhead)
- Win rate < 50% (coin flip — no edge)
- Deadline extension rate > 40% (structural risk too high)
- Fewer than 10 events with sufficient on-chain data (sample too small to conclude)

### During paper trading (first 5 trades)
- 3 consecutive losses exceeding 5% each
- Claim volume spike fails to materialize in 4 of 5 cases (leading indicator is broken)
- Two deadline extensions in first 5 trades (protocol behavior has changed)

### During live trading
- Sharpe ratio (annualized, rolling 6-month) falls below 0.8
- Any single loss exceeds 2× the average historical loss (fat tail event — reassess sizing)
- A major protocol (e.g., Arbitrum, Optimism) publicly announces they will no longer set hard deadlines — signals industry norm change

---

## Risks

### Risk 1: Deadline Extension (HIGH probability, HIGH impact)
Protocols frequently extend deadlines under community pressure. This is the single largest risk. A deadline extension announcement typically causes a price SPIKE (short squeeze). Mitigation: monitor governance forums daily from T-14. Exit immediately if extension vote passes. Do not enter if an extension vote is already proposed.

### Risk 2: Pre-Priced Sell Pressure (MEDIUM probability, MEDIUM impact)
If the market is efficient about this pattern, the sell pressure is front-run before T-7. Evidence: if price has already declined >20% in the 14 days before T-7, skip the trade. Backtest should reveal whether the T-7 entry is optimal or whether T-14 is better.

### Risk 3: Illiquidity / Short Squeeze (LOW probability, HIGH impact)
Small-cap airdrop tokens may have thin order books. A large short position can itself move the market, and covering can be painful. Hard limit: position ≤ 2% of 7-day ADV.

### Risk 4: Claim-but-Hold Behavior (MEDIUM probability, MEDIUM impact)
Recipients claim but do not sell — perhaps they are long-biased or staking immediately. This would be visible in on-chain data: claims spike but DEX/CEX volume does not. Mitigation: use claim volume as a leading indicator but also monitor DEX sell volume from claim contract recipients.

### Risk 5: Perp Funding Costs (LOW probability, LOW impact)
If the token is already heavily shorted, funding rates may be significantly negative (longs pay shorts — favorable) or positive (shorts pay longs — unfavorable). Check funding rate at entry. If funding cost over 7 days exceeds 1% of notional, reduce leverage.

### Risk 6: Smart Contract Read Error (LOW probability, HIGH impact)
Misreading the deadline from the contract (e.g., confusing a vesting cliff with a claim deadline) leads to a trade with no structural basis. Mitigation: always verify deadline via two independent sources (contract + governance forum post).

---

## Data Sources

### On-Chain Claim Data
- **Etherscan / Arbiscan / Basescan event logs:** Filter `Claimed` events on the airdrop distributor contract. Export via API.
  - Etherscan API: `https://api.etherscan.io/api?module=logs&action=getLogs&address=<CONTRACT>&topic0=<CLAIMED_TOPIC>&apikey=<KEY>`
- **Dune Analytics:** Pre-built airdrop claim dashboards exist for ARB, OP, JTO, TIA, STRK. Search `airdrop claims` on `dune.com`. Custom queries can be written in SQL against decoded event tables.
  - Example: `https://dune.com/queries/` — search "airdrop claim deadline"
- **The Graph:** Subgraphs for major protocols expose claim events. Query via GraphQL.
  - `https://thegraph.com/explorer/` — search protocol name

### Claim Deadline Verification
- **Protocol governance forums:** Snapshot.org, Commonwealth.im, Tally.xyz — search "[TOKEN] airdrop claim deadline"
- **Smart contract direct read:** Use `cast call <CONTRACT> "claimDeadline()(uint256)" --rpc-url <RPC>` (Foundry) or read via Etherscan "Read Contract" tab
- **Airdrop aggregators:** `earni.fi`, `claimr.io` — sometimes list deadlines but treat as unverified; always confirm on-chain

### Token Price History
- **Coingecko API (free):** `https://api.coingecko.com/api/v3/coins/<id>/market_chart?vs_currency=usd&days=30&interval=daily`
- **Hyperliquid:** Historical OHLCV via `https://api.hyperliquid.xyz/info` (POST with `{"type": "candleSnapshot", ...}`)
- **Binance API:** `https://api.binance.com/api/v3/klines?symbol=<SYMBOL>USDT&interval=1d&limit=30`

### Unclaimed Supply Calculation
- Read `totalClaimed` and `totalAllocated` (or equivalent) from the distributor contract at T-7 block height
- Use Etherscan's "Read Contract at Block" feature or archive node RPC: `eth_call` with `block_number` parameter
- Alchemy and Infura provide archive node access on free tiers for historical state reads

### Governance / Extension Risk Monitoring
- **Snapshot.org API:** `https://hub.snapshot.org/graphql` — query proposals by space for extension-related votes
- **Commonwealth API:** `https://commonwealth.im/api/` — search threads by keyword "extend" + token name
- Set up a Google Alert for "[TOKEN NAME] airdrop deadline extension" as a manual backstop

---

*This specification is sufficient to build a backtest. The primary unknown is whether claim volume spike is a reliable leading indicator of price decline — that is the core empirical question the backtest must answer. Everything else (deadline timing, unclaimed supply) is mechanically verifiable.*
