# Native settings smoke record

## Supplied release baseline

- Screenshot: `C:\Users\lyh\AppData\Local\PixPin\Temp\PixPin_2026-07-14_19-51-19.png`
- Captured: 2026-07-14 19:51:19 +08:00
- Observed behavior: the language select displays Simplified Chinese while the
  shell and settings labels remain English; the settings rail has no Appearance
  entry and shows the older General theme controls.
- Existing release executable:
  `D:\Documents\Code\Rust\Exp\PromptHub\src-tauri\target\release\prompthub.exe`
- Existing executable timestamp: 2026-06-01 00:28:08 +08:00
- Existing executable size: 9,086,976 bytes
- Existing executable SHA-256:
  `413ac7069350829ac0ba382fef53d5b300eb05bc4c13b33d4b9b9f37cb1bb8b2`

The executable predates the screenshot by more than six weeks and predates the
current source Appearance section. This supports, but does not by itself prove,
the stale-package diagnosis.

## Fresh package verification

Built successfully with `just tauri-build` on 2026-07-14.

- Release executable:
  `D:\Documents\Code\Rust\Exp\PromptHub\src-tauri\target\release\prompthub.exe`
- Timestamp: 2026-07-14 22:03:25 +08:00
- Size: 9,110,528 bytes
- SHA-256:
  `59169439918b73186a62f9f781c73cccb5e6557440141e84e8fd37ae542c17a8`
- NSIS:
  `D:\Documents\Code\Rust\Exp\PromptHub\src-tauri\target\release\bundle\nsis\PromptHub_0.1.0_x64-setup.exe`
- MSI:
  `D:\Documents\Code\Rust\Exp\PromptHub\src-tauri\target\release\bundle\msi\PromptHub_0.1.0_x64_en-US.msi`

### Launch-path diagnosis

Launching by registered application id opened the installed stale executable at
`C:\Users\lyh\AppData\Local\PromptHub\prompthub.exe`, reproducing the old
General-only UI. After closing it, the release executable was launched directly;
the running process path was verified as the repository `target\release` path
above before interaction. This confirms the supplied screen came from stale
packaging rather than the current source.

### Native checks

- Persisted `zh` rendered General and the settings shell in Simplified Chinese
  on first paint. Switching `zh -> en -> zh` immediately rerendered the header,
  settings rail, active panel, and save status without restart.
- Catppuccin and Claude family selection worked independently from
  light/dark/system. Claude light visibly repainted the complete shell; returning
  Catppuccin to system followed the host dark preference.
- Added `Inter` after `System`; both rows rendered with reorder/remove controls
  and the update reached the visible saved state.
- After closing and directly relaunching the same release executable, `zh`,
  Claude light, and `System -> Inter` all restored. The test preferences were
  then returned to Catppuccin, system mode, and a single System family.
- Window captures were inspected at 1200x830 and at the minimum-size boundary
  (802x631 including the Windows frame, corresponding to the configured
  800x600 client minimum). At minimum size the secondary settings rail collapsed
  to icons, family copy wrapped, accent swatches wrapped, font rows stayed
  aligned, and scale/density/specimen remained reachable by vertical scrolling.
- English and Simplified Chinese captures, Catppuccin system dark, and Claude
  light were visually inspected. No blank canvas, overlap, clipped control, or
  unreadable primary action was observed.
- After the final hardening build, the repository release executable was
  launched directly without changing preferences. Process path, responsive
  state, and the `PromptHub` main-window title were verified before closing the
  exact process by id.
