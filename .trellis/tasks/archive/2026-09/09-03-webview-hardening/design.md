# WebView 加固设计

## CSP

在 `tauri.conf.json` `app.security.csp` 使用 Tauri 2 能解析的策略：`default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'`（若 Tailwind 运行时需要 inline；以实际加载为准，禁止 `unsafe-eval` 与任意 https）。

## Path allowlist

共享 helper：`ensure_under_runtime_paths(path, &RuntimePaths)`，canonicalize 后前缀匹配 data/media/rule/backup/log。`openPath`/`revealPath` 使用它。可执行后缀黑名单：`.exe .bat .cmd .ps1 .msi .com .scr`。

## Command surface

从 `invoke_handler` 删除：`ai.request`、`ai.stream`、`ai.cancel`、`webdav.upload|download|stat|ensureDir`、`s3.upload|download|stat`。`webdav.test` / `s3.test` 保留，传输仍须过 SSRF 任务。`rules.*` 与已有 `safe_join` 的 `media.*` 保留。导出 destination 收口是必须的。

## Data path

`applyChange`/`recoveryApply` 增加 `confirmToken`（前端从 preview 获得，一次性）。无 token 返回 `VALIDATION`。
