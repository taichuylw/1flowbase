---
memory_type: reference
topic: API 查看相关引用
summary: 记录本项目 OpenAPI JSON、文档页和控制面健康检查入口，供调试和查看接口时快速定位。
keywords:
  - api
  - openapi
  - docs
  - health
match_when:
  - 需要查看 OpenAPI 文档
  - 需要访问控制面健康检查入口
created_at: 2026-04-12 19
updated_at: 2026-05-28 11
last_verified_at: 2026-05-28 11
decision_policy: index_only
scope:
  - api-server
  - openapi.json
  - /docs
  - /api/console/health
  - https://help.aliyun.com/zh/model-studio/qwen-api-via-openai-chat-completions
---

# API 查看相关引用

## 本项目接口查看

- `http://127.0.0.1:7800/openapi.json`
  - 本机访问的 OpenAPI JSON。
- `http://<本机IP>:7800/openapi.json`
  - 本项目 OpenAPI JSON。
- `http://127.0.0.1:7800/docs`
  - 本机访问的 API 文档查看入口。
- `http://<本机IP>:7800/docs`
  - 局域网访问的 API 文档查看入口。
- `http://127.0.0.1:7800/api/console/health`
  - 本机访问的控制面健康检查入口。
- `http://<本机IP>:7800/api/console/health`
  - 局域网访问的控制面健康检查入口。

## 外部兼容协议参考

- `https://help.aliyun.com/zh/model-studio/qwen-api-via-openai-chat-completions`
  - 阿里云百炼 / DashScope OpenAI-compatible Chat Completions 文档入口；排查 Qwen 供应商适配、图片输入、Function Calling 和 `tool_calls` 字段形态时优先核对。

## 使用说明

- 这里只记录接口查看入口、接口参考地址、调试入口。
- 若某个 API 约束已经稳定，应沉淀到正式规格文档，而不是只留在这里。
