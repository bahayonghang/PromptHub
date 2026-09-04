# WebView IPC 与路径打开加固

## Goal

缩小渲染进程能碰到的操作系统与任意 HTTP 面。CSP 为空、`open::that` 任意路径、以及未接线但仍注册的命令，使 XSS 或调试调用等于本地 RCE / 代理。

## User value

即便出现脚本注入，也不能打开任意 exe，也不能走未审查的 `ai.*` 代理。

## Confirmed facts

- `tauri.conf.json:25-27`：`"csp": null`。
- `commands/window.rs:302-314`：`app.openPath` / `app.revealPath` 只检查 `exists`，然后 `open::that`。
- `commands/portable.rs` 导出 `destination` 无根目录限制。
- `data.applyChange` / `data.recoveryApply` 接受任意路径（产品功能，缺确认令牌）。
- 前端无 `ai.*`、`rules.*`、多数 `media.*`、低层 webdav/s3 传输调用，但 `invoke_handler` 已注册。
- UI 当前用 React 文本节点，无 `dangerouslySetInnerHTML`。

## Requirements

- **R1** 为 Tauri webview 设置严格 CSP：`default-src 'self'`，禁止 `unsafe-eval`；图片/字体按现有本地资源收紧。
- **R2** `app.openPath` / `app.revealPath` 只允许 `RuntimePaths` 下的路径；拒绝可执行扩展名。
- **R3** portable 导出默认写 backup 目录；若保留自定义路径，必须落在用户选定且规范化后的允许根下。
- **R4** 从 `invoke_handler` 删除未接线的危险命令：`ai.request`、`ai.stream`、`ai.cancel`，以及前端未调用的 `webdav.upload|download|stat|ensureDir`、`s3.upload|download|stat`。本轮不补对应 UI。`rules.*` 与已有 `safe_join` 的 `media.*` 可保留。
- **R5** 数据路径变更/恢复需要显式确认载荷（例如一次性 token），拒绝把活库拷到任意位置的静默调用。

## Acceptance Criteria

- [x] 打包配置里 CSP 不再是 `null`；测试或配置快照锁住策略字符串。
- [x] `app.openPath("C:\\Windows\\System32\\cmd.exe")`（或 POSIX 等价）返回 `VALIDATION`/`NOT_FOUND`，进程不被启动。
- [x] 打开 `paths.media` 下真实媒体文件仍然成功。
- [x] 未授权 `destination` 的 bundle 导出失败；默认导出路径在 backup 根下。
- [x] `invoke_handler` 不再注册 `ai.request`、`ai.stream`、`ai.cancel`、`webdav.upload|download|stat|ensureDir`、`s3.upload|download|stat`。`webdav.test` / `s3.test` 仍在。
- [x] `just clippy` 与相关 `cargo test` / 前端 boundary 测试通过。

## Out of scope

- 主密码信封格式。
- `prepare_public_url` 实现（见 `09-03-outbound-ssrf`）。
- 真实 updater 签名密钥。

## Evidence

`src-tauri/tauri.conf.json:25-27`；`commands/window.rs:302-314`；`services/window.rs:482-490`；`src-tauri/src/lib.rs` invoke 列表；`src/` 无 `ai.request` 调用。
