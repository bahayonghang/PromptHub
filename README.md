<div align="center">

# PromptHub

**本地优先的 AI Prompt 与规则管理桌面应用**

基于 Tauri 2 + React 18 + Rust 重写

简体中文 · [English](./README.en.md)

![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.77+-000000?style=flat-square&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-FTS5-003B57?style=flat-square&logo=sqlite&logoColor=white)

![macOS](https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat-square&logo=windows&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat-square&logo=linux&logoColor=black)

</div>

---

PromptHub 把你的 Prompt、Prompt 版本和平台规则集中放进一个本地工作区，提供全文搜索、加密保护、多服务商 AI 测试以及备份同步能力。

数据默认存在你自己的电脑上。

> 本仓库是 PromptHub **桌面端**在 [Tauri](https://tauri.app/) 上的全新重写：Rust 后端重新实现原 Electron 主进程的全部职责，并通过 Tauri 命令 / 事件暴露给前端；React 前端经由统一的运行时桥接层（Runtime Bridge）调用后端。原 Electron 实现作为只读参考保留在 `ref/PromptHub`。

## 目录

- [核心能力](#核心能力)
- [技术栈](#技术栈)
- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [常用命令](#常用命令)
- [项目结构](#项目结构)
- [架构概览](#架构概览)
- [许可证](#许可证)

## 核心能力

### 📝 Prompt 管理
- 文件夹、标签层级组织，CRUD 全覆盖
- 模板变量 `{{variable}}`，复制 / 测试时填值
- 基于 SQLite FTS5 的全文搜索
- 多媒体附件（图片 / 视频）管理与预览

### 🕒 版本控制
- 每次保存自动写入历史版本
- 版本对比、差异查看、一键回滚

### 📐 Rules（AI 编程规则）
- 集中管理 `.cursor/rules`、`.claude/CLAUDE.md`、`AGENTS.md` 等规则文件
- 基于内容哈希（SHA-256）检测同步状态与本地改动
- 历史快照预览与恢复

### 🤖 AI 测试
- 多服务商支持（OpenAI、Anthropic、Gemini、Azure、自定义 endpoint 等）
- 流式响应，支持取消在途请求
- 同一 Prompt 多模型并行对比

### 💾 数据、同步与备份
- 本地优先，数据默认留在本机
- WebDAV 与 S3 备份 / 同步传输
- ZIP 导出选定范围
- 启动自动拉取 + 后台定时同步

### 🔐 隐私与安全
- 主密码保护，AES-256-GCM 加密（scrypt 派生密钥）
- 私密文件夹内容加密存储
- 跨平台离线运行：Windows / macOS / Linux

### 🌐 系统集成与多语言
- 全局快捷键、开机自启、系统通知、托盘
- 应用内自动更新（签名校验）
- 7 种界面语言：简体中文、繁體中文、English、日本語、Deutsch、Español、Français

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 后端 | Rust 2021（rusqlite + FTS5、r2d2 连接池、tokio、reqwest/rustls） |
| 前端 | React 18 + TypeScript 5 + Vite 6 |
| 状态管理 | Zustand |
| 样式 | TailwindCSS 3 + PostCSS |
| 国际化 | i18next + react-i18next |
| 图标 | lucide-react |
| 加密 | aes-gcm + scrypt + rand |
| 测试 | Vitest + fast-check（前端）、proptest（Rust） |

## 环境要求

- **Node.js** ≥ 18（建议 LTS）
- **Rust** ≥ 1.77.2（含 Cargo）
- **Tauri 系统依赖**：请按官方文档安装对应平台的依赖（Windows 需 WebView2 与 MSVC 构建工具，macOS 需 Xcode Command Line Tools，Linux 需 webkit2gtk 等）。详见 [Tauri Prerequisites](https://tauri.app/start/prerequisites/)。

## 快速开始

```bash
# 克隆仓库
git clone https://github.com/bahayonghang/PromptHub.git
cd PromptHub

# 安装前端依赖
npm install

# 启动桌面端开发环境（Vite + Tauri 原生窗口）
npm run tauri dev
```

构建可分发的安装包：

```bash
npm run tauri build
```

构建产物会输出到 `src-tauri/target/release/bundle/`，按宿主操作系统生成对应安装包（Windows: nsis / msi，macOS: app / dmg，Linux: appimage / deb）。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm run dev` | 仅启动 Vite 前端开发服务器（http://localhost:5173） |
| `npm run tauri dev` | 启动完整桌面端开发环境 |
| `npm run build` | 前端类型检查 + Vite 生产构建 |
| `npm run tauri build` | 构建桌面端安装包 |
| `npm run preview` | 预览前端生产构建 |
| `npm run test` | 运行前端 Vitest 测试 |
| `cargo test` | 运行 Rust 后端测试（在 `src-tauri/` 目录） |

## 项目结构

```text
PromptHub/
├── src/                    # React 前端
│   ├── components/         # 布局与视图组件
│   ├── features/           # 业务模块（prompts / settings / system）
│   ├── locales/            # 7 种语言的国际化资源
│   ├── runtime/            # Runtime Bridge —— 前端访问后端的唯一入口
│   ├── store/              # Zustand 状态
│   └── theme/              # 主题与设计 token
├── src-tauri/              # Rust 后端（Tauri）
│   ├── src/
│   │   ├── commands/       # Tauri 命令层（替代 Electron IPC）
│   │   ├── services/       # 业务逻辑（prompt / rules / ai / sync …）
│   │   ├── models/         # 领域模型
│   │   └── storage/        # SQLite 存储引擎 + FTS
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ref/PromptHub/          # 只读参考实现（原 Electron 版本）
└── .kiro/specs/            # 重写的需求 / 设计 / 任务规格文档
```

## 架构概览

- **命令层（Command Layer）**：Rust 后端将每个能力暴露为 Tauri 命令，返回统一的 `CommandResult<T>` 结构（成功携带数据，失败携带错误码与可读信息）；异步通知（更新状态、快捷键触发、窗口状态、AI 流式分片）通过 Tauri 事件下发。
- **运行时桥接层（Runtime Bridge）**：前端所有后端调用与事件订阅都经过 `src/runtime`，组件不直接依赖 `@tauri-apps/api`。桥接层还提供能力描述符（capability descriptor），对不可用能力的调用直接短路返回错误。
- **存储引擎（Storage Engine）**：Rust 原生持久化层，使用 bundled SQLite（含 FTS5）与 r2d2 连接池，时间戳统一以 UTC 毫秒 / ISO 8601 表示。

> 说明：本次重写**不**兼容旧 Electron 版本的数据，也不提供迁移；存储层为性能重新设计。CLI 与 Web 变体不在本仓库范围内。

## 许可证

本项目重写自 [legeling/PromptHub](https://github.com/legeling/PromptHub)（AGPL-3.0）。如需发布，请遵循上游许可证条款。
