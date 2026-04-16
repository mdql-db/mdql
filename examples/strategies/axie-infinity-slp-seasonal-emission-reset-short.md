---
title: "Axie Infinity SLP Seasonal Emission Reset Short"
status: HYPOTHESIS
mechanism: 4
implementation: 3
safety: 6
frequency: 1
composite: 72
categories:
  - defi-protocol
  - token-supply
created: "2025-01-31T00:00:00"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

When Sky Mavis announces a reduction in Smooth Love Potion (SLP) earn rates, rational SLP farmers face a game-theoretic forcing function: their accumulated SLP inventory will generate less future income, so the marginal value of holding declines immediately upon announcement. Farmers who have been accumulating SLP in anticipation of higher prices must now sell their float before the implementation date, because:

1. Post-implementation, the daily earn rate drops, reducing the "replacement cost" signal that justified holding
2. Other farmers will reach the same conclusion simultaneously, creating a race-to-exit dynamic
3. SLP has no yield, no governance utility, and no burn mechanism sufficient to absorb the sell pressure

**Causal chain:**
```
Announcement confirmed
        ↓
Rational farmer: "My future SLP income drops in ≤14 days"
        ↓
Holding SLP now has negative carry (opportunity cost vs. USDT)
        ↓
On-chain: SLP moves from Ronin wallets → DEX/bridge → CEX
        ↓
Sell pressure concentrated in announcement-to-implementation window
        ↓
Price declines, recoverable only if new game mode absorbs demand
```

This is not "SLP tends to fall on bad news." It is: rational actors with a depreciating inventory and a known deadline MUST act before that deadline or accept a worse outcome. The mechanism is game-theoretic, not sentiment-based.

---

## Structural Mechanism

**Why this MUST happen (to the extent anything must):**

SLP is a pure utility token with no store-of-value properties. Its price is determined almost entirely by:
- Current earn rate (supply faucet speed)
- Current burn rate (breeding demand)
- Speculative float held by farmers

When earn rates drop, the supply faucet slows, but the existing float does not disappear. Farmers holding SLP earned at the old rate now face:

- **Replacement cost compression:** New SLP will be harder to earn, but the market will price SLP at the new equilibrium earn rate, not the old one. Existing float is not "more valuable" because it was harder to earn historically — the market prices the marginal unit.
- **Coordination failure risk:** Every farmer knows every other farmer is facing the same incentive. First-mover advantage in selling is real. This is a classic prisoner's dilemma: cooperate (hold) and risk being last out, or defect (sell) and capture better prices.
- **No natural buyer:** Breeders (the burn mechanism) do not increase breeding activity because earn rates drop — they breed based on Axie floor prices and ROI calculations independent of SLP earn rates.

**Degree of guarantee:** This is probabilistic, not contractual. The mechanism is strong but not airtight — a simultaneous positive catalyst (new game mode, partnership, AXS pump) can overwhelm the sell pressure. Score reflects this: 5/10.

---

## Entry Rules


### Event Detection (Manual or Automated)

**Trigger conditions (ALL must be met):**
1. Sky Mavis publishes official patch note or blog post confirming SLP earn rate reduction
2. Implementation date is ≥3 days and ≤14 days from announcement date
3. SLP spot price has NOT already declined >20% in the 48h prior to announcement (pre-pricing filter)
4. On-chain SLP transfer volume on Ronin is ≤150% of 30-day average at time of announcement (confirms sell wave has not already started)

**Entry:**
- Enter short position within 4 hours of announcement confirmation
- Venue preference: SLP/USDT spot short via margin on any CEX offering it (Binance has listed SLP spot; check current availability); if SLP perp exists, use perp
- Entry price: Market order or limit within 0.5% of mid at time of signal

## Exit Rules

**Exit — take profit:**
- Primary: 48 hours after implementation date (earn rate change goes live)
- Secondary: If SLP price declines >30% from entry before implementation date, close 50% of position and trail stop on remainder at -15% from entry

**Exit — stop loss:**
- Hard stop: +15% adverse move from entry price at any point
- Soft stop: If on-chain SLP transfer volume fails to increase above 30-day average within 72h of announcement, reduce position by 50% (mechanism not activating)

**Re-entry:** No re-entry on same event after stop-loss is hit.

---

## Position Sizing

**Base rule:** Risk no more than 1% of portfolio NAV per event.

**Sizing formula:**
```
Position size (USD) = (Portfolio NAV × 0.01) / Stop distance (%)
Stop distance = 0.15 (15% hard stop)
→ Position size = Portfolio NAV × 0.0667
```

**Liquidity cap:** Position size must not exceed 2% of SLP's 24h spot volume on the execution venue at time of entry. SLP is illiquid — this cap will frequently bind and reduce position size below the formula output. Do not override the liquidity cap.

**Scaling modifier based on on-chain signal:**
- If Ronin SLP transfer volume is already 120–150% of 30-day average at entry: reduce position by 25% (partial pre-pricing)
- If non-exchange Ronin wallet SLP balance has declined >5% in 24h before entry: reduce position by 50% (sell wave already underway, entry is late)

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format | Notes |
|---|---|---|---|
| SLP price history (OHLCV, daily + hourly) | CoinGecko API (`/coins/smooth-love-potion/market_chart`) | JSON | Free, no auth required |
| SLP earn rate change dates | Sky Mavis blog archive (`axie.substack.com`, `axie.infinitygames.io/blog`) | Manual extraction | ~8–12 events since 2021 |
| Ronin chain SLP transfer volume | Ronin Explorer (`explorer.roninchain.com`) or Sky Mavis's Ronin RPC | On-chain | SLP contract: `0xa8754b9fa15fc18bb59458815510e40a12cd2014` |
| SLP wallet balance snapshots | Ronin RPC `eth_getBalance` calls or Dune Analytics (Ronin schema) | SQL/API | Dune has Ronin data as of 2023 |
| CEX SLP volume | CoinGecko `/coins/smooth-love-potion/tickers` | JSON | Cross-reference with Binance historical data |

### Event Universe

Manually compile all SLP earn rate change announcements from 2021–present. Expected count: 8–15 events. For each event, record:
- Announcement date and time (UTC)
- Implementation date
- Direction (rate cut vs. rate increase)
- Pre-announcement SLP price (T-1 close)
- SLP price at: T+1, T+3, T+7, T+14, T+30 (where T = announcement date)
- Ronin SLP transfer volume: 30-day average prior, and daily volume for T through T+14

### Metrics to Compute

**Primary:**
- Mean and median SLP return from entry (announcement day close) to implementation date +48h
- Win rate (% of events where price declined from entry to exit)
- Maximum adverse excursion (MAE) per event — needed to validate 15% stop
- Maximum favorable excursion (MFE) per event — needed to validate 30% TP

**Secondary:**
- Correlation between on-chain transfer volume spike and price decline magnitude
- Pre-pricing filter effectiveness: compare events where price had already dropped >20% pre-announcement vs. those that had not
- Rate cut vs. rate increase asymmetry (do rate increases cause pump-and-dump?)

**Baseline comparison:**
- Random 14-day SLP return distribution (sample 1,000 random 14-day windows from SLP price history)
- Compare strategy event returns against this null distribution using a t-test or Mann-Whitney U test (non-parametric preferred given small sample)

### Backtest Constraints
- No look-ahead bias: use only data available at announcement time
- Assume 0.5% slippage on entry and exit (illiquid market assumption)
- Assume 0.1% maker fee per leg
- Do not assume perp availability — model as spot short with borrow cost of 5% annualized (conservative estimate for illiquid token borrow)

---

## Go-Live Criteria

All of the following must be satisfied before moving to paper trading:

1. **Sample size:** ≥8 qualifying events identified in historical data
2. **Win rate:** ≥62% on rate-cut events (above random chance with statistical significance p < 0.10, given small sample)
3. **Mean return:** Mean return from entry to exit ≥+8% net of fees and slippage (short side, so price must fall ≥8% on average)
4. **MAE validation:** ≤20% of events breach the 15% stop loss level (confirms stop placement is not too tight)
5. **On-chain signal correlation:** Ronin transfer volume spike (>120% of 30-day avg) must be present in ≥60% of winning events — confirms the mechanism is activating, not just noise
6. **Liquidity check:** At least one liquid execution venue (CEX spot margin or perp) must exist for SLP at time of go-live decision

If fewer than 8 events exist in history, the strategy cannot be statistically validated — park it and monitor for future events to expand the sample.

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Backtest failure:** Win rate <55% or mean net return <4% across historical events
2. **Mechanism breakdown:** Two consecutive live events where on-chain transfer volume does NOT spike within 72h of announcement (farmers are no longer behaving rationally, or the player base has shrunk below the threshold needed to generate measurable flow)
3. **Liquidity collapse:** SLP 24h spot volume on best execution venue drops below $500K consistently — position sizing becomes too small to be worth operational overhead
4. **Game shutdown / pivot:** Sky Mavis announces discontinuation of SLP as the primary earn token, or migrates to a new token economy (this would eliminate the mechanism entirely)
5. **Paper trading failure:** After ≥4 paper-traded events, win rate is <50% and mean return is negative
6. **Pre-pricing becomes systematic:** If the pre-pricing filter (>20% drop before announcement) triggers on ≥3 consecutive events, the market has learned the pattern and the edge is gone

---

## Risks

### Execution Risks
- **Illiquidity:** SLP is thinly traded on CEXs. A $50K position could move the market. The 2% of daily volume cap is essential but will severely limit position size. This strategy may only be viable at small scale.
- **No perp venue:** As of writing, no major perp venue lists SLP. Spot short requires borrow, which may be unavailable or expensive. This is the single largest practical barrier.
- **Ronin bridge friction:** If the sell wave happens on Ronin DEX (not CEX), the price impact may not be visible on CoinGecko/Binance until after the move. Entry timing is critical.

### Signal Risks
- **Pre-pricing:** Sky Mavis announcements often leak via Discord before official blog posts. By the time the official announcement is confirmed, the move may be 50–80% complete.
- **Offsetting catalysts:** A simultaneous AXS pump, new game mode launch, or partnership announcement can overwhelm the sell pressure. These are not predictable.
- **Shrinking player base:** Axie's active player count has declined dramatically from 2021 peaks. Fewer farmers = smaller float = smaller sell pressure = weaker signal. The mechanism may have been stronger in 2021–2022 than it is today.

### Structural Risks
- **Earn rate changes are now less frequent:** As the game matures, Sky Mavis has moved toward more stable tokenomics. Fewer events per year reduces strategy frequency and makes it harder to maintain statistical validity.
- **SLP burn rate changes can offset:** If Sky Mavis simultaneously increases breeding costs (SLP burn), the net supply impact is ambiguous. Filter out events where burn rate changes accompany earn rate changes, or model them separately.

### Model Risks
- **Small sample size:** 8–15 historical events is statistically weak. A 5/10 score reflects this. Do not over-optimize entry/exit rules to fit the small sample — keep rules simple and robust.

---

## Data Sources

| Source | URL / Endpoint | What to Pull |
|---|---|---|
| CoinGecko SLP price history | `https://api.coingecko.com/api/v3/coins/smooth-love-potion/market_chart?vs_currency=usd&days=max&interval=daily` | Full OHLCV history |
| CoinGecko SLP tickers (CEX volume) | `https://api.coingecko.com/api/v3/coins/smooth-love-potion/tickers` | Exchange-level volume |
| Sky Mavis blog (Substack) | `https://axie.substack.com` | Patch note archive — manual scrape |
| Sky Mavis Medium archive | `https://medium.com/axie-infinity` | Older announcements (2021–2022) |
| Ronin Explorer | `https://explorer.roninchain.com/token/0xa8754b9fa15fc18bb59458815510e40a12cd2014` | SLP transfer history |
| Dune Analytics (Ronin) | `https://dune.com` — search "Ronin SLP transfers" | Pre-built dashboards exist; fork and modify |
| Ronin RPC (SLP contract) | `https://api.roninchain.com/rpc` — `eth_getLogs` on SLP contract | Raw transfer event logs |
| Binance historical trades | `https://data.binance.vision/?prefix=data/spot/daily/trades/SLPUSDT/` | Tick-level CEX trade data |

**SLP contract address on Ronin:** `0xa8754b9fa15fc18bb59458815510e40a12cd2014`

**Recommended first step:** Pull the Sky Mavis blog/Medium archive manually and build the event table before touching price data. The event table is the foundation — if fewer than 8 qualifying events exist, stop here and do not proceed to backtest.
