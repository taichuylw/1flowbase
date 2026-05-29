---
memory_type: tool
topic: local-api-server-detach-and-env-json
summary: 本地启动 api-server / plugin-runner 时，普通 nohup 后台进程会随工具会话退出被清理；api-server .env 中 JSON 值不能用 shell source，需用 setsid -f 脱离会话并显式传递 JSON env。
keywords:
  - shell
  - setsid
  - api-server
  - plugin-runner
  - API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON
match_when:
  - 本地 api-server 或 plugin-runner 用 nohup 后台启动后立刻消失、端口没有监听
  - source api/apps/api-server/.env 后启动报 JSON parse error 或 trusted public keys JSON 被破坏
created_at: 2026-05-28 23
updated_at: 2026-05-28 23
last_verified_at: 2026-05-28 23
decision_policy: reference_on_failure
scope:
  - shell
  - api/apps/api-server/.env
  - api/apps/api-server
  - api/apps/plugin-runner
---

# Local API Server Detach And Env JSON

## 时间

`2026-05-28 23`

## 失败现象

- `nohup env ... cargo run ... &` 返回后，`api-server` / `plugin-runner` 没有持续运行，`ss -ltnp` 看不到 `7800/7801`。
- 直接 `source api/apps/api-server/.env` 再启动 `api-server` 会把 `API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON=[{"key_id":...}]` 中的 JSON 引号破坏，启动时报类似 `key must be a string` 的 JSON 解析错误。

## 触发条件

在 Codex 桌面工具会话里需要为真实 smoke 长时间启动本地 `api-server`、`plugin-runner`，并复用 `api/apps/api-server/.env` 的本地开发配置。

## 根因

- 普通后台子进程仍可能被工具会话收尾逻辑清理。
- shell `source` 按 shell 语义解析未加引号的 `.env` 值，JSON 对象字段名的双引号不会按 dotenv 原样保留。

## 解法

- 用 `setsid -f bash -c 'cd ... && exec env ... cargo run ... > log 2>&1' < /dev/null` 脱离当前工具会话。
- `api-server` 启动时显式传入 env；`API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON` 用单引号包住完整 JSON，不要 `source` `.env`。

## 验证方式

- `ss -ltnp | rg ':7800|:7801'` 能看到 `target/debug/api-server` 和 `target/debug/plugin-runner`。
- `curl -sS -i http://127.0.0.1:7800/health` 和 `curl -sS -i http://127.0.0.1:7801/health` 返回 `200 OK`。

## 复现记录

- `2026-05-28 23`：普通 `nohup` 启动生成 0 字节日志且端口未监听；改用 `setsid -f` 后 `api-server` pid `71929` 监听 `7800`，`plugin-runner` pid `71679` 监听 `7801`。
