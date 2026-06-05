# Fuck My Shit Mountain Audit Report

**Project:** 1flowbase-latest  
**Audit mode:** full  
**Date:** 2026-06-05  
**Reviewer:** Codex / GPT-5

---

## 1. Executive Summary

本次审计覆盖后端 Rust/Axum、前端 React/Vite、插件运行器、Docker 发布配置、GitHub Actions 质量门禁、测试与覆盖率治理脚本。整体来看，项目边界和质量治理意识较强：后端模块拆分清晰，前端有较多专项测试，CI 有静态检查、格式化、质量门禁、容器扫描和夜间/手动全量流程，插件/JS Block 运行策略也有明确限制。

主要风险集中在三类：第一，安全基线存在可立即利用或可误用的问题，包括 provider secret 使用 XOR 可逆混淆、生产 compose 保留默认密钥/密码、会话 Cookie 未设置生产环境 Secure、内部错误消息原样返回客户端；第二，稳定性和发布信心存在缝隙，包括 data source/capability stdio 超时后未确保杀死子进程、PR CI 未运行后端测试分片；第三，可维护性压力已经显性化，多个前端页面/编辑器容器和后端服务文件超过 1000 行，局部状态、effect、保存、导航和运行逻辑混在同一层。

建议修复顺序是先处理密钥/加密与生产默认值，再补齐 Cookie/错误响应安全边界和插件子进程生命周期，随后把 PR 测试门禁和覆盖率门槛扩到关键运行时包。维护性问题不建议大爆炸重构，应围绕高风险文件按行为边界拆 hook/service/module，并以现有测试保护拆分。

### Score Dashboard

```
Security        ████░░░░░░  4.8  C   XOR secret encryption, production defaults, cookie/error leakage
Stability       ██████░░░░  6.2  B   good Result usage, but stdio timeout leaks and silent fallback
Performance     ███████░░░  6.8  B   no clear hot-path disaster, subprocess leak can become resource pressure
Testing         ██████░░░░  6.5  B   many tests, but PR lacks backend tests and coverage slices are narrow
Maintainability █████░░░░░  5.4  C   several 1000+ line containers/services with mixed responsibilities
Design          ██████░░░░  5.6  B   boundaries mostly clear, fail-fast and SRP uneven in critical paths
Release         ██████░░░░  5.8  B   CI/Docker assets exist, but production defaults and tracked artifacts hurt readiness
─────────────────────────────────────
Overall         ██████░░░░  5.8  B
```

Each dimension scored 0.0-10.0. **Higher = better (10 = clean, 0 = shit mountain).** Scores are judgment-based, not formula-based.

### Finding Statistics

| Severity | Count | Confirmed | Suspected |
|----------|-------|-----------|-----------|
| Critical | 1 | 1 | 0 |
| High | 2 | 2 | 0 |
| Medium | 7 | 7 | 0 |
| Low | 1 | 1 | 0 |
| Info | 0 | 0 | 0 |
| **Total** | **11** | **11** | **0** |

## 2. Project Map

后端位于 `api/`，以 Rust workspace 组织，主要入口包括 `api/apps/api-server`、`api/apps/plugin-runner`，核心业务能力分布在 `api/crates/control-plane`、`api/crates/orchestration-runtime`、`api/crates/storage-durable/postgres`、`api/crates/runtime-core` 等包中。API Server 负责身份、应用、运行时、工作区等 HTTP 接口；control-plane 承担领域服务、应用公开 API、编排运行和观测记录；storage-durable/postgres 是持久化边界；plugin-runner 负责 provider/data source/capability stdio 执行边界。

前端位于 `web/`，以 Vite/React/TypeScript 和 pnpm workspace 组织，`web/app` 是主应用，`web/packages/page-runtime` 等包提供页面运行时和 JS Block 策略。关键用户界面包括 agent-flow 编辑器、frontstage 页面、application monitoring 页面等。前端状态主要由 React local state、Zustand store、React Query、Ant Design Form/Message 等组合承担。

发布与治理位于 `.github/workflows`、`scripts/node`、`docker/`。CI 通过 `verify.yml` 运行 PR 基础门禁，通过 `quality-gate.yml` 和脚本提供更完整的手动/夜间质量门禁；Docker compose 提供一键部署路径；容器镜像 workflow 包含镜像构建与 Trivy 扫描。最容易包含风险的边界是：密钥与 cookie 安全配置、插件 stdio 子进程生命周期、运行时 JSON/fallback 处理、PR 测试门禁、超大 UI/服务文件。

## 3. Top Risks

1. **Critical - Provider secret 使用 XOR 混淆而非认证加密。** 数据库泄露或已知明文结构会显著降低 secret 恢复难度。
2. **High - 生产 Docker compose 保留默认密码和默认 provider secret。** 未配置 `.env` 时可直接以可预测 root/DB 密码运行生产环境。
3. **High - data source/capability stdio 超时后未确保杀死子进程。** 恶意或故障插件可在超时后继续占用进程和资源。
4. **Medium - 会话 Cookie 未设置 Secure。** 生产 HTTPS 场景下浏览器仍允许该 Cookie 通过非 TLS 请求发送。
5. **Medium - 内部错误消息原样返回客户端。** DB、路径、插件、配置等内部细节可能泄露到 API 响应。
6. **Medium - 结构化 LLM 输出解析失败被静默转成 null。** 无效模型输出可能被下游当成合法空值继续处理。
7. **Medium - 前端页面/编辑器容器状态和 effect 链过重。** 页面选择、对话框、保存、设计态和调试面板状态互相耦合，回归风险上升。
8. **Medium - 后端核心服务文件超过 1000 行且职责密集。** 编译、运行、持久化、公开 API 文件成为变更集中点。
9. **Medium - PR CI 未运行后端测试分片，也未运行完整前端测试。** 回归主要依赖夜间/手动流程，PR 合并前信心不足。
10. **Medium - 覆盖率阈值只覆盖少数关键目录/包。** plugin-runner、orchestration-runtime、page-runtime 等运行时关键区域缺少阈值约束。
11. **Low - 仓库中跟踪了本地/生成/环境类文件。** `docker/middleware.env`、`test_dir.txt`、`web/app/tsconfig.tsbuildinfo` 会污染发布与审计信号。

## 4. Detailed Findings

### Finding: Provider secret encryption uses XOR instead of authenticated encryption

- Severity: Critical
- Confidence: High
- Category: Security
- Status: Confirmed
- Affected area: `api/crates/storage-durable/postgres` model provider secret storage
- Evidence:
  - File: `api/crates/storage-durable/postgres/src/model_provider_repository/secret_crypto.rs:3-14`
  - File: `api/crates/storage-durable/postgres/src/model_provider_repository/secret_crypto.rs:44-68`
  - Function / Module: `encrypt_secret_json`, `decrypt_secret_json`, `xor_hex`, `xor_hex_decode`
  - Relevant behavior: secret JSON is serialized, labeled as `algorithm: "xor_v1"`, then XORed with a repeating master key and hex-encoded.
- Problem: Provider secret 存储使用重复密钥 XOR，可逆但不提供现代加密所需的机密性强度、随机 nonce、认证标签或篡改检测。JSON 明文通常有稳定结构，已知明文攻击场景现实。
- Why it matters: 一旦数据库备份、日志、只读凭据或查询权限泄露，攻击者获得密文后可利用固定 JSON 结构恢复 key stream 或 secret 内容。密文被篡改时也没有认证失败信号，可能导致错误 secret 被当作合法数据使用。
- Realistic failure scenario: 生产数据库快照泄露；攻击者看到 provider secret 的 `xor_v1` 密文和已知字段结构，例如 `{`、`api_key`、`secret`；通过重复 XOR 规律恢复部分 key stream，并推导多个 provider secret。
- Minimal fix: 使用 AEAD，例如 ChaCha20-Poly1305 或 AES-GCM；每条 secret 随机 nonce；保存 `algorithm`、`nonce`、`ciphertext`、`tag`；校验 master key 长度；生产环境拒绝占位 key；保留 `xor_v1` 只用于迁移读取。
- Better long-term fix: 引入 key version、rotation、迁移任务和 envelope encryption；将 secret 解密路径收敛到单一服务层，并记录解密失败审计事件。
- Regression test suggestion: 相同明文加密两次密文不同；篡改任一字节解密失败；生产环境使用占位 key 启动失败；旧 `xor_v1` 数据迁移后不再写回 XOR。
- Estimated effort: 1-2 days

### Finding: Production Docker compose keeps predictable secrets and passwords

- Severity: High
- Confidence: High
- Category: Security / Release
- Status: Confirmed
- Affected area: Docker deployment configuration
- Evidence:
  - File: `docker/docker-compose.yaml:40-50`
  - File: `docker/docker-compose.yaml:60-63`
  - Function / Module: Compose environment defaults
  - Relevant behavior: `API_ENV` defaults to `production`, while DB password defaults to `1flowbase`, provider secret defaults to `change-me-provider-secret-master-key`, bootstrap root password defaults to `1flowbase`.
- Problem: 生产 compose 文件允许在未提供 `.env` 或环境变量时以可预测凭据启动。虽然 `docker/.env.example` 和部署脚本可能提示修改，这个 compose 本身仍是可直接运行的生产配置。
- Why it matters: 默认 root/DB/provider secret 是公开可见的部署风险。任何跳过初始化脚本、复制 compose 片段或误配 CI/CD 的部署都可能暴露默认管理员账号和可预测加密 key。
- Realistic failure scenario: 运维直接执行 `docker compose up -d`；API 以 `production` 运行并创建默认 root；公网或内网攻击者使用 `root` / `1flowbase` 登录，随后访问工作区或 provider 配置。
- Minimal fix: 生产 compose 中对 secret 使用 `${VAR:?message}` 强制要求配置；移除密码类 fallback；为本地开发提供单独 `docker-compose.dev.yaml`。
- Better long-term fix: 部署脚本生成强随机 `.env` 并在 compose config 前做 schema 校验；发布文档明确禁止直接使用默认生产 compose。
- Regression test suggestion: 在无 `.env` 情况下运行 `docker compose -f docker/docker-compose.yaml config` 应失败；带完整 secret 的配置应成功。
- Estimated effort: 2-4 hours

### Finding: Data source and capability stdio timeout does not guarantee child termination

- Severity: High
- Confidence: High
- Category: Stability / Performance
- Status: Confirmed
- Affected area: `api/apps/plugin-runner`
- Evidence:
  - File: `api/apps/plugin-runner/src/data_source_stdio.rs:17-21`
  - File: `api/apps/plugin-runner/src/data_source_stdio.rs:37-49`
  - File: `api/apps/plugin-runner/src/capability_stdio.rs:49-53`
  - File: `api/apps/plugin-runner/src/capability_stdio.rs:69-81`
  - File: `api/apps/plugin-runner/src/stdio_runtime.rs:264-269`
  - Function / Module: `data_source_stdio::call_executable`, `capability_stdio::call_executable`, provider `stdio_runtime::call_executable`
  - Relevant behavior: data source/capability wrap `child.wait_with_output()` in `tokio::time::timeout`, but do not call `.kill_on_drop(true)` or explicitly kill on timeout; provider stdio does set `.kill_on_drop(true)`.
- Problem: Timeout 返回错误后，等待 future 被丢弃，但子进程没有被明确终止。provider stdio 的实现已经显示项目知道需要 `kill_on_drop`，但 data source/capability 两条路径遗漏。
- Why it matters: 故障或恶意插件可以超过 timeout 后继续运行，累计进程、内存、文件句柄或 CPU 占用，最终影响 plugin-runner 和 API 可用性。
- Realistic failure scenario: 一个 data source executable 内部 `sleep` 或死循环；请求超时报错返回；子进程继续存活；高并发重试后 runner 主机进程表和内存耗尽。
- Minimal fix: 在 data source/capability `Command` 构建时同样设置 `.kill_on_drop(true)`；或 timeout 分支显式 `child.kill().await` 并等待回收。
- Better long-term fix: 抽出统一 stdio child runner，集中处理 timeout、kill、stderr、内存限制和退出码，减少 provider/data source/capability 三条路径漂移。
- Regression test suggestion: 使用临时可执行脚本超过 50ms timeout，断言调用失败且进程不再存在；provider/data source/capability 三条路径共用同一测试夹具。
- Estimated effort: 2-6 hours

### Finding: Session cookies are not marked Secure in production path

- Severity: Medium
- Confidence: High
- Category: Security
- Status: Confirmed
- Affected area: API identity/session routes
- Evidence:
  - File: `api/apps/api-server/src/routes/identity/auth.rs:87-91`
  - File: `api/apps/api-server/src/routes/identity/session.rs:67-72`
  - Function / Module: login cookie creation, expired session cookie
  - Relevant behavior: cookies are `http_only`, `SameSite::Lax`, and `path("/")`, but no `.secure(true)` or environment-based secure configuration is applied.
- Problem: 生产环境会话 Cookie 未被浏览器限制为仅 HTTPS 发送。项目已有 CSRF 机制，不应把这个问题误报为缺失 CSRF；这里的问题是传输安全标志缺失。
- Why it matters: 在 TLS 终止、代理、错误 HTTP 跳转或混合内容场景中，未设置 Secure 的会话 Cookie 更容易通过非加密请求泄露。
- Realistic failure scenario: 用户登录 HTTPS 控制台后访问同域 HTTP 链接或被代理错误重定向；浏览器可能附带会话 Cookie，导致 session id 暴露。
- Minimal fix: 在配置中加入 `cookie_secure`，生产默认 true；login 和 expired cookie 都设置 `.secure(cookie_secure)`。
- Better long-term fix: 统一 session cookie builder，集中设置 name、path、httpOnly、SameSite、Secure、TTL，并用环境配置驱动。
- Regression test suggestion: production 配置下登录响应 Set-Cookie 包含 `Secure`；development 配置下可按本地调试需要关闭；删除 session 的过期 cookie 同样带 Secure。
- Estimated effort: 1-3 hours

### Finding: Internal error messages are returned to clients

- Severity: Medium
- Confidence: High
- Category: Security / Backend API
- Status: Confirmed
- Affected area: API error response adapter
- Evidence:
  - File: `api/apps/api-server/src/error_response.rs:29-60`
  - Function / Module: `ApiError::into_response`
  - Relevant behavior: unknown errors map to HTTP 500 with code `internal_error`, but response body `message` is always `self.0.to_string()`.
- Problem: 内部错误的详细字符串会进入客户端响应。`anyhow` 错误可能包含数据库 URL、文件路径、插件路径、上游响应、配置名或其它内部细节。
- Why it matters: 错误消息泄露会帮助攻击者枚举内部结构，也会把敏感配置意外暴露给普通客户端或前端日志采集。
- Realistic failure scenario: 某个 handler 返回 `anyhow!("failed to connect to postgres://...")`；API 响应 `500 internal_error` 但 message 中包含数据库连接细节；前端错误上报保存该字符串。
- Minimal fix: 对 `internal_error` 返回固定通用 message，例如 `internal server error`；详细错误只写服务端日志并带 request id。
- Better long-term fix: 定义结构化 API 错误类型：可公开错误与内部错误分层；所有 route 只暴露 allowlisted message。
- Regression test suggestion: 构造 `ApiError(anyhow!("secret path /tmp/x"))`，断言响应 code 为 `internal_error` 且 message 不包含原始错误文本。
- Estimated effort: 1-3 hours

### Finding: Structured LLM output parse failure silently becomes null

- Severity: Medium
- Confidence: High
- Category: Stability / Fallback
- Status: Confirmed
- Affected area: Orchestration runtime final LLM content parsing
- Evidence:
  - File: `api/crates/orchestration-runtime/src/execution_engine/llm_final_content.rs:3-5`
  - File: `api/crates/orchestration-runtime/src/code_runtime.rs:530-537`
  - File: `api/crates/control-plane/src/runtime_observability.rs:40-47`
  - Function / Module: `parse_structured_llm_output`, console log parsing, runtime observability hash
  - Relevant behavior: structured LLM output JSON parse uses `unwrap_or(Value::Null)`; other observability/debug paths also convert parse/serialization failure into empty/default values.
- Problem: 结构化输出是业务数据边界，解析失败不应静默变成合法 JSON `null`。调试/观测路径的默认值有时可以接受，但执行输出边界应 fail-fast 或至少记录明确错误。
- Why it matters: 下游节点、前端展示或持久化可能把 `null` 当成模型真实输出，掩盖 provider 响应格式错误，增加排障成本，并可能触发错误分支。
- Realistic failure scenario: LLM 返回带说明文字的非 JSON 内容；运行时把它解析成 `null`；后续节点按空对象/空值继续执行；用户看到的是业务结果异常而非 provider 输出格式错误。
- Minimal fix: 将 `parse_structured_llm_output` 改为返回 `Result<Value>`，解析失败时生成明确的 node error payload。
- Better long-term fix: 为结构化输出建立 schema validation、原始输出保留、错误分类和可观测字段，区分 provider invalid response 与业务空值。
- Regression test suggestion: 输入非法 JSON 应使节点运行失败并包含 parse error code；合法 `null` 只有模型显式返回 `null` 时才被接受。
- Estimated effort: 4-8 hours

### Finding: Frontend page/editor containers carry too much local state and effect coupling

- Severity: Medium
- Confidence: High
- Category: Maintainability / Frontend State
- Status: Confirmed
- Affected area: Frontstage page and agent-flow editor
- Evidence:
  - File: `web/app/src/features/frontstage/pages/FrontStagePage.tsx:119-148`
  - File: `web/app/src/features/frontstage/pages/FrontStagePage.tsx:310-420`
  - File: `web/app/src/features/agent-flow/components/editor/AgentFlowCanvasFrame.tsx:102-196`
  - Function / Module: `FrontStagePage`, `AgentFlowCanvasFrame`
  - Relevant behavior: `FrontStagePage` is 1199 lines and holds page tree operations, selected block state, dialogs, design mode, save state and JS Block trial state; `AgentFlowCanvasFrame` is 1191 lines with many store selectors, refs, local dock resize states and mutations.
- Problem: 页面容器同时管理导航解析、对话框开关、保存状态、设计权限、block selection、trial panel 和编辑器面板尺寸，导致 effect 链之间隐式耦合。
- Why it matters: 小变更容易触发非目标状态重置，例如切页、切设计态、权限刷新或内容刷新时关闭错误面板、清空错误、丢失选择。测试也难以覆盖所有组合状态。
- Realistic failure scenario: 增加一个 block 配置面板状态后，某个 `selectedPageId` 或 `pageContent` effect 在保存/刷新时重置所有 panel，用户正在编辑的配置被关闭或错误提示消失。
- Minimal fix: 优先抽出 `useFrontstagePageSelection`, `useFrontstageBlockPanels`, `useFrontstagePageContentSave` 等 hooks；agent-flow 抽出 dock resize/controller hooks，减少主组件直接状态数量。
- Better long-term fix: 为页面编辑态引入显式 reducer/state machine，按 page selection、block editing、trial/debug、save lifecycle 分层；UI 组件只消费状态和 dispatch。
- Regression test suggestion: hook 级测试覆盖切页、权限变化、pageContent 刷新、退出设计态时各 panel/selection 的保留与清理规则。
- Estimated effort: 2-5 days

### Finding: Backend core files exceed manageable size and concentrate responsibilities

- Severity: Medium
- Confidence: High
- Category: Maintainability / Backend API
- Status: Confirmed
- Affected area: Orchestration compiler, public run service, API runtime routes, persistence
- Evidence:
  - File: `api/crates/orchestration-runtime/src/compiler.rs` has 1342 lines
  - File: `api/crates/control-plane/src/application_public_api/run_service.rs` has 1314 lines
  - File: `api/apps/api-server/src/routes/applications/application_runtime.rs` has 1209 lines
  - File: `api/crates/control-plane/src/orchestration_runtime/persistence.rs` has 1208 lines
  - Function / Module: compiler, public API run service, application runtime routes, orchestration persistence
  - Relevant behavior: multiple critical backend files are above 1000 lines, close to the project AGENTS limit of 1500 lines and above the skill rubric's maintainability warning threshold.
- Problem: 这些文件位于编译、公开运行、API 路由和持久化核心路径，体积大且职责密集。虽然没有单个文件超过项目硬性 1500 行建议，但已经达到维护压力区。
- Why it matters: 核心路径大文件会提升 review 难度、测试定位成本和局部修改误伤概率；新规则容易继续塞进同一文件，形成更高耦合。
- Realistic failure scenario: 修改公开运行 API 的鉴权或输入校验时，同一文件还承载执行、状态、响应组装逻辑；review 难以识别某个状态写入顺序回归。
- Minimal fix: 按已有模块边界拆分 helper/service，而不是重写：例如 run request validation、runtime response assembly、persistence mapping、compiler pass 分文件。
- Better long-term fix: 为 orchestration 编译和 public run 建立明确阶段对象和测试夹具，每个阶段有单独输入输出合同。
- Regression test suggestion: 拆分前先固定现有 compiler/run/persistence 行为测试；拆分后运行相同夹具，确保 DTO 和状态写入顺序不变。
- Estimated effort: 3-7 days

### Finding: PR verification does not run backend tests or full frontend tests

- Severity: Medium
- Confidence: High
- Category: Testing / Release
- Status: Confirmed
- Affected area: GitHub Actions verify workflow
- Evidence:
  - File: `.github/workflows/verify.yml:74-84`
  - File: `.github/workflows/verify.yml:100-119`
  - File: `scripts/node/verify/index.js:1030-1048`
  - Function / Module: `verify.yml`, repo quality gate scopes
  - Relevant behavior: PR backend matrix runs static/fmt/check scopes but not backend test scopes; PR frontend scope is `repo-frontend-pr`, while full/page-regression commands exist separately.
- Problem: PR 合并前的自动化更多验证编译、格式和静态检查，缺少后端行为测试分片，也没有覆盖完整前端/page-regression 测试。夜间/手动质量门禁是正向补充，但不能替代 PR 级回归保护。
- Why it matters: 状态机、Repository、API handler、插件运行路径等行为回归可能在 PR 阶段漏过，延后到夜间或人工发现，修复成本更高。
- Realistic failure scenario: PR 修改 `control-plane` 状态写入口，通过 cargo check 和前端 PR 测试；未运行相关 Rust tests；合并后夜间才发现状态迁移断言失败。
- Minimal fix: 在 PR 加入轻量后端 test smoke shard，例如核心 crate/unit tests 或路径敏感 test shards；对触及关键前端目录的 PR 运行 page-regression 子集。
- Better long-term fix: 建立 path-aware test selection 和 merge queue full gate，让小 PR 快速反馈，大 PR 自动升级测试范围。
- Regression test suggestion: 为 verify scope 配置加脚本测试，断言 PR expected scopes 包含至少一个 backend test scope。
- Estimated effort: 1-2 days

### Finding: Coverage thresholds cover only a narrow subset of critical runtime areas

- Severity: Medium
- Confidence: High
- Category: Testing
- Status: Confirmed
- Affected area: Coverage governance
- Evidence:
  - File: `scripts/node/testing/coverage-thresholds.js:3-24`
  - File: `scripts/node/testing/coverage-thresholds.js:26-30`
  - Function / Module: frontend/backend coverage thresholds
  - Relevant behavior: frontend thresholds only include `agent-flow` and `settings`; backend thresholds only include `control-plane`, `storage-postgres`, `api-server`.
- Problem: 关键运行时区域未被阈值治理覆盖，例如 `plugin-runner`、`orchestration-runtime`、`runtime-core`、`web/packages/page-runtime`。仓库测试文件数量较多，这是优势，但 coverage gate 的约束面仍偏窄。
- Why it matters: 没有阈值的关键包可以在新增逻辑时没有对应测试，CI 仍然显示通过；长期会导致风险最高的 runtime 边界反而缺少量化保护。
- Realistic failure scenario: 修改 plugin-runner timeout 或 page-runtime JS Block 策略时未新增测试；coverage gate 不看这些包，PR 只靠 reviewer 发现。
- Minimal fix: 增加 plugin-runner、orchestration-runtime、page-runtime 的初始阈值；若短期无法达标，先设置低门槛和明确提升计划。
- Better long-term fix: 将 coverage threshold 与风险等级绑定，关键 runtime/security 包有更高阈值，并在质量报告中解释显式豁免。
- Regression test suggestion: scripts/node 的 coverage config test 断言关键包/目录都在 thresholds 中；缺失时输出 warning 到 `tmp/test-governance/`。
- Estimated effort: 4-8 hours

### Finding: Tracked local/generated/environment files pollute repository hygiene

- Severity: Low
- Confidence: High
- Category: Release / Code Consistency
- Status: Confirmed
- Affected area: Version-control hygiene
- Evidence:
  - File: `docker/middleware.env:1-4`
  - File: `test_dir.txt`
  - File: `web/app/tsconfig.tsbuildinfo`
  - Function / Module: tracked repository files
  - Relevant behavior: `git ls-files` shows these files are tracked; `docker/middleware.env` includes `POSTGRES_PASSWORD=1flowbase`; `tsconfig.tsbuildinfo` is a TypeScript build artifact.
- Problem: 本地环境文件、scratch 文件和生成文件被纳入版本控制。当前 `docker/middleware.env` 的密码是默认值而非真实 secret，但它仍会弱化仓库对 env 文件的治理规则。
- Why it matters: 这类文件会制造发布审计噪音，增加误提交真实 secret 的概率，也可能导致不同机器上的构建缓存状态进入 diff。
- Realistic failure scenario: 开发者以为 env 文件可以提交，在 `docker/middleware.env` 中加入真实中间件密码；或 `tsconfig.tsbuildinfo` 因机器差异产生无意义变更，干扰 review。
- Minimal fix: 将 `docker/middleware.env` 改为 `.example` 或从索引移除；删除 `test_dir.txt` 和 `web/app/tsconfig.tsbuildinfo` 的跟踪状态；更新 `.gitignore`/repo hygiene 规则。
- Better long-term fix: 扩展 repo-hygiene：非 `.example` env 文件、`*.tsbuildinfo`、根目录 scratch 文件统一 warning，并把产物落到 `tmp/test-governance/`。
- Regression test suggestion: repo hygiene 测试应捕获 tracked `*.env`、`*.tsbuildinfo`、根目录临时文本文件。
- Estimated effort: 30-90 minutes

## 5. Security Concerns

最重要的安全问题是 secret 加密和生产默认值。`xor_v1` 需要优先替换为 AEAD；生产 compose 需要强制 secret 输入，避免一键误部署。Cookie Secure 和内部错误消息泄露属于中等严重度，但实现成本低，建议与前两项同批修复。

正向信号：身份/session 路径存在 HttpOnly 与 SameSite；项目有 CSRF 中间件和相关路由保护；公开应用 API 路径有 bearer token 和发布状态校验；前端 JS Block source policy 禁用 `eval`、`fetch`、`WebSocket`、`localStorage` 等高风险能力，并有对应测试。

## 6. Stability Concerns

插件 stdio 子进程生命周期是当前最明确的稳定性风险。provider stdio 已设置 `kill_on_drop(true)`，说明修复方向清晰，data source 和 capability 应补齐一致策略。另一个稳定性问题是结构化输出解析失败静默变成 `null`，这会把 provider 契约错误延后成业务异常。

其它 fallback 例子如 console logs 解析失败返回空数组、观测 hash 序列化失败使用默认 bytes，风险低于执行输出边界；可以保留但建议至少记录 debug/trace 级事件。

## 7. Performance Concerns

未发现明显的同步阻塞热点或前端 bundle 灾难证据。性能风险主要来自稳定性问题的资源外溢：超时插件子进程不终止会累积 CPU/内存/进程资源，最终表现为性能退化和可用性问题。前端依赖如 Ant Design、Monaco、XYFlow、ECharts/图表相关依赖体量较大，但与产品域匹配，且 Vite 配置中存在 chunk 管理，暂不作为独立缺陷。

## 8. Testing Gaps

项目有大量测试文件和质量脚本，本次粗略扫描 `api`、`web`、`scripts` 下测试/规格文件数量为 1164，这是明显优点。主要缺口不是“没有测试”，而是 PR 门禁和覆盖率治理没有覆盖足够多的关键行为路径。

优先补齐：PR 后端 smoke test shard；plugin-runner timeout 生命周期测试；structured LLM output 非法 JSON 测试；production cookie Secure/error redaction 测试；coverage threshold 增加 plugin-runner、orchestration-runtime、page-runtime。

## 9. Maintainability Concerns

前端维护性压力集中在大型页面/编辑器容器，后端维护性压力集中在运行、编译、持久化和公开 API 大文件。当前还不是无法维护的状态，因为项目模块边界总体存在，测试资产也多；但如果继续把新规则追加到这些文件，后续 review 与回归成本会快速增加。

建议按风险驱动拆分，不做一次性大重构。每次只围绕一个用户可观察行为或一个领域阶段拆出 hook/service/module，并把现有行为测试固定住。

## 10. Type Safety Concerns

Rust 后端整体受类型系统保护较好，未在本轮发现需要标为 Critical/High 的 unsafe 或大规模裸类型问题。真正需要关注的是边界类型：内部错误被 `anyhow` 包装后直接暴露为 API message，结构化 LLM 输出解析从 `Result` 降级为 `Value::Null`，这两处都是类型边界失去语义的问题。

前端 TypeScript 中没有在本轮形成可确认的高危类型断言结论；建议后续若做专项 type-safety 审计，再集中扫描 `as any`、非空断言、DTO 字段别名和动态 i18n key。

## 11. Release Concerns

发布治理有基础：GitHub Actions 有 verify workflow、质量门禁聚合、容器镜像 workflow 和扫描；Docker runtime 镜像有非 root 运行的积极迹象。主要短板是生产 compose 默认 secret 和 tracked env/build artifact。它们会让“可直接部署”的路径和“仓库干净度”低于项目已有治理水平。

## 12. Fallback / Defensive Code Analysis

### Fallback Summary

| Subtype | Count | KeepWithAlert | FailFast | Remove |
|---------|-------|---------------|----------|--------|
| SilentFallback | 3 | 2 | 1 | 0 |
| EmptyCatch | 0 | 0 | 0 | 0 |
| CompatibilityBranch | 1 | 1 | 0 | 0 |
| SilentCorrection | 0 | 0 | 0 | 0 |
| DefensiveGuess | 1 | 0 | 1 | 0 |

最需要 fail-fast 的 fallback 是 `parse_structured_llm_output`。console log fallback 和 observability hash fallback 可以保留，但建议增加可观测记录。生产 secret fallback 属于 defensive/default guess，应移除，改为显式配置失败。

## 13. Testing Authenticity Analysis

### Confidence Assessment

| Test Area | Real Confidence | Risk | Action |
|-----------|-----------------|------|--------|
| Repo PR backend gate | Medium-Low | 行为回归可能只在夜间/手动流程暴露 | Add backend smoke tests |
| Frontend PR gate | Medium | page-regression/full 测试未在 PR 常规运行 | Path-aware upgrade |
| Plugin runner timeout | Low | 子进程泄漏未被测试捕获 | Add process lifecycle tests |
| Secret encryption | Low-Medium | 可能只验证可往返，不能验证加密强度 | Add AEAD/security property tests |
| JS Block source policy | High | 禁用高危 API 的测试证据较强 | Keep |

### Valuable Tests

JS Block source policy、CSRF/session 相关路由测试、repo quality gate 脚本测试和已有大量前端 feature 测试是有价值资产。它们说明项目不是“绿勾装饰”，而是已有可演进的测试治理基础。

### Suspicious Tests

本轮未逐个审查测试实现，因此不列出具体过度 mock 测试。风险在于 CI 选择范围，而不是单个测试虚假通过的已确认案例。

### Missing Tests

缺失的关键测试包括：AEAD 加密属性测试、生产默认 secret 启动失败测试、session Secure cookie 测试、internal error redaction 测试、stdio timeout kill 测试、invalid structured LLM output failure 测试、coverage threshold 配置完整性测试。

## 14. Frontend State Analysis

### Summary

| Subtype | Count | Affected Components |
|---------|-------|---------------------|
| ComponentSize | 3 | `FrontStagePage`, `AgentFlowCanvasFrame`, `ApplicationMonitoringPage` |
| StateDuplication | 2 | frontstage block panels, agent-flow dock resize states |
| EffectCoupling | 2 | page selection/content refresh/design mode reset, editor resize/panel lifecycle |
| StoreSelectorDensity | 1 | `AgentFlowCanvasFrame` |

前端最大风险是容器层吸收太多局部状态。短期可通过 hooks 降低复杂度；长期建议 reducer/state machine 明确 page/block/editor 面板生命周期。

## 15. Backend API Analysis

后端 API 边界总体比前端状态更清晰：identity、applications、control-plane、storage、runtime 分层存在，公开应用 API 有 token 和 enabled publication 检查。需要修正的是错误响应边界和大文件职责密度。API 层不应把内部 `anyhow` message 直接当公开 DTO 字段，公开运行服务也应逐步拆出验证、执行、响应组装和持久化阶段。

## 16. Dependency Weight Analysis

未发现明显“依赖为装饰而引入”的证据。前端重依赖与产品域相关：Ant Design 支撑业务 UI，XYFlow 支撑流程编辑，Monaco 支撑代码编辑，ECharts/图表能力支撑报表和运行视图。建议持续关注 chunk、懒加载和页面级引入，避免把编辑器/图表依赖带入不需要的首屏路径。

## 17. Code Consistency / Comment Coverage

代码一致性问题主要是仓库 hygiene：tracked env/build/scratch 文件。注释覆盖没有发现严重“无注释导致无法理解”的确认问题；真正需要的是在复杂状态迁移、secret migration、runtime fallback 和 stdio lifecycle 处补充少量契约注释，而不是扩大普通解释性注释。

---

## 18. Principles Compliance

### Principles Violated

| Principle | Violations | Severity | Affected Areas |
|-----------|------------|----------|----------------|
| Fail-Fast | 3 | High | secret defaults, structured output parse, internal error boundary |
| Secure by Default | 4 | Critical/High | XOR secret, compose defaults, cookie Secure, error response |
| Single Responsibility (SRP) | 4 | Medium | `FrontStagePage`, `AgentFlowCanvasFrame`, compiler, public run service |
| File Size Limit | 7 | Medium | 1000+ line frontend/backend files |
| Resource Lifecycle Ownership | 1 | High | plugin-runner data source/capability stdio |
| Test Gate Authenticity | 2 | Medium | PR backend tests, coverage threshold scope |
| Repository Hygiene | 1 | Low | tracked env/generated/scratch files |

### Principles Respected

项目尊重了多个重要原则：后端按 API/server、control-plane、storage、runtime、plugin-runner 划分边界；前端有 workspace/package 结构和 feature 目录；CI 有集中质量门禁和测试治理脚本；CSRF 和公开 API token 校验显示安全边界有基础；JS Block source policy 对浏览器危险能力有明确 deny list 和测试；Docker/CI 发布链路不是空白。

---

## 19. Recommended Fix Order

1. 替换 provider secret `xor_v1` 为 AEAD，并加 key version/迁移测试。
2. 移除 production compose 的默认密码和默认 provider secret，改为必填变量。
3. 为 data source/capability stdio 增加 `kill_on_drop(true)` 或 timeout kill 回收测试。
4. 生产 session cookie 设置 Secure，内部错误响应做 redaction。
5. 结构化 LLM 输出解析失败改为明确运行错误。
6. PR 加入后端 smoke tests，coverage thresholds 扩到 plugin-runner/orchestration-runtime/page-runtime。
7. 清理 tracked env/generated/scratch 文件并补 repo hygiene warning。
8. 以 hook/service/module 为单位拆分最高风险前端容器和后端大文件。

## 20. Quick Wins

- `auth.rs` 和 `session.rs` 统一 cookie builder，加 production Secure 测试。
- `error_response.rs` 对 `internal_error` 返回固定 message。
- `data_source_stdio.rs` 和 `capability_stdio.rs` 补齐 `.kill_on_drop(true)`。
- `docker/docker-compose.yaml` secret/password fallback 改为 `${VAR:?required}`。
- `scripts/node/testing/coverage-thresholds.js` 增加关键 runtime 初始阈值。
- de-track `web/app/tsconfig.tsbuildinfo`、`test_dir.txt` 和非 example env 文件。

## 21. Long-term Refactor Plan

第一阶段，修安全与稳定边界，目标是一周内消除 Critical/High：AEAD、生产配置必填、stdio timeout kill、cookie/error redaction。第二阶段，提升测试门禁，把关键 runtime 的行为测试纳入 PR 或 path-aware gate，并让 coverage threshold 反映真实风险。第三阶段，维护性减压：先拆 frontstage 页面选择/block panel/save hooks，再拆 agent-flow dock/editor controller，后端按 compiler pass、public run validation/execution/response/persistence mapping 分割。每次拆分都应有旧行为夹具或 snapshot/DTO 测试保护，不建议无测试大规模重排。
