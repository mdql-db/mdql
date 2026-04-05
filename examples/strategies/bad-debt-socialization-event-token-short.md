---
title: "Bad Debt Socialization Event — Token Short"
status: HYPOTHESIS
mechanism: 7
implementation: 5
safety: 6
frequency: 1
composite: 210
categories:
  - defi-protocol
  - token-supply
  - lending
created: "2026-04-03"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When an on-chain lending protocol accumulates bad debt that exceeds its reserve buffer, the smart contract is programmatically obligated to mint and auction governance tokens to recapitalize the system. This minting event is a contractually guaranteed supply increase — not a probabilistic tendency, but a deterministic outcome encoded in the protocol's governance contracts. The short thesis is that newly minted supply, sold into the open market via a time-bounded auction, creates directional downward price pressure on AAVE or MKR between auction announcement and auction clearance. The edge is structural: the contract mints, the auction sells, the market absorbs. The only variable is magnitude of price impact, not direction.

---

## Structural Mechanism

### Aave Safety Module (AAVE)

1. Aave maintains a Safety Module (SM) where staked AAVE acts as a backstop insurance pool.
2. When a liquidation gap creates bad debt exceeding the protocol's reserve treasury, the SM is triggered.
3. The SM smart contract (`StakedAave.sol` and the `AaveGovernance` shortfall module) initiates a **Shortfall Event**: up to 30% of staked AAVE is slashed and auctioned.
4. If slashing staked AAVE is insufficient, the protocol mints **new AAVE tokens** and auctions them for stablecoins to cover the deficit.
5. The auction runs for a fixed window (governance-parameterized, historically ~72 hours).
6. New tokens are sold at a discount to market price to attract auction participants.
7. Net effect: circulating supply increases, tokens are sold at below-market rates, creating guaranteed sell pressure.

### Maker Debt Auctions (MKR)

1. Maker maintains a Surplus Buffer (currently ~$50M DAI). When bad debt exceeds this buffer, a **Debt Auction** is triggered.
2. The `Flop.sol` contract mints new MKR and auctions it for DAI in a reverse auction (bidders compete to accept the least MKR for a fixed DAI amount).
3. Auction duration: 72 hours per lot, with multiple lots possible for large shortfalls.
4. MKR minting is deterministic once the `Vow` contract's `sump` threshold is breached — no governance vote required, the contract executes autonomously.
5. Net effect: MKR supply inflates, tokens are sold below market, guaranteed sell pressure until auction clears.

### Why This Is Structural, Not Statistical

The minting does not "tend to happen" — it is the only possible outcome once the bad debt threshold is crossed. The smart contract has no discretion. This is equivalent to a vesting cliff: the supply event is scheduled by code, not by human decision. The price impact is probabilistic, but the supply event is not.

---

## Historical Event Universe (Backtest Candidates)

| Date | Protocol | Event | Approx. Bad Debt | Token Impact |
|------|----------|-------|-----------------|--------------|
| Mar 12–13, 2020 | Maker | ETH crash, zero-bid auctions, ~$5.4M DAI deficit | $5.4M | MKR minted, -~40% in surrounding period |
| Nov 2022 | Aave v2 | Avi Eisenberg CRV attack, ~$1.6M bad debt | $1.6M | AAVE -~15% over 7 days post-announcement |
| Nov 2023 | Aave v2 | CRV/WBTC positions, ~$2.7M residual bad debt | $2.7M | Partial SM slash discussed, AAVE -8% |
| Various 2021–2022 | Maker | Multiple small Flop auctions post-liquidation gaps | <$1M each | MKR -3–8% per event |

*Note: This universe is small. Backtest will be illustrative, not statistically robust. Treat as case study analysis, not frequency-based backtest.*

---

## Entry Rules

### Trigger Conditions (ALL must be met)

1. **On-chain confirmation of bad debt event**: Monitor `Vow` contract on Maker for `heal()` calls that fail (indicating deficit), or Aave's `LendingPool` for `liquidationCall()` events where `debtToCover > collateralAmount`. Use Dune Analytics query or direct RPC polling — do not rely on Twitter/news.
2. **Auction initiation confirmed**: For Maker, `Flop` contract emits `kick()` event. For Aave, governance shortfall proposal reaches executable state or SM slash transaction is broadcast. This is the entry signal — not rumor, not social media, on-chain event only.
3. **Auction size ≥ 0.1% of circulating supply**: Below this threshold, dilution is noise. Calculate: `(bad_debt_USD / token_price_USD) / circulating_supply`. If ratio < 0.001, skip the trade.
4. **Token is liquid on Hyperliquid**: Confirm AAVE-PERP or MKR-PERP has >$500K open interest and bid-ask spread <0.3% at time of entry.
5. **No concurrent positive catalyst**: Check governance forums (Aave Discourse, Maker Forum) for any simultaneous buyback, token burn, or major protocol upgrade announcement that could offset dilution pressure.

### Entry Execution

- Enter short position within **2 hours** of on-chain auction `kick()` event.
- Use limit orders within 0.2% of mid-price to avoid slippage on entry.
- Do not chase if price has already moved >8% from pre-event price — the trade is partially priced in and risk/reward degrades.

---

## Exit Rules

### Primary Exit (Take Profit)

- **Auction clearance**: Monitor on-chain for final auction `deal()` call (Maker) or SM auction settlement transaction (Aave). Exit within 1 hour of confirmed auction close. Rationale: supply shock is absorbed, structural pressure ends.

### Secondary Exit (Time Stop)

- If auction has not cleared within **96 hours** of entry, exit regardless. Prolonged auctions indicate unusual market conditions that invalidate the base case.

### Stop Loss

- Exit if token price rises **>8% from entry price**. This indicates the market is pricing in a positive offset (e.g., governance response, buyback announcement) that overrides the dilution mechanism.
- Hard stop: **>12% adverse move** — exit immediately, no exceptions.

### Partial Profit Taking

- Close 50% of position when price is down **5% from entry**. This locks in partial profit and reduces risk if auction clears early or market reverses.
- Let remaining 50% ride to auction clearance.

---

## Position Sizing

### Base Formula

```
Position Size = (Account Risk per Trade) / (Stop Loss Distance)

Where:
- Account Risk per Trade = 1.5% of total capital
- Stop Loss Distance = 8% (primary stop)
- Therefore: Position Size = 1.5% / 8% = 18.75% of capital (notional)
```

### Dilution Scalar

Scale position size by dilution magnitude:

| Dilution (% of circulating supply) | Size Multiplier |
|------------------------------------|-----------------|
| 0.1% – 0.5% | 0.5× |
| 0.5% – 1.5% | 1.0× |
| 1.5% – 3.0% | 1.5× |
| >3.0% | 2.0× (cap at 2×) |

### Leverage

- Use **2–3× leverage maximum** on Hyperliquid perpetuals.
- Higher leverage is not warranted given low event frequency and the need to hold through 72-hour auction windows with potential volatility.
- Funding rate cost over 72 hours must be factored into expected return — if annualized funding exceeds 50%, reduce size by 25%.

### Maximum Position Cap

- Never exceed **5% of account capital** in notional exposure on a single bad debt event.
- If multiple protocols trigger simultaneously (rare but possible in systemic crashes), treat as correlated risk — total exposure across all positions capped at 8% of capital.

---

## Backtest Methodology

### Approach

Given the small event universe (estimated 8–15 qualifying events since 2020), this cannot be a statistically valid frequency backtest. Instead, conduct **structured case study analysis** on each confirmed event:

### Data Collection Per Event

1. **T=0**: Timestamp of on-chain auction `kick()` or equivalent trigger event (block number, not news timestamp).
2. **Price at T=0**: Token price at the block immediately preceding the trigger transaction.
3. **Price series**: 1-hour OHLCV for token from T-24h to T+168h (7 days post-event).
4. **Auction parameters**: Lot size, duration, number of lots, final clearing price.
5. **Dilution magnitude**: Tokens minted / circulating supply at T=0.
6. **Market context**: BTC price movement over same window (to isolate idiosyncratic vs. market-wide moves).

### Metrics to Calculate Per Event

- **Max drawdown from T=0 to auction close**: Primary return metric.
- **Return at auction close**: P&L if held to `deal()` transaction.
- **Return at T+24h, T+48h, T+72h**: Time-bucketed returns.
- **Idiosyncratic return**: Token return minus BTC return over same window (isolates protocol-specific effect).
- **Slippage estimate**: Based on order book depth at T=0 for the position size we would have taken.

### Confounds to Control For

- **Systemic crash events** (e.g., March 2020): Bad debt events during market-wide crashes conflate the dilution signal with macro panic. Flag these separately and analyze with/without.
- **Governance intervention**: Cases where the community voted to use treasury funds instead of minting — these are false positives for the minting trigger.
- **Pre-announcement leakage**: If on-chain data shows the bad debt accumulating over days before the auction kick, the market may have partially priced it in. Measure price movement from first detectable on-chain signal vs. from auction kick.

### Minimum Viable Backtest Output

- Mean idiosyncratic return from T=0 to auction close across all events.
- Win rate (% of events where short was profitable at auction close).
- Sharpe-equivalent: mean return / standard deviation of returns across events.
- Qualitative assessment of whether larger dilution events produced larger price impacts (dose-response relationship).

---

## Go-Live Criteria

The strategy moves from paper trading to live trading when ALL of the following are met:

1. **Backtest case studies complete**: All identified historical events analyzed with the methodology above. Document must exist before paper trading begins.
2. **Paper trade ≥ 3 live events**: Monitor and paper trade the next 3 qualifying events in real time. Record entry/exit prices, slippage, and P&L as if live.
3. **Paper trade win rate ≥ 60%**: At least 2 of 3 paper trades must be profitable at auction close.
4. **Execution infrastructure confirmed**: Hyperliquid API connection tested, on-chain monitoring alerts (Dune, custom RPC, or Tenderly) confirmed to fire within 15 minutes of trigger event.
5. **Funding rate policy confirmed**: Establish rule for maximum acceptable funding rate cost as % of expected return before entering any live trade.

---

## Kill Criteria

Abandon the strategy (stop trading, archive) if ANY of the following occur:

1. **3 consecutive losses** on live trades exceeding the 8% stop loss.
2. **Protocol governance change**: Aave or Maker modifies the Safety Module or Debt Auction mechanism to eliminate token minting (e.g., switches to pure treasury backstop). The structural mechanism no longer exists — the edge is gone.
3. **Market microstructure change**: AAVE-PERP or MKR-PERP open interest drops below $200K, making position sizing impractical without excessive market impact.
4. **Systematic front-running detected**: If price consistently moves >6% within the first 30 minutes of the on-chain trigger (before our 2-hour entry window), the edge is being captured by faster participants and our entry is degraded.
5. **Negative expected value confirmed**: After 10 live or paper trades, if mean idiosyncratic return at auction close is negative, the mechanism is not producing the expected price impact and the hypothesis is falsified.

---

## Risks

### Risk 1: Governance Override (HIGH PROBABILITY, MEDIUM IMPACT)
The community may vote to use protocol treasury funds or external capital to cover bad debt instead of triggering the minting mechanism. This happened partially in the Aave CRV event (Aave DAO used treasury USDC). **Mitigation**: Only enter after on-chain auction `kick()` is confirmed — governance override is no longer possible at that point. Do not enter on bad debt announcement alone.

### Risk 2: Market-Wide Crash Correlation (MEDIUM PROBABILITY, HIGH IMPACT)
Bad debt events often occur during market crashes. If BTC/ETH are simultaneously crashing, AAVE/MKR will fall for macro reasons, making it impossible to isolate the dilution signal. The trade may still be profitable but for the wrong reasons — and the stop loss may be hit on a recovery bounce before auction clears. **Mitigation**: Use idiosyncratic return (token return minus BTC return) as the primary P&L metric. Consider hedging with a small long BTC position to isolate the protocol-specific effect.

### Risk 3: Auction Fails to Clear / Extended Duration (LOW PROBABILITY, HIGH IMPACT)
If no auction participants appear (as happened in March 2020 Maker zero-bid auctions), the auction may be extended or restructured. This creates uncertainty about when the supply shock ends. **Mitigation**: The 96-hour time stop exits the position regardless of auction status.

### Risk 4: Reflexive Recovery (MEDIUM PROBABILITY, MEDIUM IMPACT)
After the initial drop, the market may interpret auction clearance as "crisis resolved" and bid the token aggressively. If the recovery is faster than expected, the 50% partial exit at -5% may not trigger before price reverses. **Mitigation**: The 50% partial exit rule and the 8% stop loss on the remaining position limit downside from a fast recovery.

### Risk 5: Small Event Universe / Overfitting (HIGH CERTAINTY)
With 8–15 historical events, any backtest result is statistically fragile. A 60% win rate on 10 events has enormous confidence intervals. **Mitigation**: Do not optimize parameters on historical data. Use fixed rules defined here. Treat the backtest as hypothesis validation, not parameter optimization.

### Risk 6: Funding Rate Erosion (LOW PROBABILITY, MEDIUM IMPACT)
During a bad debt event, perpetual funding rates on the shorted token may spike negative (shorts pay longs) if the market is already heavily short. A 72-hour hold at -0.1%/8h funding = -0.9% cost, which is meaningful on a trade targeting 5–10% return. **Mitigation**: Check funding rate at entry. If annualized funding cost exceeds 30% of expected return, reduce position size by 50%.

---

## Data Sources

| Data Type | Source | Access Method | Cost |
|-----------|--------|---------------|------|
| Maker `Vow`/`Flop` contract events | Ethereum mainnet RPC | Etherscan API, Alchemy, or self-hosted node | Free (Etherscan) / ~$50/mo (Alchemy) |
| Aave Safety Module events | Ethereum mainnet RPC | Same as above; Aave subgraph on The Graph | Free |
| Historical bad debt events | Dune Analytics | Pre-built dashboards exist for Aave/Maker bad debt | Free |
| Token price (OHLCV) | CoinGecko, Kaiko | REST API | Free (CoinGecko) / paid (Kaiko for tick data) |
| Hyperliquid order book depth | Hyperliquid API | REST + WebSocket | Free |
| Funding rates (historical) | Hyperliquid API, Coinalyze | REST API | Free |
| Governance forum monitoring | Aave Discourse, Maker Forum | RSS feed or manual monitoring | Free |
| On-chain alert infrastructure | Tenderly, OpenZeppelin Defender, or custom webhook | Webhook on contract event | Free tier available |

### Key Dune Queries to Build or Fork

- Maker Debt Auction history: query `flop` table for all `kick` events with lot size and bid history.
- Aave Safety Module shortfall events: query `StakedAave` contract for slash events and auction initiations.
- Token supply changes: query ERC-20 `Transfer` events from zero address (minting events) for AAVE and MKR.

---

## Open Questions for Researcher Review

1. **Aave v3 mechanism change**: Aave v3 introduced a different risk management framework. Confirm whether the Safety Module minting mechanism is identical in v3 or has been modified. If modified, update the structural mechanism section.
2. **MKR tokenomics change**: Maker's "Endgame" restructuring (2024) introduced SubDAOs and changed the surplus buffer mechanics. Verify whether `Flop.sol` is still the active debt auction contract or has been replaced.
3. **Minimum viable event size**: The 0.1% dilution threshold is a hypothesis. Backtest should test whether events below 0.5% dilution produce any measurable idiosyncratic price impact — if not, raise the threshold.
4. **Pre-announcement signal**: Bad debt accumulates on-chain before the auction kicks. Is there a detectable signal (e.g., health factor approaching liquidation threshold on large positions) that allows earlier entry with better risk/reward? This would require a separate signal specification.

---

*This document represents a hypothesis with a plausible structural mechanism. No backtest has been run. Do not allocate live capital until go-live criteria are met.*
