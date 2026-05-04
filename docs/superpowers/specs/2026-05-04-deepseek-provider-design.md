# DeepSeek Provider Design

Date: 2026-05-04

## Context

The official plugin source repository is `/home/taichu/git/1flowbase-official-plugins`.
`api/plugins/installed/` is an installed artifact area and must not be used as the source entry.

The current official provider set has only `openai_compatible`. The host provider stdio contract currently exposes `validate`, `list_models`, and `invoke`; it has no first-class balance method. The DeepSeek provider therefore needs two coordinated changes:

- add a dedicated `deepseek` runtime model provider plugin in the official plugins repository;
- extend the main 1flowbase provider runtime contract and console API with a provider balance capability.

## External API Facts

Sources:

- DeepSeek Chat Completions: `https://api-docs.deepseek.com/zh-cn/api/create-chat-completion`
- DeepSeek Models: `https://api-docs.deepseek.com/zh-cn/api/list-models`
- DeepSeek Balance: `https://api-docs.deepseek.com/zh-cn/api/get-user-balance`
- DeepSeek Pricing: `https://api-docs.deepseek.com/zh-cn/quick_start/pricing`
- DeepSeek Context Cache: `https://api-docs.deepseek.com/zh-cn/guides/kv_cache`
- DeepSeek Thinking Mode: `https://api-docs.deepseek.com/zh-cn/guides/thinking_mode`

DeepSeek OpenAI-format base URL is `https://api.deepseek.com`.

Required endpoints:

- `POST /chat/completions`
- `GET /models`
- `GET /user/balance`

Current model IDs are:

- `deepseek-v4-flash`
- `deepseek-v4-pro`

DeepSeek V4 supports JSON output, tool calls, thinking mode, 1M context, and max 384K output. DeepSeek usage reports cache hit and miss input tokens as `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens`.

Current published prices are per million tokens:

| Model | Input Cache Hit | Input Cache Miss | Output |
| --- | ---: | ---: | ---: |
| `deepseek-v4-flash` | CNY 0.02 | CNY 1 | CNY 2 |
| `deepseek-v4-pro` | CNY 0.025 | CNY 3 | CNY 6 |

Pricing is time-sensitive. The plugin should store current price metadata with `as_of: 2026-05-04` and doc/source fields, so later price refreshes can be explicit versioned changes.

## Approach

Use a dedicated DeepSeek plugin rather than making users configure `openai_compatible` manually.

This keeps provider identity, icon, localized strings, DeepSeek-specific parameters, static model metadata, price metadata, cache token mapping, and balance support together. It also keeps OpenAI-compatible generic behavior from accumulating provider-specific rules.

## Main Repository Contract Changes

Add a first-class provider balance method through the existing provider runtime path:

- extend `ProviderStdioMethod` with `GetBalance` or `Balance`;
- add `ProviderBalanceInfo` and `ProviderBalanceResult` structs in `plugin-framework`;
- add `ProviderHost::get_balance`;
- add `ProviderRuntimePort::get_balance`;
- expose an API route under the model provider instance surface, recommended:
  - `GET /api/console/model-providers/{id}/balance`

Balance route behavior:

- requires the same manage-level model provider permission as validate/refresh;
- loads the instance, installation, and decrypted provider config through the existing provider runtime config path;
- calls plugin runtime method `balance`;
- returns `is_available` and `balance_infos`;
- never returns provider secrets.

No billing ledger, persisted cost table, or UI cost dashboard is in scope for this pass.

## DeepSeek Plugin Shape

Create:

`/home/taichu/git/1flowbase-official-plugins/runtime-extensions/model-providers/deepseek`

Expected files:

- `manifest.yaml`
- `Cargo.toml`
- `src/main.rs`
- `src/lib.rs`
- `provider/deepseek.yaml`
- `models/llm/_position.yaml`
- `models/llm/deepseek-v4-flash.yaml`
- `models/llm/deepseek-v4-pro.yaml`
- `i18n/en_US.json`
- `i18n/zh_Hans.json`
- `_assets/icon.svg`
- `readme/README_en_US.md`

Provider config fields:

- `api_key`, secret, required
- `base_url`, string, required, default `https://api.deepseek.com`
- `validate_model`, boolean, optional advanced, default true

No organization/project/api-version/default-headers fields are needed for the dedicated plugin.

## Chat Invocation

The host invocation path is streaming-first. The DeepSeek plugin should call DeepSeek with:

- `stream: true`
- `stream_options: { "include_usage": true }`

It should parse Server-Sent Events and emit provider stream events incrementally.

Message handling:

- forward system, user, assistant, and tool messages;
- preserve `tool_call_id` for tool messages when present;
- map text deltas to `TextDelta`;
- map reasoning deltas / `reasoning_content` to `ReasoningDelta`;
- map function tool calls to `ToolCallDelta` and `ToolCallCommit`;
- map terminal finish reasons to the host enum, including unknown handling for `insufficient_system_resource`.

Request parameters exposed by the provider form:

- `thinking_type`: enum `enabled` / `disabled`, sent as `thinking: { "type": value }`
- `reasoning_effort`: enum `high` / `max`
- `temperature`
- `top_p`
- `max_tokens`
- `response_format`: enum `text` / `json_object`, sent as `response_format: { "type": value }`
- `stop`
- `tool_choice`: `none` / `auto` / `required`
- `logprobs`
- `top_logprobs`
- `user_id`

Tools come from the host `tools` array first. A raw `tools` model parameter may still be accepted for compatibility but should not be shown as a normal provider form field.

Deprecated DeepSeek parameters `frequency_penalty` and `presence_penalty` should not be exposed in the dedicated provider UI.

## Usage And Pricing Metadata

DeepSeek usage normalization:

- `prompt_tokens` -> `input_tokens`
- `completion_tokens` -> `output_tokens`
- `total_tokens` -> `total_tokens`
- `completion_tokens_details.reasoning_tokens` or top-level `reasoning_tokens` -> `reasoning_tokens`
- `prompt_cache_hit_tokens` -> `cache_read_tokens`
- `prompt_cache_miss_tokens` remains in `provider_metadata.usage.prompt_cache_miss_tokens`

The host `ProviderUsage` has `cache_write_tokens`, but DeepSeek's miss tokens are not a write-token count. Do not store miss tokens as `cache_write_tokens`; keep them in provider metadata to avoid semantic drift.

Model metadata should include:

- `owned_by`
- `context_window: 1000000`
- `max_output_tokens: 384000`
- capability flags for streaming, tool call, structured output, reasoning
- pricing object:
  - currency `CNY`
  - unit `million_tokens`
  - input cache hit price
  - input cache miss price
  - output price
  - `as_of: 2026-05-04`
  - source URL

Dynamic `/models` results should merge with static metadata so model IDs returned by DeepSeek keep the richer built-in metadata.

## Error Handling

The plugin should map HTTP status and DeepSeek error payloads through the existing provider runtime error normalization:

- 401/403 -> auth failed
- 404 or model-not-found style payload -> model not found
- 429/quota/rate messages -> rate limited
- network/connect/timeout/5xx -> endpoint unreachable when possible
- malformed responses -> provider invalid response

The balance route should return a normal provider runtime error response rather than hiding upstream failures.

## Testing

Tests are required before production implementation.

Official plugin repository:

- unit test config normalization and DeepSeek default base URL;
- unit test chat request body includes `thinking`, `reasoning_effort`, `response_format`, `tool_choice`, `user_id`, tools, and stream usage;
- unit test non-streaming JSON completion parsing if implemented as an internal helper;
- unit test streaming SSE parsing for text, reasoning, tool call, usage, finish;
- unit test DeepSeek usage maps cache hit tokens and preserves miss tokens in metadata;
- unit test `/models` normalization merges static price/capability metadata;
- unit test `/user/balance` normalization.

Main repository:

- plugin-framework contract serialization for the new balance method/result;
- plugin-runner route or host test for `balance`;
- api-server route test for `GET /api/console/model-providers/{id}/balance`;
- control-plane test that balance uses decrypted provider config and does not leak secrets;
- targeted contract tests to ensure existing `validate`, `list_models`, and streaming `invoke` behavior still works.

## Acceptance Evidence

Minimum verification before delivery:

- `cargo test` for the DeepSeek provider crate;
- official plugin script tests that discover both `openai_compatible` and `deepseek`;
- targeted main-repo Rust tests covering provider balance contract, plugin-runner, control-plane/API route;
- package dry-run for DeepSeek provider if the local host packaging CLI and cross target are available;
- no generated warnings/coverage files outside `tmp/test-governance/`.

## Stop Conditions

Stop and ask before implementation continues if any of these are true:

- main-repo provider contract cannot accept a new balance method without changing persisted schema;
- package format rejects the dedicated DeepSeek provider metadata shape;
- DeepSeek docs change model IDs, balance response, or pricing during implementation;
- local environment cannot build provider crates or run targeted tests.
