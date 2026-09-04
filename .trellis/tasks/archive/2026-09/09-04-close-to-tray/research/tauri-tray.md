# Research: Tauri 2 tray, close-requested, and single instance

Date: 2026-09-04

## Tray

Tauri 2 replaced the v1 `SystemTray` API with `tauri::tray::TrayIconBuilder`.
It requires the `tray-icon` feature on the `tauri` crate:

```toml
tauri = { version = "2", features = ["tray-icon"] }
```

The app already ships PNG/ICO/ICNS icons in `src-tauri/icons/`.
`app.default_window_icon()` is enough for the tray image; no extra
`image-png` feature is required if we clone that icon.

Menu items are `tauri::menu::MenuItem` / `MenuBuilder`. Left-click handling
is `on_tray_icon_event` matching `TrayIconEvent::Click` with
`MouseButton::Left` + `MouseButtonState::Up`. Quit must call `app.exit(0)`,
not `window.close()`, so it bypasses the close-action decision.

If tray construction fails, emit the existing non-fatal
`app:capability-degraded` channel for `PlatformFeature::Tray` (Req 23.5)
and keep running.

ACL: `capabilities/default.json` currently has `core:default` and
`updater:default`. Tray setup from Rust does not need a frontend invoke, but
confirm at build time whether `core:tray:default` must be added.

## CloseRequested vs `window.close`

`WebviewWindow::close()` emits `WindowEvent::CloseRequested`. An
`on_window_event` handler that always `prevent_close()`s will also block the
Terminate path of `window.close` and the current `confirmClose()` flow.

Shared decision (already pure in `services/window.rs`):

| CloseAction | CloseDecision        | Runtime effect                                      |
|-------------|----------------------|-----------------------------------------------------|
| `ask`       | EmitCloseRequested   | `prevent_close`; emit `window:close-requested`      |
| `minimize`  | Hide                 | `prevent_close`; `hide()`; emit visibility-changed  |
| `exit`      | Terminate            | do not prevent; or `app.exit(0)`                    |

Force-quit (tray Quit, ask-dialog Exit) must not read CloseAction.
Add `window.quit` that calls `app.exit(0)`.

Native title-bar close currently bypasses `window.close` because
`tauri.conf.json` still has default decorations and there is no
`on_window_event` handler (`src-tauri/src/lib.rs`). Both the custom
title-bar button and the OS chrome must call the same helper.

## Persistence

`CloseAction` lives only in `CommandRuntimeState.close_action`
(`commands/mod.rs`), default `Ask`. Settings JSON already has optional
`minimizeOnLaunch` / `launchAtStartup` and round-trips unknown-absent
fields. Add optional `closeAction` (`ask` \| `minimize` \| `exit`) and
validate with `parse_close_action`. No schema migration: settings is one
JSON document in the `settings` table.

## Single instance

While hidden, a second Start-menu launch would otherwise start a new
process. `tauri-plugin-single-instance` version 2 matches the current
plugin line. On a second launch, show + focus `main`. Without this,
close-to-tray is broken on Windows for users who re-click the app icon.

## i18n

Tray labels are native, created in Rust before the webview. A 7-locale
static table keyed by `settings.language` (same codes as the frontend)
covers Show / Exit. Rebuild the menu when language is updated.

## Degradation

If tray is unavailable, `CloseAction::Minimize` must not `hide()` (the
window would vanish with no restore path). Fall back to OS minimize so
the window stays on the taskbar / dock.
