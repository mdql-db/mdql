---
title: "Points-to-Token Migration Forced Sell — Protocol Transition Dump"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 5
frequency: 2
composite: 300
categories:
  - defi-protocol
  - token-supply
  - airdrop
created: "2026-04-04T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When a DeFi protocol transitions from a points-based incentive program to a live token, it creates a structurally predictable double-compression event. The airdrop dump (Phase 1) is well-understood and widely traded. The less-understood Phase 2 is the TVL exodus: liquidity that was attracted *specifically* by points yield has no organic reason to remain once points stop accruing. This TVL exit simultaneously increases token supply (claims + sells) and destroys the valuation narrative (TVL → implied fee revenue → FDV multiple). The two legs reinforce each other. The edge is not that "tokens dump after launch" — that is a pattern. The edge is that the *mechanism forcing the dump is structural and observable in real time via on-chain TVL data*, allowing a staged entry that avoids the crowded pre-TGE short and captures the secondary compression.

---

## Structural Mechanism

### Why This Must Happen (The Causal Chain)

**Step 1 — Points programs attract mercenary capital by design.**
Points yield is the product. LPs deposit not because they believe in the protocol but because they are farming a future token claim. This capital is explicitly temporary — it was always going to leave when the incentive ended.

**Step 2 — Token launch terminates the incentive.**
The moment the token launches and claims open, the points program ends. The forward yield on staying in the LP position collapses to whatever organic fee revenue the protocol generates — typically a fraction of the implied points APR that attracted the capital.

**Step 3 — TVL exits because the yield product is gone.**
Mercenary LPs withdraw. This is not probabilistic — it is the rational action for capital that entered solely for points yield. The only question is *how fast* the exit occurs, not *whether* it occurs. Protocols that successfully retain TVL post-launch do so by replacing points with token emissions, which is a new incentive program, not organic retention.

**Step 4 — Valuation multiple compresses simultaneously.**
Pre-launch FDV is often justified by high TVL ("$2B TVL at 5x P/TVL implies $400M FDV"). When TVL drops 40-60%, the same multiple math now implies a much lower fair value. This is a narrative compression event, not just a supply event.

**Step 5 — The double-hit creates a non-linear price decline.**
Supply increase (airdrop sellers) + demand destruction (TVL narrative collapse) + potential secondary unlock cliffs = compounding downward pressure over a 30-90 day window.

### Why This Is Structural, Not Pattern-Based

The exit of mercenary capital is *contractually implied* by the nature of the points program itself. Points programs are explicit about their temporary nature — they end at TGE. The capital that entered under those terms has no contractual or economic reason to stay. This is closer to a guaranteed supply event than a historical tendency.

### The Timing Asymmetry (The Real Edge)

Pre-TGE shorts are crowded, expensive (negative funding), and subject to squeeze risk if launch is delayed. The structural edge is in the *post-launch confirmation window*: waiting for on-chain TVL to confirm the exit is happening, then entering a short that most participants have already abandoned ("the dump already happened"). The secondary compression is less crowded precisely because it is slower and less dramatic than the day-one dump.

---

## Universe Definition

**Eligible protocols must have ALL of the following:**

| Criterion | Threshold |
|---|---|
| Points program duration | ≥ 60 days pre-TGE |
| Peak TVL during points program | ≥ $200M |
| Token listed on perp exchange (Hyperliquid or equivalent) | Required |
| Points program explicitly terminated at TGE | Required |
| No immediate replacement emissions program announced | Required at entry; monitor |

**Disqualifying factors:**
- Protocol announces token emissions to replace points at TGE (the incentive continues; TVL may not exit)
- Token is listed with <$5M daily perp volume (position sizing becomes impractical)
- Protocol is a DEX where TVL = liquidity for the token itself (circular; TVL drop is partly the token price drop, not independent)

---

## Entry Rules

### Phase 1 Entry — TVL Confirmation Short (Primary)

**Trigger:** TVL drops ≥ 20% within 7 days of claim open date, measured from peak TVL in the 7 days *before* TGE.

**Entry mechanics:**
- Enter short on the perp at market open of the day following TVL confirmation
- Do not enter on TGE day or day-one dump — this is the crowded trade
- Minimum 48 hours post-claim-open before entry is permitted (avoids day-one volatility)

**Entry size:** 50% of target position at Phase 1 trigger

### Phase 2 Entry — Narrative Compression Add (Secondary)

**Trigger:** TVL drops an additional ≥ 15% from Phase 1 entry level (cumulative ≥ 35% from pre-TGE peak) AND token price has *not* declined proportionally (i.e., price is within 20% of TGE open price while TVL has dropped 35%+). This divergence between TVL and price is the compression setup.

**Entry size:** Remaining 50% of target position

**If Phase 2 trigger never fires:** Hold Phase 1 position only; apply Phase 1 exit rules.

### Entry Checklist (Must Pass All)

- [ ] Funding rate on perp is not more negative than -0.10% per 8 hours (crowded short; skip or reduce size)
- [ ] TVL data confirmed from ≥ 2 independent sources (DeFiLlama + protocol dashboard)
- [ ] No major protocol announcement in prior 48 hours (new emissions, partnership, exchange listing)
- [ ] Broader market (BTC) is not in a >15% 7-day drawdown (systemic risk distorts signal)

---

## Exit Rules

### Primary Exit — Target

**Exit 50% of position** when token price declines 35% from entry price.

**Exit remaining 50%** when ANY of the following:
- Token price declines 55% from entry price (full target)
- TVL stabilizes: less than 5% change over any rolling 7-day window post-entry (organic floor found)
- 45 days have elapsed since entry (time stop)

### Secondary Exit — Stop Loss

**Exit 100% of position immediately** if:
- Token price rises 20% above entry price on a closing basis (thesis invalidated)
- Protocol announces a new emissions/incentive program that replaces points (structural mechanism broken)
- Funding rate exceeds -0.15% per 8 hours for 3 consecutive funding periods (short squeeze risk; cost of carry destroys edge)

### Funding Rate Management

Monitor funding every 8 hours. If funding cost exceeds 0.05% per period for more than 5 consecutive periods, reduce position by 25% regardless of P&L. The carry cost of a crowded short can erode a structurally correct thesis.

---

## Position Sizing

**Base position size:** 2% of portfolio NAV per trade (full position across both phases)

**Rationale:** This is a medium-conviction structural trade (6/10) with fuzzy timing. The mechanism is sound but the magnitude and speed of compression vary significantly across protocols. 2% allows meaningful exposure without catastrophic drawdown if the protocol successfully retains TVL via new incentives.

**Scaling rules:**
- If TVL drop is ≥ 40% within 7 days (strong confirmation): scale to 3% NAV
- If perp liquidity is thin (<$3M daily volume): cap at 1% NAV
- Maximum concurrent positions in this strategy: 3 (avoid correlated exposure during broad market risk-off when all new tokens dump together)

**Leverage:** 2-3x maximum. This is a slow-moving structural trade, not a momentum play. High leverage introduces liquidation risk during the volatile post-TGE period.

---

## Backtest Methodology

### Target Dataset

Identify all protocols that meet the universe criteria from 2022 to present. Estimated universe size: 15-30 protocols. This is a small-N problem — statistical significance will be limited, and the backtest is primarily a *mechanism validation* exercise, not a curve-fitting exercise.

**Candidate protocols for backtest (hypothesis — verify eligibility):**
- Blur (BLUR) — points program → TGE Feb 2023
- Friend.tech (no token launched; useful as negative control)
- EigenLayer (EIGEN) — points → TGE 2024
- Ethena (ENA) — sats campaign → TGE 2024
- Pendle (PENDLE) — points adjacent; verify structure
- Kelp DAO (KEP) — restaking points → TGE
- Renzo (REZ) — EigenLayer points → TGE 2024
- Puffer Finance — points → TGE
- Zircuit — points → TGE

*Note: Each protocol must be individually verified against eligibility criteria before inclusion. Do not assume eligibility.*

### Measurement Protocol

For each protocol in the backtest universe:

**Pre-TGE baseline:**
- Record TVL at T-7, T-3, T-1 (days before claim open)
- Record FDV at TGE open price
- Record points program duration and peak TVL

**Post-TGE tracking:**
- TVL at T+1, T+3, T+7, T+14, T+30, T+45, T+60 (days after claim open)
- Token price at same intervals
- Funding rate (8h) at same intervals
- Whether a replacement emissions program was announced and when

**Outcome variables:**
- Maximum drawdown from TGE open to T+60
- Drawdown from Phase 1 entry signal (T+2 to T+7) to T+60
- TVL % change from pre-TGE peak to T+30
- Correlation between TVL % drop and price % drop (test the double-compression thesis)
- Funding rate trajectory (test crowding dynamics)

### Hypotheses to Test

| Hypothesis | Test |
|---|---|
| H1: TVL drops ≥20% within 7 days for >70% of eligible protocols | Count protocols meeting threshold |
| H2: Protocols with TVL drop ≥35% show greater price decline than those with <35% drop | Compare median price decline by TVL drop cohort |
| H3: Phase 1 entry (T+2 to T+7) outperforms TGE-day short on risk-adjusted basis | Compare Sharpe of entry timing |
| H4: Protocols that launch replacement emissions retain TVL and show smaller price decline | Compare TVL-retaining vs. TVL-losing protocols |
| H5: Funding rate is less negative at Phase 1 entry than at TGE open | Measure funding at TGE open vs. T+3 |

### Data Sources

| Data Type | Source |
|---|---|
| TVL (historical) | DeFiLlama API (free, public) |
| Token price (historical) | CoinGecko API, CoinMarketCap |
| Perp funding rates | Hyperliquid historical data, Coinglass |
| Airdrop/TGE dates | CryptoRank, Tokenomist, protocol announcements |
| Emissions program announcements | Protocol Discord, governance forums, Twitter/X |
| On-chain LP withdrawals | Dune Analytics (protocol-specific dashboards) |

### Backtest Limitations (Acknowledge Upfront)

- **Small N:** 15-30 protocols is insufficient for robust statistical inference. Treat as mechanism validation.
- **Survivorship bias risk:** Protocols that launched tokens and immediately failed may be underrepresented in data sources.
- **Look-ahead bias risk:** TGE dates and emissions announcements must be recorded as they were known *at the time*, not retroactively.
- **Market regime dependency:** 2022 bear market TGEs vs. 2024 bull market TGEs may behave differently. Segment by market regime.
- **Perp availability:** Not all tokens had perp markets at launch. Backtest must note where spot short (borrow) would have been required instead.

---

## Go-Live Criteria

The strategy moves from hypothesis to paper trading when ALL of the following are met:

- [ ] Backtest completed on ≥ 10 eligible protocols
- [ ] H1 confirmed: TVL drop ≥20% in ≥65% of cases
- [ ] H2 confirmed: statistically meaningful (even if not statistically significant given N) difference in price decline between high-TVL-drop and low-TVL-drop cohorts
- [ ] H3 confirmed: Phase 1 entry timing shows better risk-adjusted return than TGE-day entry in ≥60% of cases
- [ ] Funding rate analysis shows Phase 1 entry is less crowded than TGE-day entry in ≥60% of cases

**Paper trading period:** Minimum 3 live protocol TGEs observed and tracked in real time before live capital deployment.

**Live capital criteria:** Paper trading shows positive P&L on ≥ 2 of 3 paper trades, with no single paper trade exceeding the 20% stop loss.

---

## Kill Criteria

**Kill the strategy immediately if:**

- Live trading produces 3 consecutive stop-loss exits (20% adverse move each) — suggests the mechanism is broken or market has adapted
- A structural change occurs: major perp exchanges begin listing tokens *before* TGE, allowing pre-launch price discovery that eliminates the post-TGE compression window
- Protocols systematically begin launching with simultaneous emissions programs (the disqualifying factor becomes the norm, shrinking the universe to zero)
- Funding rates at Phase 1 entry are consistently more negative than -0.08% per 8 hours across ≥ 3 consecutive trades (market has learned the Phase 1 entry timing; edge is crowded out)

**Review (do not kill, but pause and reassess) if:**

- Win rate drops below 40% over any rolling 6-trade window
- Average holding period exceeds 45 days without hitting target or stop (time stop is triggering consistently; mechanism is slower than modeled)

---

## Risks

### Risk 1 — Replacement Emissions (High Probability, High Impact)
**Description:** Protocol announces token emissions to replace points at or shortly after TGE. TVL does not exit. The entire structural mechanism is neutralized.
**Mitigation:** Disqualify at entry if announced. Monitor governance forums and Discord continuously post-entry. Exit immediately if announced post-entry.
**Residual risk:** Announcement may come after Phase 1 entry but before Phase 2. Accept this as a known risk; stop loss protects against catastrophic loss.

### Risk 2 — Short Squeeze / Listing Pump (Medium Probability, High Impact)
**Description:** New exchange listing, partnership announcement, or broader market rally causes a violent short squeeze in the 7-30 day post-TGE window.
**Mitigation:** 20% stop loss. Low leverage (2-3x). Funding rate monitoring.
**Residual risk:** Gap risk on announcements. Cannot be fully hedged without options.

### Risk 3 — TVL Metric Manipulation (Low Probability, Medium Impact)
**Description:** Protocol inflates TVL figures via circular deposits, wash deposits, or protocol-owned liquidity that is not actually mercenary capital. TVL "drop" is artificial or the TVL was never real.
**Mitigation:** Cross-reference TVL across DeFiLlama, protocol dashboard, and Dune Analytics. Look for TVL composition (what assets, what pools). Prefer protocols where TVL is in third-party assets (ETH, stablecoins) rather than the protocol's own token.

### Risk 4 — Perp Market Illiquidity (Medium Probability, Medium Impact)
**Description:** Perp market for the token is thin, making entry and exit costly. Slippage erodes the edge.
**Mitigation:** Minimum $3M daily perp volume requirement at entry. Cap position at 1% NAV for thin markets. Use limit orders for entry; accept partial fills.

### Risk 5 — Correlated Macro Drawdown (Medium Probability, Medium Impact)
**Description:** Broad crypto market sells off simultaneously, making it impossible to distinguish the structural mechanism from beta. Also, if market rallies strongly, all new tokens may pump regardless of TVL dynamics.
**Mitigation:** BTC 7-day drawdown filter at entry (>15% drawdown = skip). Consider hedging with BTC long to isolate the alpha from beta.

### Risk 6 — Timing Uncertainty (High Probability, Low-Medium Impact)
**Description:** The TVL exit happens more slowly than modeled (over 60-90 days rather than 7-30 days). The 45-day time stop triggers before the thesis plays out.
**Mitigation:** Extend time stop to 60 days if TVL is still declining at day 45 (trend confirmation). Accept that some trades will be time-stopped at small losses.

### Risk 7 — Small Universe / Adverse Selection (Structural)
**Description:** The universe of eligible protocols is small (perhaps 5-10 per year). A few bad trades can dominate the track record. Additionally, the protocols that are most obvious candidates may be the ones where the market has already priced in the mechanism.
**Mitigation:** Maintain strict eligibility criteria. Do not force trades. Accept that this is a low-frequency strategy (perhaps 4-8 trades per year) and size accordingly.

---

## Open Questions for Research

1. **What is the median TVL half-life post-TGE for protocols without replacement emissions?** This determines whether the 45-day time stop is appropriate or needs adjustment.

2. **Is there a TVL composition signal?** Protocols where TVL is predominantly stablecoins (yield-seeking capital) may show faster exit than protocols where TVL is ETH (conviction holders). Test this.

3. **Does the size of the airdrop relative to FDV predict the magnitude of Phase 2 compression?** A larger airdrop = more sell pressure = faster narrative collapse = faster TVL exit?

4. **Can Dune Analytics dashboards be built to monitor LP withdrawal rates in real time?** This would allow a more granular entry signal than aggregate TVL.

5. **Is there a spot borrow market (e.g., Aave, Morpho) for newly launched tokens that could be used instead of perps when perp liquidity is thin?** This would expand the executable universe.

6. **Do protocols that launch on multiple chains simultaneously show different TVL dynamics?** Multi-chain TVL may be stickier due to bridging friction.

---

## Strategy Summary Card

| Field | Value |
|---|---|
| **Strategy type** | Structural short — protocol transition mechanics |
| **Asset class** | Crypto perp futures (newly launched tokens) |
| **Frequency** | Low — estimated 4-8 trades per year |
| **Holding period** | 7-45 days |
| **Target return per trade** | 35-55% on position (2-3x levered) |
| **Stop loss** | 20% adverse move from entry |
| **Position size** | 2-3% NAV |
| **Max concurrent positions** | 3 |
| **Score** | 6/10 |
| **Key data source** | DeFiLlama TVL API |
| **Primary risk** | Replacement emissions announcement post-entry |
| **Next step** | Build backtest dataset; verify universe eligibility |

---

*This document is a research hypothesis. No backtest has been completed. No live trading has occurred. All claims about mechanism and edge are theoretical and require empirical validation before capital deployment.*

## Data Sources

TBD
