# Design: close to tray and persist close action

## Architecture

```
OS chrome close ─┐
Custom title X ──┼─► apply_close(app) ─► CloseAction.decision()
window.close ────┘         │
                           ├ Ask      → prevent_close + emit window:close-requested
                           ├ Minimize → prevent_close + hide (or OS minimize if no tray)
                           └ Exit     → app.exit(0)

Tray Show / left-click → show + unminimize + focus main + visibility-changed(true)
Tray Exit / window.quit → app.exit(0)   // ignores CloseAction

settings JSON closeAction ◄► window.setCloseAction ◄► CommandRuntimeState
                           ▲
                    startup seed after run_startup
```

Window_Manager stays Tauri-free for decisions (`CloseAction`, `parse_close_action`, tray label lookup, degradation). Command_Layer / `lib.rs` owns `TrayIconBuilder`, `on_window_event`, `app.exit`, and the single-instance plugin.

## Persistence contract

Add to `models::Settings` and `src/features/settings/types.ts`:

```text
closeAction?: string | null   // "ask" | "minimize" | "exit"
```

- Serde: `rename_all = "camelCase"`, `skip_serializing_if = "Option::is_none"`.
- `settings::validate` rejects anything `parse_close_action` rejects (`VALIDATION`).
- `settings::defaults` leaves it `None` (runtime treats None as `Ask`).
- `window.setCloseAction` is the UI write path: parse → mutex → `settings::update` patch `{ "closeAction": "..." }`.
- `settings.update` that includes `closeAction` also updates the mutex so a backup restore after restart still matches; no extra frontend dual-write.
- Property round-trip (`storage/proptest_roundtrip.rs`) includes the new optional field.

No SQLite migration. Settings is one JSON document.

## Close application helper

Extract from `commands/window.rs` a helper used by `window.close` and `Builder::on_window_event`:

1. Read `CommandRuntimeState.close_action`.
2. Map through `CloseAction::decision()`.
3. Execute side effects (hide / emit / exit).
4. If tray is unavailable and decision is Hide, call `minimize()` instead of `hide()`.

`on_window_event` for `CloseRequested`:

- Always `api.prevent_close()` first when the decision is Ask or Hide, then apply.
- For Terminate, call `app.exit(0)` (do not rely on letting the window destroy; last-window-close vs hidden windows is easier to reason about with explicit exit).

`window.quit`: `app.exit(0)` after `ensure_ready` is optional — quitting during a failed startup should still work. Prefer allowing quit even when not ready.

Register `window.quit` in `invoke_handler` and in `src/features/system/api.ts`. `confirmClose` and the dialog’s Exit button call it. `CAPABILITY_GATES` already prefixes `window.`, so the new command is gated automatically.

## Tray

Enable `tray-icon` on `tauri` in `src-tauri/Cargo.toml`.

In `setup`, after `run_startup`:

1. If `probe_capabilities().tray` is false, skip and emit degradation.
2. Else build menu + `TrayIconBuilder::with_id("main")` using `default_window_icon()`.
3. On failure, emit degradation and continue (R9).

Show helper (shared by menu, left-click, single-instance callback):

```text
unminimize → show → set_focus → emit visibility-changed(true)
```

Tray strings: a pure `tray_menu_labels(language: &str) -> (show, quit)` in `services/window.rs` covering `en|zh|zh-TW|ja|fr|de|es`, defaulting to English. Rebuild menu from `settings.language` at setup and when `settings.update` changes `language`.

Do not import `@tauri-apps/api` in the frontend for tray.

Check whether `capabilities/default.json` needs `core:tray:default` at build time; add it if the ACL generator requires it.

## Single instance

Add `tauri-plugin-single-instance` version 2 next to the other Tauri 2 plugins in `lib.rs`:

```text
.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
    show_main_window(app);
}))
```

Second launch must restore a hidden window, not only focus a visible one.

## Frontend

- `Settings.closeAction?: string | null` mirrors the backend.
- `SystemPanel` seeds `closeAction` from `settings?.closeAction` via a local setter (same pattern as `setAutoLaunchLocal` / `launchAtStartup`).
- `WindowBehaviorPanel` keeps calling `setCloseAction`; backend persistence makes that enough.
- `CloseDialog`: third button Minimize to tray calling a new store action `hideToTray` → `window.toggleVisibility` only if visible, or a dedicated hide. Prefer invoking `window.close` after temporarily… **No** — that would re-enter Ask. Add `window.hide` or reuse visibility: if visible, `toggleVisibility` hides. `toggleVisibility` already hides and emits. Use that for the dialog’s Minimize to tray.
- `confirmClose` → `api.quitWindow()` (`window.quit`), not `closeWindow`.
- i18n: add `systemView.close.minimize` (and hint/message tweak if needed) to all seven bundles and `i18nKeys.test.ts`. Do not wire the unused `closeDialog.*` leftovers.
- Exhaustive switch already exists for `CloseAction` union via `CLOSE_ACTIONS`; keep it.

## Compatibility

- Existing DBs: missing `closeAction` → `ask`.
- Existing tests for `parse_close_action` stay.
- `window.setCloseAction` wire name unchanged.
- Event names unchanged (`window:close-requested`, `window:visibility-changed`).

## Trade-offs

| Option | Why chosen | Cost |
|--------|------------|------|
| Persist via Settings JSON, not a new table | Matches every other preference; no migration | `window.setCloseAction` needs a DB write |
| Always-on tray while process lives | Restore path is never missing when hidden | Extra icon when CloseAction is `exit` (icon disappears on quit) |
| `app.exit(0)` for Terminate | Avoids CloseRequested re-entry | Slightly different from destroying only the window |
| Single-instance plugin | Required for Windows 常驻 via Start menu | New plugin + capability |
| Keep native decorations | Smaller UI change; intercept is enough | Dual chrome (custom + OS) remains |

## Rollback

Revert the feature branch. Stored `closeAction` in Settings JSON is ignored by older builds (`deny_unknown_fields` is not set on Settings).
