# 导入安全、外键、列表设计

ZIP：`std::io::Read::take(limit+1)`；超过即 `VALIDATION`。

导入顺序：validate → 私有密钥检查 → safety backup → SQL 事务（只 DB）commit → 安装媒体 → 失败删新媒体。不要在打开的写事务里 `rename`。

FK：`prompt_labels.prompt_id` → `prompts(id) ON DELETE CASCADE`；`prompt_runs.prompt_revision_id` → `prompt_versions(id)` 视现有删除语义（版本随 prompt cascade）。`evaluation_runs.test_set_id` 建议 `ON DELETE CASCADE` 或限制删除有 run 的 test set。选一种写测试。

列表：新 DTO `PromptListItem` 无正文。`prompt.search` 可改为返回该 DTO（前端卡片已只用投影）。counts：`prompt.counts` 一次返回 `{ folders: {id: n}, tags: {name: n}, total }`。
