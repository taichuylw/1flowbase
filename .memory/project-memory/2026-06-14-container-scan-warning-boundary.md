---
memory_type: project
topic: 容器扫描 warning 暂不作为当前质量修复 blocker
summary: 用户于 2026-06-14 14 明确：容器扫描出的 warning 当前不用处理，后续随基础镜像升级修复；QA 比较 beta/dev 或质量门禁结果时不要把这类 warning 作为本轮功能影响或 blocker。
keywords:
  - container
  - Trivy
  - image
  - quality-gate
  - warning
match_when:
  - 评估容器镜像扫描、Trivy、GHCR、Dockerfile 或 container-images gate warning
  - 比较 beta/dev 质量门禁修复是否影响功能
  - 输出 QA 风险时需要区分 blocker 与已知 warning
created_at: 2026-06-14 14
updated_at: 2026-06-14 14
last_verified_at: 2026-06-14 14
decision_policy: verify_before_decision
scope:
  - docker
  - .github/workflows
  - scripts/node/github-quality-gate
  - scripts/node/container-image-security
---

# 容器扫描 warning 暂不作为当前质量修复 blocker

## 时间

`2026-06-14 14`

## 谁在做什么

用户要求比较 `dev` 与 `beta` 分支时，明确说明容器扫描出来的 warning 当前不用管，后续会随着基础镜像升级修复。

## 为什么这样做

本轮目标是确认 `beta` 上质量门禁修复是否影响 `dev` 已完成的完整功能。容器基础镜像漏洞 warning 属于已知基础镜像升级项，不应混入本轮功能回归判断。

## 为什么要做

后续 QA 或分支比较遇到 container-images / Trivy warning 时，应先按已知 warning 归类，只在出现确定的发布阻断、镜像构建失败、运行时不可用或用户重新要求处理时再升级为当前修复项。
