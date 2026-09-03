# 日志与错误码 — 实现清单

1. 选已有或小封装文件日志（避免无必要的大 tracing 平台）。写到 `paths.log/prompthub.log`。
2. 替换 `state.rs` unwrap。
3. `ensure_ready` 保留原始 code。
4. 取消路径停止滥用 `INTERNAL`。
5. updater classify 测试。
6. `just test-rust` 聚焦 state/startup/updater。
