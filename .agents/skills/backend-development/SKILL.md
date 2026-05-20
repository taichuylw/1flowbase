---
name: backend-development
description: Use for 1flowbase backend work in api/: implementing, fixing, refactoring, reviewing, or planning Rust/Axum APIs, routes, services, repositories, storage adapters, migrations, domain models, state transitions, write paths, module boundaries, permissions, HostExtension/RuntimeExtension boundaries, or core business logic. Also use when a request mentions 后端, 接口, API, Rust, Axum, service, repository, storage, 状态一致性, 数据一致性, 状态机, 写入口, or backend coupling and consistency.
---

# Backend Development

## Overview

后端最容易失控的原因，是把核心规则、外部协议、存储细节和状态改写入口混在一起。本 Skill 用来约束 API 设计、边界切分、状态入口和一致性，减少“一处改动牵一片”的后端腐化。

## When to Use

- 设计或修改接口、动作入口、模块边界
- 调整状态流转、关键模型或写路径
- 评估是否拆 service、handler、repository、adapter
- 发现多个模块都能直接改同一状态
- 需要判断该直接实现、先收敛边界，还是先问人

**不要用于**

- 纯视觉、交互、信息架构设计
- 纯项目事实同步或技术栈介绍

## The Iron Law

稳定核心决定“该不该做”；边界适配层负责“怎么做到”；关键状态只能从清晰唯一入口改变。

## Quick Reference

- 非平凡后端需求如果还没有明确目标、范围、成功标准、关键假设、方案和用户拍板点，先使用 `problem-framing`；本 Skill 承接已收敛的后端设计和实现边界。
- 核心状态机、对外协议、权限策略、插件边界、核心对象定义：先问人
- 先分清稳定核心和边界适配层，再写代码
- 能力边界优先使用能力名，具体实现留在 adapter / repository / driver
- HostExtension 扩展核心业务时只走 `Resource Action Kernel`、声明式 hook、受控 route / worker / migration，不直接改 Core 真值表
- Redis、队列、锁、event bus 等基础设施只作为 HostExtension provider 实现 host contract，不进业务代码直连
- native HostExtension v1 是可信 in-process、restart-scoped；启停升级写 desired state，不设计 Rust 热卸载
- API 输入保持短、平、单动作
- 运行日志节点必须稳定暴露 `input_payload` / `debug_payload` / `output_payload` 三段；接口层不得把节点真实 payload 强制重塑成摘要结构
- 运行日志大字段按字段路径做 preview / artifact，不按整节点 payload 无脑压缩
- 状态必须写清：状态集合、流转规则、动作约束
- 多个模块都能改同一关键状态：立即收口
- Rust 后端实现要用类型表达核心不变量、显式传播错误、封装状态转换，并把阻塞 IO、锁、事务和外部副作用放在清晰边界内
- Rust 后端开发完成前必须按 `references/rust-backend-practices.md` 的 completion self-check 自检；不能保证的项要标为风险或待办
- 涉及可测试行为变化时，先联动 `test-driven-development`；不能走 TDD 时，在交付说明里写明替代验证

## Implementation

- AI-friendly API rules: `references/api-design.md`
- State and consistency review: `references/state-and-consistency.md`
- Rust backend practice rules: `references/rust-backend-practices.md`
- Stable core vs adapter rules: `references/boundary-design.md`
- Local implementation rules: `references/implementation-rules.md`
- Anti-decay patterns: `references/anti-patterns.md`
- Pressure scenarios: `references/examples.md`

## Runtime Node Payload Contract

- 节点运行记录的真值边界是 `input_payload`、`debug_payload`、`output_payload`；展示摘要只能作为独立 view / display metadata，不能替代或改写节点语义。
- `input_payload` 记录节点实际消费的输入；`debug_payload` 记录数据处理、变量变更、console logs、外部调用过程证据；`output_payload` 记录节点真实产出。
- Start 节点输入必须完整保留 `query`、`model`、`files`、`sys`、`env`；`env` 不脱敏，`files` 先保留完整数组。
- Start 节点的 `history`、`tools` 属于明显可能超长字段，允许字段级 preview / artifact；不能因为它们超长而把整个 Start 输入压成 `start_input_summary`。
- 其他节点同理：短标量、已解析配置、实际消费值和状态快照优先保留真值；大数组、大文本、raw provider response、长 history / tools / console logs 使用字段级 artifact wrapper。
- 字段级 artifact 必须保留原字段路径、预览值、`artifact_ref`、大小信息和是否截断；加载完整值时恢复到原字段位置。
- 查询接口可以返回额外 `payload_view` 或 display metadata，但前端默认展示的三段结构必须仍能反映节点真实输入、数据处理和输出。
- 测试要覆盖：Start 真值字段不丢失、长字段单独截断、非 Start 节点不会被整包摘要、artifact 加载能恢复字段完整值。

## Common Mistakes

- 业务规则直接依赖外部协议格式
- 多个入口同时写同一核心状态
- 一个接口塞进多个动作语义
- 为了“一次查全”造出深层嵌套结构
- 用隐式副作用完成状态变化
- 用整包 preview / summary 替代节点运行 payload，导致历史日志无法说明节点实际消费、处理和产出
