---
memory_type: tool
topic: web/app 的 test package script 当前可接文件参数，但不要追加 Jest-only 选项
summary: 2026-05-06 已验证：`pnpm --dir web/app test src/...test.tsx` 会通过仓库 wrapper 定向执行目标 Vitest 文件；不要追加 Jest 风格的 `--runInBand`，Vitest 3 会报 `Unknown option --runInBand`。
keywords:
  - bash
  - pnpm
  - vitest
  - package-script
  - targeted-test
  - web/app
  - runInBand
match_when:
  - 需要在 `web/app` 里只跑单个或少量 Vitest 文件
  - 想给前端 Vitest 追加 `--runInBand`
  - 看到 Vitest 报 `Unknown option --runInBand`
created_at: 2026-04-14 15
updated_at: 2026-05-06 15
last_verified_at: 2026-05-06 15
decision_policy: reference_on_failure
scope:
  - bash
  - pnpm
  - vitest
  - web/app
  - web/app/package.json
---

# web/app 的 test package script 当前可接文件参数，但不要追加 Jest-only 选项

## 当前验证结果

`2026-05-06 15` 执行：

```bash
pnpm --dir web/app test src/features/settings/_tests/data-models-page.test.tsx
```

实际只运行目标文件，结果为：

```text
src/features/settings/_tests/data-models-page.test.tsx (8 tests)
Test Files  1 passed (1)
Tests  8 passed (8)
```

## 失败现象

执行：

```bash
pnpm --dir web/app test src/features/settings/_tests/data-models-page.test.tsx --runInBand
```

Vitest 3 报错：

```text
CACError: Unknown option `--runInBand`
```

## 根因

`--runInBand` 是 Jest 风格参数，不是当前 Vitest CLI 支持的选项。当前仓库的 `web/app` test 脚本已经走 `run-frontend-vitest.js run`，可以直接把目标测试文件路径追加到脚本后面。

## 已验证解法

定向跑单个前端测试文件时使用：

```bash
pnpm --dir web/app test src/features/settings/_tests/data-models-page.test.tsx
```

不要追加 `--runInBand`。如果需要调 Vitest 自身并发/池参数，先查当前 Vitest 支持的参数，再通过仓库 wrapper 或 `pnpm --dir web/app exec vitest ...` 做最小验证。
