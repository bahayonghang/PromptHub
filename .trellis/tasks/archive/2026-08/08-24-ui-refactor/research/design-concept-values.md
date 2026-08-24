# The design concept, recorded for review

## Why this file exists

The source is a Claude Design project, `PromptHub 界面重设计`, id
`b31c96f8-b82e-4d92-9e47-9734e0d3e899`, file `PromptHub.dc.html`, read
2026-08-24. The share link requires a login. A reviewer without that login
cannot open it, so every concept value the plan depends on is written here
instead of being reachable only through the link.

**Not recovered.** The file itself is not exported into this repository, and no
screenshot was captured at read time. The values below were transcribed while
reading it. They are the plan's record of the concept, not a re-derivable
artifact. A reviewer can check that the plan is internally consistent with this
file; a reviewer cannot check this file against the original.

**How to make it re-derivable.** Export `PromptHub.dc.html` into
`.trellis/tasks/08-24-ui-refactor/research/`, or capture the five screens as
images, and record the export date. Until then, treat every "the design
specifies X" line in a child `design.md` as sourced from this file.

## Screens in the concept

Five: shell + library, detail overlay with four tabs, command palette, batch
selection bar, toast.

## Values the children's designs cite

| Value                     | Concept                                                      | What the plan does                                                                                                    | Where                   |
| ------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Shell minimum width       | `min-width: 1280px`, fixed                                   | Kept responsive; the existing container queries stay                                                                  | Parent A3               |
| Sidebar width             | 264px                                                        | Adopted                                                                                                               | `shell-sidebar`         |
| Grid card minimum         | 372px                                                        | 272px, because 372px overflows at the app's minimum usable width behind a 264px sidebar; measured in implement step 7 | `library-views` D7      |
| Detail overlay width      | `min(1180px, 100%)`, blurred scrim                           | Adopted                                                                                                               | `detail-modal` D8       |
| Toast lifetime            | ~1.8s                                                        | ~4s, because 1.8s is too short to read a file path or an import summary, and the toast is dismissible                 | `command-palette` D6    |
| Palette prompt group size | 5                                                            | Adopted as `limit: 5`                                                                                                 | `command-palette` D4    |
| Palette groups            | PROMPT, 操作                                                 | Adopted                                                                                                               | `command-palette`       |
| Palette actions           | 新建 Prompt `⌘N`, 只看收藏, 切换列表视图, 切换深浅主题 `⌘⇧L` | Adopted                                                                                                               | `command-palette` D2    |
| Overlay footer hints      | `⌘S` 保存, `⌘Enter` 复制                                     | Adopted                                                                                                               | `command-palette` D2    |
| Overlay tabs              | 内容, 版本历史(n), 试跑对比(n), 引用(n)                      | Adopted                                                                                                               | `detail-modal`          |
| Overlay header            | 标题, 版本 chip, 元数据行, 复制正文, 编辑/只读, 收藏, 关闭   | Adopted, plus pin / duplicate / delete the concept drops                                                              | `detail-modal` D4, R7c  |
| Content-tab sections      | 组织方式, 补充信息, 安全与同步                               | Adopted                                                                                                               | `detail-modal` D3       |
| Sidebar saved views       | 全部, 收藏, 最近使用, 置顶                                   | 全部 / 收藏 / 最近; 最近使用 maps to `updatedAt`; 置顶 dropped                                                        | Parent A1, A5           |
| Sort axes                 | includes 最近使用                                            | Dropped as a sort field; it duplicates 最近更新                                                                       | `library-toolbar` D5    |
| List item badges          | includes 草稿                                                | Dropped; no `is_draft` exists                                                                                         | Parent A6               |
| Batch bar actions         | 移动, 加标签, 收藏, 删除                                     | 收藏 dropped; no `prompt.batchFavorite`                                                                               | Parent A7               |
| Header subtitle           | includes an aggregate usage count                            | Dropped; no source for the aggregate                                                                                  | Parent A10              |
| Reference token           | `@@<title>`                                                  | Adopted, plus an explicit `@@Title@@` form                                                                            | `prompt-references` D7  |
| Token set                 | see `08-24-design-tokens` design                             | Adopted in full                                                                                                       | Parent scope decision 2 |

## Deviation rule

A child `design.md` may deviate from a row above. When it does, it states the
concept value, the chosen value, and the code fact that forces the difference.
Every deviation in the table already does this. A deviation with no stated
reason is a planning defect.
