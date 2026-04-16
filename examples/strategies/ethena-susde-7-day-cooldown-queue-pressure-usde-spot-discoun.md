---
title: "Ethena sUSDe 7-Day Cooldown Queue Pressure — USDe Spot Discount Arb"
status: HYPOTHESIS
mechanism: 7
implementation: 3
safety: 6
frequency: 3
composite: 378
categories:
  - stablecoin
  - defi-protocol
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When sUSDe redemption demand spikes — driven by stress, negative funding, or market panic — holders who cannot tolerate a 7-day lockup sell sUSDe or USDe on secondary markets at a discount to par. This discount is mechanically bounded: any buyer who can tolerate the 7-day wait can redeem at par via the smart contract. The gap between secondary market price and par (1.00 USD) is therefore a time-bounded arbitrage, not a speculative bet. The strategy buys the discount, initiates the cooldown, and collects at par. The edge is the impatience premium paid by forced or panic sellers, captured by patient capital.

**Core claim:** The 7-day cooldown timelock converts impatience into a tradeable spread. The contract enforces convergence. Solvency is the only residual risk.

---

## Structural Mechanism

### 2.1 How sUSDe Redemption Works (Protocol Mechanics)

```
User holds sUSDe
       │
       ▼
calls cooldownAssets() or cooldownShares()
       │
       ▼
sUSDe is burned; USDe enters cooldown escrow (7-day timelock)
       │
       ▼  [T + 7 days]
calls unstake()
       │
       ▼
USDe released at NAV (≈ 1:1 with USD, accrued yield included)
```

- The 7-day cooldown is enforced by the `StakedUSDeV2` smart contract on Ethereum mainnet
- There is **no discretionary override** — the timelock is absolute
- USDe itself is redeemable 1:1 for USDC/USDT via Ethena's mint/redeem mechanism (subject to Ethena solvency and liquidity)
- sUSDe NAV accrues yield continuously; the redemption value at T+7 is par **plus** accrued yield

### 2.2 Why the Discount Exists

During stress events, holders face a binary choice:

| Option | Cost | Timeline |
|--------|------|----------|
| Sell sUSDe/USDe on Curve/secondary | Accept spot discount (e.g., 0.3–1.5%) | Immediate |
| Initiate cooldown + unstake | Zero discount, full NAV | 7 days |

Sellers who choose Option A are paying an **impatience premium**. This premium is the strategy's edge. The premium is bounded above by the discount at which rational arbitrageurs enter. It is bounded below by zero (no discount = no trade). The 7-day contract guarantee is the floor that makes this a convergence trade, not a directional bet.

### 2.3 Why This Is Structural, Not Pattern-Based

The mechanism is not "USDe tends to recover after discounts." It is:

1. Smart contract **guarantees** 1:1 USDe redemption from sUSDe at T+7
2. Ethena's mint/redeem mechanism **guarantees** 1:1 USDe↔USDC redemption (conditional on solvency)
3. Therefore, any secondary market discount > (7-day carry cost + gas + slippage) is a **riskless spread** conditional on solvency

The only non-mechanical risk is Ethena protocol insolvency — which is a binary tail event, not a gradual drift.

---

## Entry Rules

### 3.1 Primary Signal: Curve Pool Imbalance

Monitor the **USDe/USDC** and **USDe/USDT** Curve pools (and any sUSDe pools):

```
Pool imbalance ratio = USDe_balance / (USDe_balance + USDC_balance)
```

| Imbalance Ratio | Interpretation | Action |
|-----------------|----------------|--------|
| < 50% | USDe in demand, at or above peg | No trade |
| 50–60% | Mild sell pressure | Watch only |
| 60–70% | Moderate discount forming | Prepare entry |
| > 70% | Significant sell pressure | Active entry zone |

### 3.2 Secondary Signal: Spot Discount Magnitude

Calculate the **effective discount** from the best available execution:

```
Discount (%) = (1.00 - best_execution_price_USDe) × 100
```

**Minimum entry threshold:**

```
Discount_min = gas_cost_USD / position_size_USD
             + slippage_estimate (0.05–0.15%)
             + 7_day_opportunity_cost (risk-free rate × 7/365)
             + safety_buffer (0.10%)
```

At current rates (assuming 5% annualised risk-free, $50 gas round-trip, $50,000 position):

```
Opportunity cost = 5% × 7/365 = 0.096%
Gas cost = $100 / $50,000 = 0.20%
Slippage = 0.10%
Safety buffer = 0.10%
─────────────────────────────
Minimum viable discount ≈ 0.50%
```

**Do not enter below 0.50% discount on a $50k position. Scale threshold down as position size increases.**

### 3.3 Tertiary Signal: Funding Rate Context

Check Ethena's published funding rate (available via Ethena dashboard and on-chain):

- If 7-day average funding is **deeply negative** (< -20% annualised), NAV erosion risk increases
- In this regime, raise minimum discount threshold to 1.00%+ to compensate for potential NAV decay during cooldown
- If funding is positive or mildly negative, standard threshold applies

### 3.4 Entry Execution

1. **Source USDe** on Curve (preferred for depth) or 1inch/Paraswap aggregator
2. **Verify execution price** — confirm discount exceeds threshold post-slippage
3. **Do NOT buy sUSDe directly** unless sUSDe discount > USDe discount + conversion friction (sUSDe→USDe conversion is the cooldown itself, so buying sUSDe and initiating cooldown is equivalent)
4. If buying **sUSDe**: call `cooldownShares()` immediately upon receipt
5. If buying **USDe**: hold and redeem via Ethena's USDe→USDC redemption at T+0 (no cooldown needed for USDe itself — only sUSDe has the 7-day lock)

> **Important clarification:** USDe itself does NOT have a 7-day cooldown. The cooldown applies to sUSDe→USDe conversion. If buying USDe at a discount, redemption to USDC via Ethena is near-instant (subject to Ethena's redemption queue, typically same-day). The 7-day mechanic applies specifically to the sUSDe→USDe leg. Adjust execution path accordingly.

---

## Exit Rules

### 4.1 Primary Exit: Cooldown Completion

- For **sUSDe** purchases: call `unstake()` at T+7 days, receive USDe at NAV
- Immediately redeem USDe→USDC via Ethena redemption portal or swap on Curve if peg has restored
- **Target exit price:** 1.00 USDC per USDe (par)

### 4.2 Early Exit: Peg Restoration Before T+7

If USDe spot price recovers to > 0.998 before cooldown completes:

- For **USDe holdings** (no cooldown): sell immediately on Curve, capture spread early
- For **sUSDe in cooldown**: cannot exit early — cooldown is irrevocable once initiated
- This asymmetry means: **only initiate sUSDe cooldown if you are committed to the 7-day hold**

### 4.3 Stop-Loss / Risk Exit

There is **no mechanical stop-loss** for sUSDe in cooldown — the position is locked. This is a feature (forces discipline) and a risk (no escape hatch). Therefore:

- **Pre-entry risk sizing is the only stop-loss mechanism**
- If Ethena announces insolvency or reserve fund breach during cooldown: accept loss, no action available
- For USDe spot positions (not in cooldown): exit immediately if discount widens beyond 2% (suggests systemic risk, not impatience premium)

---

## Position Sizing

### 5.1 Base Sizing Framework

```
Max position per trade = min(
    0.5% of total portfolio,
    available Curve liquidity × 10%,   // avoid moving the market
    $100,000 USD equivalent             // absolute cap per event
)
```

### 5.2 Scaling by Discount

| Discount | Position Size (% of max) |
|----------|--------------------------|
| 0.50–0.75% | 25% |
| 0.75–1.00% | 50% |
| 1.00–1.50% | 75% |
| > 1.50% | 100% (maximum stress event) |

### 5.3 Concentration Risk

- **Never exceed 2% of total portfolio** in Ethena-related positions simultaneously
- Ethena solvency is a **single point of failure** — treat all USDe/sUSDe exposure as correlated
- Do not run this strategy concurrently with any other Ethena yield strategies (e.g., long sUSDe carry) — they share the same tail risk

---

## Backtest Methodology

### 6.1 Data Requirements

| Data Source | Metric | Frequency |
|-------------|--------|-----------|
| Curve Finance subgraph (The Graph) | USDe pool balances, swap prices | Per-block |
| Ethena dashboard / on-chain | sUSDe NAV, funding rate | Daily |
| Dune Analytics | sUSDe cooldown initiations, unstake events | Daily |
| CoinGecko / CoinMarketCap | USDe spot price history | Hourly |
| Etherscan | `StakedUSDeV2` contract events | Per-block |

### 6.2 Backtest Period

- **Start:** May 2023 (Ethena mainnet launch)
- **End:** Present
- **Key stress events to capture:**
  - August 2024 crypto market drawdown (negative funding episode)
  - Any periods where Curve USDe pool imbalance exceeded 60%
  - March 2024 (rapid TVL growth period — potential redemption pressure)

### 6.3 Backtest Procedure

```
For each hour t in backtest period:
    1. Calculate Curve pool imbalance ratio
    2. Calculate USDe spot discount vs 1.00
    3. If discount > threshold AND imbalance > 60%:
        a. Record simulated entry at spot price + 0.10% slippage
        b. Record gas cost ($50 one-way estimate)
        c. Calculate 7-day forward NAV from on-chain sUSDe data
        d. Record P&L = (exit_NAV - entry_price - gas - slippage)
    4. Track: number of events, avg discount, avg P&L, max drawdown
       (drawdown only meaningful for USDe spot positions, not locked sUSDe)
```

### 6.4 Key Metrics to Measure

- **Event frequency:** How often does discount exceed 0.50%? (Expected: rare, < 20 events/year)
- **Average discount at entry:** Expected 0.3–1.0% based on anecdotal data
- **Hit rate:** % of trades where convergence occurred within 7 days (expected: >95%)
- **Average annualised return per event:** Discount / (7/365) — should be high on annualised basis
- **Maximum discount observed:** Establishes tail sizing parameters
- **False positives:** Events where discount widened further after entry (solvency scare)

### 6.5 Hypothesis Validation Criteria

The backtest **supports** the hypothesis if:
- ≥ 10 qualifying events identified in history
- Hit rate ≥ 90% (convergence within 7 days)
- Average net P&L per trade > 0.30% after costs
- No events where loss exceeded 2% (would indicate solvency event, not impatience premium)

---

## Go-Live Criteria

Before deploying real capital, ALL of the following must be satisfied:

- [ ] Backtest shows ≥ 10 historical events meeting entry criteria
- [ ] Hit rate ≥ 90% in backtest
- [ ] Manual paper trade of ≥ 3 live events with documented entry/exit
- [ ] Smart contract interaction tested on mainnet with $500 test position (verify cooldown, unstake flow)
- [ ] Gas cost model validated against actual mainnet costs
- [ ] Ethena reserve fund balance monitored and > $30M (current published figure)
- [ ] Curve pool monitoring script operational with alerting (Discord/Telegram webhook)
- [ ] Legal/tax treatment of USDe redemption confirmed for jurisdiction

---

## Kill Criteria

**Immediately halt and do not initiate new positions if:**

1. Ethena reserve fund balance drops below $10M (published on Ethena dashboard)
2. USDe discount exceeds 3% AND is not recovering within 24 hours (systemic risk signal, not impatience premium)
3. Ethena team announces protocol changes to redemption mechanics
4. Smart contract audit reveals vulnerability in `StakedUSDeV2`
5. Regulatory action against Ethena in a major jurisdiction
6. Funding rate stays below -50% annualised for > 3 consecutive days (NAV erosion risk exceeds arb spread)

**Note:** Positions already in cooldown CANNOT be exited. Kill criteria prevent new entries only. This reinforces the importance of pre-entry sizing discipline.

---

## Risks

### 9.1 Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Ethena insolvency / reserve fund depletion | Low | Catastrophic (100% loss) | Hard position cap (0.5% portfolio), monitor reserve fund daily |
| Funding stays deeply negative > 7 days, NAV erodes | Low-Medium | Moderate (reduces spread) | Raise discount threshold in negative funding regime |
| Curve liquidity dries up mid-entry | Medium | Low (partial fill, higher slippage) | Cap entry at 10% of pool liquidity |
| Gas spike makes trade uneconomical | Medium | Low (miss trade) | Pre-calculate gas-adjusted threshold before entry |
| Ethena changes cooldown period (governance) | Very Low | Strategy invalidation | Monitor governance proposals |
| Regulatory freeze on Ethena redemptions | Very Low | High (locked capital) | Jurisdiction monitoring, position cap |
| Smart contract bug in cooldown mechanism | Very Low | High | Only interact with audited, production contract |
| USDe discount is permanent (Ethena wind-down) | Very Low | High | Kill criterion #2 triggers before full exposure |

### 9.2 The Solvency Risk in Detail

This is the strategy's **only non-mechanical risk** and deserves explicit treatment:

Ethena's NAV guarantee is backed by:
1. Delta-neutral hedge positions (long spot BTC/ETH, short perps)
2. Reserve fund (~$30–50M as of early 2024)

If funding rates go deeply negative for an extended period, the hedge positions lose money faster than the reserve fund can absorb. In this scenario:
- sUSDe NAV could be < 1.00 at T+7
- The "guaranteed" convergence fails
- This is not a 7-day risk — it is a protocol-level risk

**Sizing implication:** Never size this trade as if it is truly riskless. The 7-day convergence is *contractually guaranteed conditional on solvency*. The solvency condition is the residual risk that justifies the discount existing at all.

---

## Data Sources

### 10.1 On-Chain Data

```
Contract: StakedUSDeV2
Address: 0x9D39A5DE30e57443BfF2A8307A4256c8797A3497 (Ethereum mainnet)
Key functions to monitor:
  - cooldownAssets() / cooldownShares() — entry events
  - unstake() — exit events
  - totalAssets() — NAV tracking
```

### 10.2 Pool Monitoring

```
Curve USDe/USDC pool: monitor via Curve API or The Graph subgraph
Endpoint: https://api.curve.fi/api/getPools/ethereum/main
Alert trigger: USDe weight > 60% in pool
```

### 10.3 Ethena Dashboard

- Reserve fund balance: https://app.ethena.fi/dashboards/hedging
- Funding rate: published daily, also available via Ethena API
- sUSDe APY: proxy for funding environment

### 10.4 Alerting Setup

```
Recommended: Python script polling Curve API every 15 minutes
Alert via: Telegram bot or Discord webhook
Alert message: "USDe pool imbalance: {ratio}% | Spot discount: {discount}% | 
               Action required if discount > {threshold}%"
```

---

## Open Questions for Research

Before backtesting, the following must be answered:

1. **How many historical discount events > 0.50% have occurred?** If fewer than 5, the strategy is too rare to be operationally worthwhile.
2. **What is the actual Ethena redemption queue time for USDe→USDC?** If same-day, USDe spot arb is faster than assumed.
3. **Are there gas-efficient batching options** for the cooldown + unstake flow? (Reduces minimum viable position size)
4. **Does sUSDe discount typically exceed USDe discount?** If so, which leg is more efficient to trade?
5. **What is the correlation between USDe discount events and broader stablecoin stress?** (USDC depeg March 2023 as reference — would Ethena stress coincide with USDC stress, creating double-sided risk?)

---

## Summary Scorecard

| Dimension | Assessment |
|-----------|------------|
| **Structural mechanism** | Strong — smart contract enforces convergence |
| **Edge source** | Impatience premium from forced/panic sellers |
| **Convergence guarantee** | Conditional (solvency required) |
| **Frequency** | Low (rare stress events) — limits annual return |
| **Execution complexity** | Medium (on-chain interaction required) |
| **Data availability** | Good (on-chain + Curve API) |
| **Backtest feasibility** | High — all data publicly available |
| **Tail risk** | Binary solvency event — manageable with position sizing |
| **Overall score** | **7/10** |

**Bottom line:** This is a genuine structural arb with a mechanical floor. The edge is real. The primary constraint is frequency — discount events are rare, limiting annual return potential. The strategy is best treated as an **opportunistic allocation** (deploy when signal fires, not a continuous position) with strict portfolio concentration limits given the binary solvency tail. Proceed to backtest.
