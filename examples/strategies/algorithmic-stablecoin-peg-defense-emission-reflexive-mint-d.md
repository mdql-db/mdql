---
title: "Algo Stablecoin Peg-Defense Emission Short (ASPED)"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 4
frequency: 3
composite: 300
categories:
  - stablecoin
  - token-supply
  - defi-protocol
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When an algorithmic stablecoin's on-chain peg-defense mechanism triggers a governance token mint, the minted tokens represent **contractually guaranteed dilution** of the governance token supply. The mint volume is deterministic (formula is in the contract), the recipient address is known (typically a stability module or AMM pool), and the tokens will be sold into the market to purchase stablecoins. This creates a **predictable, size-quantifiable sell event** in the governance token that most market participants are not watching because their attention is on the stablecoin depeg itself.

**Causal chain:**

1. Algo stablecoin trades below peg (e.g., FRAX < $0.995)
2. Protocol's stability module detects depeg via on-chain oracle
3. Smart contract mints governance tokens (FXS, SNX, etc.) according to a deterministic formula
4. Minted tokens are sold (directly or via AMM) to buy stablecoins and restore peg
5. Governance token supply increases → sell pressure → price decline
6. Market attention is on stablecoin recovery; governance token dump is a **second-order effect** that is under-watched

The edge is **information asymmetry in attention**, not information asymmetry in data. The mint event is public, but the implication for governance token price is not the focus of most market participants during a depeg event.

---

## Structural Mechanism — WHY This Must Happen

The mechanism is contractually enforced, not behavioral:

**FRAX v1/v2 (FXS):** The Algorithmic Market Operations Controller (AMO) and the original v1 mechanism define that when FRAX trades below peg, the collateral ratio is adjusted and FXS is minted to recapitalize. The mint formula is: `FXS_minted = (stablecoin_deficit × (1 - CR)) / FXS_price`. This is deterministic given inputs. The minted FXS must be sold to acquire USDC to back FRAX — there is no other use for these tokens in the peg-defense context.

**Synthetix (SNX/sUSD):** When sUSD depegs, the protocol relies on stakers minting sUSD against SNX collateral. Peg defense creates SNX sell pressure as stakers who minted sUSD to sell it (to arb the peg) are now underwater on their collateral ratio and may be forced to sell SNX to deleverage. This is a **forced liquidation cascade**, not a voluntary mint, but the mechanism is equally contractual.

**Why the sell pressure MUST materialize:**
- Minted governance tokens have no utility in the peg-defense transaction — they exist solely to be sold for stablecoins
- The protocol's stability module or treasury executes this sale programmatically or via keeper bots
- The sale cannot be deferred without the peg-defense mechanism failing

**Why this is NOT guaranteed to produce a price drop:**
- Speculators may buy governance tokens anticipating peg recovery ("buy the dip on FXS")
- If the depeg is small, mint volume may be too small to move price
- Liquidity in governance token may absorb the sell without significant impact
- This is why the score is 5/10, not 8/10

---

## Target Universe

| Protocol | Governance Token | Stablecoin | Mechanism Type | Status |
|---|---|---|---|---|
| Frax Finance v1/v2 | FXS | FRAX | Algorithmic mint | Historical only (v3 is mostly collateralized) |
| Synthetix | SNX | sUSD | Collateral stress / forced deleverage | Active but mechanism is indirect |
| Liquity v1 | LQTY | LUSD | No mint mechanism — exclude | N/A |
| Crvusd (Curve) | CRV | crvUSD | LLAMMA rebalancing, not mint | Different mechanism — separate strategy |
| Angle Protocol | ANGLE | agEUR | Bonding curve mint | Low liquidity — exclude |

**Practical focus for backtest:** FXS/FRAX (2021–2023 data) and SNX/sUSD (2021–2024 data). These have sufficient on-chain history and CEX liquidity for governance token shorting.

**New entrant watch list:** Any new algo stablecoin launching with explicit mint-to-defend mechanics. Monitor governance forums and contract deployments on Ethereum, Arbitrum, and Base.

---

## Entry Rules

**Trigger conditions (ALL must be met):**

1. **Depeg detection:** Target stablecoin price < $0.993 for ≥ 2 consecutive 5-minute OHLC candles (using Chainlink oracle price, not CEX price, to avoid CEX-specific anomalies)
2. **Mint event confirmed:** On-chain governance token mint event emitted by the protocol's stability contract within the last 2 blocks (≤ 24 seconds on Ethereum). Mint amount must be ≥ 0.1% of circulating governance token supply to filter noise mints.
3. **Governance token price condition:** Governance token is trading at ≥ 95% of its 7-day VWAP (i.e., it has not already priced in the dilution). This filters entries where the market has already front-run the trade.
4. **Liquidity check:** Governance token 24h volume on target venue ≥ $2M (ensures position can be entered and exited without excessive slippage)

**Entry execution:**
- Short governance token perpetual on Hyperliquid (FXS-PERP, SNX-PERP) at market price within 1 block of mint event detection
- Do not chase if price has already moved >5% below 7-day VWAP before entry

---

## Exit Rules

**Take profit (primary):**
- Stablecoin returns to ≥ $0.998 for ≥ 3 consecutive 5-minute candles → close short at market. Rationale: peg defense has succeeded, mint pressure is over, recovery narrative may now pump governance token.

**Take profit (secondary):**
- Governance token drops ≥ 15% from entry price → close 75% of position, trail stop on remainder at entry price (lock in gains, allow for continued weakness)

**Stop loss:**
- Governance token rises ≥ 8% from entry price → close entire position. Rationale: speculative "recovery buy" is overwhelming the dilution sell pressure; the trade thesis is broken for this event.

**Time stop:**
- If neither TP nor SL is hit within 72 hours → close position at market. Depeg events that persist >72 hours enter a different regime (potential death spiral or protocol intervention) where the original thesis no longer applies cleanly.

**Forced exit:**
- Any governance token governance proposal to pause minting or change peg-defense mechanics → close immediately regardless of P&L. Protocol intervention invalidates the mechanical thesis.

---

## Position Sizing

**Base size:** 1% of portfolio per event

**Scaling rules:**
- Mint size ≥ 0.5% of circulating supply → 1.5% of portfolio
- Mint size ≥ 1.0% of circulating supply → 2.0% of portfolio (hard cap)
- Never exceed 2% of portfolio on a single ASPED position
- If multiple protocols trigger simultaneously (rare but possible in broad market stress), cap total ASPED exposure at 4% of portfolio

**Rationale for small sizing:** Low-frequency strategy with sparse historical data. Position sizing reflects uncertainty about the edge, not conviction. Size up only after live paper trading confirms the mechanism.

**Leverage:** 2x maximum on Hyperliquid perpetuals. The edge is directional but not high-conviction enough to justify higher leverage. The 8% stop loss at 2x leverage = 16% of position value at risk, which is acceptable given 1-2% portfolio allocation.

---

## Backtest Methodology

### Data Sources

| Data Type | Source | Endpoint/URL |
|---|---|---|
| FXS/FRAX price history | CoinGecko API | `https://api.coingecko.com/api/v3/coins/frax-share/market_chart?vs_currency=usd&days=1000` |
| SNX/sUSD price history | CoinGecko API | `https://api.coingecko.com/api/v3/coins/havven/market_chart` |
| FRAX on-chain price (oracle) | Chainlink FRAX/USD feed | Contract: `0xB9E1E3A9feFf48998E45Fa90847ed4D467E8BcfD` (Ethereum mainnet) |
| FXS mint events | Ethereum archive node | Filter: `Transfer` events from zero address on FXS token contract `0x3432B6A60D23Ca0dFCa7761B7ab56459D9C964D0` |
| SNX mint events | Ethereum archive node | Synthetix `Issued` event on `SynthetixDebtShare` contract |
| sUSD depeg history | Curve pool price | sUSD/3CRV pool: `0xA5407eAE9Ba41422288d5FAc3A1127E4B9D1bC4a` |
| Governance token circulating supply | Etherscan API | `https://api.etherscan.io/api?module=stats&action=tokensupply&contractaddress=<address>` |
| FXS perpetual price history | Hyperliquid API | `https://api.hyperliquid.xyz/info` (candles endpoint) |

### Backtest Period
- **FXS/FRAX:** January 2021 – December 2023 (covers multiple depeg events including the FRAX v1 stress periods and the broader 2022 crypto crash)
- **SNX/sUSD:** January 2021 – December 2024
- **Out-of-sample test:** Hold back Q4 2023 – Q4 2024 for forward validation

### Event Identification Protocol
1. Pull all `Transfer` events from zero address on FXS contract → these are mints
2. Filter mints where `amount > 0.1% of circulating supply at that block`
3. Cross-reference with FRAX oracle price at same block — confirm stablecoin was below $0.993 at time of mint
4. Record: mint timestamp, mint amount, FXS price at mint, FRAX price at mint, 7-day VWAP of FXS at mint
5. Apply entry filter: FXS price ≥ 95% of 7-day VWAP at mint time
6. Simulate short entry at next available 5-minute candle open after mint detection
7. Apply exit rules and record P&L for each event

### Metrics to Compute
- **Number of qualifying events** (expect <20 for FXS, <30 for SNX over the full period)
- **Win rate** (% of events where governance token declined ≥ 5% before stop loss hit)
- **Average P&L per event** (in % terms, before leverage)
- **Maximum adverse excursion (MAE)** per event — critical for stop loss calibration
- **Maximum favorable excursion (MFE)** per event — for TP calibration
- **Average hold time** (hours)
- **Correlation with BTC drawdown** — does this strategy only fire during broad market stress? If yes, it's not independent alpha.
- **Slippage sensitivity:** Re-run with 0.5%, 1%, and 2% slippage assumptions on entry/exit

### Baseline Comparison
- Compare against: "Short governance token whenever stablecoin depegs, regardless of mint event" — this tests whether the mint event detection adds value over simple depeg detection
- Compare against: "Buy governance token on depeg" (the contrarian recovery trade) — establishes opportunity cost

---

## Go-Live Criteria

The backtest must show ALL of the following before moving to paper trading:

1. **≥ 10 qualifying events** across the backtest period (fewer events = insufficient statistical power; if <10, the strategy cannot be validated and should be parked)
2. **Win rate ≥ 55%** on the primary exit condition (TP hit before SL)
3. **Positive expectancy** after 1% slippage assumption: `(win_rate × avg_win) - (loss_rate × avg_loss) > 0`
4. **No single event loss > 20% of position value** (validates stop loss placement)
5. **Strategy fires in at least 2 different market regimes** (bull and bear) — confirms it's not purely a bear-market phenomenon
6. **Sharpe ratio > 0.5** on the event-by-event P&L series (low bar given low frequency, but must be positive)

**Paper trading duration:** Minimum 3 months or 3 qualifying events (whichever comes later) before live capital deployment.

---

## Kill Criteria

Abandon the strategy if ANY of the following occur:

1. **Backtest produces <10 qualifying events** — insufficient data to validate; park the strategy and revisit if new algo stablecoins launch
2. **Backtest win rate <45%** — the mechanism is not producing the expected directional bias
3. **Protocol universe collapses to zero** — if all remaining algo stablecoins migrate to fully collateralized models (Frax v3 trajectory), there are no more triggers
4. **Governance token pumps on ≥ 60% of mint events** — indicates the "recovery narrative" trade is dominant and the dilution signal is structurally overwhelmed
5. **Paper trading: 3 consecutive stop-loss hits** — live market conditions differ materially from backtest; halt and diagnose
6. **Regulatory action** targeting algo stablecoins causes protocol shutdowns mid-trade — exit all positions, kill strategy

---

## Risks

**Risk 1: Universe extinction (HIGH probability, HIGH impact)**
Frax v3 is now predominantly collateralized. The pure algo mint mechanism that creates this edge is being engineered away across the industry post-UST. The strategy may have excellent historical logic but no future firing opportunities. This is the primary reason the score is 5/10 and not higher.

**Risk 2: Recovery narrative overwhelms dilution (MEDIUM probability, HIGH impact)**
During a depeg event, sophisticated traders often buy governance tokens anticipating that peg recovery = governance token recovery. If this "long the recovery" trade is larger than the dilution sell pressure, the governance token pumps into the short. The 8% stop loss is designed to limit damage here, but repeated stop-outs will erode the strategy.

**Risk 3: Mint events are front-run by MEV bots (HIGH probability, LOW-MEDIUM impact)**
By the time a non-HFT system detects the mint event and submits a short order, MEV searchers may have already pushed the governance token price down. This means entries are at worse prices than the backtest assumes. The 5% entry filter (governance token must be ≥ 95% of 7-day VWAP) partially addresses this but does not eliminate it. Slippage sensitivity analysis in the backtest is critical.

**Risk 4: Sparse data / overfitting risk (HIGH probability)**
With <20 qualifying events in the entire backtest period, any parameter tuning (stop loss %, TP %, VWAP lookback) risks overfitting to noise. The backtest parameters above should be set BEFORE running the backtest and not adjusted afterward. If the first backtest fails, do not re-optimize — kill or park the strategy.

**Risk 5: Hyperliquid perpetual availability (MEDIUM probability)**
FXS-PERP and SNX-PERP may have insufficient open interest or may be delisted on Hyperliquid during a depeg event (exchanges sometimes restrict trading on stressed assets). Verify perpetual availability and funding rate behavior during historical depeg events before assuming the short can be executed.

**Risk 6: Death spiral misidentification (LOW probability, HIGH impact)**
If a depeg event is the beginning of a UST-style death spiral (not a recoverable depeg), the governance token will eventually go to near-zero. This is a massive winner for the short, but the 72-hour time stop and 15% TP may cause premature exit. Consider adding a "death spiral" detection rule: if stablecoin drops below $0.95 and governance token drops >40%, extend hold and remove time stop. This is a separate regime that needs separate handling.

**Risk 7: Smart contract upgrade risk**
Protocols can upgrade their peg-defense contracts, changing the mint formula or disabling the mechanism entirely. Monitor governance proposals for all target protocols. A governance vote to change peg-defense mechanics should trigger immediate strategy review.

---

## Implementation Notes

**Monitoring infrastructure required:**
- Ethereum archive node (Alchemy or Infura) with WebSocket subscription to `Transfer` events on governance token contracts
- Chainlink oracle price feed subscription for stablecoin prices
- Alert system: Telegram or PagerDuty notification within 30 seconds of qualifying mint event
- Hyperliquid API integration for order execution

**Contracts to monitor:**
- FXS token: `0x3432B6A60D23Ca0dFCa7761B7ab56459D9C964D0` (Ethereum)
- FRAX token: `0x853d955aCEf822Db058eb8505911ED77F175b99e` (Ethereum)
- Synthetix: Monitor `SynthetixDebtShare` and `Issuer` contracts — see Synthetix GitHub for current addresses
- sUSD: `0x57Ab1ec28D129707052df4dF418D58a2D46d5f51` (Ethereum)

**Manual override requirement:** Given the low frequency and high stakes of each event, a human should review and approve each trade before execution during paper trading phase. Do not fully automate until at least 5 live paper trades have been reviewed.

## Data Sources

TBD
