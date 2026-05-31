---
memory_type: tool
tool: bash
topic: api-server-dotenv-json-source
summary: Sourcing api/apps/api-server/.env in bash can strip JSON quotes from API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON and make api-server fail at startup.
keywords:
  - api-server
  - .env
  - source
  - API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON
  - key must be a string
created_at: 2026-05-31 10
updated_at: 2026-05-31 10
last_verified_at: 2026-05-31 10
decision_policy: reference_on_failure
---

# API Server Dotenv JSON Source

2026-05-31 本地临时启动 `target/debug/api-server` 时，直接 `set -a; source apps/api-server/.env; set +a` 会让 `API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON=[{"key_id":...}]` 里的 JSON 双引号被 shell 解释掉，启动失败：

```text
Error: key must be a string at line 1 column 3
```

已验证处理办法：临时 smoke 不需要 trusted keys 时，显式传 `API_OFFICIAL_PLUGIN_TRUSTED_PUBLIC_KEYS_JSON='[]'`；需要真实 trusted keys 时，不要让 bash `source` 原样解释该 JSON，改用能保留原始值的 env loader 或手工单引号包裹后的导出。
