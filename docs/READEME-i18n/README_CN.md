# 1flowbase

<p align="center">
  <img src="../assets/logo_index_cn.png" alt="1flowbase Logo">
</p>

<p align="center">
  <a href="../../README.md">English</a> | <b>简体中文</b>
</p>

> **面向本地 AI Agent 客户端的开源虚拟模型网关。**
> **Harness 优化的第一步是清晰看清 Agent 的执行路径。**

1flowbase 允许你构建多模型工作流，将其发布为 OpenAI / Claude 兼容的虚拟模型接口，并能够观测每次请求背后的真实执行细节：模型调用、节点输入与输出、工具回调、Token 消耗、延迟、失败原因和成本。

你可以使用它来：

- 将多个模型、工具、校验器、路由和格式化器组合到单个工作流中。
- 通过 OpenAI Responses、OpenAI Chat Completions 或 Claude 兼容的 Messages API 对外暴露该工作流。
- 在支持自定义模型接口的本地 AI 客户端和 SDK 中调用该工作流。
- 逐个节点地调试执行过程，而不仅仅是看到最终结果。
- 通过模型级联、fallback（回退）、校验和格式化步骤来优化成本。

> LiteLLM 路由模型。  
> 1flowbase 将模型组合成由工作流支撑的虚拟模型接口。

```text
构建工作流 → 发布接口 → 客户端调用 → 观测 Trace / Token / 成本 → 优化
```

---

## 为什么？

许多 AI 工具仅展示最终回答。而一次真实的 AI 请求包含的内容远比可见的用户消息要多得多：

```text
用户输入 + System Prompt + Developer Prompt + 工具定义 + 项目上下文
+ 对话历史 + 命令输出 + 中间模型调用 + 校验器/格式化器步骤
```

这些隐藏的执行路径直接影响 Token 成本、延迟、模型表现、失败率、输出质量和单位经济学。

哪怕是像 `hi` 这样一条简短的用户输入，一旦附加了周围的上下文、工具 Schema、历史记录和工作流步骤，依然可能变成一次昂贵的请求。

1flowbase 帮助你看清请求背后的工作流，并基于真实的运行时数据而非凭感觉进行优化。

---

## 核心功能现状

当前核心关注点：**基于工作流的虚拟模型接口**以及 **1flowbase 工作流内部的执行可见性**。

**已实现功能**：

- 可视化工作流编辑器
- 多节点工作流编排
- 虚拟模型接口发布
- 支持 OpenAI Responses API
- 支持 OpenAI Chat Completions API
- 支持 Claude 兼容的 Messages API
- 支持流式响应（Streaming）
- 基础运行日志
- 1flowbase 工作流内部的工具回调 Trace
- 应用级 Token 用量统计
- Prompt 与模型配置的版本历史管理

**进行中功能**：

- 更深度的本地 Agent 对话收集
- 会话搜索与回放
- Token 物料清单：拆解 System Prompt、工具定义、对话历史、命令输出以及节点级来源的 Token 占比
- 异常成本检测
- 会话 Recall Pack 导出
- 更多 Claude Code / Codex / aionui 模板
- 兼容 MCP 的插件节点与工具调用源头追踪

> 注意：1flowbase 目前的定位并非 MCP 服务端或 MCP 网关。兼容 MCP 的能力仍在路线图规划中。当前产品专注于发布兼容的模型接口并追踪 1flowbase 工作流的执行。

---

## 工作原理

### 1. 构建工作流

```text
视觉模型 → 小模型 → 强推理模型 → 校验器 → 格式化器
```

### 2. 发布为模型接口

```text
/v1/responses
/v1/chat/completions
/v1/messages
```

### 3. 在现有客户端中调用

对客户端而言，它看起来像是一个普通的模型；对你而言，它是一个可观测、可调节的工作流。

### 4. 观测执行细节

```text
请求
  → 工作流节点输入
  → 模型调用
  → 工具回调
  → 节点输出
  → Token 用量 / 延迟 / 错误
  → 最终响应
```

### 5. 优化与复用

压缩 Prompt、拆分长上下文、将简单步骤移交给更便宜的小模型、添加校验器与格式化器、配置 fallback（回退）策略，然后将优化后的工作流发布为可复用的虚拟模型。

---

## 快速开启

### Docker 一键部署

部署脚本会检查本机是否已有可用的 Docker / Compose 环境，拉取 `docker/` 目录，并将 `docker/.env.example` 复制为 `docker/.env`。

```bash
curl -fsSL https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/shell/docker-deploy.sh | sh
```

PowerShell：

```powershell
irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex
```

Windows CMD：

```bat
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex"
```

### 从源码运行

运行环境要求：Node.js `>= 24.0.0`，最新稳定版 Rust 编译器，以及用于本地中间件运行的 Docker。

```bash
git clone https://github.com/taichuy/1flowbase.git
cd 1flowbase

docker compose -f docker/docker-compose.middleware.yaml up -d

cd web
pnpm install
pnpm dev
```

前端地址：

```text
http://127.0.0.1:3100
```

启动后端服务：

```bash
cd api
# 首次运行前请确保将 api/apps/api-server/.env.example 复制并保存为 .env。
cargo run -p api-server --bin api-server
cargo run -p plugin-runner --bin plugin-runner
```

默认后端服务地址：

```text
API 服务：http://127.0.0.1:7800
插件运行器：http://127.0.0.1:7801
```

使用脚本辅助启动：

```bash
node scripts/node/dev-up.js
node scripts/node/dev-up.js status
node scripts/node/dev-up.js stop
node scripts/node/dev-up.js restart
```

更多配置项请参考 [scripts/README.md](../../scripts/README.md)。

---

## 功能预览

### 构建多模型工作流

创建包含多个模型、工具、校验器和格式化节点的复杂工作流。

![工作流编辑器预览](../assets/workflow_editor_preview.jpeg)

### 发布为 OpenAI 兼容的 API

![发布 OpenAI API](../assets/api_endpoint_publish_1.jpeg)

### 发布为 Claude 兼容的 Messages API

![发布 Claude API](../assets/api_endpoint_publish_2.jpeg)

### 自定义对外暴露的模型信息

![自定义模型信息](../assets/custom_model_settings.jpeg)

### 在兼容的本地 AI 客户端中使用

在支持自定义模型接口的兼容客户端中调用已发布的工作流。

![Claude Code 终端使用预览](../assets/claude_code_terminal_usage.png)

### 查看运行日志

直观追踪模型请求、节点输入与输出、工具回调、响应内容、延迟与错误。

![运行日志详情](../assets/detailed_execution_logs.jpeg)

### 查看工具回调 Trace

![工具回调 Trace 日志](../assets/tool_callback_trace_logs.png)

### 追踪 Token 消耗

![Token 消耗看板](../assets/token_consumption_dashboard.jpeg)

---

## 协议兼容性

| 协议 | API 路径 | 典型用途 |
|---|---:|---|
| OpenAI Responses API | `/v1/responses` | 新版 OpenAI 风格的客户端与应用代码 |
| OpenAI Chat Completions API | `/v1/chat/completions` | SDK、编程工具、聊天客户端、应用开发框架 |
| Claude 兼容的 Messages API | `/v1/messages` | 支持自定义接口的 Claude 兼容客户端 |

只需构建一次工作流，即可通过多种协议对外提供服务。

---

## 典型应用场景

### 在文本模型前级联视觉能力或 OCR

```text
图片 / 截图 / PDF → 视觉或 OCR 节点 → 结构化文本上下文 → 强文本模型 → 校验器 → 最终回答
```

### 通过模型级联控制成本

```text
简单分类步骤 → 便宜小模型
格式化处理 → 便宜小模型
复杂推理步骤 → 强推理模型
最终结果校验 → 校验节点
```

### 保证输出的结构与格式

在返回最终结果前，通过校验器、JSON Schema 验证和格式化节点确保结构完整。这适用于 JSON 输出、API 响应、工具调用参数、代码补丁、文档生成及自动化任务结果。

### 为 Coding Agent 打造可编程的上游模型

```text
代码生成 → 测试 / Lint 检查 → 评审节点 → 修复节点 → 最终补丁
```

现有客户端仅需调用同一个模型名称，而 1flowbase 会在后台隐式运行整套工作流。

### 调试工作流执行过程

使用运行日志和 Trace 追踪链，清晰回答：哪个节点失败了、哪个模型调用最慢、哪一步消耗了最多 Token、哪个工具回调返回了异常结果，以及哪个校验器或格式化器修改了最终输出。

---

## 与同类项目的差异

1flowbase 不仅仅是一个模型代理，也不仅仅是一个普通的工作流画布。

它专注于解决一个痛点：

> 构建多模型工作流，将其发布为标准的模型接口，并能够深度观测每次请求背后的真实执行细节。

| 工具类别 | 常见功能与定位 | 1flowbase 的不同之处 |
|---|---|---|
| 模型路由器 / LLM 网关 | 将单次请求路由至特定提供商或模型 | 将多个模型和工具节点组合成由工作流支撑的虚拟模型 |
| AI 工作流构建器 | 构建 AI 应用或流程工作流 | 将工作流直接暴露为 OpenAI / Claude 兼容的模型 API |
| Agent 开发框架 | 协助开发者用代码编写 Agent 图 | 提供可视化运行时、协议发布接口以及运行日志 Trace |
| 成本追踪工具 | 统计 Token 消耗量或账单总额 | 将成本精确关联至工作流节点、模型调用和执行 Trace |

```text
模型路由器负责“选择”模型。
1flowbase 负责将工作流“组合”成全新的虚拟模型。
```

---

## 透明性与安全

1flowbase 致力于提供透明、自托管的 AI 工作流运行环境。

我们推荐以下原则：

- 自托管优先
- 透明的模型链条
- 可审计的节点调用
- 可追踪的 Token 消耗
- 可配置的日志保留周期
- 敏感数据脱敏过滤
- 显式的模型与工作流配置

1flowbase 不提倡在用户不知情的情况下隐式替换模型。发布的每一个接口都应当由项目所有者精心配置、清晰观测并妥善治理。

---

## 路线图

### 已实现核心功能

- [x] 可视化工作流编辑器
- [x] 多种内置节点类型
- [x] 虚拟模型接口发布
- [x] 支持 OpenAI Responses 协议
- [x] 支持 OpenAI Chat Completions 协议
- [x] 支持 Claude 兼容的 Messages 协议
- [x] 支持流式响应输出（Streaming）
- [x] 基础运行日志
- [x] 1flowbase 工作流内部的工具回调 Trace
- [x] 应用级 Token 消耗统计
- [x] Prompt 与模型配置版本历史管理

### 持续增强中

- [ ] 增强本地 Agent 对话数据收集
- [ ] 精确拆解 Token 物料清单（包含 Prompt、历史上下文、工具定义、命令输出及节点级用量）
- [ ] Agent 会话搜索与回放
- [ ] 会话导出及 Recall Pack 生成
- [ ] 异常成本检测与优化建议
- [ ] 编写更多针对 Claude Code / Codex / aionui 的使用模板
- [ ] 兼容 MCP 的插件节点与工具调用源头追踪

### 长期规划中

- [ ] 面向 AI 组织的低代码应用构建平台
- [ ] 团队协作空间与多租户管理
- [ ] 权限、审批、审计与成本治理机制
- [ ] 适配更多本地 AI Agent 客户端
- [ ] 模板市场与工作流 Recipes（食谱）生态

---

## 仓库目录布局

```text
web/          前端根目录，基于 pnpm + Turbo 运作
api/          Rust 后端 Workspace 工作区
api/apps/     后端服务入口
api/crates/   共享后端 Crate 包
api/plugins/  插件源码工作区、HostExtension 清单与模板
docker/       本地中间件容器编排
scripts/      仓库级开发、测试、验证与调试脚本
```

---

## 参与贡献

非常欢迎社区贡献。在提交 Pull Request 之前，请运行以下验证脚本：

```bash
node scripts/node/verify.js repo
```

项目开发指导准则：

- [AGENTS.md](../../AGENTS.md)
- [web/AGENTS.md](../../web/AGENTS.md)
- [api/AGENTS.md](../../api/AGENTS.md)

---

## 鸣谢

感谢 [Linux.do](https://linux.do/) — 学 AI，上 L 站。

---

## 协议

本项目基于 [Apache-2.0](../../LICENSE) 开源协议授权。

---

## 贡献者

<p align="center">
  <a href="https://github.com/taichuy/1flowbase/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=taichuy/1flowbase&max=50" alt="Contributors" />
  </a>
</p>

---

## Star 增长历史

<p align="center">
  <a href="https://www.star-history.com/#taichuy/1flowbase&Date" target="_blank">
    <img src="https://api.star-history.com/svg?repos=taichuy/1flowbase&type=Date" alt="Star History" width="600">
  </a>
</p>

---

<div align="center">

**如果你想构建由工作流支撑的虚拟模型并清晰看清每次请求背后的执行路径，欢迎为 1flowbase 点一个 Star。**

[报告 Bug](https://github.com/taichuy/1flowbase/issues) · [提出新需求](https://github.com/taichuy/1flowbase/issues)

</div>
