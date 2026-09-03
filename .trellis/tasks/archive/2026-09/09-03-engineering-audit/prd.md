# 工程级代码审查整改

## Goal

把 2026-09-03 对 PromptHub Desktop 第一方代码的深度审查结果落成可独立交付的子任务。父任务只拥有问题清单、任务切分和最终对照；不在本任务里改产品代码。

用户价值：后续实现按严重程度推进，避免一次混改安全、存储和前端竞态。

## Background

审查层级：`deep`。范围：`src/**`、`src-tauri/src/**`、`src-tauri/capabilities/**`、`src-tauri/tauri.conf.json`。排除：`ref/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/`、`node_modules/`。

本仓库没有 `docs/audits/` 历史报告。已归档的 `09-03-core-test-coverage` 中「`security.setMasterPassword` 无门控」已修复（`commands/security.rs:17-41`），不再作为新债。

第一方 `src/` 与 `src-tauri/src/` 中无 `TODO` / `FIXME` / `HACK` / `XXX`。

## Confirmed facts

- Runtime Bridge 是前端唯一 `@tauri-apps/api` 入口。
- 媒体下载与 evaluation 的 openai-compatible 出站走 `prepare_public_url`；`ai.request` / `ai.stream` 与 WebDAV/S3 不走。
- 主密码 verifier 的 `hash` 就是 AES-256-GCM 密钥，与 `ENC::` 密文同库。
- `RuntimePaths.log` 已创建并在 UI 展示，进程只对启动失败 `eprintln!`。
- `evaluation.rs` 约 2882 行，`prompt.rs` 约 2925 行，`portable.rs` 约 1880 行。
- `prompt` ↔ `reference` 存在 Rust 模块环。
- 前端不渲染 HTML（无 `dangerouslySetInnerHTML`）。
- `tauri.conf.json` 中 `csp` 为 `null`；updater `pubkey` 是占位符且 `createUpdaterArtifacts` 为 `false`。

## Requirements

### R1 — 审查产物

- **R1.1** 问题按严重程度（critical/high/medium/low/info）和修改成本（S/M/L）分类，并带 `file:line`。
- **R1.2** 每个可独立验收的整改项对应一个子任务，父子关系写在 `task.json.children`。
- **R1.3** 父任务不实现代码。子任务可单独 `task.py start`、实现、检查、归档。

### R2 — 子任务覆盖

每个子任务的验收标准必须能在不依赖其他子任务合并的情况下测试。跨任务依赖写在该子任务的 `prd.md` / `implement.md`，不靠目录顺序暗示。

| 子任务 | 覆盖的审查 ID |
|---|---|
| `09-03-at-rest-crypto` | SEC-001, SEC-004, SEC-005 |
| `09-03-webview-hardening` | SEC-002, SEC-003, SEC-010, SEC-011, ARCH-01/06 的命令面 |
| `09-03-outbound-ssrf` | SEC-006, SEC-007, SEC-008, SEC-009, SEC-012 |
| `09-03-sqlite-backup-fts` | CORR-backup-01, CORR-backup-02, CORR-fts-03 |
| `09-03-frontend-store-races` | FE-01, FE-02, FE-03, FE-04, FE-05, FE-09, TEST-01 |
| `09-03-logging-error-taxonomy` | LOG-01, CORR-poison, CORR-error-codes |
| `09-03-library-eval-integrity` | CORR-zip, CORR-import-tx, CORR-fk, CORR-stuck-run, PERF-list/counts |

### R3 — 明确不做（父任务与默认子任务）

- 不拆 `evaluation.rs` / `prompt.rs` 神模块（ARCH-02/03，成本 L，不阻塞安全修复）。
- 不批量删除 `#![allow(dead_code)]`（可在后续 hygiene 任务做）。
- 不把 updater 占位 `pubkey` 换成真实密钥（仓库政策：密钥 gitignore）。
- 不迁移 Electron 旧数据。
- 不修改 `ref/PromptHub/**`。

## Acceptance Criteria

- [x] 父任务 `prd.md` / `design.md` / `implement.md` 与 `research/audit-findings.md` 列出全部 high 及以上发现，每条有 `file:line`。
- [x] 七个子任务均有可观察的验收标准，且不要求先合并另一个子任务才能跑测试。
- [x] 审查与规划阶段不修改 `src/**` 或 `src-tauri/**` 产品代码。
- [x] 父任务在子任务全部归档后才做对照：每条 high 发现要么已修，要么在子任务中标记延期及原因。

## Key Decisions

2026-09-03 用户确认采用审查推荐：

1. **静态加密目标是防磁盘失窃。** DEK 与 verifier 分离；现有 `ENC::` 迁移。见 `09-03-at-rest-crypto`。
2. **localhost / RFC1918 默认拒绝。** 需要 Ollama 或局域网 NAS 时用默认关闭的显式开关。见 `09-03-outbound-ssrf`。
3. **未接线危险命令先从 `invoke_handler` 拿掉。** 不在本轮补 UI。`ai.request` / `ai.stream` / `ai.cancel` 必须下线；低层 `webdav.upload|download|stat|ensureDir` 与 `s3.upload|download|stat` 同样下线。`rules.*` 与已有 `safe_join` 的 `media.*` 可保留，路径仍须落在 RuntimePaths。见 `09-03-webview-hardening`。

## Technical notes

完整表见 `research/audit-findings.md`。热路径：`services/security.rs`、`services/ai.rs`、`services/sync.rs`、`services/evaluation.rs`、`services/network_safety.rs`、`storage/fts.rs`、`commands/window.rs`、`src/runtime/index.ts`、`src/features/evaluation/evaluationStore.ts`、`src/features/prompts/promptStore.ts`。
