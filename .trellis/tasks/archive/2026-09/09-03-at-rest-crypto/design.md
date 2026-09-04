# 静态加密设计

## Boundaries

- `services/security.rs` 拥有 KDF、信封、re-key。
- settings 秘密与 evaluation run 载荷调用同一 `encrypt`/`decrypt`。
- Command_Layer 不接收 DEK。

## Proposed envelope

当前：`hash = scrypt(password, salt)` 且 `hash` 即 AES key。

目标：

1. 随机 32-byte DEK，只在解锁后留在 `EncryptionState`。
2. `KEK = scrypt(password, kdf_salt)`。
3. 存储 `{ v: 2, kdf_salt, verifier, wrapped_dek }`。`verifier` 用与 KEK 不同的 info/salt（或第二段 KDF），**不等于** DEK。
4. `ENC::` 继续 AES-256-GCM；re-key 只换 DEK 包装和密文。

迁移：读 v1 `{salt, hash}` → 把 `hash` 当旧 DEK → 生成新 DEK → 重加密所有 `ENC::` 与本任务纳入的列 → 写 v2 → 同一事务。

## Settings secrets

`sync.password` / `githubToken`：有主密码则 `ENC::`；`settings.get` 用 `hasSyncPassword` / `hasGithubToken`。无主密码时拒绝持久化这些秘密（`VALIDATION`），已有明文在首次设主密码时封存。

## Evaluation runs

private revision：`inputs`/`rendered_messages`/`output` 加密。`list_runs` 在锁定下 redact 或 `UNAUTHORIZED`。re-key 扫描这些列。

## Compatibility

Schema：settings 行格式版本在 JSON 内，不一定升 `CURRENT_SCHEMA_VERSION`。若加列则升版本并写迁移测试。
