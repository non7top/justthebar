# Implementation Plan: Task grouping, pins and drag and drop

**Spec**: spec.md

## Design

**jtb-core** (pure, unit-tested)
- `tasks.rs`: `Task` gains `exe`; `Tasks::set_exe`.
- `grouping.rs`: `Rules` (custom groups, `key_for(exe, id)`), `build_rows(tasks, pins, rules, folded, order, mode) -> Vec<Row>`. `Row` is `Window { id, child }` or `Header { key, name, pin, windows, folded }`.
- `mouse.rs`: `Action`, `Target`, `Button`, `ClickMap` with defaults and config overrides.
- `state.rs`: `State { folded, order, pins }` read and written as JSON.
- `config.rs`: `grouping.mode` (`off` | `accordion`), `mouse` section, validation of `pins` and `groups`.
- `buttons.rs` / `scene.rs`: `ButtonView` gains `kind` (window, child, header), indent for children, fold marker in header titles.

**jtb-win**: `process_path(id)`, `launch(path, args)`, `exe_icon(path)`, `resolve_shortcut(path)`, popup menu.

**jtb-app**: builds rows from live tasks, maps each row to a `ButtonView`, dispatches clicks through `ClickMap`, handles middle click, drag, file drops and the header context menu, loads and saves `jtb.state.json`.

## Order of work

1. Grouping model in core (no UI).
2. Wire accordion rendering; same-app groups; header launch; middle click fold.
3. Custom groups.
4. Pins from config.
5. Mixed-exe groups claimed by pins.
6. Drag to reorder, drop to pin, unpin menu, state file.

Each step is its own commit and passes `make build` (fmt, clippy, `jtb-core` tests, release build).

## Risks

- Windows behavior (launch, drop, menu) cannot be run from WSL; the user tests at the end and reports back.
- Process path lookup can fail for elevated processes; those windows stay ungrouped.
