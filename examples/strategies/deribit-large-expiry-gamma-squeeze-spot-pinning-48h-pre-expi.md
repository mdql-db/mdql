---
title: "Deribit Large Expiry Gamma Squeeze — Spot Pinning 48h Pre-Expiry"
status: HYPOTHESIS
mechanism: 5
implementation: 6
safety: 6
frequency: 2
composite: 360
categories:
  - options-derivatives
  - calendar-seasonal
created: "2026-04-04"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

In the 48 hours before a major Deribit options expiry, market makers who are net short near-the-money options are mechanically compelled to delta-hedge in a reflexive, price-resisting manner. This creates a gravitational well around the max-pain strike — the price level at which aggregate options OI expires with minimum total value. The pinning force is not statistical tendency; it is the direct mechanical output of risk-limit-driven hedging. If spot is already close to max-pain, the hedging flows from multiple MMs converge directionally, reinforcing the pin. The edge is: **spot will resist large moves away from max-pain in the final 48h window, making breakout fades structurally profitable when proximity and OI conditions are met.**

---

## Structural Mechanism

### Why this must happen (not just tends to happen)

1. **Gamma spikes near expiry.** For an at-the-money option, gamma scales approximately as `1 / (σ√T)`. As T → 0, gamma → ∞. A market maker short a $50M notional near-the-money call cluster may need to buy or sell hundreds of BTC within minutes of a 1% price move. This is not discretionary — it is mandated by their delta-neutral risk mandate.

2. **Hedging flow is reflexive and directional.** If spot rises toward a large short-call strike cluster:
   - MMs must buy delta on the way up (increasing exposure as price rises)
   - If price retraces, MMs must sell delta back
   - Net effect: MMs are always buying into rises and selling into falls *near the strike* — this is mechanical resistance to breakouts, not opinion

3. **Max-pain is the convergence point.** Max-pain is the strike price at which the total dollar value of all open options (calls + puts) expires worthless at maximum. It is calculable from public OI data. When spot is near max-pain, the aggregate hedging flows from all short-options MMs point toward keeping spot near that level. No coordination is required — each MM independently hedges, and the collective output is pinning.

4. **The 48h window is the critical zone.** Beyond 48h, gamma is still manageable and MMs can absorb moves with smaller hedge adjustments. Inside 48h, gamma is so elevated that even small moves trigger large hedge trades. The pinning force is strongest — and most tradeable — in this final window.

5. **Why it breaks down.** If a large directional catalyst (macro news, liquidation cascade, whale flow) overwhelms MM hedging capacity, the pin breaks. MMs may also be net long options in some expiries, reversing the dynamic entirely. This is why the trade is 5/10, not 8/10.

### What this is NOT

- This is not "max-pain always works" — it is a conditional, proximity-gated, OI-size-gated trade
- This is not a prediction that spot will move TO max-pain — it is a prediction that spot will RESIST moving AWAY from max-pain when already nearby
- This is not a volatility crush trade (vega) — it is purely a delta-hedging flow trade (gamma)

---

## Market Conditions Required

All three conditions must be met to enter:

| Condition | Threshold | Rationale |
|---|---|---|
| Time to expiry | ≤ 48h | Gamma elevated enough to create meaningful hedging flow |
| Spot proximity to max-pain | ≤ 3% | Pinning force is concentrated; beyond 3%, hedging flows diverge |
| Notional OI at dominant strike cluster | ≥ $200M | Below this, MM hedging volume is insufficient to resist organic flow |

---

## Entry Rules

### Step 1 — Calculate max-pain (T-48h)
At exactly 48h before monthly expiry (Deribit monthly expiries settle at 08:00 UTC on the last Friday of the month):

```
For each strike K:
  call_loss(K) = sum over all call strikes C ≤ K of: OI(C) × (K - C)
  put_loss(K)  = sum over all put strikes P ≥ K of: OI(P) × (P - K)
  total_loss(K) = call_loss(K) + put_loss(K)

max_pain_strike = K where total_loss(K) is minimised
```

Use Deribit public API: `GET /api/v2/public/get_book_summary_by_currency` filtered by expiry date.

### Step 2 — Check proximity filter
```
proximity = |spot_price - max_pain_strike| / max_pain_strike

IF proximity > 0.03: NO TRADE — exit process
IF proximity ≤ 0.03: proceed to Step 3
```

### Step 3 — Check OI size filter
Sum notional OI (in USD) for all strikes within ±5% of max-pain. If total < $200M: NO TRADE.

### Step 4 — Define the fade zones
```
upper_fade_trigger = max_pain_strike × 1.02   (spot breaks 2% above max-pain)
lower_fade_trigger = max_pain_strike × 0.98   (spot breaks 2% below max-pain)
```

### Step 5 — Entry execution
- **Short perp** when spot touches `upper_fade_trigger` (fading the upside breakout)
- **Long perp** when spot touches `lower_fade_trigger` (fading the downside breakout)
- Use limit orders at trigger price; do not chase
- Asset: BTC-PERP or ETH-PERP on Hyperliquid (mirroring Deribit underlying)

---

## Exit Rules

| Exit condition | Action | Notes |
|---|---|---|
| Spot returns to within 0.5% of max-pain | Close position at market | Pin confirmed, take profit |
| Spot breaks max-pain ± 4% | Close position at market | Stop-loss — pin has broken, directional move underway |
| T = 0 (settlement at 08:00 UTC) | Close all positions 30 min before settlement | Avoid settlement gap risk |
| Position held > 36h without resolution | Close at market | Time decay on thesis — if pin hasn't reasserted, it won't |

---

## Position Sizing

- **Base size:** 0.5% of portfolio notional per trade
- **Maximum size:** 1.0% of portfolio notional (never pyramid into a breaking pin)
- **Rationale for small size:** This is a 5/10 hypothesis. The stop-loss (±4% from max-pain) represents a ~2% move from entry (entry at ±2%, stop at ±4%). Risk per trade = 0.5% portfolio × 2% stop = 0.01% portfolio loss if stopped out. This is intentionally conservative pending backtest validation.
- **No leverage above 3x** — gamma squeezes can produce violent, fast moves if the pin breaks

---

## Backtest Methodology

### Data requirements
- Deribit historical OI snapshots by strike and expiry (available via Deribit API history or third-party providers: Amberdata, Laevitas, Tardis.dev)
- BTC and ETH hourly OHLCV spot/perp data
- Monthly expiry dates (Deribit last Friday of month, 08:00 UTC)

### Universe
- BTC monthly expiries: January 2021 — present (~40 expiries)
- ETH monthly expiries: January 2021 — present (~40 expiries)
- Exclude expiries where total OI < $200M (likely pre-2021 for ETH)

### Procedure
1. For each expiry, snapshot OI at T-48h and calculate max-pain strike
2. Check proximity filter (spot within 3% of max-pain) — record how many expiries qualify
3. For qualifying expiries, record all instances where spot touched ±2% from max-pain in the 48h window
4. Measure: did spot revert to within 0.5% of max-pain before hitting ±4%?
5. Calculate win rate, average P&L per trade, max drawdown, Sharpe

### Key metrics to validate
| Metric | Minimum threshold to proceed |
|---|---|
| Win rate | > 55% |
| Profit factor | > 1.3 |
| Sample size | ≥ 30 qualifying trades |
| Max drawdown | < 5% of portfolio |

### Confounds to control for
- **Macro event overlap:** Flag expiries that coincide with FOMC, CPI, major protocol events — test with and without these excluded
- **Bull vs. bear regime:** Segment by market regime (BTC > 200d MA = bull) — pinning may be weaker in trending markets
- **OI concentration:** Test whether higher OI concentration at max-pain strike (vs. distributed OI) improves win rate

---

## Go-Live Criteria

All of the following must be satisfied before live deployment:

- [ ] Backtest win rate > 55% on ≥ 30 qualifying trades
- [ ] Profit factor > 1.3 after estimated 0.05% round-trip transaction costs
- [ ] Strategy survives exclusion of top 5 best trades (robustness check)
- [ ] Paper traded for minimum 3 live expiry cycles with positive P&L
- [ ] No single expiry accounts for > 30% of total backtest profit
- [ ] Reconciliation with `strategy-options-expiry-gravity` completed — confirm these are non-overlapping or one supersedes the other

---

## Kill Criteria

Deactivate immediately if any of the following occur in live trading:

- 5 consecutive losing trades
- Drawdown exceeds 3% of portfolio from this strategy alone
- Deribit changes expiry settlement mechanics or timing
- A structural shift in who provides options liquidity (e.g., on-chain options protocols absorb majority of OI, changing MM hedging dynamics)
- Win rate drops below 45% over trailing 20 trades

---

## Risks

### Primary risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Pin breaks on macro catalyst** | High | Stop-loss at ±4%; no trade within 6h of scheduled macro events (FOMC, CPI) |
| **MMs are net long options** (not short) | High | Check put/call OI ratio; if MMs are net long, hedging flow reverses — skip trade |
| **OI data staleness** | Medium | Use OI snapshot as close to T-48h as possible; re-check at T-24h |
| **Liquidity gap on stop** | Medium | Use perp markets (deep liquidity); avoid entering near low-liquidity hours |
| **Correlation with existing strategies** | Low-Medium | Check portfolio correlation before sizing; this strategy may be long vol in disguise |

### Structural risks (reasons this might not work at all)

1. **Max-pain is widely known.** If every participant knows max-pain, the pin may already be priced in, or adversarial flow may deliberately push through max-pain to trigger MM stop-losses (a "pin hunt"). This would invert the strategy.

2. **MMs hedge with options, not just spot/perp.** Sophisticated MMs may hedge gamma with other options (buying wings), reducing their spot hedging flow. If this is the dominant hedging method, the mechanical spot-pinning effect is weaker than assumed.

3. **Deribit is not the only venue.** As OI migrates to CME, on-chain options (Lyra, Premia, Dopex), and other venues, Deribit's OI may no longer represent the full hedging picture. The max-pain calculation would be incomplete.

---

## Data Sources

| Data | Source | Access |
|---|---|---|
| Deribit OI by strike and expiry | `api.deribit.com/api/v2/public/get_book_summary_by_currency` | Free, public |
| Historical Deribit OI snapshots | Tardis.dev, Laevitas, Amberdata | Paid; ~$200-500/month |
| BTC/ETH spot price | Binance, Coinbase public APIs | Free |
| BTC/ETH perp price | Hyperliquid public API | Free |
| Expiry calendar | Deribit website / API | Free |

---

## Open Questions for Researcher Review

1. **Does the proximity filter (3%) need to be dynamic?** A 3% proximity band on BTC at $30k is $900; at $100k it is $3,000. Strike spacing on Deribit is fixed in dollar terms, not percentage terms. Consider whether the filter should be expressed as number-of-strikes rather than percentage.

2. **Should we weight OI by open interest or by notional delta?** Max-pain calculated by OI count treats a 1 BTC contract the same regardless of moneyness. Notional-weighted max-pain may be more accurate.

3. **What is the MM net positioning assumption?** The entire mechanism assumes MMs are net short options (selling to retail/institutional buyers). This is generally true but should be verified empirically — check whether put/call OI skew or funding rates provide a proxy signal for MM positioning direction.

4. **Reconcile with `strategy-options-expiry-gravity`:** Before filing, determine whether this strategy is a refinement of the existing one (replace it) or a distinct variant (file separately with cross-reference). The 48h timing filter and OI-size gate are the key differentiators — assess whether these alone justify a separate entry.

---

*Hypothesis — needs backtest. Do not allocate capital until go-live criteria are met.*
