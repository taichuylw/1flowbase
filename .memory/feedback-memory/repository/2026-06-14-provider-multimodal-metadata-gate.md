---
memory_type: feedback
feedback_category: repository
topic: provider-multimodal-metadata-gate
summary: 图片链路不是只看 provider wire shape；宿主会先按模型元数据判断是否允许多模态。官方 provider 的 vision 静态模型必须能被宿主解析为 supports_multimodal，新增或修复 provider 后要同步模型 catalog、manifest version 和契约测试。
keywords:
  - model-provider
  - multimodal
  - vision
  - image_llm
  - provider metadata
  - static model catalog
  - supports_multimodal
  - official-plugins
match_when:
  - 排查图片、文件、多模态输入无法进入 provider 的问题
  - 修改官方 model provider 的 models/llm/*.yaml
  - 调整 provider package loader 或 runtime media gate
  - 修复 OpenAI Responses、Gemini 或其他 provider 的 content block 转换
created_at: 2026-06-14 22
updated_at: 2026-06-14 22
last_verified_at: 2026-06-14 22
decision_policy: direct_reference
scope:
  - api/crates/plugin-framework/src/provider_package.rs
  - api/crates/control-plane/src/orchestration_runtime/provider_invoker.rs
  - ../1flowbase-official-plugins/runtime-extensions/model-providers
---

# Provider Multimodal Metadata Gate

## Rule

图片 / 多模态链路要同时检查两层：provider 插件是否把 native content blocks 转成供应商协议，以及宿主 runtime 是否通过模型元数据允许 media 进入 provider。

官方 provider 静态模型如果声明 `capabilities: vision`，宿主必须能把它识别成 `supports_multimodal`；新增或修复 vision 模型时，`models/llm/*.yaml`、`_position.yaml`、provider contract test 和 manifest patch version 要一起更新。缺少静态模型 ID 或缺少 multimodal 元数据，会让 `image_llm` / `visible_internal_llm_tool` 在宿主侧提前降级或拒绝，即使 provider 的 OpenAI Responses / Gemini payload 转换本身是正确的。

## Reason

这次 Gemini 图片失败不是 provider 没有把真实 tool-result 图片块转成 `inlineData`，而是宿主选择的 `gemini-3-flash` 在已安装 Gemini 0.1.8 静态 catalog 中不存在；已有 Gemini vision 模型又只写了 `capabilities: vision`，旧 loader 没把它映射为 `supports_multimodal`。runtime 因此在 provider 前按“非多模态模型”处理图片。

## Applies When

- 排查 `message_media_unsupported`、`model_multimodal_unsupported` 或图片被文本化的问题。
- 给官方 provider 增加新模型、vision 模型或多模态能力。
- 修复 OpenAI Responses `source.data`、Gemini `content_blocks` 等 provider 级图片转换后准备发布插件包。

