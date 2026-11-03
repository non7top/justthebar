# justthebar Constitution

## Core Principles

### I. Logic stays out of Win32
`jtb-core` holds every decision (task model, grouping, layout, config, scene) and never imports `windows`. `jtb-win` talks to the system, `jtb-ui` draws a `Scene`, `jtb-app` wires them. New behavior is written and unit-tested in `jtb-core` first.

### II. Explorer is always given back
The Explorer taskbar is hidden only while the app runs, and restored on exit, on panic, and by `--restore-explorer`. Nothing may make that harder. Placement and restore changes are logged.

### III. Config is a contract
`jtb.json` is versioned, every key has a default, typos are rejected, and a broken edit keeps the last good config. Features that are not built yet may reserve keys, but a reserved key never changes meaning later.

### IV. Derived code is credited
Logic ported from Cairo Desktop's ManagedShell (Apache-2.0) is marked in the file header and listed in `NOTICE`. The project is Apache-2.0.

### V. Small and observable
No feature flags or abstractions for cases that do not exist yet. Behavior that depends on the live system (shell hook, z-order, work area) writes a line to `jtb.log` when it changes.

## Constraints

- Windows 10 and 11 desktop, Explorer keeps running, Open-Shell owns Start. Tray and shell mode come later.
- Native Win32 and Direct2D behind the `Renderer` trait.
- Per-monitor v2 DPI: scene coordinates are physical pixels, sizes in config are at 96 dpi.
- Builds and checks run in disposable containers (`make build` runs the CI workflow with `act`: fmt check, clippy with warnings denied, `jtb-core` tests, release cross-build).

## Workflow

- Commits are atomic: one change, only its files, staged by path, semantic prefix (`feat:`, `fix:`, `chore:`, `docs:`).
- Lint and the CI workflow pass before a change is committed or deployed.
- The exe is deployed to `D:\tmp\jtb` for manual runs; WSL cannot launch Windows programs, so Windows behavior is verified by the user and reported back.
- Features larger than one commit get a spec, plan and task list under `specs/`.

## Governance

This constitution wins over habit. Changes to it are their own commit with the reason in the message.

**Version**: 1.0.0 | **Ratified**: 2026-10-31 | **Last Amended**: 2026-10-31
