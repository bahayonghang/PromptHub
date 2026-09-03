# 备份与 FTS — 实现清单

1. 抽出 `snapshot_database(conn, dest)`。
2. `backup_create` / 迁移 safety backup 改用它。
3. `backup_restore` 关池 + 旁路替换。
4. `init_fts` 重建；测试「先有行再 init_fts」。
5. `cargo test`：sync backup、storage fts、startup 若有。
