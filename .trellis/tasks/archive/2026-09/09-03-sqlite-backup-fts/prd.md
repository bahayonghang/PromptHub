# 备份快照与 FTS 重建

## Goal

备份和恢复得到一致的 SQLite 快照；升级已有库后关键词搜索能命中旧行。

## User value

恢复备份不会拿到撕裂的 WAL 库；迁移后搜索不会「库里有、搜不到」。

## Confirmed facts

- `sync.rs:813-829` `backup_create`：`copy_dir_recursive` 活 `data/`（含 db/wal/shm）。
- `sync.rs:900-924` `backup_restore`：`remove_dir_all(data_dir)` 时池仍由 `AppState` 持有。
- portable import 与 schema 迁移前的 safety backup 走同一 `backup_create`。
- `fts.rs:40-72`：`CREATE VIRTUAL TABLE IF NOT EXISTS`，注释写明不 rebuild。
- `commands/startup.rs:74-81`：先 `init_schema` 再 `init_fts`。非空旧库升级后 FTS 为空，触发器只覆盖之后的写。

## Requirements

- **R1** 创建备份前对活库做 checkpoint 或使用 SQLite backup / `VACUUM INTO`。禁止直接拷贝打开中的 db+WAL 作为唯一快照手段。
- **R2** 恢复前关闭/卸下连接池（或等价地保证无打开句柄），拷到旁路目录再原子替换。失败不得留下空 data 目录。
- **R3** `init_fts` 之后若 `prompts` 非空且 FTS 为空（或行数不匹配非私有 prompt），执行 `rebuild`。
- **R4** 迁移 safety backup 使用同一安全快照路径。

## Acceptance Criteria

- [x] 有写入活动时 `backup_create` 产生的库可用 `PRAGMA integrity_check` 打开（测试用 WAL 写入夹逼）。
- [x] `backup_restore` 在池仍「逻辑打开」的测试夹具下要么先关池成功恢复，要么返回 `IO` 且原目录仍在。
- [x] 插入 N 条 prompt → 再 `init_fts` 的路径下，关键词搜索能命中这 N 条（非 private）。
- [x] 私有 prompt 仍不进 FTS。
- [x] 现有 backup id 格式与 `backup_list` 契约不变。

## Out of scope

- 列表 DTO 瘦身、计数查询。
- WebDAV 传输 SSRF。
- 加密信封。

## Evidence

`services/sync.rs:813-829,900-924`；`storage/fts.rs:40-72`；`commands/startup.rs:65-81`。
