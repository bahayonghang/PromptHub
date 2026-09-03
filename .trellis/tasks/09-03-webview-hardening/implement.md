# WebView 加固 — 实现清单

1. 写 CSP 字符串并确认 `just build` 的 webview 配置含该策略。
2. path allowlist + 可执行黑名单测试。
3. portable 导出路径收口。
4. 从 handler 移除或门控未用 `ai.*`。
5. data-path confirm token。
6. `just clippy`、相关 `cargo test`、`npx vitest run src/runtime/boundary.test.ts`。
