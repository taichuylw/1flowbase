# 1flowbase

<p align="center">
  <img src="../assets/logo_index_cn.png" alt="1flowbase Logo">
</p>

<p align="center">
  <a href="../../README.md">English</a> | <b>简体中文</b>
</p>

> **harness第一步是看清agent工作路径**

你以为只是发了一个 `hi`，模型实际可能收到了 17K input tokens。

1flowbase 是面向本地 AI Agent 的开源运行观测台与虚拟模型网关。

它帮助你还原 Claude Code、Codex、aionui 等本地 Agent 在每一次运行中：

- 最终给模型拼接了什么 Prompt
- 注入了哪些工具定义和系统规则
- 带入了哪些历史上下文和项目记忆
- 执行了哪些工具调用和命令输出
- 每一步消耗了多少 Token 和时间
- 哪些隐藏上下文造成了成本浪费
- 哪些路径导致了失败、循环或错误输出

当你能看清 Agent 的真实工作路径，就可以基于运行数据优化自己的 AI Harness：压缩 Prompt、拆分上下文、调整工具定义、优化模型路由、增加校验器，并进一步把优化后的多模型链路发布成 OpenAI / Claude 兼容的虚拟模型接口。

```text
用户输入
  → Agent Harness 拼接 Prompt / 工具 / 历史 / 记忆
  → 模型请求
  → 工具调用
  → 命令输出
  → Token / 成本 / Trace
  → Harness 优化
  → 多模型组合
  → 虚拟模型接口
````

---

## 你是否也遇到过这些问题？

### 你对 Agent 最终给模型拼接了什么感兴趣吗？

本地 Agent 并不是简单把用户输入转发给模型。

它通常会在背后自动拼接：

* System Prompt
* Developer Prompt
* 工具定义
* MCP 工具信息
* 历史对话
* 项目上下文
* 文件内容
* 命令输出
* 模型切换信息
* 记忆与规则注入

这些隐藏内容会直接影响模型效果、上下文长度、响应速度和 Token 成本。

1flowbase 会把这些隐藏内容摊开给你看。

---

### 为什么发一个 `hi`，也可能烧掉 17K tokens？

用户看到的可能只是：

```text
hi
```

但模型实际收到的可能是一整套 Agent Harness 拼接后的上下文。

例如：

```text
System Prompt
工具定义
历史上下文
项目记忆
本地命令输出
文件内容
模型切换信息
```

很多时候，真正烧 Token 的不是用户输入，而是 Harness 在背后自动拼接的大量隐藏上下文。

1flowbase 会展示模型实际收到的 Prompt、工具信息、命令输出和上下文结构，并统计每一步的输入 Token、输出 Token、耗时和状态。

你不再只看到最终回答，而是能看到这次请求的钱到底花在哪里。

---

### 你想知道 Claude Code / Codex / aionui 的 Prompt 是怎么拼接的吗？

1flowbase 会把一次 Agent 运行还原成可读的 Trace：

```text
用户输入
  → 隐藏 Prompt 注入
  → LLM 请求
  → 工具选择
  → Bash / 文件 / MCP 调用
  → 工具返回
  → 模型继续生成
  → 最终响应
```

你可以像看后端调用链一样，看清 Agent 的工作路径。

---

### 你想基于真实观察结果优化项目 Harness 吗？

当你知道 Token 和失败来自哪里，就可以进一步优化：

* 哪些 Prompt 应该压缩
* 哪些工具定义应该按需加载
* 哪些历史上下文应该总结
* 哪些任务应该交给便宜小模型
* 哪些步骤需要 Verifier
* 哪些输出需要 Formatter
* 哪些失败路径需要 fallback
* 哪些会话应该导出为 Recall Pack

1flowbase 的目标不是让你凭感觉调 Prompt，而是让你基于真实运行路径优化 AI Harness。

---

## 1flowbase 的核心递进

```text
看清路径 → 拆解成本 → 优化 Harness → 组合模型 → 发布接口
```

### 1. 看清路径

还原 Agent 每一次运行的真实路径：

* 用户输入
* Prompt 拼接
* 工具定义注入
* 历史上下文带入
* 模型请求
* 工具调用
* 命令输出
* 最终响应

### 2. 拆解成本

解释一次请求为什么贵、为什么慢、为什么失败：

* 哪些上下文最耗 Token
* 哪些工具定义被重复注入
* 哪些历史消息污染了模型
* 哪些命令输出过大
* 哪些失败任务造成异常成本
* 哪些节点延迟最高
* 哪些模型调用最贵

### 3. 优化 Harness

基于真实运行数据调整：

* Prompt
* 工具定义
* 记忆注入
* 历史上下文
* 模型路由
* Verifier
* Formatter
* fallback 策略

### 4. 组合模型

把多个模型、工具和校验节点组合成一条可复用工作流：

```text
Vision → 小模型分类 → 强模型推理 → Verifier → Formatter
```

### 5. 发布接口

把优化后的工作流发布成标准模型接口：

* OpenAI Responses API
* OpenAI Chat Completions API
* Claude-compatible Messages API

对客户端来说，它只是一个模型。
对你来说，它是一条可观测、可调教、可组合的 AI 工作流。

---

## 🚀 快速开启

### Docker 一键部署（推荐）

下面的命令不会安装 Docker。部署脚本只会先检查本机是否已经有可用的 Docker / Compose 环境，然后把 `docker/` 目录拉到当前目录，复制 `docker/.env.example` 为 `docker/.env`。

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

### 从源码启动开发环境

#### 运行环境要求

* **Node.js**: `>= 24.0.0`
* **Rust**: 最新稳定版编译器
* **Docker**: 用于启动本地开发所需中间件

#### 1. Clone 仓库

```bash
git clone https://github.com/taichuy/1flowbase.git
cd 1flowbase
```

#### 2. 启动中间件

```bash
docker compose -f docker/docker-compose.middleware.yaml up -d
```

#### 3. 启动前端

```bash
cd web
pnpm install
pnpm dev
```

前端默认访问地址：

```text
http://127.0.0.1:3100
```

#### 4. 启动后端

首次启动请确保从 `api/apps/api-server/.env.example` 复制一份并配置好 `.env`。

```bash
cd api

# 启动 API 服务
cargo run -p api-server --bin api-server

# 启动插件运行器
cargo run -p plugin-runner --bin plugin-runner
```

默认服务地址：

```text
API 服务：http://127.0.0.1:7800
插件运行器：http://127.0.0.1:7801
```

---

### 脚本启动

为了简化本地开发流程，仓库提供了一套统一的 Node 脚本：

```bash
# 全量启动前端、后端、中间件与插件运行器
node scripts/node/dev-up.js

# 仅启动前后端进程，跳过 Docker 中间件
node scripts/node/dev-up.js --skip-docker

# 查看各服务状态
node scripts/node/dev-up.js status

# 停止所有本地服务
node scripts/node/dev-up.js stop

# 重启服务
node scripts/node/dev-up.js restart
```

更多脚本说明请参阅 [scripts/README.md](../../scripts/README.md)。

---

## 🖼️ 功能预览

### 1. 构建多模型工作流

你可以在 1flowbase 中构建由多个模型、工具、校验器和格式化节点组成的工作流。

![工作流编辑器预览](../assets/workflow_editor_preview.jpeg)

---

### 2. 发布为 OpenAI 兼容 API

构建完成后，可以将工作流发布为 OpenAI 兼容接口。

![发布 OpenAI API](../assets/api_endpoint_publish_1.jpeg)

---

### 3. 发布为 Claude-compatible Messages API

同一条工作流也可以通过 Claude-compatible Messages API 对外暴露。

![发布 Claude API](../assets/api_endpoint_publish_2.jpeg)

---

### 4. 自定义对外模型信息

你可以自定义对外暴露的模型名称、描述和能力信息。

![自定义模型信息](../assets/custom_model_settings.jpeg)

---

### 5. 在 Claude Code 等客户端中使用

发布后，可以在支持自定义 endpoint 的 Claude-compatible 客户端中调用。

![Claude Code Terminal Usage](../assets/claude_code_terminal_usage.png)

---

### 6. 查看完整运行日志

你可以查看一次运行背后的模型请求、节点输入输出、工具调用和响应内容。

![运行日志](../assets/detailed_execution_logs.jpeg)

---

### 7. 查看工具回调 Trace

对于复杂任务，1flowbase 会展示详细的工具回调、命令输出和执行路径。

![工具回调日志](../assets/tool_callback_trace_logs.png)

---

### 8. 统计 Token 消耗

按应用、模型、会话维度统计 Token 消耗，帮助你理解真实成本。

![Token 消耗统计](../assets/token_consumption_dashboard.jpeg)

---

## 💡 什么是 Agent 工作路径？

大多数本地 Agent 客户端只展示最终对话结果。

但真实运行中，一个 Agent 请求通常包含：

* System Prompt
* Developer Prompt
* 工具定义
* 历史上下文
* 项目记忆
* 本地命令输出
* 文件内容
* MCP 工具返回
* 模型中间响应
* 工具调用结果

这些内容共同组成了 Agent 的工作路径。

它们会影响：

* 模型效果
* 上下文长度
* 响应速度
* Token 消耗
* 工具调用结果
* 最终成本
* 任务是否成功

1flowbase 会把这些隐藏过程记录下来，并以 Trace 的方式展示：

```text
用户输入
  → 隐藏 Prompt 注入
  → LLM 请求
  → 工具选择
  → Bash / 文件 / MCP 调用
  → 工具返回
  → 模型继续生成
  → 最终响应
```

你可以用它回答这些问题：

* 为什么这次请求这么贵？
* 为什么一个简单输入触发了上万 Token？
* 哪个工具调用失败了？
* 哪段历史上下文污染了模型？
* 哪个节点最慢？
* 哪个模型适合换成便宜模型？
* 哪条会话应该压缩、导出或复用？

---

## 💡 什么是 AI Harness？

在 1flowbase 中，AI Harness 指的是模型外部的整套工程组织方式。

它包括：

* Prompt
* System Message
* 工具定义
* 工具调用策略
* 记忆注入
* 历史上下文
* 文件上下文
* 模型路由
* 校验器
* 格式化器
* 成本控制
* 失败重试
* fallback 策略

模型本身很重要，但商业化 AI 应用不能只依赖“模型很强”。

对于个人使用，直接和强模型对话可能已经足够。
但对于商业化项目，每一次请求都会带来 Token 成本。

如果没有观测、拆解和优化，你可能会遇到：

```text
用户越多
  → 请求越多
  → Token 消耗越多
  → 模型成本越高
  → 如果收入模型没覆盖成本，增长反而扩大亏损
```

所以 1flowbase 关注的不只是模型能力，而是 AI 应用的单位经济：

* 效果
* 成本
* 延迟
* 稳定性
* 可追踪性
* 可复用性

---

## 💬 什么是虚拟模型接口？

虚拟模型接口从外部看是一个普通的 LLM API，但在内部可以运行一条完整的多模型工作流。

例如，外部客户端只看到：

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

但在 1flowbase 内部，它实际可能运行：

```text
Gemini Vision
  → 提取图片、OCR、UI 和图表上下文
  → DeepSeek-v4 进行推理与生成
  → Verifier 节点检查遗漏和格式
  → Formatter 节点生成 OpenAI 兼容响应
```

对于调用客户端而言，它只是一个模型。
对于你而言，它是一条可编排、可追踪、可优化的 AI 工作流。

---

## ✨ 核心特征

### 🧭 Agent 工作路径还原

采集并还原本地 Agent 的完整运行过程：

* 用户输入
* 隐藏 Prompt
* System / Tool / Memory 上下文
* 模型请求与响应
* 工具调用与命令输出
* 文件读写与回调结果
* 失败路径与异常状态

不是只保存聊天，而是记录 Agent 每一步到底做了什么。

---

### 📊 Token 与成本账本

按应用、会话、模型、节点统计 Token 和成本：

* 总 Token
* 输入 Token
* 输出 Token
* 工具调用 Token
* 节点耗时
* 失败运行成本
* 高消耗上下文来源

帮助你回答一个关键问题：

> 为什么用户只说了一句很短的话，模型却收到了几万 Token 的上下文？

---

### 🔍 全链路 Trace

内置全链路 Trace 追踪系统，精准还原每一次对话背后的执行路径：

* 节点输入
* 节点输出
* LLM 请求
* 工具调用
* 工具返回
* 命令输出
* 错误信息
* 耗时统计

你可以像调试后端服务一样分析 AI Agent 的执行过程。

---

### 💬 虚拟模型接口

虚拟模型接口从外部看是一个普通 LLM API，但内部可以运行完整的多模型工作流。

你可以把：

```text
视觉模型 → 便宜小模型 → 强推理模型 → 校验器 → 格式化节点
```

封装成一个模型名，例如：

```text
deepseek-with-vision
```

然后通过 OpenAI / Claude 兼容协议直接调用。

---

### 🤖 本地 Agent 客户端支持

支持对接 Claude Code、Codex、aionui 等本地 Agent 或兼容客户端，让它们使用 1flowbase 提供的观测、Trace、成本统计和虚拟模型接口。

1flowbase 的目标不是黑盒中转，也不是偷偷替换模型。

它强调的是：

* 用户自己配置模型链路
* 每一步模型调用可见
* 每一步 Token 成本可算
* 每一次工具调用可追踪
* 每一条响应路径可审计

---

## 🎯 为什么选择 1flowbase？

### 1. 因为你需要先看清 Agent 工作路径

如果你不知道 Agent 给模型拼了什么，你就很难判断：

* Prompt 是否过长
* 工具定义是否膨胀
* 历史上下文是否污染
* 命令输出是否被重复注入
* 小任务是否用了过贵模型
* 失败任务是否烧了大量 Token

1flowbase 先帮你看清这些隐藏路径。

---

### 2. 因为 AI 应用必须计算单位经济

在传统软件时代，代码部署上线后，边际成本通常比较低。

但在 AI 应用里，每一次用户请求都会消耗 Token。模型越强、上下文越长、工具越多，单次请求成本就越高。

对于个人开发者、中小团队和 AI 创业者来说，不能只看模型效果，还必须计算单位经济。

1flowbase 帮你把每一次模型调用、工具调用、Token 消耗和成本摊开来看。

---

### 3. 因为大小模型应该分工

大多数 AI 工具只能让你选择单个模型。

但真实生产级 AI 系统通常需要多步骤协作：

* 用 Vision 模型作为“眼睛”读取图片或 PDF
* 用小模型做分类、摘要、格式整理
* 用强推理模型处理真正困难的问题
* 用 Verifier 节点检查结果
* 用 Formatter 节点保证输出结构
* 用 Router 节点控制成本和 fallback

1flowbase 将这些复杂流程封装成一个开箱即用的标准模型接口。

---

### 4. 因为优化 Harness 应该基于真实数据

不要靠猜。

你可以先观察真实运行：

```text
这一轮到底拼了什么？
哪个部分最耗 Token？
哪个节点最慢？
哪个工具调用失败？
哪个模型输出不稳定？
哪个上下文应该压缩？
```

然后再优化：

```text
压缩 Prompt
拆分任务
调整模型路由
增加校验器
减少重复上下文
把固定工具定义改成按需加载
```

最后把优化后的链路发布成虚拟模型接口。

---

## 🔌 协议兼容性

1flowbase 支持通过多种主流协议对外暴露同一个工作流：

| 协议                                 | 接口路径                   | 典型客户端                                          |
| ---------------------------------- | ---------------------- | ---------------------------------------------- |
| **OpenAI Responses API**           | `/v1/responses`        | 新项目、Agentic 应用、支持 Responses 的客户端               |
| **OpenAI Chat Completions API**    | `/v1/chat/completions` | Cline、Roo Code、OpenAI 兼容 SDK 与开发框架             |
| **Claude-compatible Messages API** | `/v1/messages`         | Claude SDK 兼容客户端、支持自定义 Claude API endpoint 的工具 |

构建一条工作流，然后用多种协议调用它。

---

## 🛠️ 典型应用场景

### 1. 看清本地 Agent 到底做了什么

当 Claude Code、Codex、aionui 等本地 Agent 执行任务时，1flowbase 可以记录完整对话、隐藏 Prompt、工具调用、命令输出、Token 消耗和 Trace 路径。

你不再只能看到最终回答，而是能看到 Agent 每一步怎么走、哪里失败、哪里烧钱。

---

### 2. 分析 Token 浪费和单位经济

有时候用户只输入一句很短的话，但模型实际收到的上下文可能包含 System Prompt、工具定义、历史记录、命令输出和项目记忆。

1flowbase 帮你定位：

* 哪些隐藏上下文最耗 Token
* 哪些工具定义每轮都被重复注入
* 哪些失败任务造成了异常成本
* 哪些步骤应该交给更便宜的小模型

---

### 3. 为纯文本模型补齐缺失能力

在调用不支持 Vision 的强文本模型前，先级联 Gemini Vision、OCR 或截图理解节点，把图片转换成结构化上下文，再交给文本模型推理。

```text
图片 / 截图 / PDF
  → Vision / OCR 节点
  → 文本上下文
  → 强文本模型推理
  → Verifier 检查
  → 最终响应
```

---

### 4. 为 Coding Agent 打造可组合上游大脑

把复杂的代码任务链路：

```text
代码生成 → 测试 / lint 检查 → Reviewer 节点 → 修复节点
```

封装成一个虚拟模型接口，让现有 Agent 不改客户端也能使用更复杂的上游链路。

---

### 5. 通过模型级联控制成本

用小模型过滤常规请求，高难度请求才下钻至强推理模型。

```text
简单分类 → 小模型
格式整理 → 小模型
复杂推理 → 强模型
最终校验 → Verifier
```

---

### 6. 保证输出结构和质量

在最终响应返回前，通过 Verifier、JSON Schema 校验、Formatter 节点检测和修复输出格式。

这适合：

* JSON 输出
* 工具调用参数
* API response
* 代码 patch
* 文档生成
* 自动化任务结果

---

## ⚔️ 1flowbase 与其他项目的区别

1flowbase 不是单纯的模型代理，也不是普通的工作流画布。

它关注两个缺口：

1. 看清本地 Agent 的完整工作路径
2. 把多模型工作流发布成标准模型接口

| 工具                 | 核心理念                 | 1flowbase 的差异                                                        |
| ------------------ | -------------------- | -------------------------------------------------------------------- |
| **LiteLLM**        | 代理并路由多个 LLM 接口       | LiteLLM 路由模型；1flowbase 记录 Agent 运行过程，并组合多个模型生成新的虚拟模型接口               |
| **LangGraph**      | 在代码中构建可控 Agent 工作流   | LangGraph 适合写 Agent 图；1flowbase 更关注观测、发布、协议兼容和可视化运行时                 |
| **Dify / Flowise** | 构建可视化 AI 应用与工作流      | Dify / Flowise 更像应用构建器；1flowbase 更强调把工作流变成可被现有 Agent / SDK 调用的标准模型接口 |
| **ccusage 类工具**    | 统计本地 Agent Token 和成本 | 1flowbase 不只看用量，还还原对话、隐藏 Prompt、工具调用、Trace 和虚拟模型链路                   |

> **核心记忆点**：LiteLLM 路由模型，1flowbase 组合模型；ccusage 看账单，1flowbase 看完整 Agent 运行现场。

---

## 🧩 Recipes

后续我们会持续补充可直接复用的工作流模板：

| Recipe                      | 说明                                     |
| --------------------------- | -------------------------------------- |
| `agent-workpath-recorder`   | 采集本地 Agent 对话、工具调用、Token 和 Trace       |
| `deepseek-with-vision`      | 在文本模型前级联 Vision 节点                     |
| `json-verifier`             | 在最终输出前验证并修复 JSON                       |
| `cheap-code-reviewer`       | 小模型做前置分析，强模型做关键审查                      |
| `screenshot-to-coding-task` | 将截图转换为结构化代码任务                          |
| `cost-aware-router`         | 按任务类型、成本和上下文长度路由模型                     |
| `claude-compatible-agent`   | 将工作流发布为 Claude-compatible Messages API |
| `openai-responses-agent`    | 将工作流发布为 OpenAI Responses API           |

---

## 🔐 透明与安全

1flowbase 的价值来自可观测，但可观测也意味着需要认真对待隐私和安全。

建议在敏感项目中优先使用自托管部署，并谨慎配置日志采集范围。

1flowbase 的设计方向是：

* 本地优先
* 自托管优先
* 模型链路透明
* 节点调用可审计
* Token 成本可追踪
* 敏感数据可脱敏
* 日志保留可配置

1flowbase 不主张黑盒替换模型。
我们更关注把每一次模型组合、工具调用和成本消耗完整摊开，让开发者自己掌控 AI Harness。

---

## 📂 仓库布局

* `web/`：前端根目录，基于 `pnpm + Turbo` 运作。入口应用位于 `web/app`，共享包位于 `web/packages/*`。
* `api/`：后端根目录，基于 Rust workspace。服务入口位于 `api/apps/*`，共享 crate 位于 `api/crates/*`。
* `api/plugins/`：插件源码工作区、HostExtension 清单与模板。
* `docker/`：本地中间件 PostgreSQL / Redis 等容器编排。
* `scripts/`：仓库级开发、测试、验证与调试脚本。详细说明见 [scripts/README.md](../../scripts/README.md)。

---

## 🗺️ 路线图

### 已实现核心特征

* [x] **低代码可视化工作流编辑器**
* [x] **内置多种类型节点与混合编排**
* [x] **虚拟模型接口发布**
* [x] **OpenAI Responses 协议及流式输出支持**
* [x] **OpenAI Chat Completions 协议支持**
* [x] **Claude-compatible Messages 协议及流式输出支持**
* [x] **基础运行日志与工具回调 Trace**
* [x] **应用级 Token 消耗统计**
* [x] **Prompt 与模型配置版本历史管理**

### 持续增强中

* [ ] **本地 Agent 对话采集增强**：支持更多 Agent 客户端与日志格式
* [ ] **Token 物料清单**：拆解 System Prompt、工具定义、历史上下文、命令输出等 Token 来源
* [ ] **Agent Session 搜索与回放**
* [ ] **会话导出与 Recall Pack 生成**
* [ ] **成本异常检测与优化建议**
* [ ] **更多 Claude Code / Codex / aionui 使用模板**
* [ ] **增强型 MCP 插件节点**

### 规划中特征

* [ ] **面向 AI 组织的低代码应用构建平台**
* [ ] **企业级团队协作空间与多租户管理**
* [ ] **权限、审计、审批与成本治理**
* [ ] **更多本地 Agent 客户端适配**
* [ ] **模板市场与工作流 Recipes 生态**

---

## 🤝 贡献

我们非常欢迎社区与团队成员的贡献。

在提交 PR 前，请确保完成以下代码验证：

```bash
# 运行仓库级完整门禁
# 包括后端格式化 / Clippy / 测试、前端校验与契约测试
node scripts/node/verify.js repo
```

### 协作规则

* 开发与质量控制规则以根目录下的 [AGENTS.md](../../AGENTS.md) 为准。
* 前端质量要求参见 [web/AGENTS.md](../../web/AGENTS.md)。
* 后端质量要求参见 [api/AGENTS.md](../../api/AGENTS.md)。

---

## 鸣谢

[Linux.do](https://linux.do/) —— 学 AI，上 L 站。

---

## License

This project is licensed under [Apache-2.0](../../LICENSE).

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

**如果你也想看清 Agent 到底拼了什么、花了多少、怎么工作的，欢迎给 1flowbase 一个 Star。**

[Report Bug](https://github.com/taichuy/1flowbase/issues) · [Request Feature](https://github.com/taichuy/1flowbase/issues)

</div>
