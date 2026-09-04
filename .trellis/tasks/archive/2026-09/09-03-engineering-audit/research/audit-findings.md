# PromptHub 工程级审查发现（2026-09-03）

Tier: `deep`. Scope: first-party `src/**`, `src-tauri/src/**`, capabilities, `tauri.conf.json`.

Severity: critical / high / medium / low / info. Effort: S / M / L.

## High and above

| ID | Dim | File:Line | Sev | Effort | Description | Child |
|---|---|---|---|---|---|---|
| SEC-001 | Security | `src-tauri/src/services/security.rs:11-15,127-130` | high | L | `settings.master_password.hash` 就是 AES 密钥，与 `ENC::` 同库。读库即可解密。 | at-rest-crypto |
| SEC-002 | Security | `src-tauri/tauri.conf.json:25-27` | high | S | `csp: null`。XSS 可直接 `invoke` 全部命令。当前 UI 不注入 HTML，属纵深防御缺口。 | webview-hardening |
| SEC-003 | Security | `src-tauri/src/commands/window.rs:302-314` | high | S | `app.openPath` / `revealPath` 对任意存在路径 `open::that`。 | webview-hardening |
| SEC-004 | Security | `src-tauri/src/models/settings.rs:84-108` | high | M | `githubToken`、`sync.password` 明文写入 settings JSON 并返回前端。 | at-rest-crypto |
| SEC-005 | Security | `src-tauri/src/services/evaluation.rs:885-918,862-871` | high | M | 私有 prompt 渲染结果明文写入 `prompt_runs`；列表无 lock 门控；re-key 不覆盖这些列。 | at-rest-crypto |
| SEC-007 | Security | `src-tauri/src/services/ai.rs:36-40,174-178` | high | M | `ai.request`/`ai.stream` 无 SSRF、默认跟随重定向、任意 URL/headers。前端无调用但仍注册。 | outbound-ssrf + webview-hardening |
| CORR-B01 | Correctness | `src-tauri/src/services/sync.rs:900-924` | high | M | `backup_restore` 在连接池仍打开时 `remove_dir_all` 活数据目录。 | sqlite-backup-fts |
| CORR-B02 | Correctness | `src-tauri/src/services/sync.rs:813-829` | high | M | 备份/导出用文件拷贝活 WAL 库，快照可能撕裂。 | sqlite-backup-fts |
| CORR-F03 | Correctness | `src-tauri/src/storage/fts.rs:40-72` | high | S | `init_fts` 不 rebuild。已有行的库升级后关键词搜索漏索引。 | sqlite-backup-fts |
| FE-01 | Correctness | `src/runtime/index.ts:148-165` | high | S | `listen` 无 `.catch()`，订阅失败变成未处理拒绝。 | frontend-store-races |
| FE-03 | Correctness | `src/features/evaluation/evaluationStore.ts:108-158` | high | M | chunk 带 `runId`，store 只看 `activeRequestId`；并发 run/matrix 串台。 | frontend-store-races |
| FE-04 | Correctness | `src/features/evaluation/evaluationStore.ts:92-106,250-277` | high | M | `load`/`loadLabels` 无 generation；历史不过滤当前 prompt。 | frontend-store-races |
| FE-05 | Correctness | `src/features/prompts/promptStore.ts:321-352` | high | S | `load()` 过期请求仍写 folders/tags；catch 无序号。 | frontend-store-races |
| LOG-01 | Architecture | `src-tauri/src/lib.rs:63,97` vs `data_path.rs:103` | high | M | `paths.log` 存在但无文件日志。启动/SSRF/导入回滚/updater 无持久记录。 | logging-error-taxonomy |

No `critical` items with a currently exploitable remote unauthenticated path were confirmed. Desktop + local webview: high items become critical if XSS exists. CSP is currently off.

## High-row closure (2026-09-04)

All seven children are archived under `.trellis/tasks/archive/2026-09/`. High rows are accepted by those archives; none are deferred.

| ID | Status | Archived child |
|---|---|---|
| SEC-001 | accepted | `09-03-at-rest-crypto` |
| SEC-002 | accepted | `09-03-webview-hardening` |
| SEC-003 | accepted | `09-03-webview-hardening` |
| SEC-004 | accepted | `09-03-at-rest-crypto` |
| SEC-005 | accepted | `09-03-at-rest-crypto` |
| SEC-007 | accepted | `09-03-outbound-ssrf` + `09-03-webview-hardening` |
| CORR-B01 | accepted | `09-03-sqlite-backup-fts` |
| CORR-B02 | accepted | `09-03-sqlite-backup-fts` |
| CORR-F03 | accepted | `09-03-sqlite-backup-fts` |
| FE-01 | accepted | `09-03-frontend-store-races` |
| FE-03 | accepted | `09-03-frontend-store-races` |
| FE-04 | accepted | `09-03-frontend-store-races` |
| FE-05 | accepted | `09-03-frontend-store-races` |
| LOG-01 | accepted | `09-03-logging-error-taxonomy` |

Written deferrals (not high, listed for completeness): ARCH-02/03/04/05/07/12/13, READ-01, SEC-014 remain deferred as in the tables below.

## Medium

| ID | Dim | File:Line | Sev | Effort | Description | Child |
|---|---|---|---|---|---|---|
| SEC-006 | Security | `evaluation.rs:742-773` | medium | S | 重定向后仍带 Bearer。 | outbound-ssrf |
| SEC-008 | Security | `sync.rs:138-171,350-404` | medium | M | WebDAV/S3 无 `prepare_public_url`。 | outbound-ssrf |
| SEC-009 | Security | `media.rs:529-567` | medium | S | 下载信任 `Content-Type`，不验 magic。 | outbound-ssrf |
| SEC-010 | Security | `commands/portable.rs:11-27` | medium | S | bundle 导出任意 `destination`。 | webview-hardening |
| SEC-011 | Security | `commands/data_path.rs:53-97` | medium | S | apply/recovery 任意文件系统路径。 | webview-hardening |
| CORR-zip | Correctness | `portable.rs:277-302` | medium | S | 信任 ZIP `entry.size()` 再 `read_to_end`。 | library-eval-integrity |
| CORR-import-tx | Correctness | `portable.rs:937-1052` | medium | M | SQL 事务内做 `fs::rename`。 | library-eval-integrity |
| CORR-fk | Correctness | `storage/mod.rs:580-665` | medium | M | `prompt_runs` / labels / `evaluation_runs.test_set_id` 无 FK。 | library-eval-integrity |
| CORR-stuck | Correctness | `evaluation.rs:907,1097` | medium | S | 崩溃后 `running` 永不清理。 | library-eval-integrity |
| CORR-poison | Correctness | `state.rs:117-164` | medium | S | `Mutex::lock().unwrap()`；`conn()` 已映射 poison。 | logging-error-taxonomy |
| CORR-ready | Correctness | `commands/mod.rs:62-66` | medium | S | 启动 `IO` 被包成 `INTERNAL`。 | logging-error-taxonomy |
| TEST-01 | Testing | `evaluationStore.ts:144-158` | medium | S | `execute_run` 对 `status=error` 仍 Ok；store 当成功。 | frontend-store-races |
| PERF-02 | Performance | `prompt.rs:1018` + `libraryItem.ts:71` | medium | M | 列表 `SELECT prompts.*`，卡片带着全文。 | library-eval-integrity |
| PERF-03 | Performance | `promptStore.ts:458-494` | medium | M | 每个 folder/tag 一次 `prompt.search` 做计数。 | library-eval-integrity |
| ARCH-01 | Architecture | `commands/ai.rs` vs `evaluation.rs` | medium | L | 两套 AI HTTP 栈。 | webview-hardening / outbound-ssrf |
| ARCH-03 | Architecture | `prompt.rs` ↔ `reference.rs` | medium | L | 模块环。延期。 | parent deferred |
| ARCH-05 | Architecture | `services/updater.rs:332+` | medium | M | service 依赖 Tauri。延期或并入 hygiene。 | deferred |
| FE-06 | Correctness | `settingsStore.ts:286-294` | medium | S | hydrate 后 `load()` 跳过 refetch。 | frontend-store-races |
| FE-08 | Security | `Settings.sync.password` in Zustand | medium | S | 与 SEC-004 同因。 | at-rest-crypto |

## Low / info

| ID | Sev | Effort | Description | Action |
|---|---|---|---|---|
| SEC-012 | low | S | IPv6 6to4/NAT64 未解码内嵌 IPv4。 | outbound-ssrf |
| SEC-013 | low | S | lock 不 zeroize 密钥。 | at-rest-crypto |
| SEC-014 | info | S | updater pubkey 占位；`createUpdaterArtifacts=false`。 | 保持；发版前替换 |
| ARCH-02 | high-maintain | L | `evaluation.rs` 2882 行神模块。 | 延期拆分 |
| ARCH-04 | medium | M | `promptStore.ts` 853 行。 | 延期 |
| ARCH-07 | medium | S | `app_status`、`settings.list_system_fonts` 命名漂移。 | 延期（契约） |
| ARCH-12 | low | S | 空 `features/skills/`、`ViewPlaceholder`。 | 延期 hygiene |
| ARCH-13 | low | M | 模块级 `allow(dead_code)`。 | 延期 |
| READ-01 | low | M | 重复 `db_err` / `errorMessage`。 | 延期 |
| PERF-05 | medium | S | `increment_usage` 触发 FTS 全量重写。 | library-eval-integrity 可顺带 |
| FE-07 | info | — | 锁只保护加密字段；公开库仍可读。与现契约一致。 | 不作为缺陷 |
| FE-10 | info | — | 无 XSS 渲染路径。 | 保持文本渲染 |

## Looks bad but is actually fine

- SQL `format!` 的表名/列名和 `ORDER BY` 来自白名单或字面量，不是用户输入。
- `prepare_public_url` 对 media/evaluation：禁重定向、DNS pin、每跳复查。
- `security.setMasterPassword` 二次设置返回 `CONFLICT`（已修）。
- 私有 FTS 排除（`fts.rs:89-106`）。
- 前端 `@tauri-apps/api` 仅 `src/runtime`；`boundary.test.ts` 约束。
- 无 `TODO`/`FIXME`。
- localhost OpenAI 被 evaluation SSRF 拒绝：产品选择，不是漏检。

## Missing evidence

- 未跑 `cargo audit` / `npm audit`（需网络）。
- 未跑全量 `just ci`（审查只读，不把测试失败当新发现）。
- 未在真实 WebView 里复现 XSS；CSP 结论来自配置与渲染路径。
