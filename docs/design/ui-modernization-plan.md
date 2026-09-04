# PromptHub 界面现代化重构方案

> 方向：**精致工作台（Refined Workbench）** — 在 `DESIGN.md` 已确立的 "Prompt Workbench"
> 定位上，用层级、留白、字体节奏和克制的材质提升质感，参照 Linear / Raycast / Zed 的
> 密度与精度，而不是走 SaaS 仪表盘或玻璃拟态路线。
>
> 本文档是**分析与方案**，不含代码改动。落地时代码与 `DESIGN.md` 同步演进。

---

> **v2 更新（融合 Fable 方案）**：右侧停靠详情面板取代原方案的详情模态、
> 全局标签色板、使用次数迷你进度条、类型图标格、版本等宽徽标、
> 单一强调色渐变、工作区切换器、状态栏快捷键提示 —— 均已并入本方案并落到原型。
> 详见 [第 10 节：与 Fable 方案的融合决策](#10-与-fable-方案的融合决策)。

## 目录

1. [现状诊断](#1-现状诊断)
2. [根因：缺失的中间层](#2-根因缺失的中间层)
3. [目标设计语言](#3-目标设计语言)
4. [Token 层重构](#4-token-层重构)
5. [基元组件层（新增）](#5-基元组件层新增)
6. [布局与排版重构（逐屏）](#6-布局与排版重构逐屏)
7. [交互与动效](#7-交互与动效)
8. [实施路线图](#8-实施路线图)
9. [验收标准](#9-验收标准)
10. [与 Fable 方案的融合决策](#10-与-fable-方案的融合决策)

---

## 1. 现状诊断

代码规模：54 个非测试 `.tsx`，约 8,233 行。全部样式为内联 Tailwind 字符串，
`src/components/ui/` 下只有 `Modal.tsx` 一个共享基元。以下问题均可量化。

### 1.1 字号阶梯坍缩成两档

```
161×  text-xs   (12px)
130×  text-sm   (14px)
 13×  text-[11px]      ← 任意值
  5×  text-[10px]      ← 任意值
  2×  text-lg
  1×  text-xl / text-base / text-2xl 各 1
```

**问题**：291 次使用集中在 12px/14px 两个值上，占全部字号声明的 94%。
界面没有"标题—正文—元信息"的层级，只有"稍大一点"和"稍小一点"。
`DESIGN.md` 定义了 Title/Body/Label/Mono 四档，但代码里 Title(18px) 只用了 2 次
（Header 和 App 错误页），Label 档被 `text-xs` 和 `text-[11px]` `text-[10px]`
三个值随机顶替。

**后果**：这是"廉价感"的头号来源。高级界面的层级差通常 ≥1.25 倍，
而 12→14 只有 1.17 倍，且 line-height 未随字号调整，视觉上是一坨均质灰字。

### 1.2 圆角语汇混乱且与 token 脱节

```
110×  rounded-md   → calc(var(--radius) - 2px) = 10px
 69×  rounded      → Tailwind 默认 0.25rem = 4px  ← 未走 token！
 16×  rounded-full
  8×  rounded-lg   → var(--radius) = 12px
```

**问题**：`rounded`（裸类）出现 69 次，解析为硬编码 4px，完全绕过
`--radius` 设计令牌。这意味着用户在设置里换主题，圆角不会跟随，而且
同一个界面上同时存在 4px / 10px / 12px 三种圆角，边缘节奏是断裂的。
`DESIGN.md` 明确规定"控件 10px、面板 12px"，实际执行率不到 60%。

### 1.3 控件高度无标准

```
10×  h-8  (32px)     4×  h-9  (36px)     4×  h-10 (40px)
 3×  h-7  (28px)     4×  h-6  (24px)     3×  h-14 (56px)
```

`DESIGN.md` 写"按钮通常 38–40px 高"，实际主力是 32px，且 28/32/36/40 四种高度
在同一行工具栏里混用（见 `LibraryToolbar.tsx`：搜索框、两个 select、
视图切换组 `h-7`、批量按钮 `h-8` 并排）。**同一行内基线不齐是最直观的"业余感"信号。**

### 1.4 间距不成体系

```
75×  py-2   59×  px-3   40×  px-2   30×  px-4   24×  px-1
21×  p-1    19×  py-1.5 19×  py-1   13×  py-0.5 ...
100× gap-2  73×  gap-1  34×  gap-1.5 32×  gap-3  21× gap-0.5
```

`px-1`（4px）出现 24 次——主要在 `PromptList` 的表格单元格里，导致列间距过窄；
`gap-0.5`（2px）21 次、`gap-1.5`（6px）34 次，说明间距是"目测微调"出来的而非
从 4/8/12/16/24 节奏推导。`DESIGN.md` 定义的节奏在代码里没有强制力。

### 1.5 顶部 chrome 堆叠五层横线

Prompts 视图从上到下依次是：

| 层 | 组件 | 高度 | 分隔 |
|---|---|---|---|
| 1 | `TitleBar` | h-9 (36px) | `border-b` |
| 2 | `Header`（只有一个标题文字） | h-14 (56px) | `border-b` |
| 3 | `LibraryHeader`（标题+副标题+3 按钮） | ~64px | `border-b` |
| 4 | `LibraryToolbar`（搜索+排序+视图+批量） | ~48px | `border-b` |
| 5 | `FilterChips`（条件时显示） | ~36px | `border-b` |

**合计 200–240px 的纯 chrome**，五条等权重的 1px 分隔线，且
`Header` 显示 "Prompts"、`LibraryHeader` 显示 "All / 收藏 / 文件夹名"——
标题信息重复了一次。这是布局层面最大的浪费和最明显的杂乱来源。

### 1.6 没有共享基元，样式定义被复制

- `iconButtonClass` 这个同名常量在 **6 个文件**里各自定义了一份
  （`PromptsView`、`LibraryHeader`、`FolderPicker`、`PromptDetailModal`、
  `PromptTypePicker`、`AppearancePanel`），字符串不完全相同。
- 全项目 **152 个裸 `<button>`**，各自拼样式。
- `focus-visible` 只出现在 16 个文件里，31 个组件文件中约一半的可点击元素
  **没有焦点环**——既是可访问性缺陷，也让键盘操作显得粗糙。
- 选中态有 `bg-primary/10`(7) 和 `bg-primary/15`(11) 两套值。
- `disabled:opacity-` 有 30/35/40/50/60 **五种**取值。

### 1.7 材质与层级信息缺失

```
shadow-sm ×2   shadow ×2   shadow-md ×2   shadow-lg ×2
```

全项目只有 8 处阴影。`DESIGN.md` 的"Tonal-First"原则本身是对的，但目前
执行成了"什么都没有"：卡片、面板、侧边栏、内容区几乎同色，只靠 1px 灰线区分。
`--surface-inset` token 已定义却几乎未被使用。结果是界面**平到没有空间感**——
这与"高级"恰恰相反；高级的扁平设计靠的是精确的 tonal step，不是取消层级。

### 1.8 其他

- **`window.confirm` 9 处**：原生浏览器对话框，与自定义 `Modal` 视觉完全割裂，
  在 Tauri 桌面窗口里尤其突兀。
- **22 个原生 `<select>`**：跟随操作系统渲染，无法主题化，在深色主题下经常是
  浅色下拉，直接破坏整体质感。
- **`tabular-nums` 只用了 2 次**：列表里的使用次数、版本号、日期在滚动时会左右抖动。
- **密度 token 形同虚设**：`--density-padding` / `--density-gap` 和
  `.density-p` / `.density-gap` 工具类定义了，但全项目只被引用 **2 次**。
  设置页的"密度"选项对界面几乎没有实际影响。
- **`font-display` / `font-body` 工具类基本未用**，字体设置的作用面窄于承诺。
- **`EvaluationWorkbench.tsx` 单行 className 长达 1073 字符**，
  表格、按钮、表单挤在一行 JSX 里——不可维护，也是该屏视觉最粗糙的原因。

---

## 2. 根因：缺失的中间层

```
现在:  设计令牌 (globals.css)  ──────跨越 8000 行────→  152 个裸 <button> / 内联 className
                                    ↑
                              没有任何东西在这里
```

Token 层是干净的（语义命名、HSL 通道、双主题齐备）。`DESIGN.md` 也是专业的。
问题**不在设计意图，而在没有把意图固化成可复用的代码**。
每个组件都在从零重新解释"一个按钮该长什么样"，于是产生 5 种 disabled 透明度、
4 种控件高度、3 种圆角。

**因此本次重构的核心不是"换个皮肤"，而是补上基元层。**
只要基元层建立，80% 的视觉不一致会自动消失。

---

## 3. 目标设计语言

保持 workbench 定位，在四个维度上提升：

| 维度 | 现状 | 目标 |
|---|---|---|
| **层级** | 两档字号、一层平面 | 6 档字号阶梯 + 4 层 tonal 表面 |
| **精度** | 高度/圆角/间距随机 | 全部从 token 推导，同行基线严格对齐 |
| **材质** | 无阴影、无内衬 | 极轻的 inset 高光 + 分层表面，深色下 1px 顶部高光 |
| **节奏** | 五层 chrome 堆叠 | 合并为两层，内容区起始位置上移 ~90px |

**三条不可违背的原则**（继承并强化 `DESIGN.md`）：

1. **Signal, not decoration** — 强调色只用于主操作、选中、焦点。不做渐变背景。
2. **Tonal before shadow** — 层级优先用表面色阶表达；阴影只给浮层。
   但"tonal"必须是**可感知的色阶**（≥3% 明度差），不是几乎同色。
3. **No SaaS dashboard** — 不加大卡片、不加装饰性数据卡、不加玻璃模糊。

---

## 4. Token 层重构

### 4.1 新增字号阶梯（替代 `text-xs`/`text-sm` 二元制）

在 `tailwind.config.js` 的 `fontSize` 里定义带 line-height 和 letter-spacing 的复合 token：

| Token | size / line-height / tracking | 用途 |
|---|---|---|
| `text-micro` | 10px / 14px / +0.04em | 角标、计数徽章 |
| `text-meta` | 11px / 16px / +0.02em | 时间戳、列头、版本号 |
| `text-label` | 12px / 16px / +0.01em | 字段标签、chip、次级按钮 |
| `text-body` | 13px / 20px / 0 | **界面主力字号**（列表、表单、正文） |
| `text-title` | 15px / 22px / −0.006em | 面板标题、卡片标题 |
| `text-display` | 19px / 26px / −0.012em | 视图主标题 |

要点：

- **主力字号从 14px 降到 13px，但行高从 1.25 提到 1.54。**
  工作台类界面的高级感来自"字小、行距大、留白足"，而不是字大。
  当前 `text-sm` 配 `leading-tight` 是最容易显廉价的组合。
- 大字号收紧字距（负 tracking），小字号放开字距——这是排版专业度的基本功，
  当前项目 letterSpacing 一律为 0。
- `text-[10px]` / `text-[11px]` 这 18 处任意值全部替换为 `text-micro` / `text-meta`。

### 4.2 补齐表面色阶

当前只有 `--background` / `--card` / `--surface-inset` 三级且差异过小。改为四级 + 明确用途：

| Token | 用途 | 浅色建议 | 深色建议 |
|---|---|---|---|
| `--surface-base` | 应用画布（内容区） | 现 `--background` | 现 `--background` |
| `--surface-raised` | 侧边栏、头部 chrome | 比 base 亮 2% | 比 base 亮 2.5% |
| `--surface-overlay` | 弹窗、气泡、命令面板 | 纯白 | 比 base 亮 4% |
| `--surface-sunken` | 输入框、代码块、表格斑马纹 | 比 base 暗 1.5% | 比 base 暗 2% |

同时新增交互态 token，消灭散落的 `/10` `/15` 魔法数字：

```
--state-hover      表面 hover（当前 hover:bg-accent 用了 72 次，语义不清）
--state-selected   选中行/卡片底色
--state-pressed    按下态
```

### 4.3 材质 token（新增，这是"高级感"的关键增量）

```css
/* 深色主题下，在 raised 表面顶部加 1px 内高光，模拟受光边缘 */
--hairline-top: inset 0 1px 0 hsl(0 0% 100% / 0.04);
/* 浅色主题下反向：底部加极轻内阴影 */
--hairline-bottom: inset 0 -1px 0 hsl(220 20% 20% / 0.03);

--shadow-overlay: 0 16px 48px -12px hsl(var(--shadow-hue) / 0.4),
                  0 0 0 1px hsl(var(--border) / 0.6);
--focus-ring: 0 0 0 2px hsl(var(--surface-base)),
              0 0 0 4px hsl(var(--ring) / 0.55);
```

**这一条的价值**：`inset` 顶部高光是 Linear / Raycast / macOS 原生控件让深色界面
显得"有材质"而非"死黑"的核心手法，成本极低（一行 box-shadow），观感提升显著，
且完全不违反 "Tonal-First"（它不是投影，是边缘光）。

焦点环用"底色环 + 强调色环"双层，让焦点在任何表面上都清晰，
且**不改变元素尺寸**（当前部分组件用 `ring-2` 会挤压布局）。

### 4.4 控件尺寸 token

| Token | 值 | 用途 |
|---|---|---|
| `--control-xs` | 24px | 表格内图标按钮 |
| `--control-sm` | 28px | 工具栏图标按钮、chip |
| `--control-md` | 32px | **默认**：输入框、select、次级按钮 |
| `--control-lg` | 36px | 主按钮、搜索框 |

规则：**同一横向容器内只允许一个高度档**。这一条能独立解决 1.3 节的问题。

### 4.5 让密度 token 真正生效

`--density-padding` / `--density-gap` 目前只被引用 2 次。方案：
把基元组件（Button/Input/ListRow/Panel）的内边距全部改为
`calc(var(--density-padding) * k)` 形式，让设置页的密度选项对整个界面生效。
这同时把"紧凑/舒适"从一个假开关变成真功能。

---

## 5. 基元组件层（新增）

在 `src/components/ui/` 下补齐（当前只有 `Modal`）。这是投入产出比最高的部分。

| 组件 | 替换掉 | 说明 |
|---|---|---|
| `Button` | 152 个裸 `<button>` | variants: `primary` / `secondary` / `ghost` / `danger`；sizes: `sm` / `md` / `lg`；内置 loading、disabled、focus ring |
| `IconButton` | 6 份重复的 `iconButtonClass` | 强制 `aria-label`，内置 tooltip 挂点 |
| `Input` / `Textarea` | 各处裸 input | 统一高度、focus、error、前后缀插槽 |
| `Select` | **22 个原生 `<select>`** | 自绘下拉，可主题化，键盘可达 |
| `Chip` | FilterChips / 标签云 / 类型徽章各写一遍 | `removable` / `pressed` 变体，选中态带 ✓（非仅靠颜色） |
| `Panel` | 各处 `rounded-lg border p-3/p-4` | 标准 12px 圆角 + hairline + 标题槽 |
| `Toolbar` | LibraryToolbar 等 | 保证子项基线对齐与统一 gap |
| `ConfirmDialog` | **9 处 `window.confirm`** | 复用 `Modal`，支持 danger 语义 |
| `Tooltip` | 现在全靠 `title=` 属性 | 原生 tooltip 延迟 1s 且样式不可控 |
| `EmptyState` | 空列表处的临时拼装 | 图标 + 标题 + 说明 + 主操作 |
| `Kbd` | 快捷键提示 | 统一等宽小键帽样式 |

**边界**：基元只负责外观与可访问性，不含业务逻辑；不引入组件库依赖
（当前只有 lucide + zustand + i18next，保持轻量）。

---

## 6. 布局与排版重构（逐屏）

### 6.1 应用外壳 —— 合并 chrome，回收 ~90px

```
现在                                目标
┌─ TitleBar        36px ─┐         ┌─ TitleBar (36px)                    ─┐
├─ Header          56px ─┤         │   拖拽区 · 面包屑 · 窗口控制          │
├─ LibraryHeader   64px ─┤         ├─ ViewBar (44px)                     ─┤
├─ LibraryToolbar  48px ─┤         │   视图标题+计数 · 搜索 · 排序 ·      │
├─ FilterChips     36px ─┤         │   视图切换 · 批量 · [+ 新建]         │
└─ 内容                  ┘         ├─ FilterChips (仅有筛选时, 32px)     ─┤
   chrome 合计 ~240px               └─ 内容                               ┘
                                       chrome 合计 ~112px
```

具体动作：

1. **删除 `Header.tsx`**。它只渲染一行视图标题，而 `LibraryHeader` 已经渲染了
   更具体的作用域标题——信息重复。视图名改放到 `TitleBar` 的面包屑里
   （`PromptHub / 提示词 / 收藏`），既省一层又给了导航上下文。
2. **`LibraryHeader` + `LibraryToolbar` 合并为单条 `ViewBar`**（44px）。
   左：作用域标题（`text-title`）+ 计数（`text-meta`，`tabular-nums`）。
   右：搜索、排序、视图切换、批量、主按钮，全部 `--control-md`，`gap-2`。
   导入/导出移入"更多"菜单——它们是低频操作，不该常驻占位。
3. **导入面板从内联展开改为浮层**。现在 `showImport` 会把整条 header 撑高，
   造成布局跳动（`LibraryHeader.tsx` 的 `showImport &&` 分支）。
4. **分隔线降权**：五条同权 `border-b` 改为——TitleBar 与 ViewBar 之间无线
   （靠 `--surface-raised` 色差区分），ViewBar 与内容之间保留 1px，
   滚动时该线变为带极轻阴影的 sticky 状态。这一个改动就能显著降低"横条感"。
5. **数据路径副标题移除**。当前 `LibraryHeader` 副标题在有 `dataPath` 时会显示
   完整文件系统路径（等宽小字）——这是调试信息，不该出现在主界面，
   移到设置 → 数据路径。

### 6.2 侧边栏 —— 从平铺列表到分组导航

现状问题：`PromptLibraryNav` 里"保存的视图 / 文件夹 / 标签"三段用
`text-[11px] uppercase tracking-wide` 标题分隔，标签云是一片
`rounded-full border` 的碎片，视觉噪音大；折叠态（`w-16`）下退化成一列
无标签图标，信息量骤降。

改造：

- 宽度 `264px` → `248px`，并支持拖拽调宽（工作台的基本预期）。
- 分组标题改用 `text-meta` + `--muted-foreground-subtle`，**去掉 uppercase**
  （中文界面下 uppercase 无效，且英文全大写在 11px 下反而降低可读性）。
- 导航行统一 `--control-md`(32px) 高度、`gap-2`、6px 圆角内嵌于 8px 侧边距，
  选中态用 `--state-selected` 底色 + 左侧 2px 强调色指示条（**非整块蓝底**，
  符合 "Signal, not decoration"），当前的 `bg-primary/15` 满底色在长列表里过重。
- 计数右对齐、`tabular-nums`、`text-meta`。
- 标签云：默认只显示前 8 个高频标签 + "更多"，避免几十个 chip 淹没文件夹树。
- 折叠态给每个图标加 hover tooltip（用新的 `Tooltip` 基元，非 `title=`）。
- 底部把主题切换与设置分组，与上方留 `mt-auto` + 一条 hairline。

### 6.3 提示词列表（`PromptList`）—— 从"表格"到"数据行"

现状：`<table>` + `border-spacing-y-1`，10 列，单元格 `px-1`（4px），
列宽用百分比硬分（22/22/16/10/8/8/10%），窄窗口下全部挤压成省略号。

改造：

- **保留 `<table>` 语义**（可访问性正确，不要退化成 div），但重排视觉：
  - 列减到 6 个主列：标题 / 描述 / 标签 / 类型 / 更新时间 / 操作。
    使用次数与版本号移入标题下方的次级行或 hover 时显示。
  - 单元格内边距 `px-1` → `px-3`，行高 40px（`--control-lg` 对齐）。
  - 表头：`text-meta`、`--muted-foreground-subtle`、sticky、下方 1px 线。
  - 行分隔从 `border-spacing-y-1` 的间隙改为 1px 底线 + hover 整行
    `--state-hover`；选中行用 `--state-selected` + 左 2px 强调条。
  - 所有数字/日期列加 `tabular-nums`。
- **标题单元格双行化**：第一行标题（`text-body`, medium），
  第二行描述（`text-label`, muted, 单行省略）。这样描述列可以取消，
  给标签和类型让出宽度，窄窗口下也不再全是省略号。
- 图标操作（收藏、复制）默认 `opacity-0`，行 hover 或键盘聚焦时显现——
  减少静息状态的视觉噪音，这是列表类界面显精致的标准手法。
  （注意：必须保证键盘 focus 时可见，不能只绑 hover。）
- 中等宽度断点下自动切换为紧凑双列布局，而不是无限压缩。

### 6.4 提示词网格（`PromptGrid`）

现状：`minmax(272px, 1fr)`、`gap-4`、卡片 `border p-3`，选中时
`border-primary ring-1 ring-primary`（双重描边，会导致 1px 布局偏移）。

改造：

- 卡片改用 `Panel` 基元：`--surface-raised` 底 + hairline，**不加投影**；
  hover 时表面提亮一档 + 边框转为 `--border-strong`（不是加阴影，
  避免违反 Tonal-First，也避免 hover 抖动）。
- 选中态：`ring-2 ring-primary/60` + `ring-offset-2 ring-offset-surface-base`，
  去掉 `border-primary`，消除偏移。
- 卡片内部三段式固定节奏：头部（标题+收藏）16px → 描述 2 行（`line-clamp-2`，
  固定 40px 高，保证卡片等高）→ 底部元信息条（`text-meta`，`mt-auto`）。
  当前底部一行把类型/次数/时间/版本/复制按钮五项挤在一起且无分隔，
  改为用 `·` 分隔的三项 + 右侧操作。
- 最小列宽 272 → 260，`gap-4` → `gap-3`，中等窗口能多放一列。

### 6.5 详情弹窗（`PromptDetailModal`）

现状：`max-w-[1180px]` / `max-h-[min(90vh,56rem)]`，内部 header + tabs +
滚动体 + footer。这是全应用最复杂的组件，也是最需要克制的地方。

改造：

- **Tab 样式**：改为下划线式（2px 强调色底边 + medium 字重），
  去掉背景块；tab 栏与内容之间用 hairline 而非实线。
- **内容区限宽**：正文表单加 `max-w-[68ch]` 居中，
  长文案不要横跨 1180px（`DESIGN.md` 已有"65–75 字符"规定但未执行）。
- **章节间距**：`gap-6` → 章节之间 24px，章节内字段之间 12px，
  每个章节标题用 `text-title`，形成清晰的呼吸节奏。
- **字符计数**那行（`characterCount`）从内容顶部移到编辑器右下角，
  它是元信息不是标题。
- **Footer**：左侧快捷键提示用 `Kbd` 基元渲染（现在是纯文本
  `"⌘S 保存 · ⌘C 复制"`，视觉上像未完成的占位符）。
- **脏数据确认弹窗**：三个按钮当前两个是同样的 `border-input` 次级样式，
  层级不清。改为：`保存并关闭`(primary) / `放弃`(ghost-danger) / `继续编辑`(secondary)。
- Modal 遮罩：`bg-background/70 backdrop-blur-sm` → 用中性深色遮罩
  `hsl(220 25% 4% / 0.55)` + `backdrop-blur-[2px]`。当前用 `background` 变量
  做遮罩，在浅色主题下遮罩几乎透明，弹窗失去焦点感。

### 6.6 设置页

现状：左 `w-56` 导航 + 右 `max-w-3xl` 内容，各面板自己拼 section。

改造：

- 左导航与主侧边栏视觉统一（同一 NavRow 基元、同样的选中指示条）。
- 内容区限宽 `max-w-3xl`(768px) → `max-w-[42rem]`(672px)，更接近理想阅读宽度。
- **统一 `SettingRow` 基元**：左侧标签+说明（`text-body` / `text-label` muted），
  右侧控件右对齐，行间 1px hairline 分隔。现在每个 Panel 各写各的，
  `AppearancePanel` 的 `Section` 与 `GeneralPanel` 的结构完全不同。
- `AppearancePanel` 的主题/强调色选择器改为色板网格（带选中 ✓），
  而不是当前的按钮列表——这一屏是"展示设计能力"的橱窗，值得额外打磨。
- 顶部的 `restartRequired` / `error` 横幅统一为 `Banner` 基元，
  带图标 + 语义色 + 可关闭。

### 6.7 评估工作台（`EvaluationWorkbench`）

**这是全项目视觉最差的一屏**：单行 className 长达 1073 字符，
表格、表单、按钮全挤在超长 JSX 行里，`text-xs` 从头用到尾。

改造：先做**结构性拆分**（拆成 `ProfilePanel` / `EvaluatorPanel` /
`RunTable` / `ResultDetail` 四个子组件），再套用统一基元。
不拆分直接改样式是不现实的。

### 6.8 Toast

`shadow-md` + `rounded-md` + 无图标。改为：`--surface-overlay` +
`--shadow-overlay` + 语义图标（success/danger/info）+ 左侧 2px 语义色条 +
进度条式自动消失指示。位置从 `bottom-4 right-4` 改为 `bottom-4 right-4` 保持，
但入场动画从下方 8px 上滑改为右侧 16px 左滑（更符合右下角来源）。

---

## 7. 交互与动效

现状：`transition-colors` 28 次，无统一时长；`prompt-detail-enter` 160ms 是唯一的
入场动画；`prefers-reduced-motion` 已正确处理（这点做得好，保留）。

规范：

| 场景 | 时长 | 缓动 |
|---|---|---|
| 颜色 / 不透明度（hover、focus） | 120ms | `ease-out` |
| 尺寸 / 位移（侧边栏折叠、展开） | 180ms | `cubic-bezier(0.32, 0.72, 0, 1)` |
| 浮层入场（Modal、Toast、Popover） | 200ms | 同上 |
| 浮层退场 | 120ms | `ease-in` |

要点：

- 统一定义 `--ease-out` / `--ease-spring` / `--dur-fast` / `--dur-base` token。
- 侧边栏当前用 `transition-[width] duration-200`，但内部文字是直接
  `{!collapsed && ...}` 硬切换，折叠时文字瞬间消失、宽度慢慢收——不同步。
  改为文字先 fade（80ms）再收宽度。
- **hover 不改变尺寸、不加阴影**（避免抖动），只改表面色与边框。
- 所有可交互元素必须有 `focus-visible` 环（当前约一半缺失）。

---

## 8. 实施路线图

按"改动风险从低到高、视觉收益从高到低"排序。每阶段独立可交付、可回滚。

### 阶段 0 — 基线（0.5 天）
- 补 UI 快照/视觉回归基线（当前测试覆盖逻辑但不覆盖布局，
  已有 `PromptsView.layout.test.tsx` 可扩展）。
- 截图存档现有 6 个主要屏幕，作为前后对比。

### 阶段 1 — Token 层（1 天，零组件改动）
- 扩展 `tailwind.config.js`：6 档 fontSize、控件高度、缓动、间距别名。
- 扩展 `globals.css`：表面四级、交互态、材质（hairline / overlay shadow /
  focus ring）、动效 token。**双主题（`:root` 与 `.dark`）必须同时声明**——
  `.trellis/spec/frontend/design-tokens.md` 有强约束，且有
  `token-completeness.test.ts` 在守护，新增 token 需同步更新该测试。
- 同步更新 `DESIGN.md` 与 `design-tokens.md`。
- **此阶段结束时界面观感变化很小，但地基就位。**

### 阶段 2 — 基元组件（2 天）
- 按第 5 节清单实现 `src/components/ui/`。
- 每个基元配单测（沿用现有 `Modal.test.tsx` 模式）。
- 暂不替换调用方。

### 阶段 3 — 全局替换（2 天）
- 6 份 `iconButtonClass` → `IconButton`。
- 152 个 `<button>` → `Button` / `IconButton`。
- 9 处 `window.confirm` → `ConfirmDialog`。
- 22 个 `<select>` → `Select`。
- 69 处裸 `rounded` → token 化圆角。
- 18 处 `text-[10px]/[11px]` → `text-micro` / `text-meta`。
- **这一步单独提交，diff 大但机械，评审时只需确认无行为变化。**

### 阶段 4 — 布局重构（2 天）
- 6.1 外壳合并（删 `Header`，合并 ViewBar）。
- 6.2 侧边栏。
- 6.3 / 6.4 列表与网格。
- 这是用户感知最强的一步。

### 阶段 5 — 深水区（2 天）
- 6.5 详情弹窗。
- 6.6 设置页。
- 6.7 评估工作台拆分 + 重构。

### 阶段 6 — 打磨（1 天）
- 动效统一、焦点环全覆盖、`tabular-nums`、空状态、加载骨架。
- 深色/浅色 × 4 个主题家族 × 3 档密度 的组合走查。

**合计约 10.5 人天。** 阶段 1–3 可先行合并（低风险、消除不一致），
阶段 4 起建议按屏分 PR。

---

### 实施状态（本次已完成 1／2／3／4／5／6）

| 阶段 | 状态 | 说明 |
|---|---|---|
| 1 Token 层 | ✅ | 字号阶梯、控件高度、交互态、标签色板、材质、动效 token |
| 2 基元组件 | ✅ | 15 个基元 + `useConfirm`；阶段 5 补齐 `Textarea`/`Switch`/`SettingRow`/`Banner`；`TypeIcon`/`VersionBadge`/`Chip`/`Toolbar`/`Tooltip` 未做（无迫切调用方） |
| 3 全局替换 | ✅ | 见第 9 节指标表，全部归零 |
| 4 布局重构 | ◐ | 已删外壳页头、统一控件高度；侧栏与列表/网格的重排未做 |
| 5 深水区 | ◐ | `EvaluationWorkbench` 已拆分并套基元；详情弹窗排版与层级已细化；设置页已统一 `SettingRow`/`Switch`/`Banner`；Toast 已改语义色条。**「详情模态 → 右侧停靠面板」的状态机改造未做**（见下） |
| 6 打磨 | ◐ | 动效与焦点环已统一；空状态、骨架屏、主题走查未做 |

**与原清单的偏差（已按实际实现回写）：**

- 未新增 `--primary-hi` 与 `--focus-ring`：焦点色直接复用 `--ring`，
  多一个近义 token 只会增加漂移面。
- 标签色 token 由语义命名（`--tag-eng` 等）改为编号槽位 `--tag-1..8`
  加哈希映射。标签是用户自由输入的文本，语义命名无法覆盖，
  且在 7 种语言下不成立。
- 新 token **未**纳入 `FLAVOR_OVERRIDES`：四个主题家族共用同一套
  交互态与标签色，仅在 `globals.css` 的 `:root` / `.dark` 两个 scope 声明。

**阶段 5 实际范围与取舍：**

阶段 5 原计划的三块中，`EvaluationWorkbench` 拆分、设置页统一、
详情弹窗排版细化均已完成，但 **6.5 的「详情模态 → 右侧停靠面板」没有做**。
这一条不是排版问题而是状态机改造：需要新增 `detailPanelOpen` 偏好、
让列表与面板同时可交互、处理窄窗口下的降级回模态，
且会与 `PromptsView` 现有的 `overlayOpen` 逻辑正面冲突。
把它塞进这一轮会让 PR 同时包含"改样式"和"改交互模型"两类风险，
评审时无法分开回滚，因此单独留作后续 PR。

本轮阶段 5 顺带修掉的既有缺陷：

- **8 个 `Select` 的样式叠加**：阶段 3b 的 codemod 把原 `inputClass`
  原样搬进了 `wrapperClassName`，导致 border / padding / background
  在 `Select` 自带外壳上叠加了第二层，且 `rounded-sm` 与基元的
  `rounded-md` 相互打架。改为 `block` / `size` / 布局类。
- **模态遮罩用 `bg-background/70`**：在浅色主题下是一层白雾，
  读起来像蒙在页面前面而不是弹窗后面。新增中性 `--scrim` 令牌
  （两个 scope 同值），并由守护测试锁定。该守护当场揪出
  `CloseDialog.tsx` 这个此前四处遮罩中被遗漏的一处。
- **开关旋钮硬编码 `bg-white`**：深色主题下在 Signal Blue 轨道上刺眼，
  改为 `bg-card`。

---

## 9. 验收标准

可自动化检查的硬指标：

| 指标 | 改造前 | 目标 | 当前 |
|---|---|---|---|
| 裸 `rounded` 类（绕过 token） | 69 | 0 | **0** ✅ |
| 任意值字号 `text-[Npx]` | 18 | 0 | **0** ✅ |
| 原生 `text-xs` / `text-sm` | 285 | 0 | **0** ✅ |
| `iconButtonClass` 重复定义 | 6 | 0 | **0** ✅ |
| `window.confirm` | 9 | 0 | **0** ✅ |
| 原生 `<select>` | 22 | 0 | **0** ✅ |
| `disabled:opacity-*` 取值种类 | 5 | 1 | **1** ✅ |
| 选中态 `bg-primary/*` 取值种类 | 3 | 1（走 `--state-selected`） | **1** ✅ |
| 逐组件 `focus-visible:ring` | 131 | 0（收敛到基础层单条规则） | **0** ✅ |
| 单行 className > 200 字符 | 4 | 0 | **0** ✅ |
| 未绑定时长 token 的 `transition-*` | 38 | 0 | **0** ✅ |
| 顶部 chrome 总高（Prompts 视图） | ~240px | ≤ 112px | **~124px** ⚠️ |

> chrome 未达到 112px：`LibraryHeader`（标题+统计）与 `LibraryToolbar`
> （搜索+排序+视图切换）承载的是不同职责，强行合并成一行会在窄窗口下
> 挤压搜索框。124px 已删掉全部纯重复的层级，进一步压缩需要重新设计
> 信息架构（阶段 5 的停靠面板改造），不适合放在本次机械替换里。

**以上指标已全部固化为可执行断言**：`src/theme/style-conventions.test.ts`
（15 条）与 `src/components/layout/chrome.test.ts`（3 条）。这些断言做过
反向验证——注入违规代码后会转红。手工清理完的指标若没有守护，
下一个 PR 就会重新引入。

人工走查：

- 任意一行工具栏内，所有控件顶/底边线严格对齐。
- 深色主题下，侧边栏 / 内容区 / 卡片 / 输入框四个表面层次可辨。
- 键盘 Tab 走完 Prompts 全屏，每一步焦点位置明确可见。
- 四个主题家族 × 明暗，无任何硬编码颜色泄漏。
- WCAG 2.2 AA：正文对比度 ≥ 4.5:1，大字与图标 ≥ 3:1。

---

## 附：不建议做的事

- ❌ **引入 shadcn/ui 或 Radix**：当前依赖极轻（lucide + zustand + i18next），
  Tauri 包体敏感；且 `Modal` 已自行实现了正确的焦点陷阱与 inert 处理，
  没必要为 11 个基元引入运行时依赖。
- ❌ **玻璃拟态 / 渐变 / 发光**：`DESIGN.md` 明令禁止，且与工作台定位冲突。
  "高级"来自精度而非特效。
- ❌ **加大卡片与留白到 SaaS 程度**：这是提示词管理工具，密度是功能而非缺陷。
- ❌ **换字体**：IBM Plex Mono 已打包且合适；界面字体走系统栈是正确的桌面端选择。
- ❌ **一次性大重构 PR**：8000 行 tsx，必须分阶段。

---

## 10. 与 Fable 方案的融合决策

Fable 对**同一个真实界面**做了独立诊断。两份分析在若干点上独立收敛到同一结论
（双层标题栏冗余、缺表面层级、标题信息重复、无 hover/选中反馈、纯文本难扫描），
这本身是结论可靠的旁证。以下逐条记录取舍。

### 10.1 全额采纳（Fable 更优，已替换原方案）

| Fable 提案 | 原方案做法 | 为什么 Fable 更好 |
|---|---|---|
| **右侧停靠详情面板** | 详情用 1180px **模态** | 模态遮挡列表，无法边看列表边比对；桌面端有 376px 富余宽度就该常驻。**这是本次融合最大的结构性改进。** |
| **全局标签色板** | 标签是无色 `chip--static` | 同一标签在侧栏 / 工具栏 / 列表 / 卡片 / 详情里同色，扫描成本骤降。原方案漏掉了这条。 |
| **使用次数迷你进度条** | 纯数字 + `tabular-nums` | 相对量级一眼可见，数字仍保留。符合"可视化优于纯文本"。 |
| **类型图标格（26px 圆角容器）** | 行内 13px 裸图标 | 给列表提供稳定的左锚点，行的起始位置整齐得多。 |
| **版本等宽徽标** | 裸等宽文字 | `v5` 有了容器后不再像散字。 |
| **等宽字体的使用边界** | 未明确约束 | Fable 明确"仅数字/版本/路径/快捷键"。原方案把数据路径也用了等宽，方向一致但没写成规则。**已写入规范。** |
| **工作区切换器** | 只有静态产品名+版本号 | 头像 + 工作区名比"PromptHub v1.0.0"信息量高。 |
| **状态栏快捷键提示** | 只有分页 | `↑↓ 导航 / ↵ 打开 / ⌘C 复制` 是键盘可发现性的低成本入口。 |

### 10.2 采纳方向但调整了执行

| 项 | Fable 做法 | 融合后 | 理由 |
|---|---|---|---|
| **页头** | 面包屑 → 24px 大标题 → 统计行，约 120px | 保留三段结构，收紧到 **92px** | 原方案"删 Header 合并成 44px ViewBar"过于激进，牺牲了统计摘要这个真实有用的信息。但 Fable 的页头偏高，压缩间距后两全。 |
| **强调色** | 紫罗兰渐变主按钮 | 采纳，但**仅主按钮用渐变** | 渐变泛用会滑向 SaaS 观感。`DESIGN.md` 的 "Signal, not decoration" 仍是硬约束。 |
| **行错落淡入** | 全列表逐行动画 | 保留，但 `delay` 上限 **260ms** | 16 行 ×18ms 可接受；248 行若不封顶，最后一行要等 4 秒。 |
| **收藏琥珀色** | 星标用琥珀 | 采纳，并提为语义 token `--favorite` | 避免它被误当成第二强调色使用。 |

### 10.3 未采纳（保留原方案）

| Fable 提案 | 决定 | 理由 |
|---|---|---|
| 标签用**大写字距**（`uppercase`+`tracking`） | 否 | 中文标签（工程/测试/思维）上 `uppercase` 完全无效，且 11px 全大写英文反而降低可读性。这是本项目 i18n 覆盖 7 种语言的现实约束。 |
| 引入 **JetBrains Mono** | 部分 | 字体名写进栈的首位，但**不新增打包字体**。`IBM Plex Mono` 已 bundled（latin 400/500），Tauri 包体敏感，多打一套字体不值。JetBrains Mono 仅在用户系统已装时生效。 |
| 侧栏宽度 | 保持 **236px** | Fable 未明确给值；原方案 248 略宽，压到 236 给主区多让 12px。 |

### 10.4 融合后新增的硬指标

追加到第 9 节验收表：

| 指标 | 目标 |
|---|---|
| 标签颜色定义点 | 1 处（`--tag-*` token），组件内不得硬编码 |
| 等宽字体用于正文/标题 | 0 处（仅数字、版本、路径、快捷键） |
| 详情面板打开时列表可见 | 是（停靠，非模态） |
| 列表入场动画总时长 | ≤ 260ms（无论多少行） |
| 渐变使用位置 | 仅主按钮 + 品牌标记 + 进度条填充 |

### 10.5 对实施路线图的影响

- **阶段 1（Token 层）** 增加：`--tag-*` 八色板（含双主题）、`--favorite`、`--primary-hi`。
  注意 `token-completeness.test.ts` 需同步扩充。
- **阶段 2（基元层）** 增加 `Tag`（带全局色板查表）、`UsageBar`、`TypeIcon`、`VersionBadge`；
  `ConfirmDialog` 与 `Modal` 保留（脏数据确认等仍需模态）。
- **阶段 5** 原"详情弹窗重构"改为 **"详情模态 → 停靠面板"的结构性改造**，
  工作量上升（涉及 `PromptDetailModal` 的布局重写与 `PromptsView` 的三栏化），
  从 2 天调整为 **3 天**；总工期 10.5 → **11.5 人天**。
- 面板宽度需可拖拽、可折叠，状态持久化到 settings（新增偏好项 `detailPanelOpen`）。
