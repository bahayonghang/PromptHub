# 日志与错误码

## Goal

启动、SSRF、导入回滚、updater、锁毒化写入 `RuntimePaths.log` 下的滚动文件。生产路径上的 Mutex poison 不再 panic。错误码不再把 IO/取消/未就绪一律标成 `INTERNAL`。

## User value

出问题时 log 目录里有可分享的记录；前端能按稳定 `code` 分支，而不是解析英文句子。

## Confirmed facts

- `data_path.rs` 创建 `paths.log`；`RuntimeInfoPanel` 展示该路径。
- 进程日志仅 `lib.rs:63-66,97` 的 `eprintln!`。Cargo.toml 无 `tracing`/`log`。
- `state.rs:117-164`：`pool`/`init_error`/`requests` 使用 `lock().unwrap()`。`conn()` 已把 poison 映射为 `INTERNAL`。
- `ensure_ready` 把启动错误字符串再包一层 `INTERNAL`。
- evaluation 取消路径构造 `AppError::internal("request cancelled")`（行状态随后被映射为 cancelled）。
- `ErrorCode::Locked` 从未使用。
- updater `ReleaseNotFound` → Network，`AuthenticationFailed` → Io。

## Requirements

- **R1** 在 `paths.log` 写滚动文件。记录：启动失败、SSRF（只记 host，不记密钥）、导入回滚残留路径、updater 阶段、mutex poison。禁止记录密码、DEK、`Authorization`。
- **R2** `AppState` 与 updater progress tracker 的 poison 返回 `AppError`，不 panic。
- **R3** `ensure_ready` 保留原始 `ErrorCode`（或增加稳定的未就绪码，前后端一起改）。
- **R4** 取消不要用 `INTERNAL` 作为唯一信号；`LOCKED` 要么使用要么从对外 taxonomy 文档标明保留未用。
- **R5** updater `classify` 单测覆盖 ReleaseNotFound / AuthenticationFailed。

## Acceptance Criteria

- [x] 模拟启动失败后，`paths.log` 下文件包含失败原因且不含密钥。
- [x] 对 `requests` mutex 的 poison 测试（或等价）得到 `INTERNAL`/`IO` 而进程仍在。
- [x] 前端在后端未就绪时能读到非泛化 `INTERNAL` 的 code，或文档化的 `INTERNAL`+details；不得丢失 IO 语义。
- [x] `cargo test` 覆盖 updater classify 与 ready 映射。

## Out of scope

- 前端把所有 store 改成按 code 分支（promptStore 可顺手存 code，不做 UI 大改）。
- CSP、加密。

## Evidence

`lib.rs:63,97`；`state.rs:117-164`；`commands/mod.rs:62-88`；`evaluation.rs:657,754`；`updater.rs:354-369,428`。
