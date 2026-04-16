---
title: "Perp-Spot Basis Convergence — Binance Backtest"
strategy: perp-spot-basis-convergence.md
exchange: Binance
period_start: "2021-06-01"
period_end: "2024-12-31"
total_trades: 31
win_rate: 0.033
sharpe: -0.8
max_drawdown: 0.25
status: fail
created: "2026-04-05T00:00:00"
---

## Summary

Backtested perp-spot basis convergence on Binance. Strategy failed
decisively. Basis spreads are too thin after transaction costs.
Only 1 signal met the threshold across 7 tokens over months.

## Results

| Metric | Value |
|---|---|
| Total trades | 31 |
| Win rate | 3.3% |
| Avg loss | -0.10%/trade |
| Sharpe (ann.) | -0.8 |
| Max drawdown | 25% |

97% of trades hit time stop. Spread doesn't converge within 4h window.
Strategy killed.
