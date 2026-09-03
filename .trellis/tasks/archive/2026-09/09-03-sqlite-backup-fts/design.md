# 备份与 FTS 设计

快照：对 `database_path` 使用 rusqlite `backup` API 或 `VACUUM INTO` 得到独立文件，再拷贝 media 等非 db 目录。不要拷贝 `-wal`/`-shm` 作为权威 db。

恢复：`set_ready(false)` → 卸 pool（`set_pool` 换成 None / drop）→ checkpoint → 写入 `data.next` → 替换 → 要求重启。失败则保留原目录。

FTS：`init_fts` 末尾：

```sql
INSERT INTO prompts_fts(prompts_fts) VALUES('rebuild');
```

或按 `is_private=0` 插入。rebuild 后私有行应仍不出现（触发器定义排除 private；rebuild 外部内容表会读当前 prompts，需确认 private 不会进索引——若 rebuild 索引全部列，改为显式 `INSERT ... SELECT ... WHERE is_private=0`）。
