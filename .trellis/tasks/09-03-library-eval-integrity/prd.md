# 导入安全、外键与列表性能

## Goal

portable 导入不能靠 ZIP 头尺寸绕过上限；evaluation 行有引用完整性；库列表不再为卡片传输全文，计数不再 N 次 `prompt.search`。

## User value

恶意 bundle 不会 OOM；删 prompt 不留孤儿 evaluation 行；侧栏计数在多文件夹库上可完成。

## Confirmed facts

- `portable.rs:277-302`：用 `entry.size()` 累加再 `read_to_end`。
- `portable.rs:937+`：事务内 `fs::rename` 媒体。
- `storage/mod.rs:580-665`：`prompt_runs`、`evaluation_runs.test_set_id`、`prompt_labels.prompt_id` 无 FK。
- `evaluation.rs` 插入 `running`，无启动扫描。
- `prompt.rs:1018` `SELECT prompts.*`；`promptStore` 对每个 folder/tag 调 search 做 count。
- `increment_usage` 触发 FTS AFTER UPDATE 全量重写。

## Requirements

- **R1** 按实际读取字节封顶（manifest 16MB、media 100MB 现有常量）。头尺寸只作提示。
- **R2** 媒体落盘在 SQL commit 之后；失败回滚已创建文件。导入期间不把 zip/backup I/O 绑在占用的写事务上。
- **R3** 新迁移为 evaluation/label 行补 FK（CASCADE 或 SET NULL，选能保持现有删除语义的一种）并清理孤儿。
- **R4** 启动时把遗留 `running` 的 run/cell 标为 `error` 或 `cancelled`。
- **R5** 库列表 DTO 不含 system/user/messages 正文；详情/copy 再取全文。
- **R6** 一次后端调用返回 folder/tag counts。
- **R7** （可选同任务）FTS UPDATE 触发器忽略仅 `usage_count` 变化。

## Acceptance Criteria

- [x] 头声明 size=1 但解压很大的 fixture 在上限处失败，进程不 OOM。
- [x] 导入失败后无新 SQL 行、无残留 staged media。
- [x] 删除 prompt 后无残留 `prompt_labels`；删除 test set 后 `evaluation_runs` 行为符合所选 FK 策略（测试写明）。
- [x] 插入 `running` 行后模拟启动扫描，行不再保持 `running`。
- [x] 前端卡片渲染不依赖列表响应里的 `userPrompt` 正文（类型与测试锁住）。
- [x] 有 F 个文件夹时 `load()` 不再发 F 次 search；改为 1 次 counts + 1 次 page。

## Out of scope

- 拆 evaluation.rs / prompt.rs。
- 备份 WAL（`09-03-sqlite-backup-fts`）。
- SSRF。

## Evidence

`services/portable.rs:277-302,937-1052`；`storage/mod.rs:580-665`；`services/prompt.rs:1001-1024,1205`；`promptStore.ts:458-494`。
