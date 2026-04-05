---
title: "DeFi Credit Default Swap — Short Governance Token on Large Liquidation Overhang"
status: HYPOTHESIS
mechanism: 4
implementation: 5
safety: 5
frequency: 3
composite: 300
categories:
  - liquidation
  - defi-protocol
  - governance
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a single identifiable wallet holds a >$10M borrow position on Aave V3 with a health factor between 1.0–1.15, the hosting protocol's governance token (AAVE) should reprice downward *before* liquidation occurs because:

1. **Bad debt risk is quantifiable and on-chain visible.** If the collateral asset drops faster than liquidators can execute (slippage, gas wars, illiquid collateral), the protocol's Safety Module absorbs the shortfall. This is not speculative — it happened in November 2022 (Eisenberg/CRV attack, ~$1.6M bad debt accrued to Aave).
2. **The liquidation bonus is a fixed liability.** At 8.5% bonus on a $20M position, the protocol is contractually obligated to pay ~$1.7M to liquidators. This is a known, on-chain-readable cost that governance token holders bear implicitly.
3. **Market participants are slow to price on-chain credit risk into governance tokens.** Most AAVE price discovery happens on CEXs where traders are not monitoring Aave health factor dashboards. This creates a window between "risk is visible on-chain" and "risk is priced in AAVE spot/perp."

**Causal chain:**
Large position HF < 1.15 → bad debt probability rises → Safety Module draw risk increases → AAVE token (which represents a claim on protocol revenue and Safety Module backstop obligation) should be worth less → AAVE reprices down → liquidation occurs or borrower rescues → risk resolves → AAVE recovers.

**Null hypothesis to disprove:** AAVE price does not systematically decline during large liquidation overhang windows, meaning the market already prices this risk continuously.

---

## Structural Mechanism — WHY This Must Happen

### The Fixed-Bonus CDS Analogue

Aave's liquidation mechanism is a smart-contract-enforced credit facility:

- **Liquidation threshold** is set per asset (e.g., 80% for ETH, 65% for altcoins) and is immutable per deployment.
- **Liquidation bonus** is fixed in the protocol parameters (e.g., 8.5% for WBTC, 10% for smaller assets). This is not negotiable — it is hardcoded.
- When HF = 1.0, any external actor can repay up to 50% of the debt and seize collateral at a guaranteed 8.5% discount. This is a **contractual put option** written by the protocol.

### The Safety Module Backstop Obligation

If liquidation fails to cover bad debt (collateral value < debt value at time of liquidation, e.g., due to oracle lag or illiquid collateral), Aave's Safety Module (SM) is the explicit backstop:

- SM holds staked AAVE tokens (~$300–500M historically).
- In a shortfall event, up to 30% of SM can be slashed and sold to cover bad debt.
- AAVE stakers are the implicit protection sellers in this CDS structure.
- Therefore, AAVE token price should reflect the probability-weighted expected SM slash.

### The Information Asymmetry Window

- Health factors are readable in real-time via `getUserAccountData()` on the Aave V3 Pool contract.
- CEX market makers for AAVE do not systematically monitor this.
- The window between "risk is visible on-chain" and "risk is priced in AAVE" is the tradeable edge.
- This window is estimated at minutes to hours for small events, potentially days for slow-moving large positions (e.g., a whale who is gradually approaching liquidation as collateral drifts down).

### Why This Is NOT Guaranteed (Honest Assessment)

- Borrowers can top up collateral at any time, instantly resolving the risk.
- Liquidation can be clean (no bad debt) even for large positions if collateral is liquid (ETH, BTC).
- AAVE governance token price is driven by many factors; a single liquidation event is a small signal in a noisy price series.
- The mechanism is real but the *magnitude* of AAVE repricing per dollar of liquidation risk is empirically unknown.

---

## Entry Rules


### Monitoring (Pre-Entry)

**Data feed:** Poll Aave V3 Pool contract `getUserAccountData(address)` every 60 seconds for all positions with `totalDebtBase > $5M USD`. Use The Graph subgraph for initial position discovery, then switch to direct RPC for real-time monitoring.

**Position discovery query (The Graph):**
```graphql
{
  users(where: {borrowedReservesCount_gt: 0}) {
    id
    borrowedReservesCount
    totalCollateralUSD
    totalDebtUSD
    healthFactor
  }
}
```
Filter: `totalDebtUSD > 5000000` AND `healthFactor < 1.2`.

### Entry Trigger

**All conditions must be met simultaneously:**

| Condition | Threshold |
|-----------|-----------|
| Single wallet debt | > $10M USD |
| Health factor | < 1.15 |
| Collateral asset | Not pure ETH/BTC (higher bad debt risk; altcoin collateral = worse liquidation dynamics) |
| AAVE 1h volume | > 20% of 30-day average (confirms market is active enough to trade) |
| No active SM shortfall event | Protocol not already in crisis mode |

**Entry action:** Short AAVE perpetual on Hyperliquid at market. Record entry price, position HF at entry, collateral asset, and debt size.

## Exit Rules

### Exit Triggers (first condition met wins)

| Exit Condition | Action |
|----------------|--------|
| On-chain liquidation event confirmed (LiquidationCall event emitted) | Close short 60 minutes after liquidation event, or immediately if AAVE has already moved >3% in your favor |
| Health factor recovers above 1.30 (borrower topped up) | Close immediately at market — thesis invalidated |
| Position held > 72 hours without liquidation or recovery | Close at market — time decay on thesis, risk of unrelated AAVE moves dominating |
| AAVE moves >5% against position (stop-loss) | Close immediately |

### Post-Liquidation Long (Separate Signal)

If liquidation occurs and collateral asset is an altcoin (not ETH/BTC):
- **Entry:** Long the collateral asset spot/perp immediately after liquidation event is confirmed on-chain.
- **Rationale:** Liquidator dumps collateral to repay debt, creating mechanical selling pressure. Overshoot likely if position is large relative to 1h volume.
- **Exit:** 2–4 hours post-liquidation or +3% gain, whichever comes first.
- **This is a separate, lower-conviction signal. Do not conflate with the governance token short.**

---

## Position Sizing

- **Primary trade (AAVE short):** 0.25% of NAV per event. This is a low-conviction, high-frequency-of-being-wrong trade. Do not size up.
- **Maximum concurrent positions:** 2 (two separate large liquidation events simultaneously). If a third triggers, skip it.
- **Post-liquidation collateral long:** 0.15% of NAV. Even lower conviction; pure mechanical overshoot play.
- **No leverage beyond 2x.** The edge is in the direction, not the leverage.
- **Rationale for small sizing:** This strategy will be wrong often (borrowers rescue positions, AAVE doesn't move). Small size allows many observations for statistical validity without material drawdown.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format | Cost |
|---------|--------|--------|------|
| Aave V3 all LiquidationCall events | Ethereum archive node or The Graph | Event logs with timestamp, collateral, debt, amounts | Free |
| Aave V3 health factor history for large positions | The Graph historical queries | Time-series per wallet | Free |
| AAVE/USD price history (1-minute OHLCV) | Binance API or Kaiko | CSV | Free (Binance) |
| Collateral asset price history | Binance/Coingecko | CSV | Free |
| Aave Safety Module shortfall events | Aave governance forum + on-chain | Manual list | Free, manual |

**The Graph endpoint (Aave V3 Ethereum):**
`https://api.thegraph.com/subgraphs/name/aave/protocol-v3`

**Aave V3 Pool contract (Ethereum):** `0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2`

**LiquidationCall event signature:** `LiquidationCall(address,address,address,uint256,uint256,address,bool)`

### Backtest Period

- **Primary:** January 2022 – December 2024 (covers bear market, CRV incident, multiple large liquidations)
- **Minimum events required for statistical validity:** 20 qualifying events (HF < 1.15, debt > $10M). If fewer than 20 exist, lower threshold to $5M debt.

### Event Identification Protocol

1. Pull all LiquidationCall events from Aave V3 where `debtToCover > $5M USD equivalent`.
2. For each event, walk back in time to find when HF first crossed below 1.15 for that wallet.
3. Record: (a) time of HF < 1.15, (b) time of liquidation, (c) collateral asset, (d) debt size, (e) whether bad debt occurred.
4. This gives the "entry time" and "exit time" for each historical event.

### Metrics to Compute

| Metric | Target | Notes |
|--------|--------|-------|
| AAVE return from entry to liquidation | Negative (confirms short direction) | Primary metric |
| AAVE return from entry to HF recovery (false signals) | Distribution of losses | Quantify cost of false signals |
| Win rate | >50% | Below this, strategy is a coin flip |
| Average win / average loss ratio | >1.5 | Must compensate for false signals |
| Sharpe ratio (annualized) | >0.8 | Low bar given low position size |
| Max drawdown on strategy | <2% NAV | Given 0.25% sizing, this implies <8 consecutive losses |
| Time in trade (average) | — | Characterize holding period |
| Bad debt events vs. clean liquidations | — | Segment: does bad debt produce larger AAVE moves? |

### Baseline Comparison

- **Random entry baseline:** Short AAVE at random times for the same average holding period. Compare win rate and return distribution.
- **Null hypothesis test:** If strategy win rate is not statistically different from random (p > 0.05, binomial test), score drops to 3/10 and strategy is killed.

### Segmentation Analysis

Run separately for:
1. Events where collateral is ETH/BTC (liquid) vs. altcoin (illiquid)
2. Events where debt > $20M vs. $10–20M
3. Events where HF < 1.10 vs. 1.10–1.15 at entry
4. Events that resulted in bad debt vs. clean liquidation

Hypothesis: the edge is concentrated in altcoin collateral + large debt + bad debt outcome. If the edge only exists in this narrow subset, the strategy is too rare to trade systematically.

---

## Go-Live Criteria

All of the following must be satisfied before paper trading:

1. **Minimum 15 qualifying historical events** identified and analyzed.
2. **Win rate > 55%** on AAVE short direction (entry to liquidation or stop-loss).
3. **Average win / average loss > 1.3** (net positive expectancy after transaction costs).
4. **Statistically significant vs. random baseline** (p < 0.10 acceptable given small sample; p < 0.05 preferred).
5. **At least 3 events with bad debt** — confirm the mechanism works in the extreme case where it theoretically must.
6. **Execution feasibility confirmed:** AAVE perp on Hyperliquid has sufficient liquidity to enter/exit 0.25% NAV position within 0.1% slippage.

If criteria 1–4 are met but sample size is small (15–20 events), paper trade at 0.1% NAV sizing until 5 live events are observed.

---

## Kill Criteria

Abandon strategy if any of the following occur:

| Condition | Action |
|-----------|--------|
| Backtest shows win rate < 50% across all segmentations | Kill — no edge |
| Backtest shows win rate > 55% but only in 1 of 4 segments with <5 events | Insufficient data — suspend pending more events |
| Live paper trading: 5 consecutive losses | Pause, re-examine entry criteria |
| Live paper trading: 10 events with win rate < 45% | Kill |
| Aave changes liquidation bonus structure or Safety Module mechanics | Re-evaluate structural mechanism from scratch |
| A competitor protocol (Morpho, Euler) captures most large borrowing, reducing Aave event frequency below 1/month | Kill — insufficient frequency |
| AAVE governance token is replaced or protocol migrates | Kill |

---

## Risks

### Primary Risks (Honest Assessment)

**1. Borrower rescue rate is high.**
Empirically, most large positions approaching liquidation are rescued by the borrower (collateral top-up, partial debt repayment). The strategy will generate many false signals. The 72-hour time stop is designed to limit exposure on these, but the false signal rate could be 60–70%, making the win rate requirement hard to achieve.

**2. AAVE price is dominated by macro/BTC correlation.**
On any given day, AAVE moves ±5% with BTC. A single liquidation overhang signal is a small effect size against this noise. The strategy may be structurally correct but practically undetectable in price data.

**3. Bad debt events are rare.**
The November 2022 CRV incident is the canonical example. In 3 years of Aave V3, there have been very few bad debt events of material size. The strategy's strongest theoretical case (bad debt → SM slash → AAVE dump) may have only 2–3 historical examples, which is insufficient for statistical validation.

**4. Liquidation is fast when it happens.**
Once HF = 1.0, liquidation bots execute within 1–3 blocks (12–36 seconds on Ethereum). The AAVE price impact, if any, may occur in this window — too fast to trade without HFT infrastructure. The strategy relies on the *pre-liquidation* window (HF 1.0–1.15) being tradeable, which requires the position to drift slowly toward liquidation.

**5. Governance token price is a poor proxy for protocol credit risk.**
AAVE token price reflects many things: protocol revenue, token emissions, market sentiment, BTC correlation. The credit risk signal from a single large liquidation may be too small to move the token price measurably. This is the core weakness of the CDS framing — the analogy is intellectually sound but the price sensitivity may be near zero.

**6. Front-running by sophisticated on-chain monitors.**
Other quant funds monitor Aave health factors. If the edge exists, it may already be arbitraged away by faster actors who enter the AAVE short before our monitoring system detects the trigger.

### Risk Mitigants

- Small position size (0.25% NAV) limits damage from false signals.
- 72-hour time stop prevents open-ended exposure.
- 5% stop-loss prevents catastrophic loss on any single event.
- Segmentation analysis in backtest will identify if the edge is real in any subset.

---

## Data Sources

| Resource | URL / Endpoint |
|----------|----------------|
| The Graph — Aave V3 Ethereum | `https://api.thegraph.com/subgraphs/name/aave/protocol-v3` |
| Aave V3 Pool contract (Ethereum) | `0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2` |
| Aave V3 Pool ABI | `https://docs.aave.com/developers/core-contracts/pool` |
| Etherscan LiquidationCall event logs | `https://etherscan.io/address/0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2#events` |
| DeBank large position monitor | `https://debank.com/` (manual; no public API for bulk health factor) |
| Chaos Labs Aave risk dashboard | `https://community.chaoslabs.xyz/aave/risk/overview` (real-time health factor distribution) |
| AAVE/USDT 1m OHLCV (Binance) | `GET https://api.binance.com/api/v3/klines?symbol=AAVEUSDT&interval=1m` |
| Aave governance forum (bad debt history) | `https://governance.aave.com/` |
| Aave November 2022 CRV incident post-mortem | `https://governance.aave.com/t/analysis-of-crv-short-attack/10680` |
| Hyperliquid AAVE perp | `https://app.hyperliquid.xyz/trade/AAVE` |
| Nansen wallet labeling (whale identification) | `https://app.nansen.ai/` (paid; use free tier for manual checks) |

### Recommended RPC Setup for Live Monitoring

```
Provider: Alchemy or Infura (free tier sufficient for polling)
Endpoint: https://eth-mainnet.g.alchemy.com/v2/{API_KEY}
Call: eth_call to getUserAccountData() every 60s for watchlist wallets
Watchlist construction: weekly refresh via The Graph query for all wallets with debt > $5M
```

---

## Implementation Notes

**Phase 1 (Weeks 1–2):** Build The Graph query to extract all historical LiquidationCall events > $5M on Aave V3. Map each event back to the wallet's health factor time series. This is the core dataset.

**Phase 2 (Weeks 3–4):** Merge with AAVE 1-minute price data. Compute returns from HF < 1.15 trigger to liquidation event for each historical case. Run statistical tests.

**Phase 3 (Week 5):** If backtest passes go-live criteria, build live monitoring script. Set up alerting (Telegram bot or similar) when a wallet crosses HF < 1.15 with debt > $10M.

**Phase 4 (Weeks 6–16):** Paper trade. Log every signal, entry, exit, and outcome. After 10 live events, evaluate against go-live criteria.

**Honest prior:** This strategy is more likely to be killed in backtest than to go live. The CDS framing is intellectually correct but the AAVE price sensitivity to individual liquidation events is probably too small to trade profitably at non-HFT speeds. The most likely outcome is that the backtest shows no statistically significant edge, and the strategy is abandoned in favor of the post-liquidation collateral overshoot signal (which has a cleaner mechanical story and faster resolution).
