# 1flowbase

<p align="center">
  <img src="../assets/logo_index_cn.png" alt="1flowbase Logo" width="600">
</p>

> **对话即是壁垒，AI应用原生底座**

1flowbase 旨在为面向未来的 AI 原生应用和 AI 组织（AI-Native Organizations）提供统一的底座支撑。

## 💡 核心特征

*   💬 **虚拟模型接口 (Virtual Model)**：虚拟模型接口从外部看是一个普通的 LLM API，但在内部它可以运行完整的多模型编排工作流。兼容OpenAI/anthropic协议
*   📜 **完整聊天记录和工具回调日志 (Chat Logs & Tool Trace)**：内置全链路 Trace 追踪系统，精准还原每一次对话背后的工作流执行路径、节点输入输出与工具回调详情。
*   🤖 **多 Agent 客户端原生支持 (Multi-Agent Clients)**：原生支持并适配多个本地 Agent 与客户端工具（如 aionui、codex、Claude Code 等），提供无缝的协议对接与中转能力。


---

## 💡 什么是虚拟模型 (Virtual Model)？

<p align="center">
  <video src="../assets/claude_code_use.mp4" width="100%" controls></video>
</p>

虚拟模型接口从外部看是一个普通的 LLM API，但在内部它可以运行完整的多模型编排工作流。

例如，外部调用：
```json
{
  "model": "deepseek-with-vision",
  "messages": [
    {
      "role": "user",
      "content": "分析这张截图并建议修复方案。"
    }
  ]
}
```

内部实际运行：
```text
Gemini Vision (视觉上下文提取) 
  → DeepSeek-v4 (推理与思考) 
  → 专属校验模型 (JSON 结构验证) 
  → 最终生成 OpenAI 兼容的响应
```

对于调用客户端而言，它只是一个单模型；对于你而言，它是一个完全可编程的 AI 工作流。

---

## 🎯 为什么选择 1flowbase？

大多数 AI 工具只能让你选择**单个模型**。然而，真实的生产级 AI 系统通常需要多步骤编排：
* **多模态增强**：使用 Vision 模型作为“眼睛”读取图片或 PDF，提取上下文，再由强推理模型作为“大脑”处理。
* **智能成本控制**：使用轻量廉价模型进行前置分类与过滤，仅在需要硬核推理时调用高成本模型。
* **可靠性保障**：引入专门的微型模型作为结果校验器（Verifier Node）或格式化器（Formatter Node），确保输出符合特定的 JSON 模式。

1flowbase 将上述复杂的多模型编排（Workflows），封装成一个开箱即用的标准模型接口（Drop-in Endpoints）。

你只需要像调用普通 API 一样配置客户端：
```bash
OPENAI_BASE_URL=http://localhost:3000/v1
OPENAI_API_KEY=your-key
MODEL=deepseek-with-vision
```

---

## 🔌 协议兼容性

1flowbase 支持通过多种主流协议对外暴露同一个工作流：

| 协议 | 接口路径 | 典型客户端 |
|---|---|---|
| **OpenAI Responses API** | `/v1/responses` | 新一代 OpenAI primitives 客户端 |
| **OpenAI Chat Completions** | `/v1/chat/completions` | Cline, Roo Code, 各种传统 SDK 及开发框架 |
| **Claude-compatible Messages** | `/anthropic/v1/messages` | 兼容 Claude SDK / 原生 Claude 客户端 |

---

## ⚔️ 1flowbase 与其他项目的区别

| 工具 | 核心理念 | 1flowbase 的差异化特征 |
|---|---|---|
| **LiteLLM** | 代理并路由多个 LLM 接口 | LiteLLM 路由模型；1flowbase 组合模型并生成新的模型接口 |
| **LangGraph** | 在代码中构建可控的 Agent 工作流 | 1flowbase 将复杂的编排图发布为标准、免集成修改的通用 API |
| **Dify / Flowise** | 构建可视化的 AI 应用与工作流 | 1flowbase 专注于让多模型流像单个模型一样融入现有生态 |

> **核心记忆点**：LiteLLM 路由模型，1flowbase 组合模型。

---

## 🛠️ 典型应用场景

* **为纯文本模型赋予缺失的能力**：在调用不支持 Vision 的强推理模型前，级联一个 Gemini Vision 或 OCR 节点。
* **为 Coding Agent 打造专属大脑**：将复杂的“代码生成 -> Clippy 校验 -> 语法修复”打包成虚拟模型，无需修改 Client 即可让 Agent 变聪明。
* **通过模型级联控制成本**：用小模型过滤常规请求，高难度请求才下钻至推理模型。
* **保证输出结构与质量**：在最终响应返回前，由特定的结构校验节点检测并修复损坏 JSON 格式。

---

## 🗺️ 路线图 (Roadmap)

### 已实现核心特征 (Implemented)
- [x] **低代码可视化工作流编辑器 (Visual workflow editor)**
- [x] **内置多种类型节点与混合编排 (More built-in node types)**
- [x] **调用成本与延迟 Trace 仪表盘 (Cost and latency dashboard)**
- [x] **Prompt 与模型配置版本历史管理 (Prompt/version history)**
- [x] **OpenAI Responses 协议及流式输出支持 (OpenAI Responses streaming)**
- [x] **Claude Messages 协议及流式输出支持 (Claude Messages streaming)**

### 规划中特征 (Upcoming)
- [ ] **聊天记录原生采集与全链路 Trace (Conversation/chat logs collection & trace logs)** — 沉淀组织专属“对话壁垒”的关键第一步
- [ ] **面向 AI 组织的端到端低代码应用构建平台 (AI-Native low-code application builder)** — 从虚拟接口向完整 AI 应用延伸
- [ ] **企业级团队协作空间与工作区多租户管理 (Enterprise team workspace & tenant management)**
- [ ] **支持增强型 MCP (Model Context Protocol) 插件节点 (Advanced MCP integration)**



## 📂 仓库布局 (Repo Layout)

*   `web/`：前端根目录，基于 `pnpm + Turbo` 运作。入口应用位于 `web/app`，共享包位于 `web/packages/*`。
*   `api/`：后端根目录，基于 Rust workspace。服务入口位于 `api/apps/*`，共享 crate 位于 `api/crates/*`。
*   `api/plugins/`：插件源码工作区、HostExtension 清单与模板。
*   `docker/`：本地中间件（PostgreSQL/Redis等）容器编排。
*   `scripts/`：仓库级开发、测试、验证与调试脚本。详细说明见 [scripts/README.md](scripts/README.md)。

---

## 🚀 快速开始

### 运行环境要求
*   **Node.js**: `>= 24.0.0`
*   **Rust**: 最新稳定版编译器 (Workspace)
*   **Docker**: 用于启动本地开发所需中间件

### 本地分步启动

#### 1. 启动中间件
```bash
docker compose -f docker/docker-compose.middleware.yaml up -d
```

#### 2. 启动前端
```bash
cd web
pnpm install
pnpm dev
```
*   前端默认访问地址：`http://127.0.0.1:3100`

#### 3. 启动后端
首次启动请确保从 `api/apps/api-server/.env.example` 复制一份并配置好 `.env`。
```bash
cd api
# 启动 API 服务
cargo run -p api-server --bin api-server
# 启动插件运行器
cargo run -p plugin-runner --bin plugin-runner
```
*   API 服务地址：`http://127.0.0.1:7800`
*   插件运行器地址：`http://127.0.0.1:7801`

### Docker 一键部署

```bash
cd docker
docker compose up -d
```

整套容器会启动 `web`、`api`、`plugin-runner` 和 `db`。默认访问地址：`http://127.0.0.1:3100`，初始 root 账号为 `root`，密码为 `1flowbase`。

生产部署时再复制 `docker/.env.example` 为 `docker/.env`，修改数据库密码、`API_PROVIDER_SECRET_MASTER_KEY` 和 root 密码。

---

## ⚙️ 脚本启动

为了简化本地的开发流程，仓库提供了一套统一的 Node 脚本进行一键式开发启动：

```bash
# 全量启动前端、后端、中间件与插件运行器
node scripts/node/dev-up.js

# 仅启动前后端进程，跳过 Docker 中间件
node scripts/node/dev-up.js --skip-docker

# 常用操作命令
node scripts/node/dev-up.js status   # 查看各服务状态
node scripts/node/dev-up.js stop     # 停止所有本地服务
node scripts/node/dev-up.js restart  # 重启服务
```

关于页面调试、自动化测试、清理缓存等的更多高级脚本用法，请参阅 [scripts/README.md](scripts/README.md)。

---

## 🤝 贡献

我们非常欢迎社区与团队成员的贡献！在提交 PR 前，请确保完成以下代码验证：

### 本地测试与校验
```bash
# 运行仓库级完整门禁 (包括后端格式化/Clippy/测试, 前端校验与契约测试)
node scripts/node/verify.js repo
```

### 协作规则
*   开发与质量控制规则以根目录下的 [AGENTS.md](AGENTS.md) 为准。
*   前端质量要求参见 [web/AGENTS.md](web/AGENTS.md)。
*   后端质量要求参见 [api/AGENTS.md](api/AGENTS.md)。

---
## 鸣谢

 [Linux.do](https://linux.do/) 学ai 上L站

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

## Star History

<p align="center">
  <a href="https://www.star-history.com/#taichuy/1flowbase&Date" target="_blank">
    <img src="https://api.star-history.com/svg?repos=taichuy/1flowbase&type=Date" alt="Star History" width="600">
  </a>
</p>

<div align="center">

**If you like it, give us a star**

[Report Bug](https://github.com/taichuy/1flowbase/issues) · [Request Feature](https://github.com/taichuy/1flowbase/issues)

</div>
