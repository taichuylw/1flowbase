---
memory_type: project
topic: Vite 8 前端构建链升级已在 dev 实现
summary: 2026-06-14，用户要求把 esbuild high vulnerability 挂到新 issue 并升级前端构建链；#888 已创建，dev 分支提交 73199ad6 升级 vite 到 8.0.16、@vitejs/plugin-react 到 6.0.2，并让 web/app 显式声明 vite/vitest，避免 @vitest/coverage-v8 peer 解析回 vite 6 -> esbuild 0.25。default branch 是 main，因此 GitHub Dependabot #48 仍会提示，需 dev -> main 后才会消除 default branch alert。
keywords:
  - vite
  - plugin-react
  - esbuild
  - dependabot
  - issue-888
  - frontend
  - dev-to-main
match_when:
  - 继续处理 Dependabot #48
  - 继续跟进 #888
  - push 后仍看到 esbuild high vulnerability 提示
  - 需要判断 Vite 8 升级是否已经在 dev 实现
created_at: 2026-06-14 08
updated_at: 2026-06-14 08
last_verified_at: 2026-06-14 08
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/888
  - https://github.com/taichuy/1flowbase/security/dependabot/48
  - web/package.json
  - web/app/package.json
  - web/pnpm-lock.yaml
---

# Vite 8 前端构建链升级已在 dev 实现

## 时间

`2026-06-14 08`

## 谁在做什么

用户要求将 default branch 上既有 `esbuild` high vulnerability 拆到新 issue，并按 Vite/plugin 大版本升级处理。AI 已创建 #888，并在 `dev` 提交 `73199ad6 chore(web): upgrade Vite toolchain`。

## 为什么这样做

`vite@6.4.2` 声明的 `esbuild` 范围无法自然升级到 GitHub 要求的安全版本；直接 override `esbuild` 会绕过 Vite 依赖契约。升级到 `vite@8.0.16` 并同步 `@vitejs/plugin-react@6.0.2` 是更正统的修复路径。

## 当前状态

- `web/package.json` 已升级 `vite` 和 `@vitejs/plugin-react`。
- `web/app/package.json` 已显式声明 `vite` / `vitest`，避免 `@vitest/coverage-v8` peer 自动解析出旧 `vite@6.4.2 -> esbuild@0.25.12`。
- `pnpm --dir web why esbuild` 无输出；`web/pnpm-lock.yaml` 中无 `vite@6.4.2` / `esbuild@0.25`。
- `pnpm --dir web build` 通过；`vite-config.test.ts` 定向测试通过；`git diff --check` 通过。
- `pnpm --dir web test:fast` 本地执行约 8 分钟未收束，已中断，不能作为通过证据。
- 仓库 default branch 是 `main`，当前修复只推到 `dev`；Dependabot #48 需后续 `dev -> main` 后才会作用于 default branch。
