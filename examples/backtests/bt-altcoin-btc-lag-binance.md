---
title: "Altcoin BTC Lag Trade — Binance Backtest"
strategy: altcoin-btc-lag-trade.md
exchange: Binance
period_start: "2020-01-01"
period_end: "2024-12-31"
total_trades: 187
win_rate: 0.44
sharpe: 0.6
max_drawdown: 0.18
status: inconclusive
created: "2026-04-05"
---

## Summary

Backtested the altcoin BTC lag trade on Binance. Moderate results:
win rate below target but positive expectancy. Edge appears to be
decaying in 2023-2024 sub-period.

## Configuration

- Universe: Top 10 alts by market cap (quarterly rebalance)
- BTC trigger: 4h candle move > 3%
- Entry: open of next 4h candle
- Exit: 80% catch-up target or 12h time stop
- Stop loss: 2%

## Results

| Metric | Value |
|---|---|
| Total trades | 187 |
| Win rate | 44% |
| Avg win | +1.1% |
| Avg loss | -0.8% |
| Sharpe (ann.) | 0.6 |
| Max drawdown | 18% |
| 2020-2022 Sharpe | 0.9 |
| 2023-2024 Sharpe | 0.2 |

Edge is decaying. 2023-2024 sub-period Sharpe is below 0.5 threshold.
Strategy classified as inconclusive — needs parameter re-tuning or kill decision.
