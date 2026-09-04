# 工程级审查 — 任务切分设计

## Architecture and boundaries

父任务是规划根，不是实现目标。实现落在子任务。每个子任务改动的层：

| 子任务 | 主要层 | 不碰 |
|---|---|---|
| at-rest-crypto | services/security, settings, evaluation, schema migration | UI 布局 |
| webview-hardening | tauri.conf, commands/window, portable dest, invoke_handler | SSRF 策略细节 |
| outbound-ssrf | network_safety, ai, sync, media, evaluation redirect | 主密码格式 |
| sqlite-backup-fts | storage, sync backup, startup FTS | 前端 store |
| frontend-store-races | src/runtime, promptStore, evaluationStore | Rust SQL |
| logging-error-taxonomy | logging, state mutex, ErrorCode mapping | 加密信封 |
| library-eval-integrity | portable zip/tx, schema FK, prompt list DTO | CSP |

## Data flow that the audit cares about

```
UI → runtime.invoke(domain.action) → commands → services → rusqlite pool
UI ← runtime.on(event) ← commands/events.rs
Outbound HTTP: evaluation/media → prepare_public_url
Outbound HTTP: ai + webdav/s3 → raw reqwest (gap)
Secrets: master verifier + ENC:: in same SQLite; settings.sync.password plaintext
```

## Compatibility

- 错误码字符串是稳定契约（`ErrorCode::as_str`）。子任务改码必须同步前端分支。
- Schema 变更必须加 `MIGRATIONS` 项并写升级测试。当前 `CURRENT_SCHEMA_VERSION = 6`。
- 不削弱 `SSRF_BLOCKED`。允许私网的唯一方式是显式产品开关，默认关。

## Trade-offs

- 神模块拆分（evaluation/prompt）与安全修复解耦。先修行为，再拆文件。
- 未接线命令：已确认从 handler 删除 `ai.*` 与低层 sync 传输；`rules.*` / `media.*` 保留。
- 备份使用 `VACUUM INTO` / SQLite backup API 会拉长实现，但避免 WAL 撕裂。不允许继续 `copy_dir` 活库。

## Rollout / rollback

每个子任务自己提交。父任务不发单一巨型 PR。回滚以子任务 commit 为单位。加密格式变更必须带向前迁移；失败则拒绝启动并保留备份。
