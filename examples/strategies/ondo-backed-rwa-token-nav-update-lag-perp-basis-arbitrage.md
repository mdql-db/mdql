---
title: "RWA NAV Update Lag — DEX Basis Arbitrage on Tokenized Treasuries"
status: HYPOTHESIS
mechanism: 6
implementation: 3
safety: 6
frequency: 2
composite: 216
categories:
  - basis-trade
  - defi-protocol
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Tokenized Treasury products (OUSG, bIB01, USYC) publish their on-chain NAV once per business day, sourced from custodian data after US market close. Between publications, secondary DEX prices float freely. When US Treasury yields move materially intraday — particularly on scheduled macro events (CPI, FOMC, NFP, Treasury auctions) — the DEX spot price diverges from the *expected* next NAV. Because the next NAV update is contractually forced to reflect actual underlying value, this divergence is a temporary mispricing with a known convergence catalyst.

**Causal chain:**
1. US 3-month T-bill yield moves ≥10bps intraday (e.g., CPI surprise)
2. OUSG/bIB01 DEX price does NOT reprice immediately (thin liquidity, retail holders unaware or unable to act)
3. Expected next NAV = Current NAV × (1 + daily yield accrual) ± intraday yield-driven price change in underlying fund
4. Spread between DEX price and expected next NAV opens to >15bps
5. Next business-day NAV publication forces convergence
6. Position closed at or before NAV update; profit = spread minus fees

The edge is NOT that "prices tend to converge." The edge is that the NAV update is a contractual, on-chain event that MUST set the official price to the custodian-verified underlying value. The DEX price has no mechanism to stay permanently disconnected.

---

## Structural Mechanism — WHY This MUST Happen

**The forcing function is the NAV publication contract itself.**

- Ondo Finance's OUSG and Backed Finance's bIB01 are structured as tokenized fund shares. The on-chain NAV is updated by a permissioned oracle/admin transaction, typically once per business day, referencing the custodian's official NAV (BlackRock's BUIDL for OUSG; iShares IB01 ETF NAV for bIB01).
- The custodian NAV is itself anchored to the mark-to-market value of the underlying T-bills/ETF at US market close.
- This means: if 3-month T-bill yields rise 15bps intraday, the underlying fund's NAV at close will be lower than the prior day's NAV by approximately (duration × yield change). For a ~0.25-year duration instrument, that's roughly 0.25 × 0.15% = ~3.75bps price decline.
- The DEX secondary price may not reprice this because: (a) most DEX LPs use the stale on-chain NAV as a reference, (b) retail holders don't monitor intraday yield moves, (c) liquidity is thin so no one is actively arbitraging.
- **The hard constraint:** Ondo/Backed CANNOT publish a NAV that differs from the custodian-verified value without misrepresenting the fund. The NAV update is not discretionary in direction — it must match the underlying.

**Why the arb floor is soft (not hard):**
Direct redemption at NAV requires KYC whitelisting and minimum investment thresholds ($100K+ for OUSG). This means non-whitelisted participants cannot force convergence via redemption. The DEX price could theoretically stay disconnected. However, whitelisted institutional participants CAN redeem, and they will if the discount is large enough to cover redemption friction (~1–2 day settlement). This creates a probabilistic, not guaranteed, floor.

---

## Entry/Exit Rules

### Universe
- **Primary:** OUSG (Ondo Finance) on Ethereum/Polygon DEXs (Uniswap v3, Curve)
- **Secondary:** bIB01 (Backed Finance) on Ethereum DEXs
- **Tertiary:** USYC (Hashnote) if DEX liquidity develops

### Entry Trigger (ALL conditions must be met)
1. **Macro event day:** Scheduled US macro release with yield-moving potential — CPI, FOMC decision/minutes, NFP, 3M/6M T-bill auction results. Use economic calendar (Investing.com, FRED release schedule).
2. **Yield move threshold:** 3-month T-bill yield (^IRX or FRED series DTB3) moves ≥10bps from prior close by 2:00 PM ET.
3. **Spread threshold:** DEX spot price of OUSG/bIB01 diverges from *expected next NAV* by ≥15bps net of estimated fees.
   - Expected next NAV = Prior NAV × (1 + overnight SOFR/365) ± (duration_years × intraday_yield_change_bps / 10000)
   - Use OUSG duration ≈ 0.25 years; bIB01 duration ≈ 0.08 years (ultra-short)
4. **Liquidity check:** DEX pool has ≥$500K TVL and estimated slippage for intended position size is <5bps (check via 1inch or DEX aggregator quote).
5. **NAV update not yet published for today:** Confirm via on-chain event log that today's NAV update has not yet occurred (Ondo publishes typically 6–8 PM ET).

### Direction
- **Long:** DEX price < Expected next NAV − 15bps → Buy token on DEX
- **Short:** DEX price > Expected next NAV + 15bps → Sell token on DEX (if short-selling mechanism exists; otherwise skip — do NOT use perps unless a liquid OUSG perp exists, which currently it does not)

### Exit Rules
1. **Primary exit:** Close position within 30 minutes of confirmed on-chain NAV update publication (monitor via Etherscan event logs for the NAV oracle update transaction)
2. **Secondary exit (time stop):** If NAV update does not occur within 28 hours of entry (e.g., holiday delay), exit at market regardless
3. **Convergence exit:** If DEX price converges to within 3bps of expected NAV before official update, exit early to capture most of the spread
4. **Stop-loss:** If spread widens beyond 2× entry spread (e.g., entered at 20bps discount, now 40bps discount), exit — something structural may have changed (protocol issue, redemption freeze)

### Position Sizing
- Maximum position: $50,000 notional per trade (constrained by DEX liquidity; larger sizes will move the market against you)
- Size to keep slippage ≤5bps on entry AND exit combined
- Use 1inch aggregator quote for $25K, $50K, $75K to find the liquidity cliff before entry
- No leverage — this is a spot basis trade, not a leveraged position
- Kelly criterion not applicable at this stage; use fixed fractional: 2% of total capital per trade until 50+ trades are logged

---

## Backtest Methodology

### Data Required

| Dataset | Source | Granularity | Notes |
|---|---|---|---|
| OUSG on-chain price (DEX) | Uniswap v3 subgraph (The Graph) | Per-block (~12s) | Pool address: verify on Etherscan |
| OUSG official NAV | Ondo Finance dashboard / on-chain oracle events | Daily | Extract from NAV update transactions |
| bIB01 DEX price | Uniswap/Curve subgraph | Per-block | |
| bIB01 official NAV | Backed Finance dashboard / on-chain | Daily | |
| 3M T-bill yield intraday | FRED API (series: DTB3) | Daily close only | Intraday requires Bloomberg/Refinitiv |
| 3M T-bill intraday proxy | IEF or SHY ETF intraday OHLCV | 1-minute | Free via Yahoo Finance / Polygon.io |
| Macro event calendar | Investing.com economic calendar | Event-level | Scrape or use API |
| DEX pool TVL/liquidity | DeFiLlama API | Daily | `https://api.llama.fi/protocol/ondo-finance` |

**Intraday yield proxy:** Since free intraday T-bill yield data is scarce, use SHY ETF (iShares 1-3 Year Treasury) intraday price as a proxy for yield direction. A 0.1% SHY decline ≈ ~5bps yield increase given ~1.9yr duration. Calibrate this relationship using daily data first.

### Backtest Period
- **Start:** January 2023 (OUSG launched March 2023; use bIB01 from its launch date)
- **End:** Present
- **Note:** Dataset is small (~2 years). This is a known limitation. Treat backtest as hypothesis validation, not statistical proof.

### Backtest Steps

1. **Build NAV update event log:** Extract all on-chain NAV update transactions for OUSG and bIB01. Record timestamp, old NAV, new NAV, NAV change in bps.

2. **Build DEX price series:** Pull per-block DEX prices from Uniswap subgraph. Resample to 15-minute OHLCV.

3. **Identify macro event days:** Pull economic calendar for CPI, FOMC, NFP dates. Flag days where SHY ETF moved >0.05% intraday (proxy for ≥10bps yield move).

4. **Compute spread series:** For each macro event day, at 2:00 PM ET, compute:
   - `expected_next_nav = prior_nav × (1 + sofr/365) + duration × intraday_yield_change`
   - `spread_bps = (expected_next_nav - dex_price) / dex_price × 10000`

5. **Simulate trades:** For all observations where `|spread_bps| > 15`, log a hypothetical trade. Apply:
   - Entry slippage: 5bps (conservative for thin markets)
   - Exit slippage: 5bps
   - Gas costs: $20–50 per transaction (use historical ETH gas prices)
   - Net P&L = spread − 10bps (slippage) − gas

6. **Metrics to compute:**
   - Win rate (% of trades where spread converged before time stop)
   - Average net P&L per trade in bps
   - Average hold time to convergence
   - Maximum adverse excursion (how wide did spread get before converging?)
   - Sharpe ratio (annualized, using daily P&L)
   - Total number of qualifying events (expect: low, ~10–30 over 2 years)

### Baseline Comparison
- Compare against: simply buying OUSG and holding (captures yield accrual, no timing)
- The strategy must generate excess return above passive OUSG holding on event days

---

## Go-Live Criteria

Before moving to paper trading, the backtest must show:

1. **≥10 qualifying trade events** (if fewer, dataset is too thin to conclude anything)
2. **Win rate ≥70%** (spread converges before time stop in 7 of 10 trades)
3. **Average net P&L ≥8bps per trade** after all costs (slippage + gas)
4. **No single trade loss exceeds 30bps** (would indicate a structural break, not noise)
5. **Convergence occurs within 24 hours in ≥80% of winning trades** (validates the timing assumption)
6. **DEX liquidity sufficient for $25K position** in ≥80% of qualifying events (check historical TVL)

If criteria 1 is not met (fewer than 10 events), extend backtest to include non-macro days where yield moved ≥10bps for any reason, and re-evaluate.

---

## Kill Criteria

Abandon the strategy if ANY of the following occur:

1. **Backtest shows <60% win rate** — convergence is not reliable enough without hard redemption arb
2. **Average net P&L <5bps** — fees eat the edge; not worth operational complexity
3. **Ondo/Backed introduces redemption freeze or NAV update pause** — removes the forcing function
4. **DEX liquidity drops below $200K TVL** — position sizing becomes unworkable
5. **KYC whitelisting becomes available to Zunid** — if we get direct redemption access, re-score this strategy to 8/10 and rebuild as a hard arb; current soft version may be superseded
6. **Competing protocols launch with more frequent NAV updates** (e.g., intraday NAV) — removes the lag window entirely
7. **After 20 live paper trades:** If realized Sharpe < 0.5 or win rate < 60%, kill regardless of backtest results

---

## Risks — Honest Assessment

### Critical Risks

**1. KYC gating removes the hard floor (HIGH IMPACT)**
Without direct redemption access, the DEX price can stay disconnected from NAV indefinitely if no whitelisted arb participant is active. The convergence is probabilistic, not guaranteed. This is the single biggest weakness of this strategy.

**2. Thin DEX liquidity (HIGH IMPACT)**
OUSG and bIB01 DEX pools are small. A $50K position may move the market 20–50bps, eating the entire spread. This is not a scalable strategy — it is inherently capacity-constrained to ~$50–100K per trade.

**3. NAV update timing uncertainty (MEDIUM IMPACT)**
Ondo publishes NAV "after US market close" but exact timing varies. If the update is delayed (holiday, custodian issue, technical problem), the time stop triggers and you exit at market, potentially at a loss.

**4. Intraday yield move reversal (MEDIUM IMPACT)**
If you enter based on a 2:00 PM yield move and yields reverse by 4:00 PM close, the actual NAV update will reflect the reversed yield, not the 2:00 PM level. Your expected NAV calculation will be wrong. Mitigate by waiting until 3:30 PM ET to enter (30 minutes before US close, yield move more likely to be sticky).

**5. Smart contract / oracle risk (LOW-MEDIUM IMPACT)**
If Ondo's NAV oracle is compromised or publishes an incorrect NAV, convergence fails. This is a tail risk but not zero given the protocol's permissioned oracle design.

**6. Regulatory risk (LOW IMPACT, HIGH SEVERITY)**
Tokenized RWA protocols operate in a gray regulatory area. A sudden regulatory action could freeze redemptions or DEX trading. Position sizing limits mitigate this.

### Structural Limitation
This strategy has a small opportunity set — perhaps 10–30 qualifying events per year. It cannot be a primary strategy; it is a supplementary edge to be run alongside other strategies. Expected annual gross P&L at $50K position size and 15bps average spread: ~$1,125–$3,375/year. This is a proof-of-concept and learning exercise as much as a profit center.

---

## Data Sources

| Resource | URL / Endpoint |
|---|---|
| Ondo Finance OUSG NAV dashboard | `https://ondo.finance/ousg` |
| Ondo on-chain NAV oracle (Ethereum) | Etherscan: search "Ondo NAV" or monitor `FluxPriceOracle` contract events |
| Backed Finance bIB01 | `https://backed.fi` — NAV published daily |
| Uniswap v3 subgraph (The Graph) | `https://thegraph.com/hosted-service/subgraph/uniswap/uniswap-v3` |
| FRED API — DTB3 (3M T-bill daily) | `https://fred.stlouisfed.org/series/DTB3` / `https://api.stlouisfed.org/fred/series/observations?series_id=DTB3` |
| SHY ETF intraday (yield proxy) | Yahoo Finance: `SHY` — 1-minute bars via `yfinance` Python library |
| DeFiLlama — Ondo TVL | `https://api.llama.fi/protocol/ondo-finance` |
| Investing.com economic calendar | `https://www.investing.com/economic-calendar/` (scrape or use paid API) |
| 1inch aggregator (slippage quotes) | `https://api.1inch.dev/swap/v6.0/1/quote` |
| Etherscan API (on-chain events) | `https://api.etherscan.io/api` — filter by contract address, event topic |
| Polygon.io (intraday ETF data) | `https://api.polygon.io/v2/aggs/ticker/SHY/range/1/minute/{from}/{to}` |

---

## Implementation Notes for Backtest Builder

- **Priority 1:** Build the NAV update event log first. Without accurate NAV update timestamps, the entire backtest is invalid. Pull all transactions to the Ondo NAV oracle contract and extract (block_timestamp, old_nav, new_nav).
- **Priority 2:** Cross-reference DEX price at T-2 hours before each NAV update. This is the "stale price" window.
- **Priority 3:** The intraday yield proxy (SHY ETF) needs calibration against actual DTB3 daily changes before being used as a signal. Run a regression: `ΔDTB3 ~ ΔSHY_intraday` on daily data first.
- **Expect data gaps:** OUSG and bIB01 are new protocols. Some NAV updates may be missing from on-chain data if done via proxy contracts. Check Ondo's GitHub for contract addresses.
- **Do not data-mine:** Define the 15bps threshold and 10bps yield move threshold BEFORE running the backtest. Do not optimize these parameters on the backtest data.
