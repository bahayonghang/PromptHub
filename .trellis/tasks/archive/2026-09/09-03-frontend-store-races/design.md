# 前端 store 竞态设计

事件关联：在 `evaluation:run-chunk` / `matrix-progress` 增加 `requestId`（与命令参数相同）。这是跨层契约，改 `commands/events.rs` + TS types + store。比在前端猜 `runId` 可靠。

Generation：promptStore 已有 `searchGeneration`；`load` 的 catch/else 必须检查。evaluationStore 增加 `loadGeneration`。

`run()`：若 `run.status === 'error' | 'cancelled'` 写入 `error` 或明确空闲状态，不要只靠 throw。

`runtime.on`：`listen(...).then(...).catch(...)`；handler 包 try/catch。
