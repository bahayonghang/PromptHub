# 日志与错误码设计

日志：不要为这次引入完整遥测平台。`tracing-appender` 或手工 `std::fs::OpenOptions` 追加 + 简单体积滚动即可。目标路径 `RuntimePaths.log/prompthub.log`。字段：时间、级别、模块、消息。禁止：密码、DEK、完整 URL query 中的 token。

错误：`ensure_ready` 返回启动时保存的 `AppError`，不要 `internal(error.to_string())`。`AppState` 存 `Option<AppError>` 或 `(ErrorCode, String)`。

取消：evaluation adapter 可用专用内部标记；持久化仍写 `cancelled`。若要上线新 code，必须同步 TS。更小的改法：取消只靠 token，不再构造 `AppError::internal("request cancelled")`。
