---
title: "Funding Rate Fade — Binance Backtest"
strategy: funding-rate-fade.md
exchange: Binance
period_start: "2022-01-01"
period_end: "2024-12-31"
total_trades: 342
win_rate: 0.58
sharpe: 1.4
max_drawdown: 0.12
status: pass
created: "2026-04-05"
---

## Summary

Backtested the funding rate fade strategy on Binance perpetuals across
BTC, ETH, and SOL pairs over 3 years. The strategy shows consistent
positive returns with a Sharpe of 1.4 and acceptable drawdowns.

## Configuration

- Pairs: BTCUSDT, ETHUSDT, SOLUSDT
- Timeframe: 8h funding settlement windows
- Entry threshold: funding rate > 0.05% or < -0.05%
- Position sizing: 1% risk per trade
- Stop loss: 2% adverse move

## Results

| Metric | Value |
|---|---|
| Total trades | 342 |
| Win rate | 58% |
| Avg win | +0.82% |
| Avg loss | -0.61% |
| Sharpe (ann.) | 1.4 |
| Max drawdown | 12% |
| Profit factor | 1.65 |

Strategy passes all go-live criteria.
