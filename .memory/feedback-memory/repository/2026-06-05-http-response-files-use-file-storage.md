---
memory_type: feedback
feedback_category: repository
topic: http_response_files_use_file_storage
summary: HTTP 请求节点等 runtime 产生的业务文件，生产与控制面执行路径应优先复用既有文件表、文件上传服务和对象存储 driver，不能用 runtime-inline descriptor 作为生产边界；文本/JSON/JS 响应默认不转文件，只有显式开启“转存为文件”时才转存。
keywords:
  - http-request-node
  - runtime-files
  - file-table
  - object-storage
  - file-upload-service
  - runtime-inline
match_when:
  - 实现或评审 HTTP 请求节点的 binary 请求或响应文件处理
  - 设计 runtime 节点输出 `Array[File]`、文件变量或调试产物持久化边界
  - 权衡是否返回 inline descriptor、临时 URL、文件表记录或对象存储 URL
created_at: 2026-06-05 19
updated_at: 2026-06-06 23
last_verified_at: 2026-06-06 23
decision_policy: direct_reference
scope:
  - api/crates/orchestration-runtime
  - api/crates/control-plane
  - api/crates/storage-object
  - api/apps/api-server
---

# HTTP Response Files Use File Storage

## 规则

当 HTTP 请求节点或其他 runtime 节点产生业务文件输出时，生产、调试和发布运行路径应优先走现有文件表、文件上传服务和对象存储 driver，返回真实文件记录及可用访问 URL（若 storage driver/config 支持）。

纯 runtime 单元测试或无宿主注入场景可以保留 `runtime-inline` descriptor 作为测试/降级兜底，但不能把它作为生产 contract 或完成边界。

HTTP 响应正文的默认分流规则是：文本、JSON、XML、JavaScript、form-urlencoded 等字符串型响应记录在节点输出/数据库的 `body` 中，`files` 默认为空；二进制或非 inline 文本响应才默认生成文件输出。字符串型响应只有在 HTTP 节点显式开启“转存为文件”开关时，才额外持久化为文件记录；该开关默认关闭。

## 原因

项目已经有 `file_storages`、`file_tables`、`FileUploadService` 和对象存储 driver。绕开这些能力会丢失存储绑定、文件记录、权限 grant、对象路径和后续读取能力，也会和用户对 `Array[File]` 输出的业务预期冲突。

## 适用场景

HTTP 请求节点文本/JSON/JS response、binary response、binary body、multipart file、runtime 文件变量、调试输出文件以及任何需要把节点执行结果交给后续节点消费的文件输出。
