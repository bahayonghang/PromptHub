# Close to tray and persist close action

## Goal

Closing PromptHub can keep the process resident in the system tray, or quit, according to a saved three-way preference. After a restart the same preference still applies to both the custom title-bar close button and the OS window chrome.

## User value

The app can stay available in the background without occupying a window. The user chooses once in Settings whether close means ask, hide to tray, or exit, and can always restore or fully quit from the tray.

## Background

Req 20.4 already defines `CloseAction` as `ask` | `minimize` | `exit`. Settings → System already renders those three choices (`WindowBehaviorPanel`). The user decided to keep that three-way control and make tray residency plus persistence actually work.

## Confirmed facts

- Close-action decision is implemented and unit-tested in `src-tauri/src/services/window.rs:125-171` (`Ask` → emit `window:close-requested`, `Minimize` → hide, `Exit` → terminate).
- Runtime copy lives only in `CommandRuntimeState.close_action` (`src-tauri/src/commands/mod.rs:36-47`), default `Ask`. It is not a Settings field (`src-tauri/src/models/settings.rs`).
- `window.setCloseAction` / `window.close` exist (`src-tauri/src/commands/window.rs:68-187`). `window.close` is what the custom title-bar X calls (`src/features/system/components/WindowControls.tsx:81-83`).
- No `on_window_event` / `CloseRequested` handler in `src-tauri/src/lib.rs`. `tauri.conf.json` leaves default window decorations on, so the OS chrome close destroys the window and exits, bypassing CloseAction.
- No tray icon is created. `PlatformFeature::Tray` is only a capability probe (`services/window.rs:547-589`). Cargo.toml `tauri` has no `tray-icon` feature.
- `confirmClose()` (`src/features/system/systemStore.ts:304-307`) calls `window.close` while the action is still `ask`, so confirming Exit re-emits `window:close-requested` instead of terminating.
- The ask dialog (`CloseDialog.tsx`) only offers Keep running vs Exit. Unused Electron leftover keys exist at `closeDialog.*` in locale files; live keys are `systemView.close.*`.
- Settings JSON already round-trips optional fields without a schema migration (`services/settings.rs` single-document `settings` table).

## Requirements

- **R1 — Persist close action.** Store `closeAction` on the Settings JSON document as `ask` | `minimize` | `exit`. Absent value means `ask`. `window.setCloseAction` writes both the runtime mutex and Settings. Invalid strings return `VALIDATION` and leave the previous value unchanged.
- **R2 — Apply on startup.** After the backend is ready, seed `CommandRuntimeState.close_action` from stored Settings so a native close before the UI hydrates uses the saved preference. Settings → System seeds the three-way control from `settings.get` the same way it already seeds launch-at-startup.
- **R3 — System tray.** While the process runs and tray is available, show a tray / status-bar icon using the app icon. Menu: Show PromptHub, Exit. Left-click on Windows/Linux shows and focuses `main`. Exit on the menu terminates the process (`app.exit`), ignoring CloseAction. Tray labels follow `settings.language` for all seven locales and rebuild when language changes.
- **R4 — Same decision for every close.** Custom title-bar close, OS chrome close, and `window.close` all apply the current CloseAction. Hide emits `window:visibility-changed` with `visible: false`.
- **R5 — Resident hide.** `minimize` hides the window without destroying it. The process stays running. Show from tray (or left-click) shows, unminimizes, and focuses `main`.
- **R6 — Real quit.** `exit`, tray Exit, and the ask-dialog Exit terminate the process. Ask-dialog Exit must not go through `window.close` while CloseAction is `ask`. Add `window.quit` that calls `app.exit(0)` and is capability-gated like other `window.*` commands.
- **R7 — Ask dialog.** When CloseAction is `ask`, keep the existing overlay. Buttons: Keep running (dismiss, window stays), Minimize to tray (hide via the same Hide path as `minimize`), Exit (`window.quit`). Do not add a “remember this choice” control; the Settings three-way remains the place to persist.
- **R8 — Single instance.** A second launch of the installed/dev app focuses and shows the existing `main` window instead of starting a second process.
- **R9 — Tray degradation.** If tray setup fails or `probe_capabilities().tray` is false, emit `app:capability-degraded` for tray (Req 23.5) and keep running. In that case `minimize` uses OS minimize rather than `hide()`, so the window remains reachable from the taskbar / dock.
- **R10 — Settings UI.** Keep the existing Settings → System three-way control. Do not duplicate it on General. Do not change the option set.

## Defects folded into requirements

| ID | Severity | Evidence | Owned by |
|----|----------|----------|----------|
| D1 | High | Close action not persisted (`commands/mod.rs:47`) | R1, R2 |
| D2 | High | No tray despite Req 20 tray capability (`Cargo.toml` tauri features; `lib.rs` setup) | R3, R5 |
| D3 | High | Native `CloseRequested` unhandled (`lib.rs` has no `on_window_event`; decorations default on) | R4 |
| D4 | High | `confirmClose` recalls `window.close` under `ask` (`systemStore.ts:304-307`) | R6, R7 |

## Acceptance criteria

- [ ] **AC1.** After choosing Minimize to tray in Settings → System and restarting, clicking the custom close button hides the window; the process is still running; a tray icon is visible.
- [ ] **AC2.** After choosing Exit in Settings → System and restarting, custom close and OS chrome close both terminate the process; the tray icon is gone.
- [ ] **AC3.** After choosing Ask (or leaving the default), close shows the dialog. Keep running leaves the window visible. Minimize to tray hides to tray. Exit terminates. Exit does not reopen the dialog.
- [ ] **AC4.** `settings.get` after `window.setCloseAction("minimize")` returns `closeAction: "minimize"`. An invalid `closeAction` patch returns `VALIDATION` and does not change the stored value.
- [ ] **AC5.** With the window hidden to tray, tray Show and left-click restore and focus the window. Tray Exit quits even if CloseAction is `minimize` or `ask`.
- [ ] **AC6.** Starting a second PromptHub instance while one is running (including while hidden) shows the existing window; a second process does not remain.
- [ ] **AC7.** `window.setCloseAction` with a bad string still returns `VALIDATION` (existing test). New settings tests cover persist/default/reject. Frontend tests cover `confirmClose` → `window.quit`, the three ask-dialog actions, i18n keys in all seven bundles, and seeding `closeAction` from settings.
- [ ] **AC8.** `just fmt-check`, `just clippy`, `just test-rust`, `just build`, and `just test` pass.

## Out of scope

- Startup-minimized (`minimizeOnLaunch` field and UI).
- Removing native window decorations / switching to a frameless-only chrome.
- “Remember my choice” on the close dialog.
- A dedicated global hotkey to show the window (existing shortcut machinery stays as-is).
- Changing autostart wiring (General vs System launch-at-startup duplication).
- Linux AppIndicator packaging beyond best-effort tray setup and R9 degradation.

## Key decisions

| Decision | Choice |
|----------|--------|
| Close-action options | Keep existing three: ask / minimize / exit |
| Default when unset | `ask` (current `CommandRuntimeState` default) |
| Where to configure | Settings → System `WindowBehaviorPanel` only |
| Tray lifetime | Present for the whole process when tray is available |
| Ask dialog | Three actions: keep running, hide to tray, exit; no remember-choice |
| Force quit | New `window.quit` + tray `app.exit(0)` |
| Second launch | Single-instance plugin; show + focus `main` |
| Native decorations | Keep; intercept `CloseRequested` instead of hiding chrome |

## Risks

- Linux desktops without a tray host: mitigated by R9.
- `WebviewWindow::close()` re-entering `CloseRequested`: mitigated by R6 (`app.exit` / do not `prevent_close` on Terminate).
- macOS menu-bar extra vs left-click: Show+Exit remain available on the menu even if left-click opens the menu instead of toggling.

## Technical notes

Details live in `design.md`. Research: `research/tauri-tray.md`.
