---
title: "wNXM MCR Gate Reopening — Discount Compression Trade"
status: HYPOTHESIS
mechanism: 7
implementation: 3
safety: 5
frequency: 1
composite: 105
categories:
  - defi-protocol
  - basis-trade
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Nexus Mutual's capital pool falls below its Minimum Capital Requirement (MCR), the smart contract redemption gate closes — NXM members cannot exit via the bonding curve. wNXM, already trading at a structural discount due to KYC friction, widens its discount further because the *only* partial redemption pathway (KYC members unwrapping and redeeming) is now mechanically blocked. When the capital pool recovers above MCR, the redemption gate reopens on-chain, and the discount *must* compress because arbitrage becomes executable again: KYC members can now unwrap wNXM → redeem NXM → receive ETH from the bonding curve.

**Causal chain:**
1. Capital pool / MCR ratio drops below 1.0 → `getPoolValueInEth()` < `getMCR()` in Nexus contracts
2. `withdraw()` function reverts for all members → redemption gate closed
3. wNXM discount to book value widens (sellers have no exit, buyers demand larger discount)
4. Capital pool recovers (new cover purchases, investment returns, or ETH price appreciation) → ratio crosses back above 1.0
5. `withdraw()` becomes callable again → KYC members can now arbitrage: buy wNXM on Uniswap → unwrap → redeem NXM → receive ETH
6. Arbitrage pressure compresses discount toward the structural floor (~10-15% representing KYC friction alone)

The compression is not guaranteed to be fast, but the *direction* is mechanically forced once the gate reopens. The only question is speed.

---

## Structural Mechanism (WHY This MUST Happen)

The Nexus Mutual V1 bonding curve is governed by `Pool.sol`. The relevant constraint:

```
function withdraw(uint tokenAmount, bool fromNXMaster, address payable destination)
    requires: poolValueInEth >= mcr
```

This is a hard smart contract revert — not a governance decision, not a discretionary pause. When `poolValueInEth < mcr`, no member can redeem regardless of intent.

The wNXM discount exists in two layers:
- **Structural discount (~10-15%):** Permanent. Reflects KYC friction — non-members holding wNXM cannot redeem without completing Nexus KYC. This floor is unlikely to close.
- **Stress discount (additional 5-30%+):** Appears when MCR gate closes. Reflects the *temporary* elimination of even the KYC-member redemption pathway.

The trade targets only the **stress discount** — the portion above the structural floor. When the gate reopens, the stress discount is mechanically arbitrageable by existing KYC members. They are the mechanism of compression.

**Why the arb isn't already closed:** KYC members are a small, finite set. During stress periods, they may be net sellers too (fear), which is why the discount widens. Once the gate reopens, rational KYC members with capital will execute the arb. The delay between gate reopening and discount compression is the tradeable window.

---

## Entry Rules


### Entry Conditions (ALL must be true simultaneously)

| Condition | Threshold | Data Source |
|-----------|-----------|-------------|
| wNXM discount to book value | > 20% | Uniswap wNXM/ETH price vs. `getTokenPrice()` on-chain |
| Capital pool / MCR ratio | Crossed above 1.00 within last 48 hours (recovering, not falling) | `Pool.sol` `getPoolValueInEth()` vs `MCR.sol` `getMCR()` |
| Ratio trend | Pool/MCR ratio increasing over trailing 7 days | Same on-chain data, 7-day slope positive |
| wNXM 24h volume | > $100k on Uniswap | Uniswap v2/v3 subgraph |

**Entry execution:** Market buy wNXM on Uniswap v2 (primary liquidity pool). Split into 3 tranches over 24-48 hours to reduce slippage impact given illiquidity. Do not use limit orders — the signal window may be short.

## Exit Rules

### Exit Conditions (FIRST triggered wins)

| Condition | Action |
|-----------|--------|
| wNXM discount compresses to < 12% | Full exit — structural floor reached, stress premium gone |
| Capital pool / MCR ratio drops back below 0.98 | Full exit immediately — gate may reclose |
| Position held > 60 days without discount compression | Time-stop exit — thesis not playing out |
| wNXM price drops > 30% from entry in ETH terms | Hard stop — something structurally wrong |

**Exit execution:** Sell wNXM on Uniswap. Given illiquidity, may need 48-72 hours to fully exit without excessive slippage. Begin exit at first trigger, do not wait for full compression.

### Do Not Enter If:
- Discount is wide but MCR ratio is *still falling* (gate still closed, or just reopened with ratio unstable)
- Nexus governance has a live proposal to change MCR formula or bonding curve parameters
- A large unresolved claim is pending that could drain the capital pool (check `Claims.sol` for pending claim amounts)

---

## Position Sizing

**Maximum position:** 1% of Zunid's total capital per trade. This is non-negotiable given wNXM's illiquidity.

**Rationale for 1% cap:**
- wNXM market cap ~$10-30M; daily volume often <$500k
- A position >$200-300k would move the market on entry and exit
- At 1% of capital, even a 50% loss on the position is a 0.5% portfolio drawdown — acceptable for a hypothesis-stage trade

**Sizing within the 1% cap:**
- Tranche 1 (entry day): 40% of position
- Tranche 2 (24h later, if ratio still above 1.0): 35% of position
- Tranche 3 (48h later, if ratio still above 1.0 and discount still >18%): 25% of position

**No leverage.** wNXM is spot only. Do not attempt to construct a synthetic position using NXM perps — no liquid venue exists.

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format |
|---------|--------|--------|
| wNXM/ETH price (daily OHLCV) | Uniswap v2 subgraph (The Graph) | GraphQL query on `0x0d438f3b5175bebc262bf23753c1e53d03432bde` pool |
| NXM book value (bonding curve price) | Nexus Mutual `Pool.sol` `getTokenPrice()` — archive node calls | ETH per NXM, daily snapshots |
| Capital pool value in ETH | `Pool.sol` `getPoolValueInEth()` — archive node | ETH, daily |
| MCR value | `MCR.sol` `getMCR()` — archive node | ETH, daily |
| MCR ratio | Computed: pool / MCR | Dimensionless |
| wNXM Uniswap volume | Uniswap v2 subgraph | USD daily |

**Archive node requirement:** Historical `eth_call` against Nexus contracts requires an archive node (Alchemy, Infura archive tier, or self-hosted). Standard nodes only serve current state.

**Recommended archive node:** Alchemy — `eth_call` with `block` parameter. Nexus Mutual contracts deployed on Ethereum mainnet.

**Contract addresses (Ethereum mainnet):**
- Pool (V1): `0xcafea112Db32436c2390F5EC988f3aDB96870627` *(verify against Nexus docs — may have been upgraded)*
- MCR: `0xcafea7934490ef8b9D2572eAefEB9d48162ea5D8` *(verify)*
- wNXM token: `0x0d438f3b5175bebc262bf23753c1e53d03432bde`
- Nexus Mutual GitHub for ABI: `https://github.com/NexusMutual/smart-contracts`

**Verification step before building backtest:** Confirm current contract addresses via `https://api.nexusmutual.io/v1/contracts` — Nexus has upgraded contracts and the above addresses may point to proxies.

### Historical Period to Analyze

- **Full history:** wNXM launch (October 2020) to present
- **Key stress events to identify manually first:**
  - March 2021: ETH price crash stress on capital pool
  - May 2021: Crypto market crash
  - November 2022: FTX collapse (ETH price crash → pool stress)
  - 2023-2024: Any MCR breach events

**Step 1:** Pull daily MCR ratio. Identify all periods where ratio crossed below 1.0 and then recovered above 1.0. These are your candidate signal events.

**Step 2:** For each recovery event, measure:
- wNXM discount at moment of MCR recovery (gate reopening)
- wNXM discount 7, 14, 30, 60 days after recovery
- Maximum discount during the stress period
- Speed of compression

**Step 3:** Simulate entry at gate reopening (MCR ratio crosses 1.0 from below) with discount >20%, exit at discount <12% or time-stop at 60 days.

### Metrics to Compute

| Metric | Target | Kill Level |
|--------|--------|------------|
| Win rate (discount compressed before time-stop) | > 60% | < 40% |
| Average return per trade (ETH-denominated) | > 15% | < 5% |
| Average holding period | < 45 days | > 55 days |
| Max drawdown during trade (ETH terms) | < 35% | > 50% |
| Number of historical signal events | > 3 | < 2 (insufficient sample) |

**Baseline comparison:** Compare returns to simply holding ETH during the same periods. The trade should outperform ETH hold during stress recovery periods, since wNXM should compress faster than ETH recovers.

**ETH-denominate all returns.** wNXM is priced in ETH. A USD-denominated return is misleading — if ETH doubles and wNXM stays flat, that's a loss in real terms.

### Known Backtest Limitations

1. **Slippage not modeled:** Historical Uniswap prices don't reflect the actual fill price for a $100-300k order. Apply a 2-3% slippage haircut to all entry/exit prices.
2. **Small sample size:** MCR breach-and-recovery events may number only 3-6 in history. Statistical significance will be low. Treat backtest as qualitative validation, not statistical proof.
3. **Survivorship:** Nexus Mutual still exists. If it had failed, we wouldn't be analyzing it. This biases the backtest positively.
4. **Contract upgrades:** Nexus has upgraded contracts. Ensure the MCR calculation method was consistent across the historical period being analyzed. V1 vs V2 mechanics differ.

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show ALL of the following:

1. **≥ 3 historical signal events** identified where MCR ratio recovered above 1.0 with discount >20%
2. **≥ 60% of events** showed discount compression to <12% within 60 days of gate reopening
3. **Average ETH-denominated return > 10%** across all events (including losses), after applying 3% slippage haircut
4. **No event showed a loss > 40%** in ETH terms (i.e., the hard stop at -30% was never catastrophically insufficient)
5. **Discount compression preceded or coincided with ETH recovery** — confirming the mechanism is the gate reopening, not just general market recovery

If fewer than 3 historical events exist, the strategy cannot be backtested meaningfully. In that case: move directly to paper trading with a $5,000 position on the next live signal, treating the live trade as the "backtest."

---

## Kill Criteria

Abandon the strategy if ANY of the following occur:

| Trigger | Reason |
|---------|--------|
| Nexus Mutual migrates fully to V2 with different bonding curve mechanics | Structural mechanism changes — re-evaluate from scratch |
| Nexus governance removes the MCR gate entirely | The binary trigger no longer exists |
| wNXM daily volume drops below $50k consistently | Liquidity insufficient to enter/exit even a $50k position |
| Two consecutive live trades hit the 60-day time-stop without compression | Mechanism may be broken or market has adapted |
| wNXM is delisted from Uniswap or primary liquidity migrates to an inaccessible venue | Execution impossible |
| Nexus Mutual suffers a catastrophic claim event that drains >50% of capital pool | Existential risk — discount may never compress |

---

## Risks

### Critical Risks

**1. Illiquidity is the primary killer.** wNXM's thin order book means entry/exit slippage can consume the entire expected return. A 20% discount sounds attractive; a 3% entry slippage + 3% exit slippage + 2% Uniswap fee = 8% friction before the trade makes a cent. Net expected return may be 7-12%, not 20%.

**2. Small sample size means the backtest will be inconclusive.** If only 2-3 MCR breach events exist in history, we cannot distinguish "the mechanism works" from "we got lucky twice." This strategy may need to be run live at small size to generate data.

**3. Timing of compression is unknown.** The gate reopening is mechanical. The *speed* of compression depends on how many KYC members are watching and have capital to deploy. In a bear market, even KYC members may be capital-constrained. The 60-day time-stop may trigger frequently.

**4. Nexus V2 migration risk.** Nexus Mutual has been developing V2 with different tokenomics. If the bonding curve or MCR mechanism changes materially, this strategy's structural basis evaporates. Monitor `https://nexusmutual.io/blog` and governance forum for migration announcements.

**5. Governance can change MCR formula.** MCR is not purely algorithmic — governance can vote to adjust the formula. A governance vote to lower MCR could artificially "reopen" the gate without genuine capital recovery, creating a false signal.

### Secondary Risks

**6. ETH price correlation.** The capital pool is denominated in ETH. If ETH crashes, the pool falls below MCR even with no claims. This means MCR breaches often coincide with ETH bear markets — the worst time to be long anything crypto-correlated. wNXM may compress its discount while still losing value in USD terms.

**7. Pending large claims.** A large unresolved claim (e.g., a major protocol hack with Nexus coverage) can drain the pool after the gate reopens, causing a second breach. Always check `Claims.sol` for pending claim amounts before entry.

**8. KYC member base may be shrinking.** If fewer active KYC members exist over time, the arbitrage mechanism weakens. The compression relies on *someone* executing the arb. If the KYC member base is inactive or small, compression may be slow or incomplete.

---

## Data Sources

| Data | URL / Endpoint |
|------|---------------|
| Nexus Mutual contract addresses | `https://api.nexusmutual.io/v1/contracts` |
| Nexus Mutual GitHub (ABIs) | `https://github.com/NexusMutual/smart-contracts` |
| Nexus Mutual capital pool stats (dashboard) | `https://nexusmutual.io/capital-pool` |
| wNXM token (Etherscan) | `https://etherscan.io/token/0x0d438f3b5175bebc262bf23753c1e53d03432bde` |
| Uniswap v2 wNXM/ETH pool | `https://v2.info.uniswap.org/pair/0x0d438f3b5175bebc262bf23753c1e53d03432bde` *(verify pool address)* |
| Uniswap v2 subgraph (historical prices/volume) | `https://thegraph.com/hosted-service/subgraph/uniswap/uniswap-v2` |
| Archive node for historical `eth_call` | Alchemy: `https://www.alchemy.com` (archive tier required) |
| Nexus governance forum | `https://forum.nexusmutual.io` |
| Nexus Mutual blog (V2 migration updates) | `https://nexusmutual.io/blog` |
| NXM book value (current) | `https://api.nexusmutual.io/v1/token-price` |
| Historical MCR ratio tracker (community) | Check Dune Analytics: `https://dune.com` — search "Nexus Mutual MCR" |

**Recommended Dune query starting point:** Search Dune for existing Nexus Mutual dashboards. Community analysts have built MCR ratio trackers that may provide pre-computed historical data, avoiding the need to replay archive node calls from scratch.

**Primary implementation path:**
1. Pull NXM book value and wNXM market price from Uniswap subgraph (daily, back to Oct 2020)
2. Pull MCR ratio from Dune or archive node calls
3. Compute discount series: `(book_value - market_price) / book_value`
4. Identify MCR breach-and-recovery events
5. Simulate strategy returns per event
6. Build monitoring script that alerts when MCR ratio crosses 1.0 from below with discount >20%

---

*This document is sufficient to begin backtest construction. The primary uncertainty is sample size — if fewer than 3 clean MCR breach-and-recovery events exist in history, escalate to the team before proceeding. Do not paper trade without completing step 3 (historical event identification) first.*
