# Implement: close to tray and persist close action

Ordered so each step is testable before the next. Do not start from `lib.rs` tray wiring until persistence and the shared close helper exist.

## Checklist

1. **Settings field**
   - Add `close_action: Option<String>` to `models/settings.rs` and `src/features/settings/types.ts`.
   - Validate with `parse_close_action` in `settings::validate`.
   - Tests: persist `minimize`; default `None`; reject `"quit"` with `VALIDATION`; property round-trip includes the field.
   - Gate while iterating: `cargo test settings --manifest-path src-tauri/Cargo.toml`

2. **Runtime seed + `window.setCloseAction` write-through**
   - After `run_startup` succeeds, read settings and set `CommandRuntimeState.close_action`.
   - `window.setCloseAction` updates mutex and `settings::update`.
   - `settings.update` that patches `closeAction` also updates the mutex.
   - Gate: `cargo test parse_close_action --manifest-path src-tauri/Cargo.toml`

3. **`window.quit` + shared `apply_close`**
   - Helper used by `window.close` and later by `CloseRequested`.
   - `window.quit` → `app.exit(0)`. Register in `lib.rs` `invoke_handler`.
   - Hide path: tray available → `hide()`; else OS `minimize()`.
   - Terminate path: `app.exit(0)`, not `WebviewWindow::close()`.
   - Gate: existing window service unit tests plus any new helper tests.

4. **Tray + CloseRequested + single instance**
   - `tauri` feature `tray-icon`; plugin `tauri-plugin-single-instance` v2.
   - Tray builder in `setup`; Show / Exit; left-click restore.
   - `tray_menu_labels(language)` unit-tested for the seven locales + unknown → en.
   - `Builder::on_window_event` for `CloseRequested` calls `apply_close`.
   - Single-instance callback calls the same show helper.
   - ACL: add tray permission if the build complains.
   - Rebuild tray menu when `settings.update` changes `language`.
   - Gate: `just fmt-check` and `just clippy` (tray code is hard to unit-test).

5. **Frontend store + dialog + seed**
   - `SystemApi.quitWindow` → `window.quit`.
   - `confirmClose` uses quit. Add `hideToTray` using `toggleVisibility` when visible.
   - `CloseDialog` third button; update copy via `systemView.close.*`.
   - `SystemPanel` seeds `closeAction` from settings (`setCloseActionLocal`).
   - Tests: `systemStore.test.ts`, new/updated CloseDialog test, `i18nKeys.test.ts`, all seven locales.
   - Gate: `npx vitest run src/features/system`

6. **Quality gate**
   - `just fmt-check`
   - `just clippy`
   - `just test-rust`
   - `just build`
   - `just test`

## Risky files

| File | Why |
|------|-----|
| `src-tauri/src/lib.rs` | setup, plugins, window events; a mistake here prevents launch |
| `src-tauri/Cargo.toml` | new feature/plugin; lockfile churn |
| `src-tauri/capabilities/default.json` | ACL can fail the Tauri build |
| `src-tauri/src/commands/window.rs` | close/hide/exit behavior |
| `src/features/system/systemStore.ts` | confirmClose currently wrong; easy to re-break |

## Rollback points

- After step 1–2: settings field only; behavior unchanged except persistence if UI already called `setCloseAction`.
- After step 3: quit command exists but unused by UI.
- After step 4: native close and tray go live; this is the first user-visible behavior change.
- After step 5: dialog and seeding match the backend.

If tray setup panics, that is a launch blocker — must be `if let Err` + degradation, never `unwrap` in `setup`.

## Follow-up before `task.py start`

- [x] `prd.md` has observable acceptance criteria.
- [x] `design.md` names contracts (`closeAction`, `window.quit`, events).
- [x] `implement.jsonl` / `check.jsonl` have real spec + research entries.
- [ ] User approved this planning summary (required; do not start until then).
