<div align="center">

# PromptHub

**A local-first desktop app for managing AI prompts and rules**

Rewritten on Tauri 2 + React 18 + Rust

[简体中文](./README.md) · English

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

PromptHub brings your prompts, prompt versions, and platform rules into a single local workspace, with full-text search, encryption, multi-provider AI testing, and backup/sync.

Your data stays on your own machine by default.

> This repository is a clean rewrite of the PromptHub **desktop app** on [Tauri](https://tauri.app/): the Rust backend reimplements all former Electron main-process responsibilities and exposes them through Tauri commands/events, while the React frontend calls the backend through a unified Runtime Bridge. The original Electron implementation is kept as read-only reference under `ref/PromptHub`.

## Table of Contents

- [Features](#features)
- [Tech Stack](#tech-stack)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
- [Common Commands](#common-commands)
- [Project Structure](#project-structure)
- [Architecture](#architecture)
- [License](#license)

## Features

### 📝 Prompt Management
- Folder and tag hierarchy with full CRUD
- Template variables `{{variable}}`, filled in when copying / testing
- Full-text search powered by SQLite FTS5
- Image / video attachment management and preview

### 🕒 Version Control
- Every save records a history version automatically
- Version diff, comparison, and one-click rollback

### 📐 Rules (AI Coding Rules)
- Centrally manage rule files like `.cursor/rules`, `.claude/CLAUDE.md`, `AGENTS.md`
- Content-hash (SHA-256) based sync-status and local-change detection
- History snapshot preview and restore

### 🤖 AI Testing
- Multi-provider support (OpenAI, Anthropic, Gemini, Azure, custom endpoints, etc.)
- Streaming responses with in-flight cancellation
- Compare multiple models on the same prompt in parallel

### 💾 Data, Sync & Backup
- Local-first: data stays on your machine by default
- WebDAV and S3 backup / sync transport
- ZIP export of a selected scope
- Auto-pull on startup + scheduled background sync

### 🔐 Privacy & Security
- Master-password protection with AES-256-GCM encryption (scrypt-derived key)
- Encrypted storage for private folder contents
- Cross-platform offline operation: Windows / macOS / Linux

### 🌐 System Integration & i18n
- Global shortcuts, auto-launch, system notifications, tray
- Built-in auto-updater with signature verification
- 7 UI languages: Simplified Chinese, Traditional Chinese, English, Japanese, German, Spanish, French

## Tech Stack

| Layer | Technology |
| --- | --- |
| Desktop framework | Tauri 2 |
| Backend | Rust 2021 (rusqlite + FTS5, r2d2 pool, tokio, reqwest/rustls) |
| Frontend | React 18 + TypeScript 5 + Vite 6 |
| State | Zustand |
| Styling | TailwindCSS 3 + PostCSS |
| i18n | i18next + react-i18next |
| Icons | lucide-react |
| Crypto | aes-gcm + scrypt + rand |
| Testing | Vitest + fast-check (frontend), proptest (Rust) |

## Prerequisites

- **Node.js** ≥ 18 (LTS recommended)
- **Rust** ≥ 1.77.2 (with Cargo)
- **Tauri system dependencies**: install the platform-specific dependencies per the official docs (WebView2 + MSVC build tools on Windows, Xcode Command Line Tools on macOS, webkit2gtk and friends on Linux). See [Tauri Prerequisites](https://tauri.app/start/prerequisites/).

## Getting Started

```bash
# Clone the repository
git clone https://github.com/bahayonghang/PromptHub.git
cd PromptHub

# Install frontend dependencies
npm install

# Start the desktop dev environment (Vite + native Tauri window)
npm run tauri dev
```

Build a distributable bundle:

```bash
npm run tauri build
```

Artifacts are written to `src-tauri/target/release/bundle/`, producing installers for the host OS (Windows: nsis / msi, macOS: app / dmg, Linux: appimage / deb).

## Common Commands

| Command | Purpose |
| --- | --- |
| `npm run dev` | Start the Vite frontend dev server only (http://localhost:5173) |
| `npm run tauri dev` | Start the full desktop dev environment |
| `npm run build` | Frontend type-check + Vite production build |
| `npm run tauri build` | Build the desktop installer |
| `npm run preview` | Preview the frontend production build |
| `npm run test` | Run frontend Vitest tests |
| `cargo test` | Run Rust backend tests (from `src-tauri/`) |

## Project Structure

```text
PromptHub/
├── src/                    # React frontend
│   ├── components/         # Layout and view components
│   ├── features/           # Domain modules (prompts / settings / system)
│   ├── locales/            # i18n resources for 7 languages
│   ├── runtime/            # Runtime Bridge — the only entry to the backend
│   ├── store/              # Zustand state
│   └── theme/              # Theme and design tokens
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/
│   │   ├── commands/       # Tauri command layer (replaces Electron IPC)
│   │   ├── services/       # Business logic (prompt / rules / ai / sync …)
│   │   ├── models/         # Domain models
│   │   └── storage/        # SQLite storage engine + FTS
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ref/PromptHub/          # Read-only reference implementation (original Electron app)
└── .kiro/specs/            # Rewrite requirements / design / tasks specs
```

## Architecture

- **Command Layer**: the Rust backend exposes each capability as a Tauri command returning a uniform `CommandResult<T>` (data on success, an error code + human-readable message on failure). Async notifications (updater status, shortcut triggers, window state, AI stream chunks) are delivered via Tauri events.
- **Runtime Bridge**: all frontend backend calls and event subscriptions go through `src/runtime`; components never import `@tauri-apps/api` directly. The bridge also exposes a capability descriptor and short-circuits calls to unavailable capabilities with a structured error.
- **Storage Engine**: a Rust-native persistence layer using bundled SQLite (with FTS5) and an r2d2 connection pool, with timestamps normalized to UTC milliseconds / ISO 8601.

> Note: this rewrite is **not** data-compatible with the old Electron build and provides no migration; the storage layer was redesigned for performance. The CLI and web variants are out of scope for this repository.

## License

This project is a rewrite of [legeling/PromptHub](https://github.com/legeling/PromptHub) (AGPL-3.0). If you distribute it, comply with the upstream license terms.
