# Log Query Contract Report

扫描日志、trace、ledger、artifact 查询入口的 `scope / time / cursor / limit` contract。

```bash
node scripts/node/tooling.js log-query-contract-report
```

报告输出：

- `tmp/test-governance/log-query-contract-report.json`
- `tmp/test-governance/log-query-contract-report.md`

规则：

- 缺少必需 contract 证据时为 `failed`，CLI 返回非 0。
- 允许豁免，但必须在 `config.json` 中写明 `reason` 和 `removeBy`。
- run-scope/detail 查询可以豁免 time/cursor/limit，但必须说明它不是跨 run 的高增长列表。

## API Contract

- `scope`：高增长列表必须由后端强制带上 `workspace_id`、`application_id`、`flow_run_id`、`conversation_id`、`parent_trace_node_id` 或 artifact primary key 中的明确范围；前端筛选不能替代后端 scope。
- `time`：跨 run 的 application 级日志/监控查询默认使用 7 天窗口；`time_range_days <= 0` 按默认窗口处理，不能解释为全量。
- `cursor`：run 内事件流使用 exclusive `from_sequence`/`after_stream_sequence`，响应中的 `next_sequence` 可作为下一页 cursor；trace children 使用 opaque cursor，方向是 `order_key asc, trace_node_id asc`。
- `limit`：run logs `page_size` 最大 100，trace children 最大 100，JSON debug stream `limit` 默认 500 最大 1000，artifact batch resolve 最大 50 个 refs。
- `page/page_size` 旧式 offset 只在同时具备 application scope、默认 time window、最大 page_size 和唯一 tie-breaker order 时豁免为 existing contract。

## Related Reports

- `schema-hygiene` 负责物理表 schema 红线。
- `growth-table-report` 负责高增长表 routing columns / index 风险。
- `raw-jsonb-report` 负责 summary / preview / raw 读取边界。
- 本报告负责 API / repository 查询是否避免无界读取。
