# Agent Flow Runtime Node Payload Contract

## Read When

- 调整 Agent Flow 运行日志、debug artifact、节点输入/数据处理/输出接口。
- 修改 Start 节点 `sys/env/history/tools` 展示、存储或完整值加载。
- 修改 `flow_runs`、`node_runs`、run events 的 payload offload / preview 规则。

## Goal

节点运行记录稳定表达三段真值：

- `input_payload`：节点实际消费的输入。
- `debug_payload`：数据处理、变量变更、console logs、外部调用过程证据。
- `output_payload`：节点真实产出。

展示摘要、列表预览、artifact metadata 只能作为独立 view / display metadata，不能替代或重塑节点真值。

## Hard Rules

- 禁止按整节点 payload 无脑压缩；长内容按字段路径做 preview / artifact。
- Start 节点输入必须完整保留 `query`、`model`、`files`、`sys`、`env`。
- `env` 不脱敏；`files` 先保留完整数组。
- Start 节点的 `history`、`tools` 可字段级 preview / artifact，不能因此把整个 Start 输入压成 `start_input_summary`。
- 其他节点同理：短标量、已解析配置、实际消费值和状态快照优先保留真值。
- 大数组、大文本、raw provider response、长 history / tools / console logs 使用字段级 artifact wrapper。
- 字段级 artifact 必须保留原字段路径、预览值、`artifact_ref`、大小信息和是否截断。
- 加载完整值时必须恢复到原字段位置。

## Acceptance Evidence

- Start 真值字段不丢失：`query`、`model`、`files`、`sys`、`env` 仍在 `input_payload`。
- 长字段单独截断：`history` / `tools` 可被 artifact 化，但不改变相邻真值字段。
- 非 Start 节点不会被整包摘要。
- Artifact 加载能按字段路径恢复完整值。
- 前端默认展示的输入、数据处理、输出三段仍对应节点真实语义。

## Budget

- 优先改 debug artifact / payload view 层，不顺手重构 runtime execution。
- 不为旧的整包 summary 增加兼容分支，除非用户明确要求历史数据兼容。
- 新增 contract 时先补后端测试；前端只做对应展示和完整值加载验证。

## Stop Conditions

- 发现需要迁移旧 payload 或改变公开 API contract，先回到 `problem-framing`。
- 发现 `files` 可能过大需要压缩，先回到用户确认；当前决策是完整保留。
- 发现 `env` 脱敏、安全或权限问题，先回到用户确认；当前决策是不脱敏。
