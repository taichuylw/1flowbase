---
created_at: 2026-06-08 19
memory_type: project
decision_policy: verify_before_decision
scope: qa issue 804 remediation
source_issue: "#804"
---

# QA Issue 804 Remediation

用户在 2026-06-08 认可 QA 体检建议，并要求挂到 GitHub issue 后直接开工；本轮实现边界以 #804 为主，前端/CI 子任务另挂 #805。

已确认修复方向：修复 debug continuation 取消竞态、published public API idempotency、plugin runtime 锁跨 await、HostExtension pre-state infrastructure boot、HTTP 大二进制 response-as-file、前端 ApplicationRunSummary 字段契约和 GitHub quality-gate/docs 对齐。

Provider / upstream runtime error 原样进入 RuntimeContract / API response 是 passthrough contract，不属于泄漏修复范围。后续 QA 不应要求脱敏、泛化或改写 provider stdout / stderr / upstream error；应检查宿主是否吞掉、改写或损失上游排障信息。

重型 Rust/Postgres consistency、coverage 和仓库级门禁默认放到 GitHub Actions 跑；本地只做聚焦红绿测试、格式检查和轻量 repo hygiene。
