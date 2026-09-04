# Prompt 详情弹层空白布局优化

## Goal

内容 tab 把 1180px 工作弹层用满：提示词正文是主工作面，元数据靠边，不再用阅读栏 `68ch` 居中制造大面积空白。

## Background

用户在内容 tab（对话模式）看到弹层两侧和正文周围有大面积空白。

`08-24-detail-modal` 设计的内容 tab 是双栏：左栏提示词正文，右栏标题/描述/类型/文件夹/标签等元数据。落地后变成单栏堆叠。`docs/design/ui-modernization-plan.md` §6.5 又给整张表单加了 `max-w-[68ch] mx-auto`，把 Operate 编辑器当成阅读页限宽。

`DESIGN.md` 的 65–75 字符规定针对长文案行长，不是整张工作表单的容器宽度。

## Confirmed facts

- 弹层壳：`src/components/ui/Modal.tsx:142` — `w-[min(1180px,100%)]`，`max-h-[min(90vh,56rem)]`。
- 内容 tab 限宽：`src/features/prompts/components/detail/PromptDetailModal.tsx:562-563` — `prompt-editor__body` 内 `mx-auto … max-w-[68ch] … gap-7`。68ch ≈ 34rem，两侧各空约 20rem。
- 章节顺序（单栏）：Identity → Definition → Organization → Media。定义区与组织/媒体争同一条纵向滚动带，正文编辑器不能吃掉剩余高度。
- 容器查询已存在且未覆盖整栏拓扑：`src/styles/globals.css:402-426` — `min-width: 40rem` 时 `__two-column` 变两列字段、`__message` 变成 `角色 | 正文 | 操作`。没有「左正文 / 右元数据」的弹层级网格。
- 对话正文最小高度：`.prompt-editor__message-body` 24rem，多条时 12rem（`globals.css:428-434`）。高度有下限，宽度被 68ch 卡住。
- 过渡组件 `PromptEditor.tsx:161-162` 没有 68ch 限宽，同样的四个 section 是全宽堆叠。
- `ContentTab.tsx` 在 `08-24-detail-modal` 的 `design.md` D2 中被点名，仓库里不存在；四个 section 由 `PromptDetailModal` 直接拼。
- 复制按钮在弹层 header（`PromptDetailModal.tsx:388`）和 `DefinitionSection.tsx:72` 各有一处。
- 访客模式：Operate。主任务是写/改提示词并保存。

## Requirements

- R1: 内容 tab 在弹层可用宽度内铺满工作面。禁止用 `max-w-[68ch]`（或等价阅读栏）作为整张内容表单的容器宽度。
- R2: 提示词正文（文本模式的 system/user，或对话模式的消息列表）是视觉主区域：宽于元数据，并占用内容 tab 的剩余高度。
- R3: 元数据（标题、描述、类型、私有、文件夹、标签、附件）与正文分组分离，不再插在正文上下方与之争纵向空间。
- R4: 窄于约 40rem 的编辑器容器回退为单栏全宽，仍无 68ch 居中。Identity 在上，正文吃剩余高度，组织/附件可滚动到达。
- R5: 不改变保存、校验、脏数据确认、锁定、tab、快捷键、i18n 文案语义。
- R6: 版本 / 试跑 / 引用 tab 不在本任务改版，除非内容 tab 的壳改动破坏它们的高度约束。
- R7: 现有 `@container prompt-editor` 字段级两列规则继续有效，或被等价规则替换并有测试覆盖。

## Acceptance Criteria

- [x] AC1 (R1): 内容 tab 表单根节点没有 `max-w-[68ch]` / 等价阅读栏限宽；在 1180px 弹层里正文区宽度大于弹层宽度的一半。
- [x] AC2 (R2): 对话模式单条消息和文本模式 user 编辑器在内容 tab 可见视口内占主面积；不需要先滚动才看到正文。
- [x] AC3 (R3): 标题/描述/类型/私有/文件夹/标签/附件不占据正文编辑器的主列。
- [x] AC4 (R4): 将内容 tab 容器缩到 40rem 以下时，布局变为单栏全宽，正文仍可见且可编辑。
- [x] AC5 (R5): `PromptDetailModal.test.tsx` 现有行为断言仍通过；不新增或删除字段。
- [x] AC6: `just build` 和 `just test` 通过。

## Decisions

- Q1 = A. 内容 tab 用双栏工作台：左正文、右元数据。窄于约 40rem 时单栏全宽，顺序为 Identity → 正文 → 组织/附件。不采用只删 68ch 的单栏方案。复制按钮仍保留两处（现有测试要求），本任务不 distill。

## Out of scope

- 改版本 / 试跑 / 引用 tab 的信息架构。
- 改评价引擎、版本存储、加密、复制契约。
- 重做弹层 header / footer / tab 样式，除非挡路。
- 删除 `PromptEditor.tsx` 过渡组件。
- 改 `DESIGN.md` 的 65–75 字符散文行长规定（它继续约束设置页等阅读面，不约束这张工作弹层的容器宽度）。
