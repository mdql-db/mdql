---
title: "Gauge Weight Vote Deadline — Bribe Market Emission Overhang Short"
status: HYPOTHESIS
mechanism: 5
implementation: 4
safety: 5
frequency: 7
composite: 700
categories:
  - token-supply
  - defi-protocol
  - funding-rates
created: "2025-01-30"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

veCRV/veBAL/veVELO holders receive bribe tokens from Votium, Hidden Hand, and Velodrome bribe markets as payment for directing gauge weight votes. These tokens arrive with **zero cost basis** — they are pure income to the recipient. Claim windows open at a predictable, protocol-enforced timestamp following each weekly vote close. The combination of (a) zero cost basis, (b) known claim date, and (c) altcoin liquidity constraints creates a **predictable sell window** in the 24–72h post-claim period.

**Causal chain:**

1. Gauge vote closes (Curve: Thursday 00:00 UTC; Velodrome/Aerodrome: Wednesday epoch flip)
2. Bribe distributor contracts (Votium `MultiMerkleStash`, Hidden Hand `RewardDistributor`) update Merkle roots with claimable amounts — on-chain, public, timestamped
3. veCRV/veBAL holders begin claiming within hours of root publication
4. Claimants hold zero-cost-basis altcoins with no sunk cost anchoring their sell decision
5. Rational recipients either (a) sell immediately for realised yield, or (b) hold speculatively — but the marginal seller has no loss aversion floor
6. Aggregate sell pressure from hundreds of claimants depresses bribe token price in the 24–72h window
7. Price recovers as sell overhang clears and next bribe cycle begins attracting buyers

**This is not "tends to happen" — it is structurally loaded:** the claim event is contract-enforced, the zero-cost-basis psychology is universal, and the timing is fixed to within hours each week.

---

## Structural Mechanism

### Why this MUST create sell pressure (not just tends to)

**Protocol-enforced timing:** Votium's `MultiMerkleStash` contract only allows claims after the Merkle root is set post-vote. Root publication is not discretionary — it follows vote finalisation automatically. Claim eligibility is binary: before root = 0 claimable; after root = full amount claimable. This creates a **step-function release of supply** at a known timestamp.

**Zero cost basis mechanics:** Bribe tokens are received as compensation for a vote cast with locked capital (veCRV). The voter's economic cost is the opportunity cost of the lock, not the bribe token itself. There is no purchase price to anchor a "sell below cost" psychological barrier. Every dollar of bribe token is pure profit at any positive price.

**Aggregation effect:** Votium distributes to thousands of veCRV holders simultaneously via Merkle proofs. Even if only 30–40% of recipients sell immediately, the simultaneous availability of claims creates a **supply shock** concentrated in a narrow time window. This is mechanically similar to a token unlock but with weekly frequency.

**Recycling dampens but does not eliminate:** Some sophisticated voters recycle bribe tokens into the next round (e.g., buying CVX to boost future bribes). This creates a floor but does not prevent the initial sell wave from the majority of passive recipients.

### Protocol-specific timing anchors

| Protocol | Vote close | Claim available | Bribe market |
|---|---|---|---|
| Curve | Thursday 00:00 UTC | Thursday ~02:00–06:00 UTC | Votium, StakeDAO |
| Balancer | Thursday 00:00 UTC | Thursday ~04:00–08:00 UTC | Hidden Hand |
| Velodrome (OP) | Wednesday 00:00 UTC | Wednesday ~01:00–04:00 UTC | Velodrome native |
| Aerodrome (Base) | Wednesday 00:00 UTC | Wednesday ~01:00–04:00 UTC | Aerodrome native |

---

## Entry Rules


### Universe selection (pre-trade filter)

Only trade bribe tokens that meet ALL of the following:
- Total bribe pool for that token in the current epoch ≥ $100K USD notional
- Token has either: (a) active Hyperliquid perpetual, OR (b) borrowable spot on a CEX with borrow rate < 5% APR
- Token market cap ≤ $500M (larger caps absorb sell pressure more easily)
- Bribe pool size ≥ 0.5% of token's 7-day average daily volume (ensures sell overhang is material relative to normal flow)

### Entry

1. Monitor Votium `MultiMerkleStash` contract (`0x378Ba9B73309bE80BF4C2c027aAD799766a7ED5A` on Ethereum mainnet) for `Claimed` events post-Merkle root update
2. Monitor Hidden Hand `RewardDistributor` for equivalent claim events
3. **Trigger:** First batch of claims where cumulative claimed notional for a single bribe token exceeds **$50K USD** within any 2-hour window post-root-publication
4. **Entry price:** Market order (or aggressive limit within 0.3% of mid) on Hyperliquid perp or CEX spot short
5. **Entry window:** Only enter within **4 hours** of trigger. If trigger is missed, skip the epoch — do not chase
6. **Confirmation filter (optional, reduces false positives):** Require that ≥10 unique wallet addresses have claimed before entry (prevents single large whale claim from triggering)

## Exit Rules

### Exit

| Condition | Action |
|---|---|
| 48h elapsed from entry | Close 100% of position at market |
| Bribe token price drops 6% from entry | Close 50%, trail stop on remainder at entry price |
| Bribe token price drops 10% from entry | Close 100% — take profit |
| Adverse move: price rises 8% from entry | Stop loss — close 100% |
| On-chain claims stop (< $5K/hour for 4 consecutive hours) | Close 50% — sell overhang may be exhausted |

**Do not hold through the next epoch's vote close** — bribe buyers re-enter ahead of the next round, creating a structural bid.

---

## Position Sizing

**Base size formula:**

```
Position Size ($) = min(
    Bribe Pool Size ($) × 0.15,
    Account Risk Budget per trade ($),
    Token 24h ADV × 0.05
)
```

- `Bribe Pool Size × 0.15`: Assume 15% of bribe pool hits market as immediate sells; size to capture that flow without moving the market yourself
- `Account Risk Budget`: Never exceed 2% of total account NAV at risk (using 8% stop = size such that 8% loss = 2% NAV loss → position = 25% of NAV max, but bribe pool cap will bind first in most cases)
- `ADV × 0.05`: Do not exceed 5% of 24h average daily volume — critical for illiquid altcoins

**Scaling:** If bribe pool > $500K, scale position linearly up to the ADV cap. Do not exceed ADV cap regardless of bribe pool size.

**Leverage:** Maximum 3x on Hyperliquid perp. Prefer 1–2x given illiquidity of underlying.

---

## Backtest Methodology

### Data sources

| Data | Source | URL/Endpoint |
|---|---|---|
| Votium claim events | Ethereum RPC / Etherscan | `eth_getLogs` on `MultiMerkleStash` `0x378Ba9B73309bE80BF4C2c027aAD799766a7ED5A` |
| Hidden Hand distributions | Subgraph | `https://api.thegraph.com/subgraphs/name/pie-dao/hidden-hand` |
| Bribe pool sizes by epoch | Votium API | `https://votium.app/api/v2/vlcvx/bribes` |
| Gauge vote results | Curve API | `https://api.curve.fi/api/getAllGauges` |
| Token OHLCV | CoinGecko | `https://api.coingecko.com/api/v3/coins/{id}/market_chart` |
| Hyperliquid perp availability | Hyperliquid API | `https://api.hyperliquid.xyz/info` (assetContexts) |
| On-chain claim timestamps | Etherscan API | `https://api.etherscan.io/api?module=logs&action=getLogs` |

### Backtest period

- **Start:** January 2022 (Votium v2 launch with Merkle distribution)
- **End:** Present
- **Frequency:** Weekly (one observation per epoch per bribe token)
- **Universe:** All bribe tokens meeting the pre-trade filter criteria above, evaluated epoch by epoch

### Construction steps

1. Pull all Votium/Hidden Hand epochs; record bribe token, pool size, Merkle root publication timestamp
2. For each qualifying token/epoch, record: entry price (2h after first $50K claim batch), exit price (48h later), max adverse excursion, max favourable excursion
3. Apply stop loss and take profit rules to compute actual P&L per trade
4. Aggregate by: token, epoch, bribe pool size bucket ($100K–$250K, $250K–$500K, $500K+), token market cap bucket
5. Compute: win rate, average return per trade, Sharpe ratio, max drawdown, average holding period

### Key metrics to compute

- **Primary:** Mean return over 48h holding period (entry to exit, after stops)
- **Secondary:** Win rate (% of trades profitable before stop)
- **Tertiary:** Return stratified by bribe pool / token ADV ratio (hypothesis: higher ratio = stronger signal)
- **Baseline comparison:** Random 48h short entry on same tokens on non-claim days (controls for general altcoin downtrend bias)
- **Slippage model:** Assume 0.5% entry + 0.5% exit slippage for tokens with ADV < $5M; 0.2% for ADV > $5M

### What to look for

- Mean 48h return significantly negative (i.e., shorts are profitable) vs. baseline random short
- Effect size increases with bribe pool / ADV ratio
- Effect is concentrated in first 24h (not 24–48h), which would validate the mechanism
- Win rate > 55% after stops (below this, the stop-loss structure may be too tight)

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Mean 48h return ≤ -3%** (i.e., short makes ≥3% on average) after slippage model, across ≥30 qualifying trade instances
2. **Win rate ≥ 52%** after applying stop loss rules
3. **Sharpe ratio ≥ 0.8** on the trade series (annualised, using weekly trade frequency)
4. **Baseline test passes:** Mean return on random 48h shorts on same tokens on non-claim weeks is NOT significantly negative (i.e., the edge is specific to claim windows, not just "altcoins go down")
5. **Bribe pool / ADV ratio ≥ 0.5% is confirmed as a necessary filter** — trades below this threshold should show no edge
6. **At least 3 different bribe tokens** show the effect (not a single-token artifact)

---

## Kill Criteria

Abandon the strategy if any of the following occur:

- **Backtest:** Mean 48h return is not significantly different from baseline random short (p > 0.10)
- **Backtest:** Win rate < 50% after stops — stop loss structure is eating the edge
- **Paper trading:** After 10 paper trades, cumulative P&L is negative and mean return per trade is worse than backtest by > 3 percentage points (execution slippage is too large)
- **Structural change:** Votium or Hidden Hand migrates to streaming/continuous distribution (eliminates the batch claim timing edge)
- **Market structure change:** Bribe tokens gain Hyperliquid perps with deep liquidity — pre-positioned shorts will front-run the entry, eliminating the edge
- **Recycling dominates:** On-chain analysis shows > 60% of claimed tokens are immediately re-deposited into bribe contracts (sell overhang hypothesis is wrong)

---

## Risks

### Execution risks

**Illiquid perp/spot markets:** Most bribe tokens lack Hyperliquid perps. Spot shorts require borrow availability and carry borrow costs (can be 20–100% APR for small caps). This may make the trade uneconomical even if the price move is correct. **Mitigation:** Only trade tokens with confirmed borrow rate < 5% APR or active Hyperliquid perp.

**Spread costs:** Small-cap tokens have 0.5–2% bid-ask spreads. A 3% expected move is not profitable after 2% round-trip spread. **Mitigation:** ADV filter and slippage model in backtest must be honest.

**Entry timing:** Claim events happen at 02:00–06:00 UTC — low-liquidity hours. Market orders may move price significantly. **Mitigation:** Use aggressive limits, not market orders; accept partial fills.

### Signal risks

**Pre-positioning:** Bribe pool sizes are public 48–72h before vote close. Sophisticated actors may short ahead of claim window, meaning price has already moved by entry time. **Mitigation:** Check whether price already declined > 5% in the 48h pre-claim; if so, skip the trade (overhang may be priced).

**Recycling and holding:** Protocol-native voters (e.g., Convex, Yearn) may hold or recycle bribe tokens systematically, reducing sell pressure. **Mitigation:** Exclude epochs where top 3 claimants (by address) are known protocol treasuries (identifiable via Etherscan labels).

**Bribe token appreciation:** If the bribe token is in a bull market, zero-cost-basis holders may hold rather than sell. The mechanism is real but the magnitude is market-condition-dependent. **Mitigation:** Add a filter: only enter if bribe token is below its 30-day moving average (confirms it's not in a strong uptrend that would suppress selling).

### Structural risks

**Protocol migration:** Curve's bribe infrastructure has changed before (Votium v1 → v2). A migration to continuous streaming (like Fluid or Merkl) would eliminate the batch timing edge entirely. Monitor Curve governance for distribution mechanism changes.

**Regulatory:** No regulatory risk specific to this strategy beyond standard crypto trading.

**Correlation:** In broad crypto risk-off events, all altcoins sell off simultaneously. The bribe token short may be profitable but for the wrong reason — the backtest baseline test is critical to isolate the structural edge from general altcoin beta.

---

## Data Sources

| Source | What it provides | Access |
|---|---|---|
| `0x378Ba9B73309bE80BF4C2c027aAD799766a7ED5A` (Ethereum) | Votium MultiMerkleStash claim events | Free via Etherscan API or any Ethereum RPC |
| `https://votium.app/api/v2/vlcvx/bribes` | Bribe pool sizes per epoch per token | Free, no auth |
| Hidden Hand subgraph (The Graph) | Hidden Hand claim events and reward amounts | Free |
| `https://api.curve.fi/api/getAllGauges` | Gauge weights and vote results | Free |
| CoinGecko `/market_chart` | OHLCV for bribe tokens | Free tier (rate limited); Pro for bulk |
| Hyperliquid `/info` assetContexts | Available perps and open interest | Free |
| Etherscan API `getLogs` | Raw on-chain claim event timestamps | Free (rate limited) |
| Dune Analytics | Pre-built Votium/Hidden Hand dashboards for validation | Free tier available |

**Recommended Dune queries to adapt:**
- Search "Votium claims" on Dune — several community dashboards track weekly claim volumes by token
- Search "Hidden Hand rewards" — tracks BAL/AURA bribe distributions
