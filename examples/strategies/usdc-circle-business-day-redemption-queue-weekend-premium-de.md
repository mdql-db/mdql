---
title: "USDC Circle Business-Day Redemption Queue — Weekend Premium Decay"
status: HYPOTHESIS
mechanism: 5
implementation: 2
safety: 7
frequency: 7
composite: 490
categories:
  - stablecoin
  - calendar-seasonal
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Circle processes USDC-to-USD redemptions exclusively on US business days via the banking system. Redemption requests submitted between Friday ~5pm ET and Sunday night queue and are not processed until Monday morning. During this window, institutional holders with urgent fiat liquidity needs cannot redeem at par and may instead sell USDC on secondary markets at a fractional discount. This creates a small but mechanically predictable weekend discount in USDC/USDT pricing on DEX pools and CEX order books. Buying USDC at this discount and holding until Monday redemption processing restores par is a low-risk, low-return carry trade with a structural — not statistical — basis.

**The edge is not "USDC tends to dip on weekends." The edge is: Circle's operational calendar creates a guaranteed redemption delay, and some fraction of liquidity-constrained sellers will accept a discount rather than wait. The discount is the price of immediacy.**

---

## Structural Mechanism

### Why the dam exists
Circle's USDC redemption pipeline depends on US wire transfers and ACH settlement, both of which operate on Federal Reserve business days. This is not a policy choice Circle can easily override — it is a constraint imposed by the underlying banking rails. The constraint is:

- **Guaranteed:** Circle's own documentation confirms business-day-only processing
- **Recurring:** Every Friday evening through Sunday night, every week
- **Asymmetric:** The delay only affects sellers of USDC (those needing fiat out); buyers of USDC are unaffected

### Who creates the selling pressure
Not all USDC holders are affected. The sellers who create the discount are a specific subset:

1. **Institutional desks** that need to fund USD payroll, vendor payments, or margin calls over the weekend and cannot wait until Monday
2. **DeFi protocols** that have redeemed USDC from yield positions and need to move to fiat before weekend close
3. **Arbitrageurs** who are themselves capital-constrained and cannot hold the USDC position through the weekend

This is a **thin but real** population. The discount is small precisely because most USDC holders are not liquidity-constrained on a 48-hour horizon.

### Why the discount closes
By Monday morning ET, Circle resumes processing. Arbitrageurs with Circle accounts can submit redemptions at par, which mechanically caps the discount. Any USDC trading below $1.000 is a free redemption profit for anyone with a Circle institutional account and available USD credit line. The convergence is contractually guaranteed — Circle's terms commit to 1:1 redemption for verified institutional accounts.

### The arbitrage chain
```
Weekend discount appears
        ↓
Buy USDC at $0.9997–$0.9999 on Curve/Binance
        ↓
Hold through weekend (no active management needed)
        ↓
Monday: Circle redemptions open → arbitrageurs with Circle accounts
        redeem at $1.0000 → secondary market price converges to par
        ↓
Sell USDC at $1.0000 or close position
```

The strategy does not require a Circle institutional account. It free-rides on the convergence that Circle account holders enforce.

---

## Entry Rules

### Trigger conditions (ALL must be met)
1. **Time window:** Friday 17:00 ET through Sunday 23:59 ET only
2. **Price threshold:** USDC/USDT ≤ 0.9998 on at least ONE of the following venues:
   - Curve 3pool (on-chain, Arbitrum or Base deployment)
   - Binance USDC/USDT spot order book (mid-price)
   - Uniswap v3 USDC/USDT 0.01% pool on Arbitrum
3. **Depth check:** At least $500k of USDC available at the discount price (prevents entering into illiquid micro-moves)
4. **No active depeg event:** USDC must not be trading below $0.9990 on any major venue (if it is, this is a credit event, not a weekend discount — do not enter)
5. **No major macro event scheduled for Monday open** (Fed announcements, banking holidays) that could delay convergence

### Entry execution
- Buy USDC with USDT on the venue showing the largest discount
- Preferred venue: Curve 3pool on Arbitrum (lowest gas, deepest stablecoin liquidity)
- Do not use Ethereum mainnet — gas costs will consume the entire edge
- Execute as a single market order or tight limit order; do not ladder (spread is too thin to justify complexity)

---

## Exit Rules

### Primary exit
- **Monday 10:00 ET:** Close position regardless of price. By this time Circle has been processing redemptions for ~3 hours and secondary market should have converged.

### Secondary exits
| Condition | Action |
|---|---|
| USDC/USDT returns to ≥ 0.9999 before Monday 10am | Close immediately, capture spread |
| USDC/USDT falls below 0.9985 at any point | Emergency exit — this is no longer a weekend discount, this is a depeg event |
| Monday 10am arrives and spread has NOT closed | Hold until 14:00 ET maximum, then exit regardless |
| Any Circle operational announcement of extended processing delays | Exit immediately |

### Hard stop
**Exit at 0.9985.** Below this level the weekend-discount thesis is invalidated. A 15bp loss is the maximum acceptable drawdown on a trade targeting 1–3bp of profit. This is a 5:1 adverse risk ratio — position sizing must account for this asymmetry (see below).

---

## Position Sizing

### Constraints
- This is a **micro-edge** trade. Expected gross P&L per trade is 1–3bp on notional.
- Transaction costs (swap fees on Curve: 0.04%, gas on Arbitrum: negligible) consume approximately 4bp round-trip on Curve. **This means the edge must exceed 4bp to be profitable on Curve.**
- On Binance spot, maker/taker fees are 0.1%/0.1% = 10bp round-trip. **Binance is uneconomical for this trade at current fee tiers.** Only viable with VIP fee tiers (≤1bp per side).

### Revised entry threshold accounting for costs
| Venue | Round-trip cost | Minimum discount to enter |
|---|---|---|
| Curve 3pool (Arbitrum) | ~4bp | USDC/USDT ≤ 0.9996 |
| Uniswap v3 0.01% (Arbitrum) | ~1bp + gas | USDC/USDT ≤ 0.9998 |
| Binance (VIP tier) | ~2bp | USDC/USDT ≤ 0.9997 |

### Sizing formula
```
Position size = min(
    available_capital × 0.20,        # max 20% of stablecoin reserves per trade
    venue_depth_at_discount × 0.10   # max 10% of available liquidity
)
```

**Rationale for 20% cap:** The hard stop at 0.9985 represents a ~15bp loss. On 20% of capital, this is a 3bp drawdown on total portfolio — acceptable for a stablecoin arb strategy. Do not size larger; the asymmetric stop makes this dangerous at high concentration.

**Practical example:**
- Capital: $1,000,000 USDT
- Max position: $200,000 USDC
- Target profit at 3bp net: $60
- Loss at hard stop (15bp): $300
- This is a volume/frequency game, not a large-bet game

---

## Backtest Methodology

### Data required
1. **Curve 3pool prices:** On-chain via The Graph or Dune Analytics — query `TokenExchange` events on the 3pool contract, reconstruct implied USDC/USDT price from pool balances. Available from 2020 to present.
2. **Binance USDC/USDT:** Historical OHLCV from Binance public API (1-minute candles). Available from USDC listing (~2019).
3. **US Federal Holiday calendar:** To correctly identify business-day boundaries.
4. **Circle operational announcements:** Manual review of Circle blog/Twitter for any announced processing delays.

### Backtest procedure
```
For each Friday 17:00 ET → Monday 10:00 ET window (2020–present):
    1. Record minimum USDC/USDT price during the window
    2. Record price at Monday 10:00 ET
    3. Simulate entry at minimum price (if below threshold)
    4. Simulate exit at Monday 10:00 ET price
    5. Deduct round-trip transaction costs by venue
    6. Record: gross P&L, net P&L, entry price, exit price, hold duration
    
Aggregate metrics:
    - Win rate (% of trades that closed at profit after costs)
    - Average net P&L per trade (bp)
    - Maximum adverse excursion (worst intra-trade drawdown)
    - Frequency of hard-stop triggers
    - Sharpe ratio (annualised, using weekly frequency)
    - Total annual return on deployed capital
```

### Key questions the backtest must answer
1. How often does the discount exceed the cost threshold (≥4bp on Curve)?
2. Does the discount reliably close by Monday 10am, or does it sometimes persist?
3. Are there systematic periods when the discount is larger (month-end, quarter-end, tax season)?
4. How many times has the hard stop at 0.9985 been triggered? (Stress test: March 2023 SVB event)
5. Is the edge degrading over time as more arb capital enters?

### Known backtest limitations
- Curve pool balance data gives implied price but not executable price at a given size — slippage at $200k may differ from spot price
- Binance 1-minute OHLCV does not capture intra-minute order book depth
- The SVB depeg in March 2023 (USDC briefly hit $0.87) would have triggered the hard stop — this must be included, not excluded, from the backtest

---

## Go-Live Criteria

The strategy moves to paper trading when ALL of the following are confirmed by backtest:

| Criterion | Threshold |
|---|---|
| Historical win rate (net of costs) | ≥ 60% of qualifying weekends |
| Average net P&L per trade | ≥ 2bp after all costs |
| Hard stop triggered | ≤ 3 times in full backtest history (excluding known depeg events) |
| Edge present in most recent 52 weeks | Yes — confirms edge not fully arbitraged away |
| Minimum qualifying weekends per year | ≥ 15 (otherwise frequency too low to matter) |

Paper trading period: **8 weeks minimum**, tracking slippage vs. backtest assumptions.

Move to live trading when paper trading confirms:
- Actual fill prices within 1bp of backtest assumptions
- No operational issues with Arbitrum execution
- Gas costs confirmed negligible relative to edge

---

## Kill Criteria

**Immediately kill the strategy if:**
1. Circle announces 24/7 redemption processing (structural edge disappears)
2. USDC/USDT hard stop (0.9985) is triggered on a live trade
3. Three consecutive losing trades after costs (suggests edge has been arbitraged away or costs have increased)
4. Stablecoin regulatory event that creates uncertainty about USDC's 1:1 backing

**Review and potentially kill if:**
- Win rate drops below 50% over any rolling 12-week period
- Average net P&L drops below 1bp over any rolling 12-week period
- Curve 3pool liquidity drops significantly (increases slippage, destroys edge)

---

## Risks

### Primary risks

| Risk | Severity | Probability | Mitigation |
|---|---|---|---|
| USDC depeg event (credit/bank failure) | Critical | Low | Hard stop at 0.9985; 20% position cap |
| Edge too small to survive transaction costs | High | Medium | Strict cost-adjusted entry threshold; Arbitrum/Uniswap v3 preferred |
| Discount never appears (edge doesn't exist empirically) | Medium | Medium | Backtest will reveal this before capital is deployed |
| Circle changes to 24/7 processing | Medium | Low | Kill criterion; monitor Circle announcements |
| Regulatory action freezing USDC | Critical | Very Low | Hard stop; diversify stablecoin exposure |
| Smart contract risk on Arbitrum/Curve | Medium | Low | Use only audited, battle-tested contracts |

### The fundamental risk asymmetry problem
This strategy has an **unfavourable risk/reward ratio per trade**: targeting 2–4bp of profit with a 15bp hard stop. This is only acceptable because:
1. The probability of hitting the hard stop is very low (USDC has only had one significant depeg in its history)
2. The strategy is sized at 20% of capital, limiting portfolio impact
3. The edge is structural, not statistical — convergence is contractually enforced by Circle's redemption guarantee

**If the backtest shows the hard stop is triggered more than 3 times historically, the strategy fails the go-live criteria regardless of win rate.**

### The "everyone knows this" risk
This mechanism is not secret. If sufficient arb capital is already monitoring this window, the discount will never appear or will be too small to trade after costs. The backtest will reveal whether this is the case. The edge may exist only during periods of elevated market stress when arb capital is otherwise deployed.

---

## Data Sources

| Data | Source | Access method | Cost |
|---|---|---|---|
| Curve 3pool on-chain prices | Dune Analytics (query: 3pool TokenExchange events) | Public, free | Free |
| Curve 3pool pool balances (implied price) | The Graph, Curve subgraph | Public API | Free |
| Binance USDC/USDT OHLCV | Binance public REST API | `/api/v3/klines` endpoint | Free |
| Uniswap v3 USDC/USDT prices | Uniswap v3 subgraph on The Graph | Public API | Free |
| US Federal Reserve holiday calendar | Federal Reserve website | Manual/CSV | Free |
| Circle operational announcements | Circle blog, Circle Twitter/X | Manual review | Free |
| Gas price history (Arbitrum) | Arbiscan API | Public | Free |

---

## Open Questions for Researcher

1. **Does the discount actually appear?** Before building the full backtest infrastructure, do a quick manual check: pull Curve 3pool USDC balance ratios for the last 10 weekends. If the pool is never imbalanced toward USDC, the discount may not exist at measurable scale.

2. **Who are the actual sellers?** Is there any on-chain evidence of large USDC→USDT swaps specifically on Friday evenings? A Dune query on 3pool swap direction and size by day-of-week would validate or invalidate the mechanism before full backtest.

3. **Is Tether (USDT) the right reference asset?** USDT has its own credit risk. A USDC discount vs. USDT could reflect USDT premium rather than USDC discount. Consider also checking USDC vs. DAI and USDC vs. FRAX as cleaner references.

4. **Month-end and quarter-end effects:** The mechanism should be stronger on the last Friday of the month (institutional cash management pressure). Test this as a sub-filter.

5. **Fee tier access:** The strategy is marginal on Curve (4bp costs vs. 2–4bp edge). Investigate whether Curve's fee structure on Arbitrum has changed, or whether Uniswap v3's 1bp pool offers a better execution venue.

---

*Next step: Dune Analytics query to validate discount frequency before committing to full backtest build.*
