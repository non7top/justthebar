# Implementation Plan: Open-Shell coexistence and system tray

**Spec**: spec.md | **Research**: research.md | **Date**: 2026-11-01

## Summary

Open-Shell's own button hides with the auto-hidden Explorer taskbar (research R1), so the bar draws a Start button that runs `StartMenu.exe -toggle`. The tray is taken over the way Cairo does it: our process owns a hidden topmost `Shell_TrayWnd`, decodes notify-icon packets into a model in `jtb-core`, draws the icons on one bar, and forwards everything else to Explorer's window. Both parts are behind config switches that default to off.

## Technical Context

- **Language**: Rust 1.99.1, `windows` crate 0.62, as in the rest of the workspace.
- **Storage**: none new; the tray holds no state on disk.
- **Testing**: `jtb-core` unit tests (tray model, config validation, Start button layout) in `make build`. Everything that needs Windows is checked by the user from `quickstart.md`.
- **Target**: Windows 10 and 11, Explorer running, Open-Shell installed.
- **Constraints**: restore must work after a crash (constitution II); system-dependent changes logged (V).

## Constitution Check

- I. Logic out of Win32: the icon model, change application and stale-drop live in `jtb-core`. PASS.
- II. Explorer given back: tray ownership is released on exit, panic and `--restore-explorer`; tray defaults off. PASS, with the risk in R3 handled by a forwarding test plan.
- III. Config contract: new sections optional, validated, unknown keys rejected (`contracts/config.md`). PASS.
- IV. Derived code credited: the tray protocol follows Cairo's `TrayService.cs`; the file header and `NOTICE` get the credit when the code lands. PASS (pending).
- V. Small and observable: no abstractions beyond the model; every takeover, release, add, drop and Start click logs. PASS.

## Design

**jtb-core**
- `tray.rs` (new): `TrayKey`, `TrayIcon`, `TrayChange`, `TrayIcons` as in `data-model.md`.
- `config.rs`: `StartConfig`, `TrayConfig`, validation.
- `buttons.rs` / `scene.rs`: the Start button is the first row of a bar (a `ButtonView` with a flag, no title other than the label); a tray strip of fixed-size icon cells sits between the task buttons and the clock, both on horizontal and side bars.

**jtb-win**
- `startmenu.rs` (new): run the configured command; log failures.
- `tray.rs` (new): window class `Shell_TrayWnd`, `WM_COPYDATA` decode, forwarding, `TaskbarCreated` broadcast, release on exit.
- `explorer.rs`: `restore()` also raises Explorer's tray window and broadcasts `TaskbarCreated`.

**jtb-app**: owns the `TrayIcons`, feeds it from the tray window, passes icon rects to the bar for hit testing, sends clicks to the owning window, drops dead owners on a timer.

## Order of work (each its own commit, each passes `make build`)

1. Config: `start` and `tray` sections, validation, example config.
2. Start button: layout, scene, click runs the command. Deploy and check with the user (US1).
3. Tray model in core with tests.
4. Tray window: take the role, decode, forward, log; nothing drawn yet. Deploy with `tray.enabled` true and check in `jtb.log` that packets arrive and that bars still reserve space (the R3 risk).
5. Draw icons and tooltips; click forwarding.
6. Lifecycle: stale icons, Explorer restart, release on exit, `--restore-explorer`.

Steps 4 and 6 are the ones that can break the desktop; step 4 ships with the switch off by default.

## Risks

- Our `Shell_TrayWnd` also receives our own AppBar calls (research R3): bad forwarding breaks the work area. Step 4 verifies this before anything is drawn.
- Where the Open-Shell menu opens with the Explorer taskbar hidden is unknown (R2).
- System icons are not promised (R5).
- Nothing here can run from WSL; the user tests on Windows.
