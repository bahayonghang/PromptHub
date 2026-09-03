# 静态加密 — 实现清单

1. 冻结现有 v1 单测（unlock/re-key/private prompt）。
2. 实现 v2 存储格式与迁移函数；事务内完成。
3. 改 `set_master_password` / `unlock` / `change_master_password` / `lock`（zeroize）。
4. settings 秘密封存 + get 脱敏。
5. private `prompt_runs` 封存 + 锁定门控 + re-key。
6. 测试：复制 DB 不输入密码读不出明文；旧库迁移；无主密码公开库。
7. `just fmt-check`、`just clippy`、`just test-rust`（至少 security/settings/evaluation）。

验证：`cargo test --manifest-path src-tauri/Cargo.toml security` 以及 settings/evaluation 相关名。

回滚点：迁移前 startup 已有 safety backup；失败不得写 v2。
