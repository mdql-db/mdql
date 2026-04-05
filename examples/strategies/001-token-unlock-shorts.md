---
title: "Token Unlock Shorts"
status: PAPER_TRADING
mechanism: 6
implementation: 8
safety: 6
frequency: 3
composite: 864
categories:
  - token-supply
  - calendar-seasonal
created: "2026-04-03"
pipeline_stage: "Forward validation (step 5 of 9)"
---

## Hypothesis

Large scheduled token unlocks cause predictable price drops. Tokens can be shorted on perpetual futures before the unlock and closed after, capturing the decline.

## Why it's an edge

- Unlock dates are public and scheduled months in advance
- Recipients (early investors, team members) have strong incentive to sell
- The market underprices this — likely because retail doesn't track unlock calendars
- Zunid can monitor all unlock schedules 24/7 and execute programmatically

## Backtest results

Source: `experiments/unlock_backtest.py` — 18 events across SUI, APT, ARB, STRK, OP (2025)

| Metric | Value |
|---|---|
| Unlock 14-day avg return | -7.76% |
| Random 14-day avg return | -2.60% |
| Edge vs random | -5.16% |
| Large unlocks (>=2% supply) | -16.57% avg |
| Events showing 7d pre-unlock drop | 14/18 (78%) |

## Execution

- **Exchange:** Hyperliquid (decentralized perps, no KYC, API-accessible)
- **Available tokens:** SUI (10x), APT (10x), ARB (10x), STRK (5x), OP (10x), ENA (10x), ZRO (5x)
- **Entry:** Short 14 days before unlock
- **Exit:** Close 10 days after unlock
- **Position size:** $200 notional per trade (paper trading phase)
- **Fees:** 0.045% taker per trade (0.09% round-trip)

### Go-live setup (founder action required)

1. Set up an EVM wallet (MetaMask or similar)
2. Bridge USDC to Hyperliquid (runs on Arbitrum L2)
3. Deposit USDC into Hyperliquid trading account
4. Generate API key for programmatic trading
5. Zunid places shorts via Hyperliquid REST API (`POST https://api.hyperliquid.xyz/exchange`)

## Forward validation (current)

Paper trading system: `experiments/paper_trader.py`
State file: `experiments/paper_state.json`
Runs daily via GitHub Actions (`.github/workflows/paper-trader.yml`)

### Upcoming trades

- Canonical schedule: `experiments/unlock_events.json`
- Validation command: `python3 experiments/validate_unlock_events.py`
- Operational view: `python3 experiments/paper_trader.py`
- Do not maintain a manual upcoming-trades table here; the shared dataset is the single source of truth.

## Costs to track

- Trading fees (0.09% round-trip)
- Funding rates (accrued every 8h — shorts pay when funding is negative)
- Slippage (especially on smaller tokens like STRK)

## Go-Live Criteria

Deploy real capital when:

1. At least 3 paper trades closed
2. Net P&L positive after fees and funding
3. No single trade lost more than 10% of notional
4. Founder approves Hyperliquid wallet setup + USDC deposit

## Kill Criteria

- After 5 paper trades: net P&L negative after fees → kill or redesign
- After 10 paper trades: edge < 2% after all costs → kill
- Any time: regime change makes shorts consistently lose → kill

## Risks

- **Sample size:** 18 historical events is suggestive, not conclusive
- **Regime dependence:** Works in bear/flat markets; strong bull market may override unlock selling pressure
- **Funding rates:** Persistent negative funding on shorts could eat the edge
- **Crowding:** If unlock shorting becomes popular, the edge disappears (prices drop earlier)
- **Liquidity:** Smaller tokens may have thin orderbooks

## Data Sources

- Unlock schedules: auto-fetched daily from tokenomist.ai Schema.org data (`experiments/fetch_unlocks.py`)
- Prices + funding rates: Hyperliquid API (`POST /info` with `metaAndAssetCtxs`)
- Historical prices for backtesting: Binance public API (`/api/v3/klines`)

## Future improvements

- **On-chain wallet monitoring:** Vesting schedules are encoded in smart contracts on each token's chain. After tokens unlock, we could monitor the recipient wallets for transfers to exchange deposit addresses. Actual movement of tokens to exchanges is a stronger sell signal than the unlock date alone — some recipients hold, others sell immediately. This would allow position sizing based on observed behavior rather than just the calendar event.
- **Direct contract reads:** Currently we scrape tokenomist.ai for unlock dates. We could read the vesting contracts directly, removing the dependency on a third party. Each token has its own contract and chain (SUI on Sui, STRK on Starknet, ARB on Arbitrum, etc.), so this is more work but more robust.
- **Timing refinement:** Current -14d/+10d window is based on 18 historical events. As paper trading generates more data, the entry/exit timing should be re-evaluated.
- **Expand backtest sample size:** 18 events is thin. Hundreds of historical unlock events exist across the broader crypto market. Run the backtest across more tokens to increase confidence before deploying real capital.

## Entry Rules

TBD

## Exit Rules

TBD

## Position Sizing

TBD

## Backtest Methodology

TBD
