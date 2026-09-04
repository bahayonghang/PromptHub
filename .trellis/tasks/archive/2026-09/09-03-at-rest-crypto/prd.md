# 修复静态加密与明文密钥

## Goal

让「私有内容 / 凭据」在数据库文件被复制后仍然需要主密码才能读出。当前 verifier 的 `hash` 就是 AES 密钥。

## User value

备份、导出和磁盘失窃不再等于拿到全部私有 prompt 和 provider token。

## Confirmed facts

- `security.rs:11-15,127-130`：`hash` = scrypt(password, salt)，并直接用作 AES-256-GCM 密钥。
- 同一 SQLite 内还有 `ENC::` 私有字段和 evaluation `credential`。
- `settings.rs` 把 `githubToken`、`sync.password` 明文写入 `settings` 表 `app` 文档，并经 `settings.get` 回传。
- `evaluation.rs:885-918` 把已解密的 `rendered_messages` / `output` 明文写入 `prompt_runs`；`list_runs` 无 lock 门控；`change_master_password` 不 re-key 这些列。
- `lock` 只把 `derived_key` 置 `None`，不 zeroize（SEC-013，可同任务处理）。

## Requirements

- **R1** 持久化的主密码记录不得等于数据加密密钥。解锁后 DEK 只留在内存。现有 `ENC::` 行必须可迁移。
- **R2** `githubToken` 与 `sync.password` 在存在主密码时用同一 DEK 封存；`settings.get` 返回存在性布尔，不返回秘密。无主密码时写入这些字段返回 `VALIDATION`。已有明文在首次设主密码时封存。
- **R3** 源 revision 为 private 时，`prompt_runs.inputs` / `rendered_messages` / `output` 封存。锁定下列表/详情必须拒绝或 redact。re-key 覆盖这些列。
- **R4** 无主密码时的公开库行为保持不变。
- **R5** 迁移失败不得推进 verifier 格式或 `user_version`；保留启动安全备份。
- **R6** 产品目标是磁盘失窃仍保密（2026-09-03 已确认）。禁止只改文档、把主密码当作会话锁。

## Acceptance Criteria

- [x] 复制已设主密码的 `prompthub.db` 到新进程，不输入密码无法得到私有正文或 profile credential 明文。
- [x] 正确密码解锁后，原私有 prompt 与 evaluation credential 仍可解密。
- [x] `settings.get` 不含 `sync.password` / `githubToken` 明文；有主密码时落盘为 `ENC::`；无主密码时写入返回 `VALIDATION`。
- [x] 对私有 revision 跑一次 evaluation 后，直接读 `prompt_runs` 看不到明文；锁定下 `evaluation.runList` 不泄露正文。
- [x] 改密事务同时 re-key 私有 prompt、profile credential、settings 秘密、私有 run 载荷。
- [x] 新增/更新 schema 迁移测试覆盖旧 verifier 格式。

## Out of scope

- 拆 `evaluation.rs` 文件。
- WebView CSP、SSRF、备份 WAL。
- 强制用户设置主密码。
- 仅会话锁 / 只改文档。

## Decisions

- 2026-09-03：采用防磁盘失窃。随机 DEK，KEK = scrypt(password)，verifier 不得等于 DEK。旧 `{salt, hash}` 在同一事务中重加密后写成 v2。

## Evidence

`src-tauri/src/services/security.rs:11-15,127-130,159-161,218-223`；`models/settings.rs:84-108`；`services/evaluation.rs:862-918`。
