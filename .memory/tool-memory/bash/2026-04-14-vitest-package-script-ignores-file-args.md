---
memory_type: tool
topic: web/app 的 test package script 不会按文件参数收窄 Vitest 范围
summary: 在当前仓库执行 `pnpm --dir web/app test -- <file...>` 会回落到整套测试脚本，无法稳定只跑目标文件；当前已验证可复用做法是走仓库 wrapper：`scripts/node/exec-with-real-node.sh scripts/node/run-frontend-vitest.js run <file...>`。
keywords:
  - bash
  - pnpm
  - vitest
  - package-script
  - targeted-test
  - web/app
match_when:
  - 需要在 `web/app` 里只跑单个或少量 Vitest 文件
  - 使用 `pnpm --dir web/app test -- <file>` 后发现仍然执行全套测试
  - 需要精确定向验证前端改动
created_at: 2026-04-14 15
updated_at: 2026-05-06 14
last_verified_at: 2026-05-06 14
decision_policy: reference_on_failure
scope:
  - bash
  - pnpm
  - vitest
  - web/app
  - web/package.json
---

# web/app 的 test package script 不会按文件参数收窄 Vitest 范围

## 时间

`2026-04-14 15`

## 失败现象

执行 `pnpm --dir web/app test -- src/...test.tsx` 后，命令没有只跑目标文件，而是继续执行整个前端测试集。

## 触发条件

在实现计划任务时，按常见 pnpm 透传参数习惯，直接把测试文件路径追加到 `web/app` 的 `test` package script 后面。

## 根因

当前仓库里 `web/app` 的 `test` script 行为不能稳定把文件参数透传为 Vitest 的目标文件过滤条件，因此会落回默认测试入口。

## 解法

需要定向执行 Vitest 时，按当前 `web/AGENTS.md` 的资源限制要求走仓库 wrapper，并把文件路径作为 pass-through 参数传给 `run-frontend-vitest.js`：

```bash
scripts/node/exec-with-real-node.sh scripts/node/run-frontend-vitest.js run src/features/agent-flow/_tests/templated-text-field.test.tsx
```

## 验证方式

使用 `pnpm --dir web/app exec vitest run src/routes/_tests/route-config.test.ts src/routes/_tests/section-shell-routing.test.tsx` 后，只执行目标测试文件并得到预期结果。

## 复现记录

- `2026-04-14 15`：实现共享壳计划时，`pnpm --dir web/app test -- <file...>` 意外跑了整套测试；改用 `pnpm --dir web/app exec vitest run <file...>` 后定向验证恢复正常。
- `2026-04-16 07`：调整 `agent-flow` 节点检查器时，执行 `pnpm --dir web/app test -- node-inspector.test.tsx` 仍然触发整套 `web/app` 测试；继续改用 `pnpm --dir web/app exec vitest run src/features/agent-flow/_tests/node-inspector.test.tsx ...` 后，成功只跑目标文件。
- `2026-05-05 17`：排查变量 picker 时，`pnpm --dir web/app test -- src/features/agent-flow/_tests/templated-text-field.test.tsx` 再次拉起无关测试；改用 `scripts/node/exec-with-real-node.sh scripts/node/run-frontend-vitest.js run src/features/agent-flow/_tests/templated-text-field.test.tsx` 后，只执行目标文件，并继续遵守仓库测试资源 wrapper。
- `2026-05-06 14`：合并 `latest` action 优化时，`pnpm --dir web/app test -- src/features/agent-flow/_tests/api/data-model-options.test.ts ...` 仍拉起 72 个文件的 fast suite；改用 `node scripts/node/run-frontend-vitest.js run src/features/agent-flow/_tests/api/data-model-options.test.ts ...` 后，只执行 8 个目标文件，53 个测试通过。
