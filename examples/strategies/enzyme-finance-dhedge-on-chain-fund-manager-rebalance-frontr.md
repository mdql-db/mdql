---
title: "Enzyme Finance / dHEDGE On-Chain Fund Manager Rebalance Frontrun"
status: HYPOTHESIS
mechanism: 5
implementation: 5
safety: 4
frequency: 3
composite: 300
categories:
  - defi-protocol
  - index-rebalance
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

On-chain managed funds (Enzyme Finance, dHEDGE) execute rebalances as fully public blockchain transactions. Unlike TradFi, there is zero reporting lag — every trade is visible in real time, and multi-tranche rebalances telegraph future order flow before it executes. A fund mid-rebalance is a known buyer or seller of a known quantity in a known token. If the remaining order flow is large relative to on-chain liquidity, the price impact is predictable and front-runnable without mempool access — block-level detection is sufficient because rebalances typically span multiple transactions across minutes to hours, not milliseconds.

**The core claim:** On-chain fund transparency converts what is private information in TradFi (fund positioning) into public information in DeFi. This is a structural information asymmetry baked into the protocol design, not a historical pattern.

---

## Structural Mechanism

### Why this edge exists (and must exist, structurally)

1. **Mandatory on-chain execution:** Enzyme and dHEDGE vaults cannot trade off-chain. Every swap, every rebalance tranche is an on-chain transaction. There is no dark pool, no OTC desk, no block trade. The protocol architecture makes opacity impossible.

2. **Multi-tranche necessity:** Large rebalances relative to DEX liquidity cannot be executed in a single transaction without catastrophic slippage. Fund managers are economically forced to split orders into tranches, creating a time gap between first detection and final execution.

3. **Vault state is fully readable:** Enzyme's vault contract exposes current holdings, target allocations, and pending adapter calls. dHEDGE vault compositions are similarly queryable. The "remaining order" can be estimated by comparing current holdings to the implied target derived from the first tranche's direction and size.

4. **DEX liquidity is thin for long-tail tokens:** The tokens these funds trade (small-cap DeFi governance tokens, LP positions) often have $500K–$5M in DEX liquidity. A $100K buy in a $1M liquidity pool moves price 5–10% mechanically via AMM math (x·y=k). This is not an estimate — it is deterministic from the AMM formula.

5. **No speed requirement beyond block-level:** Unlike mempool frontrunning (which requires sub-second response and is MEV territory), this strategy only needs to detect the first confirmed transaction and enter before subsequent tranches. Rebalances spanning 30 minutes to several hours are common.

### Why the edge persists

- Most quants ignore Enzyme/dHEDGE because total AUM is modest (~$50–100M). The opportunity looks too small.
- MEV bots focus on mempool (single-block extraction), not multi-block rebalance sequences.
- The strategy requires protocol-specific knowledge (vault state reading, adapter decoding) that creates a knowledge barrier.

---

## Universe Definition

### Target vaults

| Criteria | Threshold |
|---|---|
| Minimum vault AUM | $500K |
| Minimum single rebalance size | $50K notional |
| Token liquidity ceiling | <$10M DEX liquidity (to ensure meaningful price impact) |
| Vault rebalance history | ≥3 documented rebalances in past 90 days |
| Chain | Ethereum mainnet, Polygon, Optimism, Arbitrum |

### Vault monitoring list (to be populated at backtest time)

- Top 10–20 Enzyme vaults by AUM (via Enzyme subgraph)
- Top 10–20 dHEDGE vaults by AUM (via dHEDGE subgraph, Polygon/Optimism)
- Refresh list monthly — vault AUM is dynamic

---

## Entry Rules

### Detection trigger

1. Query vault holdings every block (or every 12 seconds on Ethereum, ~2 seconds on Polygon).
2. **Trigger condition:** Vault holdings of token X change by ≥$25K notional in a single block AND the change is directional (buy or sell, not a deposit/withdrawal — filter by checking if vault share supply changed simultaneously).
3. **Estimate remaining order:** `Remaining = Target_Weight × Total_AUM − Current_Holdings`. Target weight is inferred from the direction and proportional size of the first tranche relative to historical rebalance patterns for that vault.
4. **Minimum remaining threshold:** Only enter if estimated remaining order is ≥$30K AND represents ≥3% of token's DEX liquidity depth (ensuring meaningful price impact).

### Entry execution

- **Direction:** Same direction as the detected vault trade (buy if vault bought, sell/short if vault sold).
- **Instrument:** Spot token on DEX (Uniswap, Curve, etc.) OR perpetual futures on Hyperliquid if the token has a listed perp.
- **Entry price:** Market order immediately after detection block confirms. Accept up to 1% slippage on entry.
- **Entry size:** See Position Sizing section.
- **Entry timing:** Must enter within 3 blocks of detection (36 seconds on Ethereum, ~6 seconds on Polygon). If missed, skip — the edge degrades rapidly.

---

## Exit Rules

### Primary exit: Vault completion signal

- Monitor vault state each block.
- **Exit trigger:** Vault holdings of token X stop changing for 3 consecutive blocks AND holdings are within 5% of estimated target.
- Exit via market order. Accept up to 1.5% slippage on exit.

### Secondary exit: Time stop

- **Hard time stop:** Exit 4 hours after entry regardless of vault state.
- Rationale: If the vault has not completed rebalancing in 4 hours, either the manager abandoned the rebalance or something unexpected occurred. The edge thesis no longer holds.

### Tertiary exit: Adverse move stop

- **Stop loss:** Exit if position moves against entry by 5% (i.e., token price moves opposite to expected direction by 5%).
- Rationale: If price is moving against the expected direction despite the vault's buying/selling, a larger counterforce is present. Do not fight it.

### Profit target (optional, not primary)

- No fixed profit target. Let the vault completion signal determine exit. Imposing a profit target would cap upside on large rebalances.

---

## Position Sizing

### Base sizing formula

```
Position_Size = min(
    0.15 × Estimated_Remaining_Order,   # 15% of estimated remaining vault flow
    0.10 × Strategy_Capital,            # 10% of total strategy capital per trade
    0.05 × Token_DEX_Liquidity          # 5% of DEX liquidity depth (own impact cap)
)
```

### Rationale

- **15% of remaining order:** Ensures we are not larger than the vault's own flow. We need the vault to move price; we are riding, not leading.
- **10% of strategy capital:** Single-trade risk cap. These are illiquid tokens; position concentration is dangerous.
- **5% of DEX liquidity:** Prevents our own entry from being the price-moving event, which would destroy the edge and create adverse selection on exit.

### Maximum concurrent positions

- **3 simultaneous positions** across different vaults/tokens.
- Total exposure cap: 25% of strategy capital.

### Leverage

- **Spot preferred.** If using Hyperliquid perps, maximum 2× leverage. These tokens can gap 20–30% on news; leverage amplifies ruin risk.

---

## Backtest Methodology

### Data requirements

| Data source | What to pull | Cost |
|---|---|---|
| Enzyme Protocol Subgraph (TheGraph) | All vault trades, holdings snapshots, share supply changes, timestamps | Free |
| dHEDGE Subgraph (TheGraph) | Same as above, Polygon + Optimism | Free |
| DEX trade data | Uniswap v2/v3, SushiSwap — token price per block | Free (TheGraph) |
| DEX liquidity depth | Pool reserves per block (for impact estimation) | Free (TheGraph) |
| Etherscan/Polygonscan | Transaction timestamps, block numbers | Free |

### Backtest period

- **Primary:** January 2022 – December 2024 (covers bull, bear, and sideways regimes)
- **Secondary:** 2021 (DeFi summer tail — high AUM, high activity)

### Backtest procedure

1. **Reconstruct vault holdings per block** from subgraph data. Identify all rebalance events (holdings change without share supply change).
2. **Classify rebalances** as single-tranche (completed in 1 transaction) vs. multi-tranche (≥2 transactions, ≥5 minutes apart).
3. **For multi-tranche rebalances only:** Simulate entry after first tranche detection, exit after final tranche.
4. **Apply realistic costs:**
   - Entry slippage: 1% (conservative for illiquid tokens)
   - Exit slippage: 1.5%
   - Gas costs: $5–$50 per transaction depending on chain and congestion (use historical gas data)
   - No maker/taker fee assumption — we are taking liquidity on DEX
5. **Measure:** PnL per trade, win rate, average holding time, Sharpe, max drawdown, and critically — **how often the vault completed the rebalance as expected** (completion rate).
6. **Sensitivity analysis:** Vary the "remaining order" estimation method. Test naive (linear extrapolation) vs. historical weight-based estimation.

### Key backtest questions to answer

- What fraction of detected rebalances are multi-tranche (opportunity set size)?
- What is the average price impact of vault rebalances on target tokens?
- Does our entry price capture meaningful edge after slippage and gas?
- What is the false positive rate (detected "rebalance" that was actually a deposit/withdrawal)?
- Does vault AUM correlate with trade profitability (i.e., is there a minimum AUM threshold)?

---

## Go-Live Criteria

All of the following must be satisfied before paper trading begins:

| Criterion | Threshold |
|---|---|
| Backtest Sharpe (after costs) | ≥ 1.0 over full period |
| Backtest win rate | ≥ 55% |
| Minimum qualifying trades in backtest | ≥ 50 (to have statistical confidence) |
| Average trade PnL after costs | ≥ 0.5% per trade |
| Multi-tranche rebalance rate | ≥ 30% of all detected rebalances (opportunity set viability) |
| Vault completion rate | ≥ 70% (vault finishes rebalance as expected) |

**Paper trading period:** 60 days minimum before live capital deployment.

**Paper trading go-live criteria:**
- ≥ 10 paper trades executed
- Paper trade Sharpe ≥ 0.8
- No systematic detection failures (monitoring system reliability ≥ 95% uptime)

---

## Kill Criteria

Immediately suspend the strategy if any of the following occur:

| Trigger | Action |
|---|---|
| 5 consecutive losing trades | Pause, review detection logic |
| Single trade loss > 8% of strategy capital | Immediate halt, full review |
| Rolling 30-day Sharpe drops below 0 | Suspend live trading, return to paper |
| Enzyme/dHEDGE total AUM drops below $20M | Suspend — opportunity set too small |
| Protocol upgrade changes vault transparency | Full review — edge may be eliminated |
| Detection system latency exceeds 5 blocks consistently | Suspend — edge degrades with latency |
| Regulatory action against on-chain frontrunning in key jurisdiction | Legal review before resuming |

---

## Risks

### Risk 1: AUM is the binding constraint (HIGH)
**Description:** Current Enzyme/dHEDGE AUM (~$50–100M total, <$5M per vault typically) means most individual rebalances are too small to move even illiquid token prices meaningfully after costs.
**Mitigation:** Set hard minimum thresholds (see Universe Definition). Monitor AUM growth — this strategy becomes more attractive as the ecosystem grows. Do not force trades below thresholds.

### Risk 2: Deposit/withdrawal misclassification (MEDIUM)
**Description:** Vault holdings change when investors deposit or withdraw, not just on rebalances. Misclassifying a large withdrawal as a "sell rebalance" creates a false signal.
**Mitigation:** Filter by share supply change. If share supply changes simultaneously with holdings, it is a deposit/withdrawal. Only act when holdings change WITHOUT share supply change.

### Risk 3: Manager abandons mid-rebalance (MEDIUM)
**Description:** A manager starts a rebalance and stops (market moved against them, changed their mind, technical issue). Our position is now stranded.
**Mitigation:** 4-hour hard time stop. Accept the loss and move on.

### Risk 4: Our entry becomes the price-moving event (MEDIUM)
**Description:** If our position is too large relative to liquidity, we move price before the vault's remaining tranches arrive, destroying the edge and creating adverse selection on exit.
**Mitigation:** Hard cap at 5% of DEX liquidity depth. Monitor our own market impact.

### Risk 5: MEV competition (LOW-MEDIUM)
**Description:** Sophisticated MEV bots may detect the same signal and front-run our entry.
**Mitigation:** We are not competing on mempool speed. We are exploiting multi-block, multi-hour rebalance sequences. MEV bots focus on single-block extraction. Our timescale is different. Monitor for evidence of systematic front-running of our entries.

### Risk 6: Protocol upgrade eliminates transparency (LOW)
**Description:** Enzyme or dHEDGE could implement private order routing, batch execution, or other opacity mechanisms in a future upgrade.
**Mitigation:** Monitor governance forums and protocol changelogs. This is a known protocol risk for any on-chain strategy.

### Risk 7: Liquidity disappears on exit (HIGH for small tokens)
**Description:** Long-tail tokens can lose 50%+ of DEX liquidity rapidly (LP withdrawals, pool migrations). Exit slippage could be catastrophic.
**Mitigation:** Check liquidity depth at entry AND set a liquidity floor. If pool liquidity drops >30% during the hold, exit immediately regardless of other signals.

### Risk 8: Tax and regulatory treatment of frontrunning (UNKNOWN)
**Description:** Regulatory treatment of on-chain frontrunning is legally ambiguous in most jurisdictions. This is not mempool frontrunning (which is clearly MEV), but the legal distinction may not be recognized by regulators.
**Mitigation:** Legal review before live deployment. This is a known unknown.

---

## Data Sources

| Source | URL / Access Method | Data | Cost |
|---|---|---|---|
| Enzyme Protocol Subgraph | TheGraph (hosted service) | Vault holdings, trades, share supply | Free |
| dHEDGE Subgraph | TheGraph (Polygon, Optimism) | Vault compositions, trades | Free |
| Uniswap v3 Subgraph | TheGraph | Pool reserves, swaps, price per block | Free |
| SushiSwap Subgraph | TheGraph | Same as above | Free |
| Etherscan API | etherscan.io/apis | Transaction data, gas prices | Free (rate limited) |
| Polygonscan API | polygonscan.com/apis | Same, Polygon chain | Free |
| Alchemy / Infura | Node provider | Real-time block data, contract calls | ~$50–200/month |
| Dune Analytics | dune.com | Pre-built vault dashboards, historical queries | Free tier available |
| CoinGecko / CoinMarketCap | API | Token price reference data | Free tier available |

### Monitoring stack (minimum viable)

```
Block listener (Alchemy webhook)
    → Vault holdings checker (ethers.js / web3.py)
    → Rebalance detector (deposit/withdrawal filter)
    → Remaining order estimator
    → Alert → Manual or automated entry
    → Position tracker → Exit monitor
```

---

## Open Questions for Backtest Phase

1. **What is the actual multi-tranche rate?** If 90% of rebalances are single-transaction, the opportunity set is negligible.
2. **What is the average time between first and last tranche?** This determines whether block-level monitoring is sufficient or if we need mempool access.
3. **Is there a minimum vault AUM below which the strategy is unprofitable after costs?** Hypothesis: ~$2M per vault.
4. **Do vault managers show consistent rebalancing behavior** (same day of week, same time of day) that would allow pre-positioning rather than reactive entry?
5. **Has Enzyme/dHEDGE AUM grown or shrunk over the backtest period?** If shrinking, the strategy may be in secular decline.
6. **Are there specific vault managers** (identifiable by vault address) who consistently execute large, multi-tranche rebalances? If so, a watchlist of 3–5 specific vaults may be more effective than broad monitoring.

---

## Verdict

This is a **mechanically sound hypothesis** with a genuine structural edge (on-chain transparency is real and permanent by protocol design). The binding constraint is ecosystem size, not logic. The strategy should be backtested immediately to determine whether the opportunity set is large enough to be worth the monitoring infrastructure cost. If Enzyme/dHEDGE AUM grows 5–10× from current levels, this becomes significantly more attractive. **File and revisit quarterly as AUM metrics update.**
