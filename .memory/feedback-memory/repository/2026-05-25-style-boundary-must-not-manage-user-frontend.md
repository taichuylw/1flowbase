---
memory_type: feedback
feedback_category: repository
topic: style-boundary 脚本不得接管用户前端 3100
summary: `check-style-boundary` 可以探测 3100，但不得启动、停止、重启或清理用户正在使用的 3100 前端；需要 fallback 时只能使用脚本自己的临时端口并只清理自己的进程。
keywords:
  - style-boundary
  - dev-up
  - frontend-port
  - 3100
match_when:
  - 调整 `scripts/node/check-style-boundary`
  - 运行或修复前端样式边界脚本
  - 脚本需要自动启动前端 host
created_at: 2026-05-25 19
updated_at: 2026-05-25 19
last_verified_at: 2026-05-25 19
decision_policy: direct_reference
scope:
  - scripts/node/check-style-boundary
  - scripts/node/dev-up
  - web/app
---

# style-boundary 脚本不得接管用户前端 3100

## 时间

`2026-05-25 19`

## 规则

`check-style-boundary` 可以把 `http://127.0.0.1:3100` 当作已有用户前端进行只读探测，但不得通过 `dev-up` 或端口清理逻辑启动、停止、重启或杀掉 3100。

需要自动 fallback 时，脚本必须启动独立临时端口，例如 `3101+` 或 `STYLE_BOUNDARY_PORT` 指定的非 3100 端口，并且最终只清理自己 spawn 出来的临时进程。

## 原因

用户会在 3100 上手动验收前端页面。样式边界脚本如果接管 `dev-up` 的 3100 生命周期，会让正在验收的浏览器页面断开。

## 适用场景

- 修改 `scripts/node/check-style-boundary` 的 host 探测、fallback 或 cleanup 行为。
- 在 QA / style-boundary / runtime gate 中增加自动启动前端的逻辑。
- 调整 `dev-up` 与前端端口管理关系时。

## 备注

本轮已把 `check-style-boundary` 从 `dev-up ensure --frontend-only` fallback 改为独立临时 Vite host，并补测试锁定 `STYLE_BOUNDARY_PORT=3100` 禁用。
