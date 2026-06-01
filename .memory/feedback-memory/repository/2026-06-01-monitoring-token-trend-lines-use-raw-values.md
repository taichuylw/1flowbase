---
memory_type: feedback
feedback_category: repository
topic: monitoring-token-trend-lines-use-raw-values
summary: 应用监控 token 趋势图里的总量、输入、输出、缓存命中线必须按原始指标值绘制，不用 ECharts stack 让线条变成累计值。
decision_policy: direct_reference
tags:
  - application-monitoring
  - echarts
  - token-usage
  - runtime-observability
---

# Monitoring Token Trend Lines Use Raw Values

用户在 `2026-06-01 08` 纠正：应用监控 token 趋势图不能因为 ECharts `stack` 把输出 tokens 等线条画成累计高度，导致输入/输出明明差异很大但视觉上贴得很近。

适用场景：调整 `/applications/:id/monitoring` 的 token 趋势、ECharts 折线/面积图、运行日志 token 指标可视化。

规则：

- 总 tokens、输入 tokens、输出 tokens、命中缓存 tokens 的折线位置必须表示接口返回的原始值。
- 如需面积视觉效果，可以用非堆叠 area line；不要用 `stack` 展示这些原始指标。
- tooltip 和 legend 名称沿用后端 DTO / 领域语义对应的字段，不在前端伪造或改名。
