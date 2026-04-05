---
title: "Tokenized T-Bill Weekend Redemption Discount Arb"
status: PAUSED
mechanism: 7
implementation: 3
safety: 6
frequency: 6
composite: 756
categories:
  - basis-trade
  - defi-protocol
  - calendar-seasonal
created: "2025-01-31"
pipeline_stage: "Pre-backtest (step 2 of 9)"
---

## Hypothesis

Tokenized T-bill tokens (OUSG, USTB) trade at a measurable discount to NAV on secondary DEX markets during the Friday 3pm ET → Monday 9am ET window because the primary redemption mechanism is gated behind US business-day cutoffs. Holders needing immediate liquidity must sell into thin DEX pools rather than redeem at NAV. This creates a time-bounded, structurally-caused discount. When the redemption window reopens Monday morning, arbitrageurs with KYC access submit redemptions at NAV, buying pressure closes the discount, and the spread collapses. The edge is: buy the discount on Friday close, exit at or near NAV on Monday open.

**Causal chain:**
1. Ondo/Superstate publish daily NAV on-chain (or via signed attestation)
2. Friday 3pm ET: redemption cutoff passes → no T+1 settlement available until Monday
3. Holder needing weekend liquidity faces binary choice: hold until Monday or sell DEX at market
4. DEX pool is thin (OUSG circulating supply is largely held by institutions, not in AMM pools) → even modest sell pressure creates discount
5. Monday 9am ET: redemption window reopens → arbs with KYC can submit redemption at NAV → buy DEX, redeem at NAV, pocket spread
6. Buying pressure from arbs closes discount within hours of Monday open

---

## Structural Mechanism

**Why this MUST happen (to some degree):**

Ondo OUSG and Superstate USTB are legally structured as fund shares. Redemptions are processed by a fund administrator on US business days only — this is not a soft convention, it is written into the fund documents and enforced by the transfer agent. There is no on-chain mechanism to bypass this; the smart contract minting/burning is gated by the issuer's backend. This means:

- **The redemption gate is contractually enforced**, not probabilistic
- **The NAV is published daily** (Ondo posts NAV per share on-chain via oracle; Superstate posts via signed message) — the reference price is observable
- **The DEX pool is the only exit during the weekend** — this is a structural monopoly on liquidity for the window

The discount is therefore not a sentiment artifact. It is the price of liquidity during a window where the primary liquidity mechanism is administratively closed.

**Why it converges:**
Any party with redemption access (accredited investor, KYC'd) faces a risk-free trade: buy DEX at discount, submit redemption Monday at NAV, receive USDC at T+1. This is a textbook cash-and-carry with a known settlement date. The convergence is not guaranteed to be instantaneous but is guaranteed to occur within T+1 of Monday open, assuming the fund remains solvent (which is backed by short-duration US Treasuries — near-zero credit risk).

**Binding constraint on the edge:**
The arb is only executable by KYC'd redemption-eligible parties. If Zunid is not KYC'd, the strategy degrades to: "rely on other arbs to close the spread." This is the primary risk and the reason the score is 6 not 8.

---

## Entry Rules


### Entry
- **Time:** Every Friday, 3:30pm–4:00pm ET (30-minute window after redemption cutoff)
- **Instrument:** OUSG on Uniswap v3 (Ethereum mainnet) or USTB on applicable DEX
- **Condition:** `(NAV_per_share - DEX_mid_price) / NAV_per_share > 0.0020` (i.e., discount > 20bps)
- **Secondary condition:** DEX pool has ≥ $100k liquidity within 10bps of mid (to ensure fill is possible)
- **Action:** Buy OUSG/USTB on DEX, targeting $50k–$200k notional (see position sizing)
- **Do not enter** if: discount is < 20bps (fees eat the spread), pool depth < $100k, or NAV attestation is >24h stale

## Exit Rules

### Exit — Primary (with redemption access)
- **Time:** Monday 9:00am ET
- **Action:** Submit redemption request via Ondo/Superstate portal at published NAV
- **Settlement:** T+1 (Tuesday) in USDC
- **Target P&L:** Spread minus DEX swap fee (typically 5bps on Curve/Uniswap v3 stable pools) minus gas

### Exit — Secondary (without redemption access)
- **Time:** Monday 9:00am–12:00pm ET
- **Condition:** DEX price has recovered to within 5bps of NAV
- **Action:** Sell on DEX
- **Hard stop:** If by Monday 12pm ET discount has not closed to <10bps, sell anyway — do not carry into next week

### Exit — Stop Loss
- If discount widens beyond 100bps at any point (suggests something structurally wrong — fund issue, depeg event), exit immediately on DEX regardless of loss

---

## Position Sizing

- **Per-trade notional:** $50,000–$200,000
- **Rationale:** OUSG Uniswap pool TVL has historically been $1M–$5M; entering >$200k risks moving the market and eating the spread you're trying to capture
- **Scaling rule:** Size = min($200k, 5% of pool TVL at time of entry)
- **Portfolio allocation:** Max 2% of total portfolio per trade (this is a low-volatility, low-return strategy — not a large position)
- **No leverage:** This is a cash strategy. Do not use perpetual futures to hedge — the basis risk introduces more variance than the spread is worth

---

## Backtest Methodology

### Data Required

| Dataset | Source | Format |
|---|---|---|
| OUSG DEX price (tick-level) | The Graph — Uniswap v3 subgraph, pool address `0x...` (OUSG/USDC) | GraphQL, price per block |
| OUSG NAV history | Ondo Finance on-chain oracle or `docs.ondo.finance` published history | Daily NAV per share |
| USTB DEX price | Dune Analytics — query Superstate USTB pool | Daily/hourly |
| USTB NAV history | Superstate on-chain attestation contract | Daily |
| Gas cost history | Etherscan gas tracker API | Gwei per block |
| DEX fee tiers | Uniswap v3 pool config (0.05% = 5bps for stablecoin pools) | Static |

**Specific Uniswap pool to query:** Identify via Uniswap v3 pool factory — search for OUSG/USDC pair on Ethereum mainnet. As of writing, OUSG has limited DEX liquidity; confirm pool exists and has meaningful volume before proceeding.

### Backtest Period
- **Start:** January 2023 (OUSG launch) through present
- **Frequency:** Weekly observations (every Friday 3:30pm ET → Monday 9:30am ET)
- **Expected sample size:** ~100 weekly observations (2 years)

### Metrics to Compute

1. **Discount frequency:** % of Fridays where discount > 20bps
2. **Mean discount at entry:** Average (NAV - DEX price) / NAV at 3:30pm ET Friday
3. **Mean discount at Monday 9:30am ET:** Has it closed?
4. **Gross spread captured:** Entry discount minus exit discount
5. **Net P&L per trade:** Gross spread minus DEX fees (5bps) minus gas (estimate $20–$50 per round trip)
6. **Win rate:** % of trades where net P&L > 0
7. **Max adverse excursion:** Largest intra-weekend discount widening (risk sizing input)
8. **Time-to-convergence:** How many hours after Monday open until discount < 5bps

### Baseline Comparison
- Compare weekend discount distribution vs. mid-week (Tuesday–Thursday) discount distribution
- If the structural mechanism is real, Friday–Sunday discounts should be statistically larger than mid-week discounts
- Run t-test: H0 = no difference in discount magnitude by day of week

### What "Good" Looks Like
- Friday discount > 20bps on ≥ 30% of weeks
- Monday convergence occurs in ≥ 80% of cases
- Net P&L positive after fees in ≥ 70% of trades
- Annualised return on deployed capital > 200bps (otherwise not worth operational overhead)

---

## Go-Live Criteria

Before paper trading, the backtest must show:

1. **Discount exists:** Friday 3:30pm discount > 20bps on ≥ 25% of sampled weeks (proves the mechanism fires with sufficient frequency)
2. **Convergence is reliable:** In ≥ 75% of cases where discount > 20bps at entry, discount < 10bps by Monday 12pm ET
3. **Fee-adjusted edge:** Mean net P&L per trade > 10bps after DEX fees and estimated gas
4. **No structural breaks:** No single event where discount exceeded 200bps and did not converge within 48 hours (would suggest credit/redemption risk, not just liquidity)
5. **Liquidity check:** Pool TVL ≥ $500k on ≥ 80% of entry dates (confirms the trade is executable)

If KYC redemption access is obtained before go-live, upgrade score to 8/10 and reduce required convergence reliability threshold to 65% (because you can force convergence yourself).

---

## Kill Criteria

Abandon the strategy if any of the following occur:

1. **Discount disappears:** Rolling 8-week average Friday discount drops below 10bps (arbs have saturated the opportunity or pool liquidity has dried up)
2. **Convergence fails repeatedly:** 3 consecutive weeks where Monday 12pm discount > 15bps (suggests redemption mechanism is broken or KYC arbs have exited)
3. **Pool TVL collapses:** OUSG/USTB DEX pool TVL drops below $200k (trade becomes unexecutable at meaningful size)
4. **Regulatory change:** Ondo or Superstate announces same-day or 24/7 redemption capability (eliminates the structural gate)
5. **Fund-level event:** Any NAV deviation > 50bps from expected accrual (suggests credit event in underlying T-bill portfolio — exit all positions immediately)
6. **Operational cost exceeds edge:** If gas + DEX fees consistently consume > 50% of gross spread

---

## Risks

### Primary Risk: No Redemption Access
Without KYC/accredited investor status, Zunid cannot directly redeem at NAV. The strategy then depends entirely on other arbs closing the spread. This is a real risk — if institutional arbs are absent on a given Monday (holiday, thin staffing), the spread may not close until Tuesday. **Mitigation:** Obtain Ondo/Superstate KYC access. This is the single highest-leverage action to improve this strategy.

### Secondary Risk: Thin Pool / Slippage
OUSG DEX liquidity is institutionally held and not deep. A $200k buy on a $1M pool moves price by ~2–4% depending on curve shape. This can eat the entire spread. **Mitigation:** Size per the 5% of TVL rule; monitor pool depth before entry.

### Tertiary Risk: NAV Staleness
If Ondo's NAV oracle is delayed or the attestation is stale, the "discount" may be illusory — you're comparing DEX price to a stale NAV. **Mitigation:** Only enter if NAV attestation timestamp is < 24 hours old. Check on-chain oracle update time before entry.

### Tail Risk: Fund Redemption Suspension
In a stress scenario (T-bill market dislocation, Ondo operational failure), redemptions could be suspended indefinitely. The discount would widen, not converge. **Mitigation:** Stop-loss at 100bps discount widening; never size this as a large portfolio position.

### Regulatory Risk
SEC or CFTC action against tokenized securities could freeze redemptions or DEX trading. Low probability but non-zero. **Mitigation:** Position size cap at 2% of portfolio.

### Opportunity Cost Risk
Capital is locked Friday PM → Monday PM (or Tuesday for T+1 settlement). Annualised return on a 20bps spread captured 30% of weeks = ~6bps annualised on deployed capital. This is barely above money market rates on the idle capital. **Mitigation:** Only deploy if discount > 20bps; treat as a cash-management overlay, not a primary strategy.

---

## Data Sources

| Source | URL / Endpoint | Notes |
|---|---|---|
| Uniswap v3 subgraph (Ethereum) | `https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3` | Query `pool` by token address for OUSG/USDC |
| Ondo OUSG NAV oracle | On-chain: check Ondo docs at `https://docs.ondo.finance` for oracle contract address | NAV posted daily, verify contract on Etherscan |
| Superstate USTB attestation | `https://superstate.co` — check on-chain attestation contract | Signed NAV per share |
| Dune Analytics | `https://dune.com` — search "OUSG" or "USTB" for community dashboards | Pre-built dashboards may exist |
| Etherscan gas API | `https://api.etherscan.io/api?module=gastracker&action=gasoracle` | For gas cost estimation |
| Ondo OUSG token address | `0x1B19C19393e2d034D8Ff31ff34c81252FcBbee92` (Ethereum mainnet) | Verify before use |
| Uniswap pool finder | `https://app.uniswap.org/#/pool` or factory contract `getPair()` call | Find OUSG/USDC pool address |

---

## Implementation Notes

**Step 1 (this week):** Pull OUSG token address, find Uniswap/Curve pool, confirm it has >$200k TVL and meaningful weekly volume. If pool is dead, this strategy is dead — check before building anything else.

**Step 2:** Build Dune query: for every Friday 3:30pm ET block since OUSG launch, compute `(NAV - pool_price) / NAV`. Plot distribution.

**Step 3:** Overlay Monday 9:30am ET price to compute convergence rate.

**Step 4:** If backtest shows edge, initiate KYC process with Ondo Finance (accredited investor verification) in parallel with paper trading setup.

**Step 5:** Paper trade for 4 weeks minimum before live capital deployment.
