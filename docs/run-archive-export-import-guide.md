# 运行归档导出导入指南

## 概述

1flowbase 提供两种运行记录导出格式：

### 1. Trace Dump（调试格式）
- **用途**: 排障和调试，提供可读性强的 JSON 或 ZIP 格式
- **导出端点**: 
  - 单个: `GET /api/console/applications/{id}/logs/runs/{run_id}/export`
  - 批量: `POST /api/console/applications/{id}/logs/runs/export`
- **文件格式**: 
  - 单个: `.json`
  - 批量: `.zip`（包含多个 JSON 文件）
- **UI 标识**: DownloadOutlined 图标（下载图标）
- **是否可导入**: ❌ 否

### 2. Archive（归档格式）
- **用途**: 正式的运行记录归档，可以导入恢复
- **导出端点**:
  - 单个: `GET /api/console/applications/{id}/logs/runs/{run_id}/archive`
  - 批量: `POST /api/console/applications/{id}/logs/runs/archive`
- **文件格式**: `.json` (包含 archive_version、manifest、entries 等完整合约字段)
- **UI 标识**: DatabaseOutlined 图标（数据库图标）
- **是否可导入**: ✅ 是

## 如何使用

### 导出 Archive（可导入格式）

1. **单个运行归档**:
   - 在运行日志列表中，每一行的操作列有一个 **数据库图标** 按钮
   - 点击后会下载一个 `.json` 文件，格式如：`{run-id}-archive.json`

2. **批量运行归档**:
   - 在运行日志列表上方，选择多个运行
   - 点击工具栏中的 **数据库图标** 按钮（不是下载图标）
   - 下载的文件格式如：`1flowbase-runs-{app-id}-{timestamp}-{N}runs.json`

### 导入 Archive

1. 点击运行日志列表上方工具栏中的 **上传图标** 按钮
2. 选择之前导出的 `.json` archive 文件（**不是** .zip trace dump 文件）
3. 等待上传和处理完成
4. 导入成功后，恢复的运行会出现在运行日志列表中

### Archive 文件格式

导出的 archive JSON 包含以下关键字段：

```json
{
  "archive_version": 1,
  "manifest": {
    "archive_version": 1,
    "archive_semantics": "application_run_archive_v1",
    "run_count": 1,
    "selected_run_ids": ["..."],
    "content_sha256": "sha256:...",
    "checksum": "sha256:...",
    "entries": [...]
  },
  "source": {
    "application_id": "...",
    "source_kind": "application_run",
    "workspace_id": "...",
    "exported_by_user_id": "..."
  },
  "exported_at": "2026-06-24T07:58:00.458337961Z",
  "content_digest": "sha256:...",
  "entries": [
    {
      "source_run_id": "...",
      "flow_run": {...},
      "node_runs": [...],
      "events": [...],
      "checkpoints": [...],
      "callback_tasks": [...],
      "runtime_spans": [...],
      "runtime_items": [...]
    }
  ]
}
```

## 常见问题

### Q: 为什么我导出的 .zip 文件无法导入？

A: `.zip` 文件是 **trace dump 格式**，仅用于调试，不能导入。请使用 **数据库图标** 按钮导出 archive 格式（`.json` 文件）。

### Q: 如何区分两种导出按钮？

A: 
- **Trace Dump**: DownloadOutlined（普通下载图标）→ 导出 .zip → 不可导入
- **Archive**: DatabaseOutlined（数据库图标）→ 导出 .json → 可导入

### Q: 导入后的运行 ID 会变吗？

A: 是的。导入时会生成新的运行 ID，但会保留 source_run_id 映射关系。重复导入同一个 archive 会创建新的运行实例。

### Q: 可以跨 application 导入吗？

A: 可以。Archive 可以导入到不同的 application，系统会处理必要的 ID 映射和引用。

### Q: 导入失败怎么办？

A: 检查：
1. 文件格式是否正确（必须是 archive .json，不是 trace dump .zip）
2. 文件是否完整（包含 archive_version、manifest、entries）
3. checksum 是否匹配
4. 浏览器控制台是否有详细错误信息

## 技术细节

### Archive Contract v1

- **版本**: 1
- **语义**: `application_run_archive_v1`
- **完整性验证**: 使用 SHA-256 checksum
- **导入行为**: 
  - 生成新的 target run IDs
  - 保留 source → target 映射
  - 重复导入创建新批次
  - 导入后的运行出现在正式日志列表

### 相关测试

后端测试位置：
- `api/apps/api-server/src/_tests/application/application_runtime_routes/logs_routes/export.rs`
  - `application_runtime_routes_logs_archive_import_restores_visible_target_runs`
  - `application_runtime_routes_logs_archive_import_accepts_json_reserialized_export`
  - `application_runtime_routes_logs_archive_import_rejects_checksum_mismatch`

## 更新历史

- 2026-06-24: 修复 archive 导出按钮图标混淆问题（从 FileZipOutlined 改为 DatabaseOutlined）
- 2026-06-24: 实现 run archive import/export 完整功能
