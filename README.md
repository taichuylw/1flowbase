# 1flowbase

<p align="center">
  <img src="docs/assets/logo_index_cn.png" alt="1flowbase Logo">
</p>

<p align="center">
  <b>English</b> | <a href="docs/READEME-i18n/README_CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/taichuy/1flowbase/stargazers"><img src="https://img.shields.io/github/stars/taichuy/1flowbase?style=social" alt="GitHub stars"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/taichuy/1flowbase" alt="License"></a>
  <img src="https://img.shields.io/badge/OpenAI-compatible-111827" alt="OpenAI compatible">
  <img src="https://img.shields.io/badge/Claude-compatible-111827" alt="Claude compatible">
  <img src="https://img.shields.io/badge/self--hosted-1flowbase-2563eb" alt="Self-hosted">
</p>

<p align="center">
  <strong>Community:</strong>
  <a href="docs/assets/community/wechat.jpg" target="_blank">WeChat</a> |
  <a href="docs/assets/community/taichuy_doc_wechat_office.png" target="_blank">WeChat Official Account</a> |
  <a href="https://x.com/Tacihu2021" target="_blank">Twitter</a>
</p>

> **Open-source virtual model gateway for Claude Code, Codex, OpenCode, Cline, Continue, and other local AI agent clients.**

1flowbase lets you build multi-model workflows, publish them as OpenAI-compatible or Claude-compatible model endpoints, and inspect the execution trace behind every request.

Use it when a normal LLM gateway or model router is not enough:

- give text-first coding models vision by mounting a multimodal model as a tool
- publish Fusion-style multi-model review panels as one reusable model endpoint
- turn model chains, tools, verifiers, fallbacks, and formatters into a virtual model
- call that virtual model from Claude Code, Codex, OpenCode, Cline, Continue, SDKs, or any client that supports custom model endpoints
- debug node inputs, outputs, tool callbacks, tokens, latency, failures, and cost instead of only seeing the final answer

```text
Build workflow -> Publish virtual model -> Call from agent clients -> Inspect trace -> Optimize
```

> LiteLLM, Portkey, Bifrost, and similar gateways help route model traffic.
> 1flowbase helps compose several models and tools into a new observable virtual model endpoint.

---

## What You Can Build Now

### Claude Code vision for GLM-5.2, DeepSeek, and other text-first coding models

Claude Code and other coding agents can receive screenshots, UI images, charts, and design references. Some strong coding models are still best used as text-first models in those client paths.

1flowbase can intercept the image, call a vision model as a mounted tool, return structured visual context to the main coding model, and keep the whole trace visible.

```text
Claude Code
  -> 1flowbase virtual model endpoint
  -> GLM-5.2 / DeepSeek / other main coding model
  -> mounted vision tool
  -> GLM-5V-Turbo / Gemini / GPT vision / OCR model
  -> structured visual result
  -> final coding answer
```

Guide: [Make GLM-5.2 See Images in Claude Code with 1flowbase](https://github.com/taichuy/1flowbase/wiki/Make-GLM-5.2-See-Images-in-Claude-Code-with-1flowbase)

### Fusion-style multi-model review endpoint

OpenRouter Fusion made a useful pattern obvious: the next valuable model endpoint may be a compound workflow, not one larger model.

1flowbase includes a `fusion` template that turns several branch models and one synthesis model into a publishable endpoint. Your agent client calls one model name; 1flowbase runs the model panel behind it and records every branch, token count, failure, and synthesis step.

```text
User request
  -> Main LLM
  -> fusion tool
     -> Branch LLM A
     -> Branch LLM B
     -> Branch LLM C
     -> Synthesis LLM
  -> final answer
```

Guide: [Fusion-Style Workflows: Publish a Multi-Model Panel as an Observable Virtual Model](https://github.com/taichuy/1flowbase/wiki/Fusion-Style-Workflow)

### Workflow-backed model APIs

Build once, then expose the workflow through common model APIs:

| Protocol | API path | Typical usage |
|---|---:|---|
| OpenAI Responses API | `/v1/responses` | newer OpenAI-style clients and application code |
| OpenAI Chat Completions API | `/v1/chat/completions` | SDKs, coding tools, chat clients, application frameworks |
| Claude-compatible Messages API | `/v1/messages` | Claude-compatible clients that support custom endpoints |

---

## Why 1flowbase

Many AI tools only show the final response. A real agent request usually contains far more than the visible user message:

```text
user input + system prompt + developer prompt + tool definitions + project context
+ chat history + command outputs + image/file references + intermediate model calls
+ verifier steps + formatter steps + fallback calls
```

That hidden path controls quality, cost, latency, failure rate, and whether the agent can be trusted on long engineering tasks.

1flowbase makes the path visible and programmable:

- **Compose** multiple models, tools, verifiers, routers, and formatters
- **Publish** the workflow as a normal OpenAI-compatible or Claude-compatible model
- **Observe** node inputs, outputs, tool callbacks, tokens, latency, errors, and cost
- **Optimize** expensive requests with model cascading, selective vision calls, fallbacks, and review panels
- **Reuse** working workflows as named virtual models for local agent clients and application code

---

## Quick Start

### One-command Docker bootstrap

The script checks whether Docker / Compose is available, pulls the `docker/` directory, and copies `docker/.env.example` to `docker/.env`.

```bash
curl -fsSL https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/shell/docker-deploy.sh | sh
```

PowerShell:

```powershell
irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex
```

Windows CMD:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex"
```

### Run from source

Requirements: Node.js `>= 24.0.0`, latest stable Rust, and Docker for local middleware.

```bash
git clone https://github.com/taichuy/1flowbase.git
cd 1flowbase

docker compose -f docker/docker-compose.middleware.yaml up -d

cd web
pnpm install
pnpm dev
```

Frontend:

```text
http://127.0.0.1:3100
```

Start backend services:

```bash
cd api
# Copy api/apps/api-server/.env.example to .env before the first run.
cargo run -p api-server --bin api-server
cargo run -p plugin-runner --bin plugin-runner
```

Default backend endpoints:

```text
API Server: http://127.0.0.1:7800
Plugin Runner: http://127.0.0.1:7801
```

Script-assisted startup:

```bash
node scripts/node/dev-up.js
node scripts/node/dev-up.js status
node scripts/node/dev-up.js stop
node scripts/node/dev-up.js restart
```

See [scripts/README.md](scripts/README.md) for more options.

### Ask a coding agent to set it up

```text
Clone https://github.com/taichuy/1flowbase, follow the README quick start, then help me publish a workflow as an OpenAI-compatible or Claude-compatible virtual model endpoint.
```

---

## Feature Preview

### Build multi-model workflows

Create workflows with multiple models, tools, verifiers, branch models, and formatter nodes.

![Workflow Editor Preview](docs/assets/workflow_editor_preview.jpeg)

### Publish as OpenAI-compatible API

![Publish OpenAI API](docs/assets/api_endpoint_publish_1.jpeg)

### Publish as Claude-compatible Messages API

![Publish Claude API](docs/assets/api_endpoint_publish_2.jpeg)

### Customize exposed model information

![Custom Model Settings](docs/assets/custom_model_settings.jpeg)

### Use in local AI agent clients

Call a published workflow from compatible clients that support custom model endpoints.

![Claude Code Terminal Usage](docs/assets/claude_code_terminal_usage.png)

### Inspect execution logs

Trace model requests, node inputs and outputs, tool callbacks, response content, latency, and errors.

![Detailed Execution Logs](docs/assets/detailed_execution_logs.jpeg)

### View tool callback traces

![Tool Callback Trace Logs](docs/assets/tool_callback_trace_logs.png)

### Track token consumption

![Token Consumption Dashboard](docs/assets/token_consumption_dashboard.jpeg)

---

## Common Use Cases

### Make a text coding model understand screenshots

```text
Screenshot / UI mockup / chart
  -> vision tool
  -> structured visual context
  -> strong coding model
  -> patch, plan, or explanation
```

Useful for UI reconstruction, frontend debugging, visual regression analysis, chart reading, PDF page understanding, and design-to-code workflows.

### Build a Fusion-style reviewer

```text
Architecture proposal
  -> cheap broad reviewer
  -> strong reasoning reviewer
  -> provider-diverse reviewer
  -> synthesis model
  -> final recommendation
```

Useful for architecture review, research synthesis, code review, document review, and high-stakes agent decisions.

### Control cost with model cascading

```text
Simple classification -> small model
Formatting -> small model
Complex reasoning -> strong model
Final verification -> verifier node
```

### Guarantee output structure

Use verifiers, JSON Schema validation, and formatter nodes before returning the final result. This is useful for JSON outputs, API responses, tool call parameters, code patches, document generation, and automated task results.

### Build a programmable upstream model for agents

```text
Code generation -> test / lint check -> reviewer node -> fix node -> final patch
```

The client calls one model name while 1flowbase runs your workflow behind it.

---

## How 1flowbase Differs

1flowbase is not only a model proxy and not only a generic workflow canvas.

It focuses on one gap:

> Build a multi-model workflow, publish it as a standard model endpoint, and inspect the execution behind it.

| Tool category | What it usually does | How 1flowbase is different |
|---|---|---|
| LLM gateway / model router | routes one request to one provider or model | composes multiple model and tool nodes into one workflow-backed virtual model |
| AI workflow builder | builds an AI app or workflow | exposes the workflow as OpenAI / Claude-compatible model APIs |
| Agent framework | helps developers code agent graphs | provides a visual runtime, protocol publishing, and execution logs |
| Observability / cost tracker | shows token or spend totals | connects cost to workflow nodes, model calls, tool callbacks, and trace logs |

```text
Model routers choose a model.
1flowbase builds a new virtual model from a workflow.
```

---

## Current Status

### Implemented

- [x] visual workflow editor
- [x] multi-node workflow orchestration
- [x] virtual model endpoint publishing
- [x] OpenAI Responses protocol support
- [x] OpenAI Chat Completions protocol support
- [x] Claude-compatible Messages protocol support
- [x] streaming response support
- [x] mounted LLM tools for multimodal and branch-model workflows
- [x] `fusion` workflow template
- [x] execution logs
- [x] tool callback traces inside 1flowbase workflows
- [x] application-level token consumption statistics
- [x] prompt and model configuration version history

### Enhancing

- [ ] deeper local agent conversation collection
- [ ] session search and playback
- [ ] Token Bill of Materials by prompt, history, tool definitions, command outputs, media inputs, and nodes
- [ ] abnormal cost detection and optimization suggestions
- [ ] session export and Recall Pack generation
- [ ] more Claude Code / Codex / OpenCode / Cline / Continue templates
- [ ] MCP-aware plugin nodes and tool-call source attribution

### Planned

- [ ] low-code application building platform for AI organizations
- [ ] team workspace and multi-tenant management
- [ ] permissions, approval, audit, and cost governance
- [ ] support for more local AI agent clients
- [ ] template market and workflow recipe ecosystem

> Note: 1flowbase is not currently positioned as an MCP server or MCP gateway. MCP-aware capabilities are on the roadmap. The current product focuses on publishing compatible model endpoints and tracing 1flowbase workflow execution.

---

## Transparency and Security

1flowbase is designed for transparent, self-hosted AI workflow execution.

Recommended principles:

- self-hosted first
- transparent model chains
- auditable node calls
- traceable token usage
- configurable log retention
- sensitive data masking
- explicit model and workflow configuration

1flowbase does not advocate stealthy model replacement. Published endpoints should be configured intentionally, observed clearly, and governed by the project owner.

---

## Guides

- [Make GLM-5.2 See Images in Claude Code with 1flowbase](https://github.com/taichuy/1flowbase/wiki/Make-GLM-5.2-See-Images-in-Claude-Code-with-1flowbase)
- [Fusion-Style Workflows: Publish a Multi-Model Panel as an Observable Virtual Model](https://github.com/taichuy/1flowbase/wiki/Fusion-Style-Workflow)
- [1flowbase Wiki](https://github.com/taichuy/1flowbase/wiki)

---

## Repo Layout

```text
web/          Frontend root, powered by pnpm + Turbo
api/          Rust backend workspace
api/apps/     Backend service entry points
api/crates/   Shared backend crates
api/plugins/  Plugin workspace, HostExtension manifests, and templates
docker/       Local middleware orchestration
scripts/      Development, testing, verification, and debugging scripts
```

---

## Contributing

Contributions are welcome. Before submitting a pull request, run:

```bash
node scripts/node/verify.js repo
```

Project guidelines:

- [AGENTS.md](AGENTS.md)
- [web/AGENTS.md](web/AGENTS.md)
- [api/AGENTS.md](api/AGENTS.md)

---

## Friend Links

- [Linux.do](https://linux.do/) - Learn AI, on L Station.
- [Aionui](https://github.com/iOfficeAI/AionUi) - Remotely control AI to work via mobile phone.
- [OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) - Office suite designed for AI agents.
- [deepseek-pp](https://github.com/zhu1090093659/deepseek-pp) - DeepSeek web chat browser extension.
- [MuseAI](https://github.com/yejiming/MuseAI) - Local AI companion, text adventure, and story immersion app.
- [FrontAgent](https://github.com/FrontAgent/FrontAgent) - AI Agent system designed specifically for front-end engineering.
- [RedBox](https://github.com/Jamailar/RedBox) - Localized AI creative workbench for Xiaohongshu creators.

---

## License

This project is licensed under [Apache-2.0](LICENSE).

---

## Contributors

<p align="center">
  <a href="https://github.com/taichuy/1flowbase/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=taichuy/1flowbase&max=50" alt="Contributors" />
  </a>
</p>

---

## Star History

<p align="center">
  <a href="https://www.star-history.com/#taichuy/1flowbase&Date" target="_blank">
    <img src="https://api.star-history.com/svg?repos=taichuy/1flowbase&type=Date" alt="Star History" width="600">
  </a>
</p>

---

<div align="center">

**If you want local AI agents to call observable multi-model virtual models, give 1flowbase a star.**

[Report Bug](https://github.com/taichuy/1flowbase/issues) · [Request Feature](https://github.com/taichuy/1flowbase/issues)

</div>
