---
memory_type: project
topic: 插件 minimum_host_version 采用软拦截与显式风险确认
summary: 用户于 `2026-06-25 22` 确认：官方插件的 `minimum_host_version` 表示最低适配宿主版本；低于该版本时不硬禁止安装/更新，而是后端默认返回风险拦截，用户显式确认后通过 `compatibility_override` 继续，并把 override 写入任务详情、审计和安装 metadata。
keywords:
  - official-plugin
  - minimum-host-version
  - compatibility-override
  - soft-block
  - plugin-install
match_when:
  - 调整官方插件安装或更新兼容性策略
  - 修改 official registry schema 或 settings 插件安装页
  - 排查低宿主版本仍能安装官方插件的原因
created_at: 2026-06-25 22
updated_at: 2026-06-25 22
last_verified_at: 2026-06-25 22
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane/src/plugin_management
  - api/apps/api-server/src/routes/plugins_and_models/plugins.rs
  - web/app/src/features/settings/components/model-providers/OfficialPluginInstallPanel.tsx
  - ../1flowbase-official-plugins/official-registry.json
---

# 插件 minimum_host_version 采用软拦截与显式风险确认

## Decision

`minimum_host_version` 是官方插件声明的最低宿主适配版本。宿主版本低于它时，系统状态使用 `below_minimum_host_version`，安装/更新默认风险拦截，但允许用户在确认风险后继续。

## Contract

安装和更新接口不使用弱语义 `force: true`，而是接收：

```json
{
  "compatibility_override": {
    "reason": "below_minimum_host_version",
    "acknowledged_current_host_version": "0.2.0",
    "acknowledged_minimum_host_version": "0.3.0"
  }
}
```

后端必须校验 override 的 reason 与用户确认的版本值匹配当前风险；匹配后允许继续，并把 override 写入 task detail、audit detail 和 installation metadata。

## Motivation

当前插件生态仍处早期，硬禁止会阻断用户调试和生态试用；软拦截能保留用户选择权，同时让风险不再静默，后续排查“为什么低版本宿主装了高版本插件”也有审计证据。
