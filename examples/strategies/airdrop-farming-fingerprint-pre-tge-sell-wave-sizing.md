---
title: "Airdrop Farming Fingerprint — Pre-TGE Sell Wave Sizing"
status: HYPOTHESIS
mechanism: 5
implementation: 3
safety: 5
frequency: 2
composite: 150
categories:
  - airdrop
  - token-supply
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Before a Token Generation Event, protocols run points programs that attract two distinct populations: genuine users who want the product, and mercenary farmers who want only the airdrop. Farmers have zero cost basis in the token and a documented prior behavior of claiming and selling within days. The ratio of farmers to genuine users in the pre-TGE deposit pool is a measurable, on-chain predictor of sell wave magnitude at TGE. High farmer concentration → large, fast, front-loaded sell wave. This is not a sentiment prediction — it is a population census of people who were always going to sell.

The edge is not in knowing *that* a sell wave will happen (everyone knows this). The edge is in sizing the wave *before* TGE using wallet fingerprints, allowing position sizing proportional to predicted magnitude rather than treating every TGE identically.

---

## Structural Mechanism

### Why the sell wave is structural, not behavioral

1. **Zero cost basis is contractual.** Airdrop recipients receive tokens at no monetary cost. There is no sunk cost, no average-down psychology, no tax-loss harvesting consideration. The rational floor for selling is zero — any price is profit.

2. **Farmer intent is revealed by prior behavior.** A wallet that farmed Arbitrum, Optimism, Starknet, and ZkSync — depositing capital briefly, claiming tokens, and bridging proceeds out within 7 days each time — has revealed a repeatable, mechanical exit strategy. This is not inference; it is a behavioral fingerprint written on-chain.

3. **Capital was never committed to the protocol.** Farming deposits are mercenary liquidity. The capital was parked to earn points, not because the farmer believes in the protocol. When the airdrop claim opens, the farmer's job is done. Exit is the only rational next action.

4. **Claim mechanics create synchronized exit pressure.** Claim windows open at a known time. All farmers face the same decision simultaneously. This is not distributed selling — it is a coordinated, time-stamped exit event with predictable clustering in the first 48–96 hours.

5. **Perp funding and borrow rates price in *some* sell pressure but not magnitude.** The market knows TGE selling happens. It does not have a systematic way to price the *scale* of that selling before on-chain cohort data is analyzed. This is the information asymmetry.

### Farmer Fingerprint Definition

A wallet is classified as a **Confirmed Farmer (CF)** if it meets ALL of the following:

| Criterion | Threshold |
|---|---|
| Prior airdrop claims | ≥ 3 distinct protocols |
| Sell/bridge behavior post-claim | Sold or bridged ≥ 80% of tokens within 7 days on ≥ 2 of 3 prior claims |
| Deposit dwell time in current protocol | Bottom tercile of all depositors (short dwell = farming behavior) |
| Cross-protocol activity | Active on ≥ 2 other points programs simultaneously during current farming window |

A wallet is classified as a **Genuine User (GU)** if it meets ALL of the following:

| Criterion | Threshold |
|---|---|
| Protocol interaction pre-points program | At least one transaction before points program launch |
| Dwell time | Top tercile of all depositors |
| Prior airdrop behavior | Held ≥ 50% of tokens for ≥ 30 days on majority of prior claims, OR no prior airdrop history |

All other wallets are **Unclassified (UC)** — treated as partial sellers in magnitude estimates.

### Sell Wave Magnitude Estimate

```
Estimated Sell Supply (ESS) = 
    (CF_count × CF_avg_allocation × 0.85)   # 85% of farmer allocation sold
  + (UC_count × UC_avg_allocation × 0.40)   # 40% of unclassified sold
  + (GU_count × GU_avg_allocation × 0.10)   # 10% of genuine users sold

Farmer Concentration Ratio (FCR) = CF_count / Total_eligible_wallets

Sell Pressure Index (SPI) = ESS / (Circulating_Supply_at_TGE × FDV_implied_price)
```

**SPI interpretation:**
- SPI > 0.15 → High sell pressure, full position
- SPI 0.08–0.15 → Moderate sell pressure, half position
- SPI < 0.08 → Low sell pressure, no trade

*These thresholds are hypothetical and must be calibrated against historical TGE data during backtesting.*

---

## Entry Rules


### Pre-conditions (all must be met)

- [ ] TGE date is publicly announced with a specific claim-open timestamp
- [ ] On-chain wallet cohort analysis is complete ≥ 48 hours before TGE
- [ ] SPI ≥ 0.08 (minimum threshold for trade)
- [ ] Perp is listed on Hyperliquid OR a correlated liquid asset is available for the hedge
- [ ] Funding rate on the perp is not already deeply negative (> −0.10% per 8h) — if it is, the trade is crowded and edge is priced out
- [ ] No major protocol catalyst (partnership announcement, exchange listing surprise) in the 72h window that could override sell pressure

### Entry

**Primary (perp listed pre-TGE):**
- Open short position on the token perp on Hyperliquid
- Entry timing: 2–6 hours before claim-open
- Rationale: Perp price often runs up on TGE hype in the final hours; shorting into this gives a better entry and captures both the hype fade and the sell wave

**Secondary (perp not listed or insufficient liquidity):**
- Short a high-beta correlated asset (e.g., the L1/L2 the protocol is built on, or a sector ETF equivalent like a basket of similar-stage tokens)
- Reduced conviction — note in trade log as "correlated hedge, not direct"

**Tertiary (spot only, no perp):**
- Do not trade. Spot short requires borrowing; borrow rates at TGE are unpredictable and can exceed the edge.

## Exit Rules

### Exit

| Scenario | Action |
|---|---|
| 48 hours post-claim-open, price down ≥ 30% from entry | Close 75% of position, trail stop on remainder |
| 48 hours post-claim-open, price down < 30% | Close 50% of position, reassess farmer sell-through rate on-chain |
| 96 hours post-claim-open (hard stop) | Close 100% of position regardless of P&L |
| Price rallies 15% against position at any point | Close 100% — stop loss, no averaging |
| On-chain data shows CF wallets have already sold > 70% of allocation | Close 100% — sell wave is exhausted |

**Hard rule:** No position held beyond 96 hours post-claim-open. The structural mechanism (farmer selling) exhausts within this window. Holding longer converts this into a directional bet with no structural edge.

---

## Position Sizing

### Base sizing formula

```
Position Size = (Account Risk per Trade) × (SPI Multiplier) × (Liquidity Discount)

Where:
  Account Risk per Trade = 1.5% of account NAV (fixed)
  
  SPI Multiplier:
    SPI > 0.15  → 1.0 (full risk unit)
    SPI 0.08–0.15 → 0.5 (half risk unit)
  
  Liquidity Discount:
    If token perp OI < $5M → multiply by 0.5
    If token perp OI $5M–$50M → multiply by 0.75
    If token perp OI > $50M → multiply by 1.0
```

### Leverage

- Maximum 3× leverage on the perp
- Rationale: TGE tokens are volatile in both directions. A surprise exchange listing or whale accumulation can spike price 50%+ against the short. Low leverage is survival, not timidity.

### Concentration limit

- No more than 2 active TGE short positions simultaneously
- No single position > 3% of account NAV at entry

---

## Backtest Methodology

### Dataset construction

**Target universe:** All tokens that launched with a points/farming program and had a public TGE between January 2023 and present. Estimated universe: 40–80 tokens with sufficient on-chain data.

**Candidate list (starting point for manual verification):**

| Token | TGE Date | Chain | Notes |
|---|---|---|---|
| ARB | March 2023 | Arbitrum | Large farmer population, well-documented |
| OP (second airdrop) | Feb 2024 | Optimism | Repeat farmers from OP1 visible |
| STRK | Feb 2024 | Starknet | High farmer concentration reported |
| ZK | June 2024 | ZkSync | Sybil controversy, high farmer ratio |
| EIGEN | Sept 2024 | Ethereum | Points program, restaking farmers |
| PYTH | Nov 2023 | Solana | Multi-chain farming |
| JUP | Jan 2024 | Solana | Large airdrop, known farmer activity |
| W (Wormhole) | April 2024 | Multi | Points farmers |
| ZETA | June 2024 | Solana | Smaller, good test case |
| OMNI | April 2024 | Ethereum | Smaller cap, high volatility |

*This list is illustrative. Full backtest requires systematic identification of all qualifying TGEs.*

### Backtest steps

**Step 1 — Wallet cohort reconstruction (most labor-intensive)**
- Pull all wallets that interacted with the protocol's points program
- For each wallet, query prior airdrop claim history across ARB, OP, STRK, ZK, JUP, PYTH, W, EIGEN using Dune Analytics or Nansen
- Classify each wallet as CF, GU, or UC per the fingerprint definition above
- Calculate FCR and SPI for each historical TGE

**Step 2 — Price data collection**
- Pull 1-minute OHLCV data for each token from TGE listing to 96 hours post-claim-open
- Source: Hyperliquid historical data, Binance, CoinGecko API
- Record: price at claim-open, price at 24h, 48h, 72h, 96h; max drawdown; max adverse excursion

**Step 3 — Signal validation**
- Regress SPI against 48h price return post-claim-open
- Test hypothesis: higher SPI → more negative 48h return
- Minimum acceptable R² for proceeding: 0.25 (weak but directional)
- Test FCR alone as a simpler signal (fewer data requirements)

**Step 4 — Entry timing analysis**
- Compare entry at: 6h pre-claim, 2h pre-claim, at claim-open, 2h post-claim-open
- Measure which entry captures the most return with least adverse excursion

**Step 5 — Exit rule optimization**
- Test fixed exits (24h, 48h, 72h, 96h) vs. on-chain trigger exits (CF sell-through > 70%)
- Measure Sharpe and max drawdown for each exit rule

**Step 6 — Crowding filter validation**
- Test whether funding rate at entry (> −0.10% per 8h) successfully filters out crowded trades
- Measure performance with and without the funding rate filter

### Minimum backtest acceptance criteria

| Metric | Minimum threshold |
|---|---|
| Sample size | ≥ 15 TGEs with complete cohort data |
| Win rate | ≥ 55% |
| Average return per trade (gross) | ≥ 8% |
| Sharpe ratio (annualized, if trade frequency allows) | ≥ 1.0 |
| Max single-trade loss | ≤ 15% |
| SPI vs. 48h return correlation | p < 0.10 |

---

## Go-Live Criteria

- [ ] Backtest complete on ≥ 15 historical TGEs
- [ ] All minimum backtest thresholds met
- [ ] Data pipeline for wallet cohort classification is automated or semi-automated (manual analysis per TGE is acceptable at launch but must complete ≥ 48h before TGE)
- [ ] At least 2 paper trades completed with full pre-trade cohort analysis documented before live capital deployed
- [ ] Funding rate filter validated in backtest
- [ ] Position sizing formula reviewed and approved by risk process

---

## Kill Criteria

The strategy is suspended immediately if any of the following occur:

| Trigger | Action |
|---|---|
| 3 consecutive losing trades | Suspend, review cohort classification methodology |
| Single trade loss > 20% of position | Suspend, review stop-loss rules |
| Backtest replication fails on out-of-sample TGEs (post-backtest period) | Suspend, re-examine SPI formula |
| Wallet cohort data becomes unavailable or unreliable (e.g., major Dune outage, chain obfuscation) | Suspend until data restored |
| Perp funding rates at TGE consistently exceed −0.15% per 8h across multiple tokens | Suspend — market has learned the trade, edge is priced out |
| Regulatory action against airdrop farming or token launches changes the structural mechanic | Full review, likely retire |

---

## Risks

### Risk 1: Farmer behavior changes (Medium probability, High impact)
Protocols are increasingly implementing anti-farming measures: Sybil detection, KYC requirements, vesting on airdrop allocations, or linear unlock schedules instead of cliff unlocks. If farmers receive vested tokens, the synchronized sell wave is broken. **Mitigation:** Check airdrop vesting schedule before every trade. If > 30% of farmer allocation is vested beyond 30 days, do not trade.

### Risk 2: Surprise positive catalyst at TGE (Low probability, High impact)
A major exchange listing (Binance, Coinbase) announced simultaneously with TGE, or a large strategic investor publicly accumulating, can overwhelm farmer selling. **Mitigation:** 15% stop loss is non-negotiable. Monitor announcement feeds in the 24h window.

### Risk 3: Cohort misclassification (Medium probability, Medium impact)
Wallet fingerprinting is imperfect. Sophisticated farmers use fresh wallets with no prior history, making them invisible to the CF classifier. This means FCR is systematically understated for protocols that attract sophisticated farmers. **Mitigation:** Treat FCR as a floor estimate. If the protocol is high-profile (likely to attract sophisticated farmers), apply a 1.2× multiplier to estimated ESS.

### Risk 4: Perp liquidity and slippage (Medium probability, Medium impact)
New token perps on Hyperliquid often have thin order books at TGE. A large short position can move the market against entry. **Mitigation:** Liquidity discount in position sizing formula. Never size a position > 2% of open interest at entry.

### Risk 5: Crowded trade / funding rate blowout (Medium probability, Medium impact)
If this strategy becomes widely known, funding rates on TGE shorts will become deeply negative before claim-open, eliminating the edge and creating a funding cost that erodes returns. **Mitigation:** Funding rate filter (> −0.10% per 8h = no trade). Monitor for systematic degradation of this filter over time as a kill signal.

### Risk 6: On-chain data latency (Low probability, Low impact)
Dune Analytics queries on large datasets can take hours. If cohort analysis is not complete ≥ 48h before TGE, the trade is skipped. **Mitigation:** Begin cohort analysis ≥ 7 days before TGE. Do not rush the data pipeline.

### Risk 7: Token not listed on Hyperliquid perp (Medium probability, Medium impact)
Many smaller TGEs will not have a Hyperliquid perp. Correlated hedges are lower conviction and harder to size. **Mitigation:** Only trade direct perps unless the correlation to the hedge asset is > 0.7 over the prior 30 days and the hedge asset is liquid.

---

## Data Sources

| Data need | Source | Notes |
|---|---|---|
| Wallet interaction history with protocol | Dune Analytics (custom query per protocol) | Most labor-intensive step |
| Prior airdrop claim history by wallet | Dune Analytics, Nansen, Arkham | Cross-chain queries required |
| Post-claim sell/bridge behavior | Dune Analytics, Nansen portfolio tracker | Define "sold" as: token transferred to CEX deposit address or swapped on DEX within 7 days |
| Token allocation per wallet | Protocol's airdrop checker or Merkle tree (public) | Available at claim announcement |
| Circulating supply at TGE | Protocol tokenomics docs, CoinGecko | Verify against smart contract |
| Perp price data (historical) | Hyperliquid API, Binance API | 1-minute resolution preferred |
| Funding rate history | Hyperliquid API | For crowding filter validation |
| TGE calendar | CryptoRank, TokenUnlocks.app, Messari | Cross-reference multiple sources |
| CEX deposit addresses (for sell classification) | Etherscan labels, Dune labeled addresses | Incomplete but sufficient for major CEXs |

---

## Open Questions for Backtest Phase

1. **Is FCR alone sufficient as a signal, or is the full SPI calculation necessary?** FCR is simpler to compute and may capture most of the variance. Test both.

2. **What is the optimal lookback for prior airdrop behavior?** Using the last 3 airdrops vs. all-time history — does recency matter? Farmers may have evolved their behavior.

3. **Does the sell wave exhaust faster for smaller tokens?** Smaller float = faster price impact = faster exhaustion. This would imply shorter exit windows for small-cap TGEs.

4. **Can the cohort analysis be partially automated?** A Dune dashboard that auto-classifies wallets for any new protocol would reduce the manual bottleneck significantly. Assess feasibility during backtest phase.

5. **Is there a secondary entry after the initial sell wave?** Some tokens recover after farmer selling exhausts and genuine users accumulate. Is there a long trade on the other side? Out of scope for this spec but worth noting.

---

## Notes for Next Pipeline Stage

- **Step 3 (Data collection):** Priority is building the Dune query template that can be adapted per protocol. The ARB and ZK TGEs are the best starting points — both had public Sybil analysis that can be used to validate the CF classifier.
- **Biggest uncertainty:** Whether the SPI → price return relationship is strong enough to justify the data collection effort. A quick-and-dirty test using FCR alone (simpler to compute) on 5–6 historical TGEs should be done first to validate the core hypothesis before building the full pipeline.
- **Researcher note:** The strategy is more defensible than a generic "short at TGE" rule because it provides a *sizing* mechanism. Even if the directional call is obvious, the ability to size larger on high-FCR TGEs and smaller on low-FCR TGEs is where the alpha lives.
