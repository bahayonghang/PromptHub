# 前端 store 竞态与事件关联

## Goal

prompt 库与 evaluation 工作台在并发加载、流式事件和失败路径下显示正确的一份状态。

## User value

快速切换 prompt、重叠搜索、取消 evaluation 时，不再出现串台输出、过期错误条或点选无响应。

## Confirmed facts

- `runtime/index.ts:148-165`：`on()` 对 `listen` 只用 `.then()`，无 `.catch()`。
- `App.tsx` 在 `initialize()` resolve 前卸载会丢 unlisten。
- `evaluationStore.ts:108-158`：chunk 事件是 `{ runId, chunk }`，in-flight 键是客户端 `requestId`；`activeRequestId != null` 就拼接所有 chunk。
- `load()` / `loadLabels` 无 generation；失败不按 promptId 丢弃。
- `promptStore.ts:321-352`：`load()` 过期序列仍写 folders/tags；catch 无序号。`refreshPrompts` 成功不清 `error`。
- `execute_run` 对 provider 失败仍 `Ok(PromptRun{status:error})`；store 只在 throw 时设 `error`。
- `CommandPalette.tsx:36-42`：`searchPrompts().then` 无 catch。

## Requirements

- **R1** `runtime.on` 捕获 listen 失败与 handler 抛错，不产生未处理拒绝。
- **R2** evaluation 流事件必须能关联到本次 `requestId`（改后端 payload 或 store 映射）。忽略其他 id。playground 与 matrix 的 in-flight 状态分离。
- **R3** `load` / `loadLabels` / prompt `load` 使用 generation；过期响应不得覆盖新状态；失败不得把旧 error 写到新成功之后。
- **R4** `run.status === "error"` 时 store 设置 `error`（来自 `run.error`）。
- **R5** 命令面板搜索必须 catch，并忽略过期 generation。
- **R6** settings `load()` 在 hydrate 之后仍能从后端刷新（修 FE-06）。

## Acceptance Criteria

- [x] 单测：listen reject 时 `on()` 不产生未处理拒绝。
- [x] 单测：request A 的 chunk 在 request B 进行中被忽略。
- [x] 单测：较慢的 `load()` 不能覆盖较新的 folders/tags/prompts。
- [x] 单测：`run()` 返回 `{ status: "error", error: "..." }` 时 store.error 非空。
- [x] 命令面板搜索失败不抛未处理拒绝。
- [x] `npx vitest run` 覆盖上述文件；`just test` 在实现阶段跑。

## Out of scope

- 拆 promptStore 神对象。
- 虚拟化网格。
- 应用级锁屏遮罩（公开库可读是当前后端契约）。

## Evidence

`src/runtime/index.ts:148-165`；`evaluationStore.ts:92-158`；`evaluation/types.ts:165-175`；`promptStore.ts:321-379`；`CommandPalette.tsx:36-42`。
