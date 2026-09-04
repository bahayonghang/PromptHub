# Window Close Action and System Tray

## Scenario: Persist close-to-tray and keep one hide/quit path

### 1. Scope / Trigger

Use this contract when changing window close behavior, system-tray residency,
single-instance restore, or the Settings `closeAction` field. The flow crosses
Settings JSON, `CommandRuntimeState`, Tauri window/tray glue, Runtime Bridge
`window.*` commands, and `systemStore`.

### 2. Signatures

```text
window.close() -> ()
window.hide() -> ()
window.quit() -> ()
window.setCloseAction({ action: CloseAction }) -> ()
settings.get() -> Settings          # includes closeAction?: "ask"|"minimize"|"exit"|null
settings.update({ closeAction? }) -> Settings
```

`CloseAction` on the wire is `ask` | `minimize` | `exit`. Absent `closeAction`
means `ask`.

Events:

```text
window:close-requested -> {}
window:visibility-changed -> { visible: boolean }
app:capability-degraded -> { feature: "tray", ... }
```

Tray is created in `lib.rs` setup (`TrayIconBuilder` id `main`). It is not a
frontend invoke. Single-instance uses `tauri-plugin-single-instance`; the
second-launch callback must call the same show helper as tray Show.

### 3. Contracts

- `window.setCloseAction` writes Settings JSON and the runtime mutex in one
  command. `settings.update` that patches `closeAction` also updates the mutex.
- After `run_startup`, seed the mutex from stored Settings so OS chrome close
  works before the UI hydrates.
- `window.close` and native `CloseRequested` share `apply_close`:
  - `ask` → prevent close, emit `window:close-requested`
  - `minimize` → prevent close, then the Hide path
  - `exit` → `app.exit(0)` (do not `WebviewWindow::close()`)
- Hide path (`hide_or_minimize` / `window.hide`): if the tray icon exists,
  `hide()`; otherwise OS `minimize()` so the window stays on the taskbar/dock.
- `window.quit` and tray Exit call `app.exit(0)` and ignore CloseAction.
  `window.quit` must work even when the backend is not ready.
- Ask-dialog Exit uses `window.quit`, never `window.close` (that would re-emit
  `window:close-requested`). Ask-dialog Minimize to tray uses `window.hide`,
  not `window.toggleVisibility`.
- Tray menu labels follow `settings.language` (seven locales; unknown → `en`)
  and rebuild when language is updated.
- Frontend seeds the Settings → System three-way control from
  `settings.closeAction` via `setCloseActionLocal`. Do not duplicate the
  control on General.

### 4. Validation & Error Matrix

| Condition | Result |
|-----------|--------|
| `closeAction` is `ask` / `minimize` / `exit` (any case, trimmed) | Accepted |
| `closeAction` any other string (e.g. `quit`) | `VALIDATION`; previous value unchanged |
| Tray setup fails or `probe_capabilities().tray` is false | Emit `app:capability-degraded` for tray; keep running; Hide uses OS minimize |
| `window.*` while `desktopWindowControls` is false | Frontend `CAPABILITY_UNAVAILABLE` (existing gate) |

### 5. Good / Base / Bad Cases

- **Good**: Stored `minimize`; restart; custom or OS close hides to tray; tray Show restores; tray Exit quits.
- **Base**: No `closeAction` field; runtime is `ask`; close shows the three-button dialog.
- **Bad**: Ask-dialog Exit calls `window.close` → dialog reopens. Ask-dialog hide calls `toggleVisibility` when tray failed → window vanishes with no restore path.

### 6. Tests Required

- Settings: persist `minimize`; default `None`; reject `quit` with `VALIDATION`.
- `parse_close_action` and `tray_menu_labels` (seven locales + unknown → en).
- Frontend: `confirmClose` → `window.quit` not `window.close`; `hideToTray` → `window.hide`; CloseDialog three actions; i18n keys in all seven bundles; `setCloseActionLocal` seed.
- Property round-trip includes optional `closeAction`.

Live tray / OS chrome / second-instance restore are not covered by unit tests;
verify with `just dev`.

### 7. Wrong vs Correct

#### Wrong

```ts
confirmClose: async () => {
  set({ closeDialogOpen: false });
  await get().close(); // window.close while CloseAction is still "ask"
};
hideToTray: async () => {
  await api.toggleVisibility(); // always hide(), skips tray-failed OS minimize
};
```

#### Correct

```ts
confirmClose: async () => {
  set({ closeDialogOpen: false });
  await api.quitWindow(); // window.quit → app.exit(0)
};
hideToTray: async () => {
  set({ closeDialogOpen: false });
  await api.hideWindow(); // window.hide → hide_or_minimize
};
```

Terminate in Rust must `app.exit(0)`, not `WebviewWindow::close()`, so
`CloseRequested` + `prevent_close` cannot block quit.
