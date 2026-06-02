# 1flowbase

<p align="center">
  <img src="docs/assets/logo_index_cn.png" alt="1flowbase Logo">
</p>

<p align="center">
  <b>English</b> | <a href="docs/READEME-i18n/README_CN.md">简体中文</a>
</p>

> **The first step to harness is to see the Agent's execution path clearly**

You might think you just sent a simple `hi`, but the model could have actually received 17K input tokens.

1flowbase is an open-source observability platform and virtual model gateway designed for local AI Agents.

It helps you reconstruct what local Agents like Claude Code, Codex, and aionui do in every single run:

- What prompt was ultimately concatenated for the model
- What tool definitions and system rules were injected
- What historical contexts and project memories were carried over
- What tool calls were executed and command outputs were returned
- How many tokens and how much time each step consumed
- What hidden contexts caused token and cost waste
- Which paths led to failure, loops, or erroneous outputs

Once you can see the Agent's real execution path clearly, you can optimize your AI Harness based on real-world runtime data: compress prompts, split contexts, adjust tool definitions, optimize model routing, add verifiers, and further publish the optimized multi-model chains as OpenAI or Claude-compatible virtual model endpoints.

```text
User Input
  → Agent Harness concatenates Prompts / Tools / History / Memory
  → Model Request
  → Tool Calls
  → Command Outputs
  → Tokens / Cost / Trace
  → Harness Optimization
  → Multi-Model Orchestration
  → Virtual Model Endpoint
```

---

## Have You Encountered These Problems?

### Are you curious about what the Agent actually concatenated for the model?

Local Agents do not simply forward user inputs to the model.

They usually automatically concatenate behind the scenes:

* System Prompts
* Developer Prompts
* Tool definitions
* MCP tool information
* Chat history
* Project context
* File contents
* Command outputs
* Model switching information
* Memory & rules injection

These hidden contexts directly impact model performance, context length, response latency, and token costs.

1flowbase spreads these hidden details out for you to see.

---

### Why did a simple `hi` burn 17K tokens?

What the user sees might only be:

```text
hi
```

But what the model actually receives could be a complete context concatenated by the Agent Harness.

For example:

```text
System Prompt
Tool Definitions
Chat History
Project Memory
Local Command Outputs
File Contents
Model Switching Information
```

Many times, the real token burner is not the user's input, but the massive hidden context automatically concatenated by the Harness behind the scenes.

1flowbase displays the prompt, tool definitions, command outputs, and context structure actually received by the model, tracking input/output tokens, latency, and status for every step.

You no longer just see the final response—you see exactly where the money was spent.

---

### Do you want to know how prompts for Claude Code, Codex, or aionui are concatenated?

1flowbase reconstructs an Agent run into a readable Trace:

```text
User Input
  → Hidden Prompt Injection
  → LLM Request
  → Tool Selection
  → Bash / File / MCP Execution
  → Tool Return
  → Model Continues Generation
  → Final Response
```

You can see the Agent's execution path just like analyzing a backend distributed tracing system.

---

### Do you want to optimize your project Harness based on real observations?

When you know where tokens and failures come from, you can optimize further:

* Which prompts should be compressed
* Which tool definitions should be loaded on demand
* Which historical contexts should be summarized
* Which tasks should be delegated to cheaper small models
* Which steps require a Verifier
* Which outputs require a Formatter
* Which failure paths require a fallback
* Which sessions should be exported as a Recall Pack

The goal of 1flowbase is not to let you tune prompts by gut feeling, but to enable you to optimize your AI Harness based on real execution paths.

---

## Core Progression of 1flowbase

```text
See Path → Explain Cost → Optimize Harness → Combine Models → Publish Endpoint
```

### 1. See Path

Reconstruct the real execution path of every Agent run:

* User Input
* Prompt concatenation
* Tool definition injection
* Historical context inclusion
* Model requests
* Tool calls
* Command outputs
* Final responses

### 2. Explain Cost

Explain why a request was expensive, slow, or failed:

* Which contexts consumed the most tokens
* Which tool definitions were repeatedly injected
* Which historical messages bloated or polluted the model context
* Which command outputs were excessively large
* Which failed tasks caused abnormal costs
* Which steps had the highest latency
* Which model calls were the most expensive

### 3. Optimize Harness

Optimize based on real-world runtime data:

* Prompts
* Tool definitions
* Memory injection
* Historical context length
* Model routing
* Verifiers
* Formatters
* Fallback strategies

### 4. Combine Models

Combine multiple models, tools, and validation nodes into a single reusable workflow:

```text
Vision → Small Model Classification → Strong Model Reasoning → Verifier → Formatter
```

### 5. Publish Endpoint

Publish the optimized workflow as a standard model interface:

* OpenAI Responses API
* OpenAI Chat Completions API
* Claude-compatible Messages API

To the client, it is just a single model.  
To you, it is an observable, tunable, and composable AI workflow.

---

## 🚀 Quick Start

### One-Click Docker Deployment (Recommended)

The following commands will not install Docker. The deployment script checks if a working Docker/Compose environment is available on your machine, pulls the `docker/` directory to the current path, and copies `docker/.env.example` to `docker/.env`.

#### Shell

```bash
curl -fsSL https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/shell/docker-deploy.sh | sh
```

#### PowerShell

```powershell
irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex
```

#### Windows CMD

```bat
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex"
```

---

### Run from Source (Development Mode)

#### Environment Requirements

* **Node.js**: `>= 24.0.0`
* **Rust**: Latest stable compiler version
* **Docker**: For starting middleware required for local development

#### 1. Clone the Repository

```bash
git clone https://github.com/taichuy/1flowbase.git
cd 1flowbase
```

#### 2. Start Middleware

```bash
docker compose -f docker/docker-compose.middleware.yaml up -d
```

#### 3. Start the Frontend

```bash
cd web
pnpm install
pnpm dev
```

Frontend access URL:

```text
http://127.0.0.1:3100
```

#### 4. Start the Backend

Before running for the first time, make sure to copy `api/apps/api-server/.env.example` to `.env` and configure it.

```bash
cd api

# Start the API server
cargo run -p api-server --bin api-server

# Start the plugin runner
cargo run -p plugin-runner --bin plugin-runner
```

Default service endpoints:

```text
API Server: http://127.0.0.1:7800
Plugin Runner: http://127.0.0.1:7801
```

---

### Script-Assisted Startup

To simplify the local development process, the repository provides a unified Node utility script:

```bash
# Fully spin up the frontend, backend, middleware, and plugin runner
node scripts/node/dev-up.js

# Spin up only the frontend and backend processes, skipping Docker middleware
node scripts/node/dev-up.js --skip-docker

# Check the status of each service
node scripts/node/dev-up.js status

# Stop all running local services
node scripts/node/dev-up.js stop

# Restart services
node scripts/node/dev-up.js restart
```

For more detailed script options, please refer to [scripts/README.md](scripts/README.md).

---

## 🖼️ Feature Preview

### 1. Build Multi-Model Workflows

You can build workflows consisting of multiple models, tools, verifiers, and formatter nodes within 1flowbase.

![Workflow Editor Preview](docs/assets/workflow_editor_preview.jpeg)

---

### 2. Publish as OpenAI-Compatible API

Once built, workflows can be published as OpenAI-compatible interfaces.

![Publish OpenAI API](docs/assets/api_endpoint_publish_1.jpeg)

---

### 3. Publish as Claude-Compatible Messages API

The same workflow can also be exposed externally via the Claude-compatible Messages API.

![Publish Claude API](docs/assets/api_endpoint_publish_2.jpeg)

---

### 4. Customize Exposed Model Information

You can customize the model name, description, and capability details exposed to the public.

![Custom Model Settings](docs/assets/custom_model_settings.jpeg)

---

### 5. Use in Clients Like Claude Code

Once published, it can be called in any Claude-compatible client that supports custom endpoints.

![Claude Code Terminal Usage](docs/assets/claude_code_terminal_usage.png)

---

### 6. View Complete Execution Logs

You can inspect the model requests, node inputs/outputs, tool calls, and response content behind an execution.

![Detailed Execution Logs](docs/assets/detailed_execution_logs.jpeg)

---

### 7. View Tool Callback Trace

For complex tasks, 1flowbase displays detailed tool callbacks, command outputs, and execution paths.

![Tool Callback Trace Logs](docs/assets/tool_callback_trace_logs.png)

---

### 8. Track Token Consumption

Track token consumption by application, model, and session to help you understand real costs.

![Token Consumption Dashboard](docs/assets/token_consumption_dashboard.jpeg)

---

## 💡 What is the Agent Execution Path?

Most local Agent clients only display the final conversation results.

But in reality, a typical Agent request contains:

* System Prompt
* Developer Prompt
* Tool Definitions
* Chat History
* Project Memory
* Local Command Outputs
* File Contents
* MCP Tool Returns
* Intermediate Model Responses
* Tool Execution Results

All of these together make up the Agent's execution path.

They directly affect:

* Model performance
* Context length
* Response latency
* Token consumption
* Tool execution results
* Final costs
* Task success rate

1flowbase records these hidden processes and displays them as a Trace:

```text
User Input
  → Hidden Prompt Injection
  → LLM Request
  → Tool Selection
  → Bash / File / MCP Execution
  → Tool Return
  → Model Continues Generation
  → Final Response
```

You can use it to answer questions like:

* Why was this request so expensive?
* Why did a simple input trigger tens of thousands of tokens?
* Which tool execution failed?
* Which historical context bloated or polluted the model?
* Which node was the slowest?
* Which model can be replaced with a cheaper alternative?
* Which session should be compressed, exported, or reused?

---

## 💡 What is the AI Harness?

In 1flowbase, **AI Harness** refers to the entire engineering framework and orchestration outside the model itself.

It includes:

* Prompts
* System Messages
* Tool Definitions
* Tool Calling Policies
* Memory Injection
* Historical Context
* File Context
* Model Routing
* Verifiers
* Formatters
* Cost Control
* Failure Retries
* Fallback Strategies

While the model itself is crucial, commercial AI applications cannot rely solely on the model being "powerful."

For personal use, directly chatting with a premium model might suffice.  
But for commercial projects, every single request incurs token costs.

Without observation, analysis, and optimization, you might encounter:

```text
More Users
  → More Requests
  → More Token Consumption
  → Higher Model Costs
  → If the revenue model doesn't cover costs, growth only widens the loss
```

Therefore, 1flowbase focuses not just on model capabilities, but on the **unit economics** of AI applications:

* Performance
* Cost
* Latency
* Stability
* Traceability
* Reusability

---

## 💬 What is a Virtual Model Endpoint?

A virtual model endpoint acts like a standard LLM API from the outside, but internally executes a complete multi-model workflow.

For example, an external client only sees:

```json
{
  "model": "deepseek-with-vision",
  "messages": [
    {
      "role": "user",
      "content": "Analyze this screenshot and suggest a fix."
    }
  ]
}
```

But inside 1flowbase, it might actually execute:

```text
Gemini Vision
  → Extract images, OCR, UI, and chart context
  → DeepSeek-v4 processes reasoning and generation
  → Verifier Node checks for omissions and formatting
  → Formatter Node constructs an OpenAI-compatible response
```

To the calling client, it is just a single model.  
To you, it is a fully programmable, traceable, and optimizable AI workflow.

---

## ✨ Core Features

### 🧭 Agent Execution Path Reconstruct

Collect and reconstruct the complete execution process of local Agents:

* User Inputs
* Hidden Prompts
* System / Tool / Memory Contexts
* Model Requests and Responses
* Tool Calls and Command Outputs
* File Reads/Writes and Callback Results
* Failure Paths and Exception States

It goes beyond simply saving chats to record exactly what the Agent did at every single step.

---

### 📊 Token & Cost Ledger

Track tokens and costs by application, session, model, and node:

* Total Tokens
* Input Tokens
* Output Tokens
* Tool Call Tokens
* Node Latency
* Failed Run Costs
* Sources of High-Consumption Context

Helping you answer a critical question:

> Why did a user send a very short message, but the model received tens of thousands of tokens of context?

---

### 🔍 End-to-End Trace

A built-in full-link trace tracking system precisely reconstructs the execution path behind every conversation:

* Node Inputs
* Node Outputs
* LLM Requests
* Tool Calls
* Tool Returns
* Command Outputs
* Error Messages
* Latency Statistics

You can debug your AI Agent just like debugging backend distributed tracing systems.

---

### 💬 Virtual Model Endpoint

The virtual model endpoint appears as a standard LLM API externally, but runs a full multi-model workflow internally.

You can package:

```text
Vision Model → Cheap Small Model → Strong Reasoning Model → Verifier → Formatter
```

Into a single model name, such as:

```text
deepseek-with-vision
```

And call it directly using OpenAI / Claude-compatible protocols.

---

### 🤖 Local Agent Client Support

Supports integration with local Agents and compatible clients like Claude Code, Codex, and aionui, allowing them to use the observability, tracing, cost metrics, and virtual model endpoints provided by 1flowbase.

1flowbase is not about black-box proxying or stealthy model replacement.

It emphasizes:

* User-configured model chains
* Observability of every model invocation step
* Calculation of token costs at every turn
* Traceability of every tool call
* Auditability of every response path

---

## 🎯 Why Choose 1flowbase?

### 1. Because you need to see the Agent's path first

If you don't know what the Agent is concatenating for the model, it is very difficult to judge:

* Whether the prompt is too long
* Whether tool definitions have bloated
* Whether the historical context is polluted
* Whether command outputs are repeatedly injected
* Whether lightweight tasks are routed to expensive models
* Whether failed tasks burned massive amounts of tokens

1flowbase helps you see these hidden processes first.

---

### 2. Because AI applications must calculate unit economics

In the traditional software era, once code is deployed, marginal costs are typically extremely low.

But in AI applications, every single user request consumes tokens. The stronger the model, the longer the context, and the more tools used, the higher the cost of a single request.

For individual developers, small-to-medium teams, and AI startups, you cannot only look at model performance—you must calculate unit economics.

1flowbase spreads out every model invocation, tool call, token consumption, and cost for you.

---

### 3. Because large and small models should divide labor

Most AI tools only allow you to choose a single model.

But real-world production-grade AI systems typically require multi-step collaboration:

* Use a Vision model as the "eyes" to read images or PDFs
* Use small models for classification, summarization, and formatting
* Use strong reasoning models to handle truly difficult problems
* Use a Verifier Node to double-check results
* Use a Formatter Node to guarantee output structure
* Use a Router Node to control cost and fallbacks

1flowbase encapsulates this complex process into an out-of-the-box standard model interface.

---

### 4. Because optimizing Harness should be based on real data

Don't guess.

First observe real execution:

```text
What exactly was concatenated in this round?
Which part consumed the most tokens?
Which node was the slowest?
Which tool call failed?
Which model output was unstable?
Which context should be compressed?
```

Then optimize:

```text
Compress prompts
Split tasks
Adjust model routing
Add verifiers
Reduce redundant context
Load tool definitions dynamically instead of statically
```

Finally, publish the optimized chain as a virtual model endpoint.

---

## 🔌 Protocol Compatibility

1flowbase supports exposing the same workflow through multiple mainstream protocols:

| Protocol | API Path | Typical Clients |
| ---------------------------------- | ---------------------- | ---------------------------------------------- |
| **OpenAI Responses API** | `/v1/responses` | Next-generation OpenAI primitives clients |
| **OpenAI Chat Completions API** | `/v1/chat/completions` | Cline, Roo Code, traditional SDKs, and development frameworks |
| **Claude-compatible Messages API** | `/v1/messages` | Claude SDK / Native Claude clients supporting custom Claude API endpoints |

Build one workflow, then invoke it using multiple protocols.

---

## 🛠️ Typical Scenarios

### 1. See what local Agents are actually doing

When local Agents like Claude Code, Codex, or aionui execute tasks, 1flowbase records the complete conversation, hidden prompts, tool calls, command outputs, token consumption, and trace paths.

You no longer only see the final answer, but can see how the Agent moves at every step, where it fails, and where it burns money.

---

### 2. Analyze Token Waste and Unit Economics

Sometimes a user enters a very short message, but the model receives a context that includes system prompts, tool definitions, chat history, command outputs, and project memories.

1flowbase helps you locate:

* Which hidden contexts consume the most tokens
* Which tool definitions are repeatedly injected in every turn
* Which failed tasks caused abnormal costs
* Which steps should be delegated to cheaper small models

---

### 3. Equip Text-Only Models with Missing Capabilities

Before invoking a strong text model that lacks vision capabilities, cascade a Gemini Vision, OCR, or screenshot understanding node to convert images into structured text context, then hand it over to the text model for reasoning.

```text
Image / Screenshot / PDF
  → Vision / OCR Node
  → Text Context
  → Strong Text Model Reasoning
  → Verifier Check
  → Final Response
```

---

### 4. Build a Composable Upstream Brain for Coding Agents

Package complex coding task loops:

```text
Code Generation → Test / Lint Check → Reviewer Node → Fix Node
```

Into a virtual model endpoint, allowing existing Agents to use more complex upstream workflows without modifying the client.

---

### 5. Control Costs via Model Cascading

Filter routine queries with small models, and only drill down to strong reasoning models for difficult tasks.

```text
Simple Classification → Small Model
Formatting → Small Model
Complex Reasoning → Strong Model
Final Verification → Verifier
```

---

### 6. Guarantee Output Structure and Quality

Before returning the final response, use Verifiers, JSON Schema validation, and Formatter nodes to detect and fix output formatting.

This is highly suitable for:

* JSON outputs
* Tool call parameters
* API responses
* Code patches
* Document generation
* Automated task results

---

## ⚔️ How 1flowbase Differs from Others

1flowbase is neither a simple model proxy nor a generic workflow canvas.

It focuses on two specific gaps:

1. Seeing the complete execution process of local Agents.
2. Publishing multi-model workflows as standard model endpoints.

| Tool | Core Philosophy | 1flowbase Difference |
|---|---|---|
| **LiteLLM** | Proxies and routes multiple LLM endpoints | LiteLLM routes models; 1flowbase records Agent execution processes and combines multiple models to generate new virtual model endpoints |
| **LangGraph** | Builds controllable Agent workflows in code | LangGraph is for building Agent graphs; 1flowbase focuses more on observation, publishing, protocol compatibility, and visual runtimes |
| **Dify / Flowise** | Builds visual AI applications and workflows | Dify / Flowise are application builders; 1flowbase emphasizes turning workflows into standard model endpoints callable by existing Agents/SDKs |
| **ccusage-like tools** | Track local Agent tokens and costs | 1flowbase does not just look at consumption; it reconstructs conversations, hidden prompts, tool calls, traces, and virtual model chains |

> **Key Takeaway**: LiteLLM routes models, 1flowbase combines models. ccusage shows the bill, 1flowbase shows the full Agent execution scene.

---

## 🧩 Recipes

We will continue to add ready-to-use workflow templates:

| Recipe | Description |
|---|---|
| `agent-workpath-recorder` | Collect local Agent conversations, tool calls, tokens, and traces |
| `deepseek-with-vision` | Cascade a Vision node before a text model |
| `json-verifier` | Verify and repair JSON before the final output |
| `cheap-code-reviewer` | Small model processes front-end analysis, strong model reviews critical parts |
| `screenshot-to-coding-task` | Convert screenshots to structured coding tasks |
| `cost-aware-router` | Route models by task type, cost, and context length |
| `claude-compatible-agent` | Publish workflow as a Claude-compatible Messages API |
| `openai-responses-agent` | Publish workflow as an OpenAI Responses API |

---

## 🔐 Transparency and Security

The value of 1flowbase comes from observability, but observability also means taking privacy and security seriously.

We recommend self-hosted deployment for sensitive projects and configuring the log collection scope carefully.

The design principles of 1flowbase are:

* Local-first
* Self-hosted first
* Transparent model chains
* Auditable node calls
* Traceable token costs
* Sensitive data masking
* Configurable log retention

1flowbase does not advocate for black-box model replacement.  
We focus on completely spreading out every model combination, tool call, and cost consumption, giving developers full control over their AI Harness.

---

## 📂 Repo Layout

* `web/`: Frontend root directory, powered by `pnpm + Turbo`. The entry application is located at `web/app`, and shared packages reside under `web/packages/*`.
* `api/`: Backend root directory, structured as a Rust workspace. Service entry points are located at `api/apps/*`, and shared crates reside under `api/crates/*`.
* `api/plugins/`: Plugin source code workspace, HostExtension manifests, and templates.
* `docker/`: Container orchestration for local middleware (PostgreSQL, Redis, etc.).
* `scripts/`: Repository-level development, testing, verification, and debugging scripts. For details, see [scripts/README.md](scripts/README.md).

---

## 🗺️ Roadmap

### Implemented Core Features

* [x] **Low-code visual workflow editor**
* [x] **Multiple built-in node types and hybrid orchestration**
* [x] **Virtual model endpoint publishing**
* [x] **OpenAI Responses protocol and streaming output support**
* [x] **OpenAI Chat Completions protocol support**
* [x] **Claude-compatible Messages protocol and streaming output support**
* [x] **Basic execution logs and tool callback Trace**
* [x] **Application-level Token consumption statistics**
* [x] **Prompt and model configuration version history management**

### Currently Enhancing

* [ ] **Enhanced local Agent conversation collection**: Support more Agent clients and log formats
* [ ] **Token Bill of Materials (BOM)**: Break down token sources like System Prompts, tool definitions, historical context, and command outputs
* [ ] **Agent Session search and playback**
* [ ] **Session export and Recall Pack generation**
* [ ] **Abnormal cost detection and optimization suggestions**
* [ ] **More Claude Code / Codex / aionui usage templates**
* [ ] **Enhanced MCP plugin nodes**

### Planned Features

* [ ] **Low-code application building platform for AI organizations**
* [ ] **Enterprise-grade team collaboration space and multi-tenant management**
* [ ] **Permissions, auditing, approval, and cost governance**
* [ ] **Adaptation for more local Agent clients**
* [ ] **Template market and workflow Recipes ecosystem**

---

## 🤝 Contributing

We highly welcome contributions from the community and team members!

Before submitting a Pull Request, please ensure you have completed the following local validations:

```bash
# Run the repository-level complete gatekeeper
# Includes Rust formatting/Clippy/tests, and frontend verification & contract tests
node scripts/node/verify.js repo
```

### Collaborative Guidelines

* Development and quality control guidelines are governed by [AGENTS.md](AGENTS.md) in the root directory.
* Frontend quality requirements can be found in [web/AGENTS.md](web/AGENTS.md).
* Backend quality requirements can be found in [api/AGENTS.md](api/AGENTS.md).

---

## Acknowledgements

Thanks to [Linux.do](https://linux.do/) - Learn AI on L-Station.

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

**If you also want to see clearly what your Agent sent, how much it spent, and what it did, give 1flowbase a Star.**

[Report Bug](https://github.com/taichuy/1flowbase/issues) · [Request Feature](https://github.com/taichuy/1flowbase/issues)

</div>
