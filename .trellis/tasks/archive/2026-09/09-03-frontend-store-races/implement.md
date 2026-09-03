# 前端 store 竞态 — 实现清单

1. 事件 payload 加 `requestId`（Rust + TS + 测试）。
2. evaluationStore 按 id 过滤；分离 playground/matrix in-flight。
3. promptStore `load` generation；成功清 error。
4. runtime.on catch；App.tsx 取消标志。
5. CommandPalette catch；settings load refetch。
6. `npx vitest run src/runtime/index.test.ts src/features/evaluation/evaluationStore.test.ts src/features/prompts/promptStore.test.ts`
7. `just test` 与 `just build`。
