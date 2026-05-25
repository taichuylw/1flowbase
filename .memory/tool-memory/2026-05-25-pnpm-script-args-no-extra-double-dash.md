---
memory_type: tool
topic: pnpm filter 运行脚本时不要额外加 `--` 透传 Vite 参数
summary: 在 `pnpm --filter @1flowbase/web dev` 后再加额外 `--` 会让 Vite 收到字面量 `--`，导致 `--port` 等参数没有覆盖配置；正确写法是直接追加 `--host --port --strictPort`。
keywords:
  - pnpm
  - vite
  - script-args
  - style-boundary
match_when:
  - 用 `pnpm --filter @1flowbase/web dev` 启动 Vite 并传端口参数
  - Vite 仍然尝试使用 `web/app/vite.config.ts` 中的 3100
  - 输出里出现 `vite -- --help` 或 `vite -- --port ...`
created_at: 2026-05-25 19
updated_at: 2026-05-25 19
last_verified_at: 2026-05-25 19
decision_policy: reference_on_failure
scope:
  - pnpm --filter @1flowbase/web dev
  - web/app
  - vite
  - scripts/node/check-style-boundary
---

# pnpm filter 运行脚本时不要额外加 `--` 透传 Vite 参数

## 时间

`2026-05-25 19`

## 失败现象

运行：

```bash
pnpm --dir web --filter @1flowbase/web dev -- --help
```

实际执行显示为：

```text
vite -- --help
```

Vite 没有打印帮助，而是尝试按配置启动 dev server，并因为 `3100` 已占用而失败。

## 触发条件

通过 `pnpm --filter @1flowbase/web dev` 运行 `web/app/package.json` 里的 `dev: vite`，同时希望向 Vite 透传 `--host`、`--port`、`--strictPort` 等参数。

## 根因

在这个 pnpm 调用形态下，脚本名后面的参数已经会传给包脚本；再额外写一个 `--` 会成为 Vite 收到的字面量参数，后续 CLI 参数不会按预期覆盖 Vite config。

## 解法

直接把 Vite 参数接在脚本名后：

```bash
pnpm --dir web --filter @1flowbase/web dev --host 127.0.0.1 --port 3198 --strictPort
```

已验证该命令会实际启动在 `http://127.0.0.1:3198/`，不会占用 `3100`。

## 验证方式

- `pnpm --dir web --filter @1flowbase/web dev --help` 输出 Vite help。
- `pnpm --dir web --filter @1flowbase/web dev --host 127.0.0.1 --port 3198 --strictPort` 输出 `Local: http://127.0.0.1:3198/`。

## 复现记录

- `2026-05-25 19`：修复 `check-style-boundary` 临时 Vite host 时验证。
