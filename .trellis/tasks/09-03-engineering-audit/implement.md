# 工程级审查 — 执行计划（父任务）

父任务 **不要** `task.py start` 来改产品代码。本文件只规定如何派发子任务和最终对照。

## Ordered checklist

1. 保持本目录规划产物为审查真源（`prd.md` + `research/audit-findings.md`）。
2. 按风险实现子任务，建议顺序（可并行，互不阻塞）：
   1. `09-03-at-rest-crypto`（产品需确认「磁盘失窃仍保密」）
   2. `09-03-webview-hardening`
   3. `09-03-outbound-ssrf`
   4. `09-03-sqlite-backup-fts`
   5. `09-03-frontend-store-races`
   6. `09-03-logging-error-taxonomy`
   7. `09-03-library-eval-integrity`
3. 每个子任务：规划评审 → `task.py start <child>` → 实现 → `just` 对应门禁 → 归档。
4. 全部子任务完成后，对照 `research/audit-findings.md` 的 high 行，再归档本父任务。

## Validation

审查阶段不跑全量 `just ci` 作为通过条件。子任务实现时：

- 前端：`just build`、`just test`，或 `npx vitest run <path>`
- 后端：`just fmt-check`、`just clippy`、`just test-rust`，或聚焦 `cargo test`
- 跨边界：`just ci`

## Risky files

不要在父任务里编辑：`src-tauri/src/services/{security,evaluation,prompt,sync,ai,portable}.rs`、`src-tauri/src/storage/mod.rs`、`src/runtime/index.ts`。

## Rollback

父任务无产品 diff。子任务各自回滚自己的 commit。
