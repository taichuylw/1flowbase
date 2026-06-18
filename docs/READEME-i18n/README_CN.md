# 1flowbase

<p align="center">
  <img src="../assets/logo_index_cn.png" alt="1flowbase Logo">
</p>

<p align="center">
  <a href="../../README.md">English</a> | <b>简体中文</b>
</p>

<p align="center">
  <a href="https://github.com/taichuy/1flowbase/stargazers"><img src="https://img.shields.io/github/stars/taichuy/1flowbase?style=social" alt="GitHub stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/github/license/taichuy/1flowbase" alt="License"></a>
  <img src="https://img.shields.io/badge/OpenAI-compatible-111827" alt="OpenAI compatible">
  <img src="https://img.shields.io/badge/Claude-compatible-111827" alt="Claude compatible">
  <img src="https://img.shields.io/badge/self--hosted-1flowbase-2563eb" alt="Self-hosted">
</p>

<p align="center">
  <strong>交流与社区：</strong>
  <a href="../assets/community/wechat.jpg" target="_blank">微信</a> |
  <a href="../assets/community/taichuy_doc_wechat_office.png" target="_blank">微信公众号（文档）</a> |
  <a href="https://x.com/Tacihu2021" target="_blank">Twitter</a>
</p>

> **面向 Claude Code、Codex、OpenCode、Cline、Continue 等本地 AI Agent 客户端的开源虚拟模型网关。**

1flowbase 让你构建多模型工作流，将它发布为 OpenAI 兼容或 Claude 兼容的模型接口，并看清每次请求背后的完整执行 Trace。

当普通 LLM 网关或模型路由器不够用时，可以用 1flowbase 做这些事：

- 给文本优先的 Coding Model 挂载多模态模型工具，让它能处理截图、UI、图表和设计稿。
- 把 Fusion 风格的多模型评审团发布成一个可复用的模型接口。
- 将模型链、工具、校验器、fallback 和格式化节点组合成一个虚拟模型。
- 在 Claude Code、Codex、OpenCode、Cline、Continue、SDK 或任何支持自定义模型接口的客户端中调用这个虚拟模型。
- 调试节点输入、节点输出、工具回调、Token、延迟、失败原因和成本，而不只是看到最终回答。

```text
构建工作流 -> 发布虚拟模型 -> Agent 客户端调用 -> 查看 Trace -> 优化
```

> LiteLLM、Portkey、Bifrost 等网关通常解决模型流量路由。
> 1flowbase 解决的是把多个模型和工具组合成新的、可观测的虚拟模型接口。

---

## 现在可以构建什么

### 让 GLM-5.2、DeepSeek 等文本 Coding Model 在 Claude Code 里看图

Claude Code 和其他 Coding Agent 可以接收截图、UI 图片、图表和设计参考，但部分强 Coding Model 在客户端链路中仍然更适合作为文本优先模型使用。

1flowbase 可以先拦截图片，调用视觉模型作为挂载工具，再把结构化视觉上下文返回给主 Coding Model，同时保留完整执行日志。

```text
Claude Code
  -> 1flowbase 虚拟模型接口
  -> GLM-5.2 / DeepSeek / 其他主力 Coding Model
  -> 挂载的视觉工具
  -> GLM-5V-Turbo / Gemini / GPT vision / OCR 模型
  -> 结构化视觉结果
  -> 最终代码回答
```

教程：[让 GLM-5.2 在 Claude Code 里看图](https://github.com/taichuy/1flowbase/wiki/Make-GLM-5.2-See-Images-in-Claude-Code-with-1flowbase-CN)

### 发布 Fusion 风格的多模型评审接口

OpenRouter Fusion 让很多开发者意识到：下一个有价值的模型入口，未必是单个更大的模型，而可能是一个复合工作流。

1flowbase 内置 `fusion` 模板，可以把多个分支模型和一个汇总模型做成可发布接口。Agent 客户端只调用一个模型名，1flowbase 在后台执行整套模型评审团，并记录每个分支、Token、失败状态和汇总过程。

```text
用户请求
  -> 主 LLM
  -> fusion 工具
     -> 分支 LLM A
     -> 分支 LLM B
     -> 分支 LLM C
     -> 汇总 LLM
  -> 最终回答
```

教程：[Fusion 风格工作流：把多模型评审团发布成一个可观测的虚拟模型](https://github.com/taichuy/1flowbase/wiki/Fusion-Style-Workflow-CN)

### 工作流支撑的模型 API

构建一次工作流，即可通过常见模型协议对外提供服务：

| 协议 | API 路径 | 典型用途 |
|---|---:|---|
| OpenAI Responses API | `/v1/responses` | 新版 OpenAI 风格客户端与应用代码 |
| OpenAI Chat Completions API | `/v1/chat/completions` | SDK、编程工具、聊天客户端、应用开发框架 |
| Claude 兼容 Messages API | `/v1/messages` | 支持自定义接口的 Claude 兼容客户端 |

---

## 为什么需要 1flowbase

许多 AI 工具只展示最终回答，但一次真实 Agent 请求通常远比用户可见消息复杂：

```text
用户输入 + System Prompt + Developer Prompt + 工具定义 + 项目上下文
+ 对话历史 + 命令输出 + 图片/文件引用 + 中间模型调用
+ 校验步骤 + 格式化步骤 + fallback 调用
```

这条隐藏路径会决定输出质量、成本、延迟、失败率，以及 Agent 是否值得信任。

1flowbase 让这条路径变得可见、可编排、可优化：

- **编排** 多个模型、工具、校验器、路由和格式化器。
- **发布** 为普通 OpenAI 兼容或 Claude 兼容模型接口。
- **观测** 节点输入输出、工具回调、Token、延迟、错误和成本。
- **优化** 高成本请求，例如模型级联、按需视觉调用、fallback 和多模型评审。
- **复用** 已验证的工作流，将它作为命名虚拟模型供本地 Agent 客户端和应用代码调用。

---

## 快速开始

### Docker 一键引导

脚本会检查本机是否已有可用 Docker / Compose 环境，拉取 `docker/` 目录，并将 `docker/.env.example` 复制为 `docker/.env`。

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

运行环境要求：Node.js `>= 24.0.0`，最新稳定版 Rust，以及用于本地中间件的 Docker。

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

### 让 Coding Agent 帮你安装

```text
克隆 https://github.com/taichuy/1flowbase，按照 README 快速开始运行项目，然后帮我把一个工作流发布成 OpenAI 兼容或 Claude 兼容的虚拟模型接口。
```

---

## 功能预览

### 构建多模型工作流

创建包含多个模型、工具、校验器、分支模型和格式化节点的工作流。

![工作流编辑器预览](../assets/workflow_editor_preview.jpeg)

### 发布为 OpenAI 兼容 API

![发布 OpenAI API](../assets/api_endpoint_publish_1.jpeg)

### 发布为 Claude 兼容 Messages API

![发布 Claude API](../assets/api_endpoint_publish_2.jpeg)

### 自定义对外暴露的模型信息

![自定义模型信息](../assets/custom_model_settings.jpeg)

### 在本地 AI Agent 客户端中使用

在支持自定义模型接口的客户端中调用已发布工作流。

![Claude Code 终端使用预览](../assets/claude_code_terminal_usage.png)

### 查看执行日志

追踪模型请求、节点输入输出、工具回调、响应内容、延迟和错误。

![运行日志详情](../assets/detailed_execution_logs.jpeg)

### 查看工具回调 Trace

![工具回调 Trace 日志](../assets/tool_callback_trace_logs.png)

### 追踪 Token 消耗

![Token 消耗看板](../assets/token_consumption_dashboard.jpeg)

---

## 典型应用场景

### 让文本 Coding Model 理解截图

```text
截图 / UI 设计稿 / 图表
  -> 视觉工具
  -> 结构化视觉上下文
  -> 强 Coding Model
  -> 代码补丁、方案或解释
```

适用于 UI 复刻、前端调试、视觉回归分析、图表阅读、PDF 页面理解和设计稿转代码。

### 构建 Fusion 风格评审器

```text
架构方案
  -> 便宜快速评审模型
  -> 强推理评审模型
  -> 不同供应商评审模型
  -> 汇总模型
  -> 最终建议
```

适用于架构评审、研究综合、代码评审、文档复核和高价值 Agent 决策。

### 通过模型级联控制成本

```text
简单分类 -> 小模型
格式化 -> 小模型
复杂推理 -> 强模型
最终校验 -> 校验节点
```

### 保证输出结构

在返回最终结果前，通过校验器、JSON Schema 验证和格式化节点确保结构完整。适用于 JSON 输出、API 响应、工具调用参数、代码补丁、文档生成和自动化任务结果。

### 为 Agent 打造可编程的上游模型

```text
代码生成 -> 测试 / Lint 检查 -> 评审节点 -> 修复节点 -> 最终补丁
```

客户端只调用一个模型名，1flowbase 在后台运行你的工作流。

---

## 与同类项目的差异

1flowbase 不只是模型代理，也不只是普通工作流画布。

它专注于解决一个缺口：

> 构建多模型工作流，将它发布为标准模型接口，并看清每次请求背后的执行路径。

| 工具类别 | 常见功能与定位 | 1flowbase 的不同之处 |
|---|---|---|
| LLM 网关 / 模型路由器 | 将单次请求路由至特定供应商或模型 | 将多个模型和工具节点组合成由工作流支撑的虚拟模型 |
| AI 工作流构建器 | 构建 AI 应用或流程工作流 | 将工作流直接暴露为 OpenAI / Claude 兼容模型 API |
| Agent 开发框架 | 协助开发者用代码编写 Agent 图 | 提供可视化运行时、协议发布接口和执行日志 |
| 可观测性 / 成本追踪工具 | 统计 Token 消耗量或账单总额 | 将成本精确关联至工作流节点、模型调用、工具回调和 Trace |

```text
模型路由器负责选择模型。
1flowbase 负责把工作流组合成新的虚拟模型。
```

---

## 当前状态

### 已实现

- [x] 可视化工作流编辑器
- [x] 多节点工作流编排
- [x] 虚拟模型接口发布
- [x] 支持 OpenAI Responses 协议
- [x] 支持 OpenAI Chat Completions 协议
- [x] 支持 Claude 兼容 Messages 协议
- [x] 支持流式响应
- [x] 面向多模态和分支模型工作流的挂载 LLM 工具
- [x] `fusion` 工作流模板
- [x] 执行日志
- [x] 1flowbase 工作流内部的工具回调 Trace
- [x] 应用级 Token 消耗统计
- [x] Prompt 与模型配置版本历史管理

### 持续增强中

- [ ] 更深度的本地 Agent 对话收集
- [ ] 会话搜索与回放
- [ ] Token 物料清单：按 Prompt、历史上下文、工具定义、命令输出、媒体输入和节点拆解用量
- [ ] 异常成本检测与优化建议
- [ ] 会话导出和 Recall Pack 生成
- [ ] 更多 Claude Code / Codex / OpenCode / Cline / Continue 模板
- [ ] 兼容 MCP 的插件节点与工具调用源头追踪

### 长期规划中

- [ ] 面向 AI 组织的低代码应用构建平台
- [ ] 团队协作空间与多租户管理
- [ ] 权限、审批、审计与成本治理机制
- [ ] 适配更多本地 AI Agent 客户端
- [ ] 模板市场与工作流 Recipes 生态

> 注意：1flowbase 目前的定位并非 MCP 服务端或 MCP 网关。兼容 MCP 的能力仍在路线图中。当前产品专注于发布兼容模型接口，并追踪 1flowbase 工作流的执行。

---

## 透明性与安全

1flowbase 致力于提供透明、自托管的 AI 工作流运行环境。

推荐原则：

- 自托管优先
- 透明的模型链条
- 可审计的节点调用
- 可追踪的 Token 消耗
- 可配置的日志保留周期
- 敏感数据脱敏过滤
- 显式的模型与工作流配置

1flowbase 不提倡在用户不知情的情况下隐式替换模型。发布的每一个接口都应当由项目所有者清晰配置、观测和治理。

---

## 使用教程

- [让 GLM-5.2 在 Claude Code 里看图](https://github.com/taichuy/1flowbase/wiki/Make-GLM-5.2-See-Images-in-Claude-Code-with-1flowbase-CN)
- [Fusion 风格工作流：把多模型评审团发布成一个可观测的虚拟模型](https://github.com/taichuy/1flowbase/wiki/Fusion-Style-Workflow-CN)
- [1flowbase Wiki](https://github.com/taichuy/1flowbase/wiki)

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

## 友情链接

- [Linux.do](https://linux.do/) - 学 AI，上 L 站。
- [Aionui](https://github.com/iOfficeAI/AionUi) - 手机远程控制 AI 干活。
- [OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) - 专为 AI 智能体设计的 Office 套件。
- [deepseek-pp](https://github.com/zhu1090093659/deepseek-pp) - DeepSeek 网页对话浏览器扩展插件。
- [MuseAI](https://github.com/yejiming/MuseAI) - 本地 AI 伴侣、文字冒险与穿书互动应用。
- [FrontAgent](https://github.com/FrontAgent/FrontAgent) - 专为前端工程设计的 AI Agent 系统。
- [RedBox](https://github.com/Jamailar/RedBox) - 面向小红书创作者的本地化 AI 创作工作台。

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

**如果你希望本地 AI Agent 调用可观测的多模型虚拟模型，欢迎给 1flowbase 点一个 Star。**

[报告 Bug](https://github.com/taichuy/1flowbase/issues) · [提出新需求](https://github.com/taichuy/1flowbase/issues)

</div>
